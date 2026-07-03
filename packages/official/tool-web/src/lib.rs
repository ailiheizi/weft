use weft_package_sdk::*;
use serde_json::Value;

const PACKAGE_NAME: &str = "tool-web";
const CAPABILITY_NAME: &str = "tool.web";

// ─── HTML parsing helpers (mirrored from skills package) ───────────────────────

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
}

fn strip_html_tags(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;

    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }

    output
}

fn collapse_whitespace(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .trim()
        .to_string()
}

fn extract_attr_value(block: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    let start = block.find(&needle)? + needle.len();
    let rest = &block[start..];
    let end = rest.find('"')?;
    Some(decode_html_entities(&rest[..end]))
}

// ─── Bing Search HTML parsers ──────────────────────────────────────────────────

fn parse_bing_definition(html: &str) -> Option<String> {
    // Bing knowledge panel is in class="b_ans"
    let start = html.find("class=\"b_ans\"")?;
    let block = &html[start..html.len().min(start + 12000)];

    // Try to extract text from the answer block
    // Look for a heading first
    let heading = block.find("<h2").and_then(|idx| {
        let section = &block[idx..];
        let content_start = section.find('>')? + 1;
        let content = &section[content_start..];
        let content_end = content.find("</h2>")?;
        let text = collapse_whitespace(&decode_html_entities(&strip_html_tags(
            &content[..content_end],
        )));
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    });

    // Look for description text in a <p> tag
    let description = block.find("<p").and_then(|idx| {
        let section = &block[idx..];
        let content_start = section.find('>')? + 1;
        let content = &section[content_start..];
        let content_end = content.find("</p>")?;
        let text = collapse_whitespace(&decode_html_entities(&strip_html_tags(
            &content[..content_end],
        )));
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    })?;

    Some(match heading {
        Some(word) => format!("{}: {}", word, description),
        None => description,
    })
}

fn parse_bing_results(html: &str, limit: usize) -> Vec<Value> {
    let mut results = Vec::new();

    // Split by class="b_algo" markers (Bing result items have class="b_algo" possibly with other attrs)
    let marker = "class=\"b_algo\"";
    let mut cursor = html;

    while results.len() < limit {
        let Some(block_start) = cursor.find(marker) else {
            break;
        };
        cursor = &cursor[block_start + marker.len()..];

        // Determine the end of this result block (next b_algo or end of string)
        let block_end = cursor.find(marker).unwrap_or(cursor.len());
        let block = &cursor[..block_end];

        // Extract URL and title from <h2><a href="...">TITLE</a></h2>
        let url_and_title = (|| {
            // In Bing, the main result link is inside <h2><a href="URL">title text</a></h2>
            let h2_pos = block.find("<h2")?;
            let h2_block = &block[h2_pos..];
            let a_pos = h2_block.find("<a ")?;
            let a_section = &h2_block[a_pos..];
            let tag_end = a_section.find('>')?;
            let tag = &a_section[..tag_end + 1];

            let href = extract_attr_value(tag, "href")?;
            if !href.starts_with("http") {
                return None;
            }

            // Extract title: text between > and </a>
            let after_tag = &a_section[tag_end + 1..];
            let close_a = after_tag.find("</a>").unwrap_or(after_tag.len());
            let title_text = collapse_whitespace(&decode_html_entities(
                &strip_html_tags(&after_tag[..close_a]),
            ));
            Some((href, title_text))
        })();

        let Some((url, title)) = url_and_title else {
            continue;
        };

        // Extract snippet from <p> or from a caption/description area
        let snippet = block
            .find("<p")
            .and_then(|idx| {
                let section = &block[idx..];
                let content_start = section.find('>')? + 1;
                let content = &section[content_start..];
                // End at </p> or next tag boundary
                let content_end = content.find("</p>").unwrap_or(content.len().min(500));
                let text = collapse_whitespace(&decode_html_entities(&strip_html_tags(
                    &content[..content_end],
                )));
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            })
            .unwrap_or_default();

        if !url.is_empty() && (!title.is_empty() || !snippet.is_empty()) {
            results.push(serde_json::json!({
                "title": title,
                "url": url,
                "text": snippet,
            }));
        }
    }

    results
}

// ─── Query normalization ───────────────────────────────────────────────────────

fn trim_query_punctuation(value: &str) -> String {
    value
        .trim()
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '"' | '\''
                    | '`'
                    | ','
                    | '.'
                    | '!'
                    | '?'
                    | ':'
                    | ';'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '\u{3002}'
                    | '\u{FF01}'
                    | '\u{FF1F}'
                    | '\u{FF0C}'
                    | '\u{FF1A}'
                    | '\u{201C}'
                    | '\u{201D}'
                    | '\u{300C}'
                    | '\u{300D}'
                    | '\u{300E}'
                    | '\u{300F}'
            )
        })
        .trim()
        .to_string()
}

fn strip_known_prefixes(mut value: String) -> String {
    let ascii_prefixes = [
        "please ",
        "can you ",
        "could you ",
        "would you ",
        "help me ",
        "search online for ",
        "search online ",
        "search the web for ",
        "search the web ",
        "search for ",
        "search up ",
        "search ",
        "look up ",
        "research ",
        "find information about ",
        "find out about ",
        "tell me about ",
        "what is the meaning of ",
        "meaning of ",
    ];
    let unicode_prefixes = [
        "\u{4E0A}\u{7F51}\u{641C}\u{7D22}\u{4E00}\u{4E0B}",
        "\u{4E0A}\u{7F51}\u{641C}\u{7D22}",
        "\u{7F51}\u{4E0A}\u{641C}\u{7D22}\u{4E00}\u{4E0B}",
        "\u{7F51}\u{4E0A}\u{641C}\u{7D22}",
        "\u{5E2E}\u{6211}\u{641C}\u{7D22}\u{4E00}\u{4E0B}",
        "\u{5E2E}\u{6211}\u{641C}\u{7D22}",
        "\u{5E2E}\u{6211}\u{67E5}\u{4E00}\u{4E0B}",
        "\u{5E2E}\u{6211}\u{7814}\u{7A76}\u{4E00}\u{4E0B}",
        "\u{641C}\u{7D22}\u{4E00}\u{4E0B}",
        "\u{641C}\u{7D22}",
        "\u{67E5}\u{4E00}\u{4E0B}",
        "\u{67E5}\u{67E5}",
        "\u{7814}\u{7A76}\u{4E00}\u{4E0B}",
        "\u{8C03}\u{7814}\u{4E00}\u{4E0B}",
    ];

    loop {
        let lower = value.to_lowercase();
        let mut changed = false;

        for prefix in ascii_prefixes {
            if lower.starts_with(prefix) {
                value = value[prefix.len()..].trim_start().to_string();
                changed = true;
                break;
            }
        }
        if changed {
            continue;
        }

        for prefix in unicode_prefixes {
            if value.starts_with(prefix) {
                value = value[prefix.len()..].trim_start().to_string();
                changed = true;
                break;
            }
        }

        if !changed {
            break;
        }
    }

    value
}

fn strip_known_suffixes(mut value: String) -> String {
    let ascii_suffixes = [
        " for me",
        " please",
        " thanks",
        " thank you",
        " and tell me",
        " and explain it",
    ];
    let unicode_suffixes = [
        "\u{5427}",
        "\u{5440}",
        "\u{8C22}\u{8C22}",
        "\u{7136}\u{540E}\u{544A}\u{8BC9}\u{6211}",
        "\u{518D}\u{544A}\u{8BC9}\u{6211}",
        "\u{544A}\u{8BC9}\u{6211}",
    ];

    loop {
        let lower = value.to_lowercase();
        let mut changed = false;

        for suffix in ascii_suffixes {
            if lower.ends_with(suffix) {
                let keep = value.len().saturating_sub(suffix.len());
                value = value[..keep].trim_end().to_string();
                changed = true;
                break;
            }
        }
        if changed {
            continue;
        }

        for suffix in unicode_suffixes {
            if value.ends_with(suffix) {
                let keep = value.len().saturating_sub(suffix.len());
                value = value[..keep].trim_end().to_string();
                changed = true;
                break;
            }
        }

        if !changed {
            break;
        }
    }

    value
}

fn normalize_web_search_query(query: &str) -> String {
    let mut value = trim_query_punctuation(query);
    if value.is_empty() {
        return value;
    }

    value = strip_known_prefixes(value);
    value = strip_known_suffixes(value);

    let lower = value.to_lowercase();
    if let Some(rest) = lower.strip_prefix("the meaning of ") {
        let offset = value.len() - rest.len();
        value = format!("{} meaning", value[offset..].trim());
    } else if let Some(rest) = lower.strip_prefix("meaning of ") {
        let offset = value.len() - rest.len();
        value = format!("{} meaning", value[offset..].trim());
    } else if let Some(rest) = lower.strip_prefix("what does ") {
        if let Some(end) = rest.find(" mean") {
            let start = value.len() - rest.len();
            value = format!("{} meaning", value[start..start + end].trim());
        }
    }

    let chinese_meaning_suffix = "\u{7684}\u{542B}\u{4E49}";
    let chinese_meaning_question = "\u{662F}\u{4EC0}\u{4E48}\u{610F}\u{601D}";
    let chinese_meaning_alt = "\u{7684}\u{610F}\u{601D}";
    if value.ends_with(chinese_meaning_suffix) {
        let keep = value.len().saturating_sub(chinese_meaning_suffix.len());
        value = format!("{} \u{542B}\u{4E49}", value[..keep].trim());
    } else if value.ends_with(chinese_meaning_question) {
        let keep = value.len().saturating_sub(chinese_meaning_question.len());
        value = format!("{} \u{542B}\u{4E49}", value[..keep].trim());
    } else if value.ends_with(chinese_meaning_alt) {
        let keep = value.len().saturating_sub(chinese_meaning_alt.len());
        value = format!("{} \u{542B}\u{4E49}", value[..keep].trim());
    }

    collapse_whitespace(&trim_query_punctuation(&value))
}

// ─── host-based web fetch ─────────────────────────────────────────────────────

fn do_web_fetch(data: &Value) -> PackageResult {
    let url = data.get("url").and_then(Value::as_str).unwrap_or("").trim();
    if url.is_empty() {
        return PackageResult::err("web_fetch requires a url argument");
    }
    let method = data
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("GET")
        .trim()
        .to_uppercase();
    let body = data.get("body").and_then(Value::as_str).unwrap_or("");

    let headers: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    let input = serde_json::json!({
        "method": method,
        "url": url,
        "headers": headers,
        "body": body,
    })
    .to_string();

    let raw = match unsafe { host_http_request(input) } {
        Ok(v) => v,
        Err(e) => {
            return PackageResult::err(format!("web_fetch transport failed: {}", e));
        }
    };

    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            return PackageResult::err(format!("web_fetch: invalid response json: {}", e));
        }
    };

    if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
        if !err.trim().is_empty() {
            return PackageResult::err(format!("web_fetch transport failed: {}", err));
        }
    }

    let status = parsed.get("status").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
    let response_body = parsed
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    PackageResult::ok(serde_json::json!({
        "url": url,
        "status": status,
        "body": response_body,
    }))
}

// ─── Tavily Search API ────────────────────────────────────────────────────────

fn try_tavily_search(api_key: &str, query: &str, limit: usize) -> Option<PackageResult> {
    let body = serde_json::json!({
        "api_key": api_key,
        "query": query,
        "max_results": limit,
        "search_depth": "basic",
    })
    .to_string();

    let response_text = http_request(
        "POST",
        "https://api.tavily.com/search",
        &[("Content-Type", "application/json")],
        &body,
    )
    .ok()?;

    let parsed: Value = serde_json::from_str(&response_text).ok()?;

    // Check for error in response
    if parsed.get("error").is_some() {
        return None;
    }

    let tavily_results = parsed.get("results")?.as_array()?;
    if tavily_results.is_empty() {
        return None;
    }

    let results: Vec<Value> = tavily_results
        .iter()
        .take(limit)
        .map(|item| {
            serde_json::json!({
                "title": item.get("title").and_then(Value::as_str).unwrap_or(""),
                "url": item.get("url").and_then(Value::as_str).unwrap_or(""),
                "text": item.get("content").and_then(Value::as_str).unwrap_or(""),
            })
        })
        .collect();

    // Build summary
    let mut summary_parts = Vec::new();
    if let Some(first) = results.first() {
        let title = first.get("title").and_then(Value::as_str).unwrap_or("").trim();
        let text = first.get("text").and_then(Value::as_str).unwrap_or("").trim();
        if !title.is_empty() && !text.is_empty() {
            summary_parts.push(format!("Top result: {}. {}", title, text));
        } else if !text.is_empty() {
            summary_parts.push(format!("Top result: {}", text));
        } else if !title.is_empty() {
            summary_parts.push(format!("Top result: {}", title));
        }
    }
    if !results.is_empty() {
        let bullets = results
            .iter()
            .map(|entry| {
                let title = entry.get("title").and_then(Value::as_str).unwrap_or("").trim();
                let text = entry.get("text").and_then(Value::as_str).unwrap_or("").trim();
                if title.is_empty() {
                    format!("- {}", text)
                } else if text.is_empty() {
                    format!("- {}", title)
                } else {
                    format!("- {}: {}", title, text)
                }
            })
            .collect::<Vec<String>>()
            .join("\n");
        if !bullets.trim().is_empty() {
            summary_parts.push(format!("Related results:\n{}", bullets));
        }
    }

    let summary = if summary_parts.is_empty() {
        format!("No concise web result found for '{}'.", query)
    } else {
        summary_parts.join("\n")
    };

    Some(PackageResult::ok(serde_json::json!({
        "query": query,
        "results": results,
        "summary": summary,
        "source": "tavily",
    })))
}

// ─── host-based web search (Tavily preferred, Bing fallback) ──────────────────

fn do_web_search(data: &Value) -> PackageResult {
    let query = data
        .get("query")
        .or_else(|| data.get("q"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if query.is_empty() {
        return PackageResult::err("web_search requires a query argument");
    }

    let limit = data
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 8) as usize;

    let normalized = normalize_web_search_query(query);
    let search_query = if normalized.is_empty() {
        query.to_string()
    } else {
        normalized
    };

    // Try Tavily API if a search_api_key is provided in data
    let api_key = data
        .get("search_api_key")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if let Some(key) = api_key {
        if let Some(result) = try_tavily_search(&key, &search_query, limit) {
            return result;
        }
        // Tavily failed, fall through to Bing
    }

    // Fallback: Bing HTML scraping
    let url = format!(
        "https://cn.bing.com/search?q={}",
        urlencoding::encode(&search_query),
    );

    let response_text = match http_request(
        "GET",
        &url,
        &[("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")],
        "",
    ) {
        Ok(body) => body,
        Err(error) => return PackageResult::err(format!("web_search request failed: {}", error)),
    };

    let definition = parse_bing_definition(&response_text);
    let results = parse_bing_results(&response_text, limit);

    // Build summary text
    let mut summary_parts = Vec::new();
    if let Some(def) = definition.as_ref() {
        summary_parts.push(format!("Definition: {}", def));
    } else if let Some(first) = results.first() {
        let title = first.get("title").and_then(Value::as_str).unwrap_or("").trim();
        let text = first.get("text").and_then(Value::as_str).unwrap_or("").trim();
        if !title.is_empty() && !text.is_empty() {
            summary_parts.push(format!("Top result: {}. {}", title, text));
        } else if !text.is_empty() {
            summary_parts.push(format!("Top result: {}", text));
        } else if !title.is_empty() {
            summary_parts.push(format!("Top result: {}", title));
        }
    }
    if !results.is_empty() {
        let bullets = results
            .iter()
            .take(limit)
            .map(|entry| {
                let title = entry.get("title").and_then(Value::as_str).unwrap_or("").trim();
                let text = entry.get("text").and_then(Value::as_str).unwrap_or("").trim();
                if title.is_empty() {
                    format!("- {}", text)
                } else if text.is_empty() {
                    format!("- {}", title)
                } else {
                    format!("- {}: {}", title, text)
                }
            })
            .collect::<Vec<String>>()
            .join("\n");
        if !bullets.trim().is_empty() {
            summary_parts.push(format!("Related results:\n{}", bullets));
        }
    }

    let summary = if summary_parts.is_empty() {
        format!("No concise web result found for '{}'.", search_query)
    } else {
        summary_parts.join("\n")
    };

    PackageResult::ok(serde_json::json!({
        "query": search_query,
        "results": results,
        "summary": summary,
        "source": "bing",
    }))
}

// ─── Action dispatch ───────────────────────────────────────────────────────────

fn dispatch(action: &str, data: Value) -> PackageResult {
    match action {
        "describe" => PackageResult::ok(serde_json::json!({
            "package": PACKAGE_NAME,
            "capability": CAPABILITY_NAME,
            "runtime": "wasm",
            "actions": ["describe", "health", "web_fetch", "web_search"],
        })),
        "health" => PackageResult::ok(serde_json::json!({
            "healthy": true,
            "package": PACKAGE_NAME,
        })),
        "web_fetch" | "fetch" | "fetch_url" => do_web_fetch(&data),
        "web_search" | "search" => do_web_search(&data),
        other => PackageResult::err(format!("unknown action: {}", other)),
    }
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("tool-web initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: serde_json::Value::Null,
    });
    Ok(dispatch(&req.action, req.data).to_json())
}

#[plugin_fn]
pub fn call(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: serde_json::Value::Null,
    });
    Ok(dispatch(&req.action, req.data).to_json())
}

#[plugin_fn]
pub fn describe(_input: String) -> FnResult<String> {
    Ok(dispatch("describe", serde_json::Value::Null).to_json())
}

#[plugin_fn]
pub fn health(_input: String) -> FnResult<String> {
    Ok(dispatch("health", serde_json::Value::Null).to_json())
}
