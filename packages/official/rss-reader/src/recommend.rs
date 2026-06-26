//! OneReason 风格的推理推荐(reasoning recommendation)。
//!
//! 复用 mrrss 省 token 漏斗,把 OneReason 的"显式 in-text reasoning"钉在精排层:
//!   L1 本地 TF-IDF 兴趣画像   (0 token,纯 Rust:中文 bigram + 拉丁整词)
//!   L2 本地多因子初筛         (0 token:relevance/recency/popularity)
//!   L3 单次 LLM 推理精排       (1 次 chat_completion:输出匹配关键词/理由/排斥信号/分数)
//!   L4 指纹缓存               (命中 0 token)
//!
//! grounding 三道闸防 ungrounded drift:
//!   1) 占位 id → 真实 article_id 映射,映射不上丢弃(杜绝编造文章)
//!   2) matched_keywords 必须在画像 Top40 白名单内
//!   3) 每个保留的关键词必须能在该候选 title+snippet 里 substring 命中
//!
//! 冷启动门控:已读来源 < 阈值 或画像为空 → 跳过 L3,直接返回 L2 排序(不为空画像付费)。
//! 缓存失效靠 fingerprint(画像变→指纹变→自然失效),无需 rev 脏检查,故不侵入抓取/已读路径。

use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use weft_package_sdk::*;

use crate::db_path;

const PROFILE_SOURCE_LIMIT: i64 = 200; // 最近 N 篇已读作画像来源
const TOP_KEYWORDS: usize = 40;
const CANDIDATE_POOL: usize = 18; // 送入 L3 的候选上限(并集去重后)
const RELEVANCE_QUOTA: usize = 12; // 并集召回:词面相关性名额
const RECENCY_QUOTA: usize = 8; // 并集召回:最新文章名额(语义兜底通道)
const RECENCY_PER_FEED: usize = 3; // recency 通道每个 feed 最多占几席(防高产源独占)
const COLD_START_MIN_SOURCES: i64 = 15; // 低于此画像信号不足,跳过 LLM 精排
const SNIPPET_CHARS: usize = 160;

// ── LLM 精排输出 ──

#[derive(Deserialize)]
struct LlmRanked {
    #[serde(default)]
    ranked: Vec<LlmItem>,
}

#[derive(Deserialize)]
struct LlmItem {
    #[serde(default)]
    id: String,
    #[serde(default)]
    matched_keywords: Vec<String>,
    #[serde(default)]
    reasoning: String,
    #[serde(default)]
    aversion: String,
    #[serde(default)]
    score: f64,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    rationale_zh: String,
}

// ── 画像 / 候选 ──

struct Profile {
    keywords: Vec<(String, f64)>, // Top40 (term, weight),降序
    source_count: i64,
}

struct Candidate {
    id: i64,
    feed_id: i64,
    title: String,
    link: String,
    snippet: String,
    score: f64,     // L2 综合分
    relevance: f64, // 词面相关性(用于并集召回分流)
    recency: f64,   // 新鲜度(用于 recency 通道排序)
}

// ── 文本处理 ──

fn is_cjk(c: char) -> bool {
    matches!(c, '\u{4e00}'..='\u{9fff}')
}

/// 去 HTML 标签 + 解码常见实体 + 压缩空白。纯 Rust,无依赖。
fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn is_stopword(w: &str) -> bool {
    const SW: &[&str] = &[
        // 英文虚词
        "the", "a", "an", "of", "to", "and", "is", "in", "for", "on", "with", "at", "by", "from",
        "as", "it", "this", "that", "be", "are", "was", "were", "or", "not", "but", "if", "then",
        "so", "we", "you", "they", "he", "she", "his", "her", "its", "our", "your", "their", "i",
        "me", "my", "do", "does", "did", "has", "have", "had", "will", "would", "can", "could",
        // 噪音 / URL 碎片
        "http", "https", "www", "com", "html", "org", "net", "amp", "quot", "nbsp", "div", "span",
    ];
    SW.contains(&w)
}

/// 中文按字符 bigram(连续 CJK 滑窗取 2 字),拉丁/数字按整词(lowercase,len≥2)。
/// 无词典、无新 crate、wasm 体积零增长。
fn tokenize(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tokens: Vec<String> = Vec::new();
    let mut latin = String::new();
    let mut cjk_run: Vec<char> = Vec::new();

    for c in lower.chars() {
        if is_cjk(c) {
            flush_latin(&mut latin, &mut tokens);
            cjk_run.push(c);
        } else if c.is_alphanumeric() {
            // 拉丁字母/数字(CJK 已在上一分支拦截)
            flush_cjk(&mut cjk_run, &mut tokens);
            latin.push(c);
        } else {
            // 分隔符
            flush_latin(&mut latin, &mut tokens);
            flush_cjk(&mut cjk_run, &mut tokens);
        }
    }
    flush_latin(&mut latin, &mut tokens);
    flush_cjk(&mut cjk_run, &mut tokens);
    tokens
}

fn flush_latin(latin: &mut String, tokens: &mut Vec<String>) {
    if latin.chars().count() >= 2 && !is_stopword(latin) {
        tokens.push(std::mem::take(latin));
    } else {
        latin.clear();
    }
}

fn flush_cjk(run: &mut Vec<char>, tokens: &mut Vec<String>) {
    if run.len() >= 2 {
        for w in run.windows(2) {
            tokens.push(w.iter().collect());
        }
    }
    run.clear();
}

/// FNV-1a 64-bit,用于稳定指纹(同输入同值)。
fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

// ── L1 兴趣画像 ──

fn build_profile(path: &str) -> Profile {
    // 画像来源 = 已读 OR 收藏。收藏是比"读过"更强的偏好信号(读了可能觉得烂),
    // 故收藏文章在词频上额外加权(FAVORITE_BOOST)。
    let rows = match sqlite_query(
        path,
        "SELECT title, content, is_favorite FROM articles \
         WHERE is_read = 1 OR is_favorite = 1 ORDER BY fetched_at DESC LIMIT ?",
        &[json!(PROFILE_SOURCE_LIMIT)],
    ) {
        Ok(r) => r.rows,
        Err(_) => Vec::new(),
    };

    let source_count = rows.len() as i64;
    let n_docs = rows.len().max(1) as f64;

    const FAVORITE_BOOST: f64 = 2.5; // 收藏文章的词频权重倍数
    let mut tf: HashMap<String, f64> = HashMap::new();
    let mut df: HashMap<String, f64> = HashMap::new();

    for row in &rows {
        let title = row.first().and_then(|v| v.as_str()).unwrap_or("");
        let content = row.get(1).and_then(|v| v.as_str()).unwrap_or("");
        let is_fav = row.get(2).and_then(|v| v.as_i64()).unwrap_or(0) != 0;
        let weight = if is_fav { FAVORITE_BOOST } else { 1.0 };

        // title 加权 ×3,正文 ×1,收藏文章整体再 ×FAVORITE_BOOST
        let title_tokens = tokenize(title);
        let mut doc: Vec<String> = Vec::new();
        for _ in 0..3 {
            doc.extend(title_tokens.iter().cloned());
        }
        doc.extend(tokenize(&strip_html(content)));

        let mut seen: HashSet<String> = HashSet::new();
        for tk in &doc {
            *tf.entry(tk.clone()).or_insert(0.0) += weight;
            if seen.insert(tk.clone()) {
                *df.entry(tk.clone()).or_insert(0.0) += 1.0;
            }
        }
    }

    let mut scored: Vec<(String, f64)> = tf
        .iter()
        .map(|(t, &f)| {
            let d = df.get(t).copied().unwrap_or(1.0);
            (t.clone(), f * (1.0 + n_docs / d).ln())
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(TOP_KEYWORDS);

    Profile {
        keywords: scored,
        source_count,
    }
}

fn profile_fingerprint(p: &Profile) -> String {
    let mut terms: Vec<&str> = p.keywords.iter().map(|(t, _)| t.as_str()).collect();
    terms.sort_unstable();
    let bucket = p.source_count / 10;
    format!("{:016x}", fnv1a(&format!("{}|{}", terms.join(","), bucket)))
}

// ── L2 本地初筛 ──

fn prefilter(path: &str, profile: &Profile, feed_id: Option<i64>) -> Vec<Candidate> {
    let kw: HashMap<String, f64> = profile
        .keywords
        .iter()
        .map(|(t, w)| (t.clone(), *w))
        .collect();
    let max_w = profile
        .keywords
        .first()
        .map(|(_, w)| *w)
        .unwrap_or(1.0)
        .max(1e-9);

    let (sql, params): (&str, Vec<Value>) = match feed_id {
        Some(fid) => (
            "SELECT id, title, content, fetched_at, link, feed_id FROM articles WHERE is_read = 0 AND feed_id = ?",
            vec![json!(fid)],
        ),
        None => (
            "SELECT id, title, content, fetched_at, link, feed_id FROM articles WHERE is_read = 0",
            vec![],
        ),
    };

    let rows = match sqlite_query(path, sql, &params) {
        Ok(r) => r.rows,
        Err(_) => Vec::new(),
    };

    let now = now_ms() as f64;
    let mut scored: Vec<Candidate> = Vec::new();

    for row in &rows {
        let id = row.first().and_then(|v| v.as_i64()).unwrap_or(0);
        let title = row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let content = row.get(2).and_then(|v| v.as_str()).unwrap_or("");
        let fetched = row.get(3).and_then(|v| v.as_i64()).unwrap_or(0) as f64;
        let link = row.get(4).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let cand_feed_id = row.get(5).and_then(|v| v.as_i64()).unwrap_or(0);

        let snippet = truncate_chars(&strip_html(content), SNIPPET_CHARS);

        // relevance:候选 token 命中画像关键词的加权和,按词数归一
        let toks = tokenize(&format!("{} {}", title, snippet));
        let ntok = (toks.len().max(1) as f64).sqrt();
        let mut rel = 0.0;
        for t in &toks {
            if let Some(w) = kw.get(t) {
                rel += w;
            }
        }
        rel = (rel / max_w) / ntok;

        // recency:用 fetched_at(整数 ms,可靠;published_at 字符串格式杂,wasm 无 chrono 不解析)
        let age_days = ((now - fetched).max(0.0)) / 86_400_000.0;
        let recency = 1.0 / (1.0 + age_days);

        // popularity:无真实点击,用正文长度作弱代理。对数压缩,避免 arXiv 长论文恒为满分。
        let pop = (1.0 + content.chars().count() as f64).ln() / 10.0_f64.ln();
        let pop = pop.min(1.0);

        // 权重:relevance 仍主导;pop 降到 0.05(长度偏置弱),让出的 0.05 给 recency。
        let combined = 0.6 * rel + 0.35 * recency + 0.05 * pop;
        scored.push(Candidate {
            id,
            feed_id: cand_feed_id,
            title,
            link,
            snippet,
            score: combined,
            relevance: rel,
            recency,
        });
    }

    select_union(scored)
}

/// 并集召回:词面相关性 Top-N ∪ 最新 Top-M(按 feed 去重),给 L3 语义兜底通道。
/// 让词面打 0 分但新鲜的文章也能进 L3,由 LLM 语义判别救回。纯 Rust,0 token。
fn select_union(mut scored: Vec<Candidate>) -> Vec<Candidate> {
    // 通道一:综合分(以 relevance 为主)Top-N
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let mut chosen: Vec<Candidate> = Vec::new();
    let mut taken: HashSet<i64> = HashSet::new();
    for c in scored.iter().take(RELEVANCE_QUOTA) {
        if taken.insert(c.id) {
            chosen.push(clone_candidate(c));
        }
    }

    // 通道二:最新 Top-M,按 feed 去重(每 feed 最多 RECENCY_PER_FEED 席),防高产源独占
    let mut by_recency: Vec<&Candidate> = scored.iter().collect();
    by_recency.sort_by(|a, b| {
        b.recency
            .partial_cmp(&a.recency)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut per_feed: HashMap<i64, usize> = HashMap::new();
    let mut recency_filled = 0usize;
    for c in by_recency {
        if recency_filled >= RECENCY_QUOTA || chosen.len() >= CANDIDATE_POOL {
            break;
        }
        if taken.contains(&c.id) {
            continue;
        }
        let n = per_feed.entry(c.feed_id).or_insert(0);
        if *n >= RECENCY_PER_FEED {
            continue;
        }
        *n += 1;
        taken.insert(c.id);
        chosen.push(clone_candidate(c));
        recency_filled += 1;
    }

    chosen.truncate(CANDIDATE_POOL);
    chosen
}

fn clone_candidate(c: &Candidate) -> Candidate {
    Candidate {
        id: c.id,
        feed_id: c.feed_id,
        title: c.title.clone(),
        link: c.link.clone(),
        snippet: c.snippet.clone(),
        score: c.score,
        relevance: c.relevance,
        recency: c.recency,
    }
}

fn candidate_hash(cands: &[Candidate]) -> String {
    let mut ids: Vec<i64> = cands.iter().map(|c| c.id).collect();
    ids.sort_unstable();
    let joined: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
    format!("{:016x}", fnv1a(&joined.join(",")))
}

// ── L3 LLM 推理精排 + grounding ──

/// 剥离 ```json 围栏,取首个 { 到末个 } 子串。
fn extract_json(s: &str) -> String {
    let mut t = s.trim();
    if let Some(rest) = t.strip_prefix("```json") {
        t = rest;
    } else if let Some(rest) = t.strip_prefix("```") {
        t = rest;
    }
    if let Some(rest) = t.strip_suffix("```") {
        t = rest;
    }
    let t = t.trim();
    if let (Some(start), Some(end)) = (t.find('{'), t.rfind('}')) {
        if end >= start {
            return t[start..=end].to_string();
        }
    }
    t.to_string()
}

/// 返回 (grounded 推荐 JSON 列表, 是否真正调用了 LLM)。
fn rerank(profile: &Profile, candidates: &[Candidate]) -> Result<Vec<Value>, String> {
    let kw_list: Vec<&str> = profile.keywords.iter().map(|(t, _)| t.as_str()).collect();
    let kw_set: HashSet<String> = profile.keywords.iter().map(|(t, _)| t.clone()).collect();

    let mut id_map: HashMap<String, &Candidate> = HashMap::new();
    let mut cand_block = String::new();
    for (i, c) in candidates.iter().enumerate() {
        let pid = format!("A{}", i);
        cand_block.push_str(&format!("[{}] {}\n摘要: {}\n\n", pid, c.title, c.snippet));
        id_map.insert(pid, c);
    }

    let system = "你是个性化推荐精排器。只能基于给定的『用户兴趣关键词』和每条候选的『标题/摘要』推理,严禁引入外部知识或虚构候选未提及的内容。对每条候选输出:matched_keywords(必须逐字取自给定兴趣关键词,≤5个)、reasoning(一句话)、aversion(排斥/降权信号,可空)、score(0-100整数)、confidence(0到1小数)、rationale_zh(给用户看的中文理由,≤40字)。最后按 score 从高到低排序。严格只输出 JSON,形如 {\"ranked\":[{\"id\":\"A0\",\"matched_keywords\":[],\"reasoning\":\"\",\"aversion\":\"\",\"score\":0,\"confidence\":0,\"rationale_zh\":\"\"}]},不要 markdown 不要解释。";
    let user = format!(
        "用户兴趣关键词:\n{}\n\n候选文章:\n{}",
        kw_list.join("、"),
        cand_block
    );

    let content = crate::ai_chat(system, &user, "rss:recommend")?;
    if content.trim().is_empty() {
        return Err("chat response missing content".into());
    }

    let cleaned = extract_json(&content);
    let ranked: LlmRanked = serde_json::from_str(&cleaned)
        .map_err(|e| format!("parse ranked json failed: {}", e))?;

    let mut out: Vec<Value> = Vec::new();
    for item in &ranked.ranked {
        // 道闸1:占位 id 映射回真实文章,映射不上丢弃
        let cand = match id_map.get(&item.id) {
            Some(c) => *c,
            None => continue,
        };
        // 分数门控
        if item.score < 40.0 {
            continue;
        }
        let hay = format!("{} {}", cand.title, cand.snippet).to_lowercase();
        let mut grounded_kw: Vec<String> = Vec::new();
        for k in &item.matched_keywords {
            // 道闸2:白名单
            if !kw_set.contains(k) {
                continue;
            }
            // 道闸3:per-article substring 命中
            if hay.contains(k.to_lowercase().as_str()) {
                grounded_kw.push(k.clone());
            }
        }
        let grounded = !grounded_kw.is_empty();
        out.push(json!({
            "article_id": cand.id,
            "title": cand.title,
            "link": cand.link,
            "score": item.score,
            "confidence": item.confidence,
            "matched_keywords": grounded_kw,
            "reasoning": item.reasoning,
            "aversion": item.aversion,
            "rationale_zh": item.rationale_zh,
            "grounded": grounded,
        }));
    }
    Ok(out)
}

// ── L4 缓存 ──

fn read_cache(path: &str, fp: &str, ch: &str) -> Option<Vec<Value>> {
    let res = sqlite_query(
        path,
        "SELECT result FROM recommendations WHERE profile_fingerprint = ? AND candidate_hash = ?",
        &[json!(fp), json!(ch)],
    )
    .ok()?;
    let s = res.rows.first()?.first()?.as_str()?;
    match serde_json::from_str::<Value>(s).ok()? {
        Value::Array(a) => Some(a),
        _ => None,
    }
}

fn write_cache(path: &str, fp: &str, ch: &str, result: &[Value]) {
    let now = now_ms() as i64;
    let s = Value::Array(result.to_vec()).to_string();
    let _ = sqlite_execute(
        path,
        "INSERT OR REPLACE INTO recommendations \
            (profile_fingerprint, candidate_hash, result, token_used, built_at) \
            VALUES (?, ?, ?, 1, ?)",
        &[json!(fp), json!(ch), json!(s), json!(now)],
    );
}

// ── L2 兜底转推荐 ──

fn l2_to_recs(cands: &[Candidate], limit: usize) -> Vec<Value> {
    cands
        .iter()
        .take(limit)
        .map(|c| {
            json!({
                "article_id": c.id,
                "title": c.title,
                "link": c.link,
                "score": (c.score * 100.0).round(),
                "confidence": Value::Null,
                "matched_keywords": [],
                "reasoning": "",
                "aversion": "",
                "rationale_zh": "",
                "grounded": false,
            })
        })
        .collect()
}

fn take_recs(mut recs: Vec<Value>, limit: usize) -> Vec<Value> {
    recs.truncate(limit);
    recs
}

// ── 入口 action ──

pub fn do_recommend_articles(data: &Value) -> PackageResult {
    let path = db_path();
    let feed_id = data.get("feed_id").and_then(|v| v.as_i64());
    let limit = data
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(10)
        .max(1) as usize;
    let force = data
        .get("force_refresh")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let profile = build_profile(&path);

    // 冷启动门控:画像太薄,不为空画像付费产 ungrounded 理由
    if profile.source_count < COLD_START_MIN_SOURCES || profile.keywords.is_empty() {
        let cands = prefilter(&path, &profile, feed_id);
        return PackageResult::ok(json!({
            "recommendations": l2_to_recs(&cands, limit),
            "token_used": false,
            "mode": "cold_start",
            "profile_size": profile.source_count,
        }));
    }

    let candidates = prefilter(&path, &profile, feed_id);
    if candidates.is_empty() {
        return PackageResult::ok(json!({
            "recommendations": [],
            "token_used": false,
            "mode": "reasoned",
            "profile_size": profile.source_count,
        }));
    }

    let fp = profile_fingerprint(&profile);
    let ch = candidate_hash(&candidates);

    // L4 缓存命中(非强制):0 token
    if !force {
        if let Some(cached) = read_cache(&path, &fp, &ch) {
            return PackageResult::ok(json!({
                "recommendations": take_recs(cached, limit),
                "token_used": false,
                "mode": "reasoned",
                "cached": true,
                "profile_size": profile.source_count,
            }));
        }
    }

    // L3 LLM 推理精排
    match rerank(&profile, &candidates) {
        Ok(ranked) if !ranked.is_empty() => {
            write_cache(&path, &fp, &ch, &ranked);
            PackageResult::ok(json!({
                "recommendations": take_recs(ranked, limit),
                "token_used": true,
                "mode": "reasoned",
                "profile_size": profile.source_count,
            }))
        }
        other => {
            if let Err(e) = other {
                log_info(&format!("rss:recommend L3 failed, fallback to L2: {}", e));
            } else {
                log_info("rss:recommend L3 empty after grounding, fallback to L2");
            }
            PackageResult::ok(json!({
                "recommendations": l2_to_recs(&candidates, limit),
                "token_used": true,
                "mode": "l2_fallback",
                "profile_size": profile.source_count,
            }))
        }
    }
}
