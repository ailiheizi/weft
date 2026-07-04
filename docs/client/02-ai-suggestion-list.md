# AI 询问列表（02）

> 父文档：[`README.md`](./README.md)
> 关联：服务端 [`../server/05-ai-native-distribution.md`](../server/05-ai-native-distribution.md)

---

## 0. 设计哲学

> **AI 不偷偷做事，AI 把建议交给用户检查。**
>
> 防止 AI 卡住对话流去执行长操作；防止 AI 偷偷装包让用户惊讶；让用户随时清楚"AI 想为我做什么"。

---

## 1. 数据模型（纯本地）

```
桌面 Core Runtime 内嵌一个 SQLite/sled
路径：<user_data>/weft/suggestions.db
```

```sql
CREATE TABLE suggestions (
    id              TEXT PRIMARY KEY,        -- UUID
    group_id        TEXT,                    -- 成组建议关联（NULL = 单条）
    kind            TEXT NOT NULL,           -- install/upgrade/evolve_skill/cleanup/security_alert
    package_name    TEXT,
    package_version TEXT,
    trust_level     TEXT,                    -- official/verified/community
    title           TEXT NOT NULL,
    reason          TEXT NOT NULL,
    payload         TEXT NOT NULL,           -- JSON：执行时需要的参数
    state           TEXT NOT NULL,           -- pending/accepted/rejected/postponed/expired
    decided_at      INTEGER,                 -- 决策时间戳
    expires_at      INTEGER NOT NULL,        -- 7 天 TTL
    created_at      INTEGER NOT NULL
);

CREATE TABLE suggestion_groups (
    group_id        TEXT PRIMARY KEY,
    goal            TEXT NOT NULL,           -- 这组建议要达成什么
    bundle          BOOLEAN NOT NULL,        -- 是否要绑定决策
    created_at      INTEGER NOT NULL
);
```

---

## 2. 数据流

```
1. AI Agent 在对话中识别意图
   或：触发器条件命中（重复操作 / 安全告警 / 等等）
   ↓
2. 调服务端 POST /api/packages/recommend
   返回 { group_id, suggestions: [...] }
   ↓
3. 客户端写入本地 suggestions 表
   (state = 'pending', expires_at = now + 7d)
   ↓
4. UI 实时刷新（看到红点角标 / 弹通知）
   ↓
5. 用户点开列表 → 多选 → 决策
   ├─ accept → 调 Core 的 propose API → 走 Generation 流程
   ├─ reject → state = rejected
   └─ postpone → state = postponed（仍在列表）
   ↓
6. 决策完成异步上报：
   POST /api/packages/recommend/feedback
   ↓
7. 7 天 TTL 到期 → state = expired，UI 默认隐藏
```

---

## 3. UI 设计

### 3.1 入口位置

| 入口 | 触发 |
|---|---|
| 主侧边栏图标（带红点）| 任何 pending 建议 |
| 对话面板顶部横幅 | AI 在对话中产生新建议 |
| 系统托盘图标 | 桌面常驻 |

### 3.2 列表 UI 模板

```
┌────────────────────────────────────────────────────┐
│  📋 AI 给你的建议（5 项待处理）                      │
│                                          [⚙ 设置]   │
├────────────────────────────────────────────────────┤
│  💡 来自对话：你刚说想保存网页                       │
│  ┌────────────────────────────────────────────┐   │
│  │ ☑ 🟦 启用"网页存 PDF"功能                   │   │
│  │   来源：✓ 已验证 · ★4.6 · 12.3K 用户       │   │
│  │   将一同启用：网页抓取、浏览器渲染            │   │
│  └────────────────────────────────────────────┘   │
├────────────────────────────────────────────────────┤
│  🔄 自动检测：当前版本有依赖问题                     │
│  ┌────────────────────────────────────────────┐   │
│  │ ☑ 🟦 升级"AI 工具集"到最新                  │   │
│  │   来源：✓ 已验证                            │   │
│  │   原因：修复一个工具调用 bug                 │   │
│  └────────────────────────────────────────────┘   │
├────────────────────────────────────────────────────┤
│  📈 行为分析：你最近重复操作了 5 次                  │
│  ┌────────────────────────────────────────────┐   │
│  │ ☐ 🟨 演化 Skill: "自动写周报"               │   │
│  │   来源：基于你的本地操作                     │   │
│  │   ⚠️ 这是基于你私有数据的演化                │   │
│  └────────────────────────────────────────────┘   │
├────────────────────────────────────────────────────┤
│  [全选] [全不选]                                    │
│  [✅ 应用 2 项 (1.2 MB)]  [❌ 全部拒绝]  [⏰ 稍后]   │
└────────────────────────────────────────────────────┘
```

### 3.3 信任标签颜色

| 标签 | 颜色 | 含义 |
|---|---|---|
| 🟩 official | 绿 | 平台维护 |
| 🟦 verified | 蓝 | 第三方但通过自动扫描 |
| 🟨 community | 黄 | 社区上传，安装时显示警告 |

### 3.4 详情对话框（点"详情"展开）

```
┌────────────────────────────────────────────┐
│  网页存 PDF v1.2.0                          │
├────────────────────────────────────────────┤
│  来源：✓ 已验证                             │
│  作者：example-dev                          │
│  下载：12,345（30 天）                      │
│  评分：★★★★☆ 4.6（128 评价）                │
│  大小：1.2 MB（含依赖 1.5 MB）              │
│                                            │
│  描述：                                     │
│  让 AI 能把网页保存成 PDF 文件。支持长页面、 │
│  自动去广告、保留可点链接。                  │
│                                            │
│  AI 推荐它的原因：                           │
│  ✓ 你的请求"想把这网页存下来"高度匹配        │
│  ✓ 已装的"网页抓取"包能完美配合              │
│                                            │
│  会一同启用：                                │
│  · 浏览器渲染 v2.0（必需依赖）                │
│                                            │
│  权限要求：                                  │
│  · 文件写入（保存 PDF）                      │
│  · 网络访问（抓取网页）                      │
│                                            │
│  [✅ 启用]  [❌ 拒绝]  [⏰ 稍后]              │
└────────────────────────────────────────────┘
```

---

## 4. 成组建议（Bundle）

服务端可以返回"为达成某目标的多个包"：

```json
{
  "group_id": "rec-bundle-xyz",
  "goal": "实现自动写周报",
  "bundle": true,
  "suggestions": [
    { "package": "calendar-sync", ... },
    { "package": "markdown-renderer", ... },
    { "package": "cron-trigger", ... }
  ]
}
```

UI 呈现：

```
┌──────────────────────────────────────────────┐
│  🎯 目标：实现自动写周报                       │
│  需要启用以下 3 项功能：                       │
├──────────────────────────────────────────────┤
│  ☑ 日历同步                                   │
│  ☑ Markdown 渲染                              │
│  ☑ 定时触发                                   │
├──────────────────────────────────────────────┤
│  [✅ 全部启用]  [⚙ 自定义]  [❌ 拒绝]          │
└──────────────────────────────────────────────┘
```

用户可以全部启用 / 部分自定义 / 全部拒绝。

---

## 5. 决策动作的本地处理

```rust
// 用户点"应用 N 项"后
async fn apply_suggestions(ids: Vec<String>) -> Result<()> {
    let suggestions = db.get_by_ids(&ids).await?;

    // 1. 调 Core 的 Generation Manager: propose
    let payloads: Vec<_> = suggestions.iter().map(|s| s.payload).collect();
    let candidate_gen = core.propose(payloads).await?;

    // 2. verify
    let verify_result = core.verify(&candidate_gen).await?;
    if !verify_result.passed {
        ui.show_error(&verify_result.errors);
        return Ok(());
    }

    // 3. activate
    core.activate(&candidate_gen).await?;

    // 4. 标记为 accepted
    db.update_state(&ids, "accepted").await?;

    // 5. 异步上报反馈到云
    tokio::spawn(async move {
        report_feedback_to_server(&ids, "accept").await;
    });

    Ok(())
}
```

---

## 6. 触发场景

不只是用户问。客户端会在多个时机产生建议（写入本地队列）：

| 时机 | 建议类型 | 来源 |
|---|---|---|
| 用户对话中 AI 识别意图 | install | LLM Agent 触发 |
| 用户首次启动 | install onboarding bundle | 客户端启动检查 |
| 检测到重复操作 N 次 | evolve_skill | 客户端行为分析 |
| 包出现安全更新 | upgrade | 服务端推送 |
| 依赖出现冲突 | repair_attempt | 客户端 AI 修复引擎 |
| 长期未用（30 天）| cleanup | 客户端 GC 提议 |
| Foundation 升级可用 | foundation_update | 服务端推送 |

---

## 7. 推送 vs 轮询

| 场景 | 方式 |
|---|---|
| 服务端有新建议（如安全告警）| WebSocket 推送 |
| 客户端启动 | 主动拉取最近 N 条建议 |
| 客户端本地分析（如重复操作）| 直接写本地，无网络 |

---

## 8. 配置项

```toml
# 用户设置
[suggestion]
mode = "manual"                  # manual / semi-auto / full-auto
ttl_days = 7
notification = true
sound = false
auto_apply_official = false      # full-auto 模式才有意义
```

| 模式 | 行为 |
|---|---|
| **manual**（默认） | 所有建议都需用户确认 |
| **semi-auto** | official 包自动应用，verified/community 问 |
| **full-auto** | AI 判定就直接应用（高级用户）|

---

## 9. 隐私

- 建议队列**完全本地**，不上云
- 反馈上报是匿名/聚合（仅决策结果，不上传具体内容）
- 用户可关闭反馈上报
- 用户可一键清空所有建议

---

## 10. 验收清单

- [ ] suggestions.db 设计完成
- [ ] UI 入口（侧边栏 / 对话面板 / 托盘）三处接入
- [ ] 多选 / 全选 / 拒绝 / 稍后流程完整
- [ ] 详情对话框完成
- [ ] 成组建议（Bundle）渲染
- [ ] 决策后调 Generation Manager 跑通
- [ ] 7 天 TTL 自动过期
- [ ] 反馈异步上报
- [ ] 三种自动化模式可切换
