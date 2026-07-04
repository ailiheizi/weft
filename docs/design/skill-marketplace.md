# Skill 市场设计

> Status: Draft  
> Author: ailiheizi  
> Date: 2026-07-03

---

## 1. 背景与动机

Weft 的 skill 系统已有三层：

| 层级 | 格式 | 示例 | 特点 |
|------|------|------|------|
| Builtin skills | Rust 代码 (hardcoded in `skills-runtime`) | `fs_read`, `shell_exec`, `web_search` | 编译在 WASM 里，不可单独安装 |
| File skills | `skills/<slug>/SKILL.md` (YAML frontmatter + Markdown prompt) | `frontend-page`, `data-report`, `lottie-animation` | 轻量 prompt 模板，按需注入 agent context |
| Evolved skills | KV 存储的 JSON 结构体 | 用户使用过程中 AI 自动结晶的工作流 | 有生命周期：pending → active → promoted → archived |

此外，`tool-selector/service/skills/*.md` 定义了 **tool domain knowledge**（工具选择指南），本质也是 prompt-based skill。

**问题**：
- File skills 当前只有 3 个官方自带的，无法从社区获取
- 用户创建的 evolved skills 只存在本地 KV，无法分享
- 没有发现、安装、发布 skill 的统一渠道
- tool domain knowledge 和 file skill 格式不统一，管理分散

**目标**：设计一个轻量 Skill 市场，让用户能**浏览 → 安装 → 创建 → 发布** skills，且不需要编译 WASM。

---

## 2. Skill 格式规范

### 2.1 统一为 SKILL.md（向后兼容当前格式）

```
skills/
  <slug>/
    SKILL.md          # 必须，技能定义
    assets/           # 可选，附属资源（模板文件、图片等）
```

### 2.2 SKILL.md 结构

```markdown
---
# === 必填元数据 ===
name: frontend-page
version: "1.0.0"
description: 生成美观现代的 HTML 网页/报告/仪表盘
author: weft-official

# === 发现与匹配 ===
triggers:
  - 网页
  - HTML
  - dashboard
  - 前端页面
tags:
  - frontend
  - html
  - design
category: development    # development | productivity | data | creative | tools

# === 运行时约束 ===
required_tools:          # 该 skill 需要哪些 builtin/MCP 工具才能正常执行
  - fs_write
min_api_version: v1      # 兼容的 skills-runtime 最低版本

# === 市场展示 ===
icon: 🎨                 # emoji 或 assets/ 下的图片路径
license: MIT
repository: https://github.com/user/weft-skills
---

# 前端页面生成

## 角色
前端页面生成器。产出自带样式、视觉精致的现代 HTML...

## 触发场景
用户要生成任何 HTML 页面：报告、仪表盘、展示页...

## 核心规则
1. 基于本技能的样式模板写，不要裸写朴素 HTML。
2. 一步到位 `fs_write` 完整 HTML...

## 必用的样式模板
...（prompt 正文，运行时注入 agent context）
```

### 2.3 与现有格式的兼容

| 当前格式 | 迁移方案 |
|----------|----------|
| `skills/<slug>/SKILL.md` (只有 name + description) | 加 version/author/triggers/tags 字段 |
| `tool-selector/service/skills/*.md` | 迁移为标准 SKILL.md 格式，放入统一目录 |
| evolved skills (KV JSON) | 提供 `export` 命令，将 JSON 序列化为 SKILL.md |

### 2.4 验证规则

| 字段 | 规则 |
|------|------|
| `name` | 必填，`[a-z0-9-]`，3-64 字符 |
| `version` | 必填，semver |
| `description` | 必填，10-200 字符 |
| `triggers` | 必填，至少 1 个 |
| 正文 (body) | 必填，至少 50 字符 |

---

## 3. 分发机制

### 3.1 Registry 架构

```
┌───────────────────────────────────────┐
│           Skill Registry              │
│  (GitHub repo: weft-skills/registry)  │
│                                       │
│  index.toml     — 所有 skill 的索引    │
│  skills/                              │
│    frontend-page/SKILL.md             │
│    data-report/SKILL.md               │
│    git-commit-assistant/SKILL.md      │
│    ...                                │
└───────────────────────────────────────┘
        │
        │ git clone / raw download
        ▼
┌───────────────────────────────────────┐
│        Local Skills Directory         │
│  ~/.weft/skills/       (用户全局)      │
│  .weft/skills/         (项目级)        │
│  packages/official/skills/skills/ (内置)│
└───────────────────────────────────────┘
```

**为什么用 GitHub repo 而不是自建 registry？**
- 零基建成本，社区贡献通过 PR
- 版本控制天然可用（git tag / commit hash）
- 可离线工作（clone 后全本地）
- 后期可迁移到自建 registry，URL 格式兼容

### 3.2 index.toml 格式

```toml
# 自动生成，由 CI 根据 skills/ 目录内容维护
[[skills]]
slug = "frontend-page"
name = "前端页面生成"
version = "1.0.0"
author = "weft-official"
description = "生成美观现代的 HTML 网页/报告/仪表盘"
category = "development"
tags = ["frontend", "html", "design"]
downloads = 1234
updated_at = "2026-07-01"

[[skills]]
slug = "git-commit-assistant"
name = "Git Commit 助手"
version = "1.2.0"
author = "community-user"
description = "根据 diff 自动生成规范的 commit message"
category = "development"
tags = ["git", "commit", "conventional"]
downloads = 567
updated_at = "2026-06-28"
```

### 3.3 分发方式对比

| 方式 | 适用场景 | 实现复杂度 |
|------|----------|-----------|
| **GitHub Registry repo** (推荐 v1) | 官方 + 社区 curated skills | 低 |
| 单独 GitHub repo 引用 | 个人/组织私有 skill 集合 | 低 |
| 本地目录 | 项目级 skill、开发调试 | 零 |
| HTTP API registry (v2) | 大规模运营、付费 skill | 高 |

---

## 4. 安装与管理

### 4.1 CLI 命令设计

```bash
# 搜索
weft skill search "前端"
weft skill search --tag=git --category=development

# 安装（从 registry）
weft skill install frontend-page           # 安装到 ~/.weft/skills/
weft skill install frontend-page --project  # 安装到 .weft/skills/（项目级）

# 从任意 repo 安装
weft skill install --repo=https://github.com/user/my-skills --slug=my-tool

# 列出已安装
weft skill list
weft skill list --all     # 含 builtin + installed + evolved

# 更新
weft skill update                    # 更新所有
weft skill update frontend-page      # 更新指定 skill

# 卸载
weft skill remove frontend-page

# 创建新 skill
weft skill init my-new-skill         # 生成脚手架 SKILL.md
weft skill validate my-new-skill     # 验证格式

# 导出 evolved skill
weft skill export --evolved=rust-fmt-workflow   # KV → SKILL.md

# 发布到 registry（fork + PR 流程）
weft skill publish my-new-skill
```

### 4.2 安装流程

```
weft skill install <slug>
  │
  ├─ 1. 拉取 registry index.toml（缓存 1h）
  ├─ 2. 查找 slug → 获取元数据
  ├─ 3. 下载 skills/<slug>/ 目录内容
  ├─ 4. 验证 SKILL.md 格式
  ├─ 5. 写入 ~/.weft/skills/<slug>/
  └─ 6. 刷新 skills-runtime 的 file skill 索引
```

### 4.3 Skill 加载优先级

```
1. 项目级：.weft/skills/<slug>/SKILL.md
2. 用户级：~/.weft/skills/<slug>/SKILL.md
3. 内置级：packages/official/skills/skills/<slug>/SKILL.md
```

项目级覆盖用户级，用户级覆盖内置。同 slug 取最高优先级。

---

## 5. AI 自动推荐与使用

### 5.1 现有机制（已实现）

`skills-runtime` 的 `retrieve_applicable` action 已实现按 query 匹配 skill：
- 对 evolved skills：基于 triggers / anti_triggers / required_tools / 成功率综合评分
- 对 file skills：基于 name + description 分词匹配

命中的 skill 内容作为 `context_block` 注入 agent 的 system prompt。

### 5.2 增强：marketplace skill 也参与推荐

安装后的 marketplace skill 与内置 file skill 行为一致：
- 放在 `~/.weft/skills/` 下
- `skills-runtime` 在 `weft_file_skills()` 扫描时自动发现
- 匹配逻辑复用现有 `applicable_file_skills_block()`

**需要修改**：`weft_skills_dir()` 改为返回**多个目录**（内置 + 用户级 + 项目级），扫描合并。

### 5.3 增强：基于 embedding 的语义匹配（v2）

当前按分词做文本匹配。v2 可利用已有的 `tool-selector` ONNX 模型做语义匹配：
- skill 的 description + triggers 预计算 embedding
- 用户 query 实时编码
- 余弦相似度 top-K 召回

---

## 6. 客户端 UI

### 6.1 Skill 管理页面

已有 `skills_provider.dart` + `skills_api.dart`，当前只展示 evolved skills。扩展为：

```
Skill 管理
├── 已安装 (Installed)
│   ├── 内置 (builtin)     — 不可卸载
│   ├── 官方 (official)    — 可更新
│   └── 社区 (community)   — 可更新/卸载
├── 市场 (Marketplace)
│   ├── 推荐 (Featured)
│   ├── 分类浏览 (By Category)
│   ├── 搜索
│   └── Skill 详情页 → 一键安装
├── 我的 Skill
│   ├── Evolved Skills (自动结晶的)
│   │   ├── 待审核
│   │   └── 已激活
│   └── 自建 Skill → 编辑 / 发布
└── 设置
    ├── Registry 源管理（可添加私有源）
    └── 自动更新开关
```

### 6.2 核心交互

| 操作 | 交互 |
|------|------|
| 浏览市场 | 卡片网格，每张卡显示 icon + name + description + 下载量 + tags |
| 安装 | 点击 → 显示详情（完整说明 + 兼容性）→ "安装" 按钮 |
| 创建 | 引导式编辑器（name / description / triggers / 正文）|
| 导出 evolved | "导出为 Skill" 按钮 → 预填充 SKILL.md 内容 |
| 发布 | 验证 → 选择 registry → 提交 PR（或 API push）|

---

## 7. 与现有包系统的关系

| 维度 | Package | Skill |
|------|---------|-------|
| 运行时 | WASM / native 进程 | 纯文本 prompt，不执行代码 |
| 安装复杂度 | 需编译或下载 .wasm | 复制 .md 文件 |
| 权限 | 需声明 storage/process/network | 无（只是 prompt 注入）|
| 分发 | packages/index.toml + capability bindings | skills registry index.toml |
| 热更新 | 需 reload package | 即时生效（下次匹配时读新文件）|
| 目标用户 | 开发者（写 Rust/Python） | 所有用户（写 Markdown）|

**Skill 是 Package 的补充，不是替代。** Skill 让不会写代码的用户也能扩展 agent 能力。

---

## 8. 安全模型

### 8.1 Skill 本身不执行代码

Skill 只是 prompt 注入，实际操作由 agent 调用 builtin tools 完成。安全边界在 tool 层：
- `fs_write` 受 `list_allowed_directories` 约束
- `shell_exec` 受宿主 OS 权限约束
- 网络访问受 `permissions.network` 约束

### 8.2 但 prompt injection 是风险

恶意 skill 可能通过 prompt 诱导 agent 做危险操作。缓解措施：

| 措施 | 说明 |
|------|------|
| Registry 审核 | 官方 registry 的 PR 需人工审核 |
| 内容扫描 | CI 自动检查 SKILL.md 是否含危险指令（rm -rf, 凭据窃取模式等）|
| 来源标记 | UI 明确显示 "官方" vs "社区" vs "未审核" |
| 沙箱预览 | 安装前可预览完整 SKILL.md 内容 |
| 用户评分 | v2 加入评分系统，低分 skill 自动降权 |

### 8.3 私有 skill 的保护

企业用户可能有内部 skill 不想公开：
- 支持配置私有 registry URL（GitHub private repo / Gitea）
- 项目级 skill（`.weft/skills/`）不同步到公开 registry

---

## 9. 实现分期

### Phase 1: 本地基础设施（2-3 周）

- [ ] 统一 SKILL.md 格式规范（加 version/author/triggers/tags）
- [ ] `weft_file_skills()` 支持多目录扫描（内置 + `~/.weft/skills/` + `.weft/skills/`）
- [ ] `weft skill init` / `weft skill validate` CLI 命令
- [ ] `weft skill export --evolved` 导出 evolved skill 为 SKILL.md
- [ ] 迁移 `tool-selector/service/skills/*.md` 为标准格式

### Phase 2: Registry + 安装（2-3 周）

- [ ] 创建 `weft-skills/registry` GitHub repo
- [ ] 编写 `index.toml` 自动生成 CI
- [ ] `weft skill search` / `weft skill install` / `weft skill remove`
- [ ] registry 缓存机制（index.toml 本地缓存 + TTL）
- [ ] 初始化 10+ 官方 skills（从现有 tool-selector domain knowledge 迁移）

### Phase 3: 客户端 UI（2-3 周）

- [ ] Skill 管理页面（已安装 / 市场 / 我的）
- [ ] 市场浏览 + 搜索 + 安装
- [ ] Evolved skill 导出 UI
- [ ] Skill 创建/编辑器

### Phase 4: 社区与质量（持续）

- [ ] 发布流程（`weft skill publish` → 自动 fork + PR）
- [ ] 下载计数 + 排序
- [ ] 用户评分系统
- [ ] 语义匹配（embedding-based retrieval）
- [ ] 私有 registry 支持
- [ ] Skill 依赖（一个 skill 声明依赖另一个 skill）

---

## 10. 开放问题

1. **Registry 是 monorepo 还是 federated？** — v1 用单个 monorepo 简化管理，v2 可考虑联邦制（多 registry 聚合）。

2. **版本冲突** — 同一 slug 不同版本如何处理？建议 latest-wins（简单），不做 lockfile。

3. **Skill 间依赖** — `frontend-page` 能否声明"需要先加载 `design-system` skill"？v1 不支持，v2 考虑。

4. **付费 Skill** — 是否需要？当前不需要。如果未来有复杂的 skill（含大量 domain knowledge + 模板），可以通过私有 registry 实现。

5. **离线优先 vs 在线优先** — 建议离线优先：安装后完全本地工作，market 浏览需要网络。

---

## 附录 A: 与竞品对比

| 平台 | Skill/Extension 形式 | 分发 |
|------|---------------------|------|
| Claude Code (oh-my-claudecode) | `.md` skill files in `~/.claude/skills/` | 手动复制 / 本地 |
| Cursor | Rule files (`.cursorrules`) | 社区 GitHub repo |
| Cline | Custom Instructions | 手动输入 |
| **Weft (本设计)** | `SKILL.md` + YAML frontmatter | GitHub Registry + CLI install |

Weft 的优势：结构化的 frontmatter 使得 **自动匹配/推荐** 成为可能（triggers + semantic matching），而不是全量加载所有 skill。

---

## 附录 B: SKILL.md 完整示例

```markdown
---
name: git-commit-assistant
version: "1.0.0"
description: 根据代码 diff 自动生成规范的 Conventional Commit message
author: weft-official
triggers:
  - commit
  - git commit
  - 提交
  - commit message
tags:
  - git
  - commit
  - conventional-commits
category: development
required_tools:
  - git
  - fs_read
icon: 📝
license: MIT
---

# Git Commit 助手

## 角色
Git commit message 生成器。基于当前 diff 内容，生成符合 Conventional Commits 规范的提交消息。

## 触发场景
用户要提交代码、要求生成 commit message、或者说"帮我提交"。

## 核心规则
1. 先用 `git` 工具运行 `diff --staged`（有 staged）或 `diff`（无 staged）获取变更。
2. 分析变更内容，判断类型：feat / fix / refactor / docs / chore / ci / test。
3. 生成格式：`<type>(<scope>): <subject>`，subject 不超过 72 字符。
4. 如果变更较大，附带 body 说明。
5. 中文项目用中文 subject，英文项目用英文。

## 示例输出
```
feat(auth): add JWT token refresh mechanism

- Add refresh token endpoint
- Implement token rotation on expiry
- Add unit tests for refresh flow
```

## 注意
- 不要自动执行 `git commit`，只生成 message 给用户确认。
- 多个不相关变更建议拆分提交。
```
