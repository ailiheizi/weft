# 桌面客户端工作区/产物预览设计方案（类 Manus 体验）

> 状态：**阶段一已落地**（三栏布局 + 工作区面板 + 4 预览器 + chip 联动，dart analyze 0 error，见 §8）。阶段二（webview 内嵌）未开始。
> 范围：仅桌面端（Windows/macOS/Linux）。移动端是远程控制桌面端，不在本方案。
> 编制：2026-06-07

---

## 0. 背景与现状判断

- **这是全新方向**：grep 全 docs，无任何工作区/预览/双栏/webview 设计。现有 `docs/client/`（2026-05-26）写的还是 **Electron+Vue 3**，与实际的 Flutter client（`clients/weft_client`）**已脱节**——本方案以 Flutter 代码现实为准。
- **当前 chat 布局**（`chat_screen.dart:76`）：`Row[ _SessionSidebar | VerticalDivider | _ChatArea(Expanded) ]` —— 二栏。
- **agent 产物数据已存在**（D1 落地的）：`ChatNotifier` 的 `ExecutionStep`/`ToolCallStep` 已记录每次工具调用——`fs_write` 的路径、`shell_exec` 的输出、`web_fetch` 的 URL 等，经 `/api/stream/events` 实时拉取。**这是工作区面板的数据源,无需新增采集。**
- **目标**：Manus 范式 = 左对话流 / 右"工作区"实时展示 agent 动作 + 产物预览（文件/网页/终端/时间线）。

---

## 1. 总体布局：三栏

```
┌──────────┬───────────────────────┬─────────────────────────────┐
│ Session  │   对话区 _ChatArea     │   工作区 WorkspacePanel      │
│ 侧栏      │   (消息流 + 输入框)     │   ┌─ Tab: 文件│终端│网页│时间线 ┐ │
│ (现有)    │                       │   │  当前产物预览              │ │
│          │   消息内的产物变成      │   │                          │ │
│          │   可点击 chip ─────────┼──▶│  点击 → 右侧渲染          │ │
└──────────┴───────────────────────┴─────────────────────────────┘
```

- 改造点：`chat_screen.dart` 的外层 `Row` 加第三个子 widget `WorkspacePanel`（Expanded，可折叠/可拖拽分隔）。
- 工作区可隐藏（无产物时折叠，给对话区全宽）；有产物时自动展开。
- 状态：新增 `workspaceProvider`（Riverpod）持有"当前选中的产物 + 面板开合"，与 `chatProvider` 的 ExecutionStep 联动。

---

## 2. 工作区面板：4 类内容（Tab 切换）

数据全部来自现有 `ToolCallStep`，按工具名路由到对应预览器：

### 2.1 文件预览（fs_read / fs_write / fs_list 产物）— **优先级最高，零外部依赖**
- 数据：ToolCall 的 path + content（fs_write 已有写入内容）。
- 渲染：
  - 代码文件 → 语法高亮（`flutter_highlight` 或 `re_highlight`，纯 Dart）
  - markdown → `flutter_markdown`（**已是依赖**，chat 在用）
  - 图片 → `Image.file`
  - 其它 → 纯文本 + 行号
- 交互：消息里 fs_write 步骤 → chip"📄 写入 foo.rs" → 点击右侧打开该文件预览。
- **纯 Flutter 可做，无 webview 风险。建议第一阶段就做。**

### 2.2 终端/命令输出（shell_exec / git 产物）— **零外部依赖**
- 数据：ToolCall 的 command + stdout/stderr/exit_code。
- 渲染：等宽字体 + ANSI 着色（`flutter_ansi_parser` 或简单解析）的只读终端视图；保留命令头 + 退出码徽标。
- **纯 Flutter 可做。建议第一阶段做。**

### 2.3 Agent 实时动作时间线（Manus "computer" 观感核心）— **零外部依赖**
- 数据：完整 ExecutionStep 序列（D1 已有：tool.started/finished/thinking）。
- 渲染：竖向时间线，每步一张卡（图标 + 工具名 + 参数摘要 + 状态：Running…/✓/✗），实时追加（复用 D1 的 200ms 轮询）。点某步 → 跳到对应产物预览。
- 这是把"agent 正在做什么"可视化——Manus 观感的灵魂，且**数据完全现成**。
- **纯 Flutter 可做。建议第一阶段做。**

### 2.4 网页/HTML 预览（web_fetch / agent 生成的 HTML）— **有跨平台风险，分阶段**
- 见 §3 的 webview 选型。第一阶段先用**外部浏览器打开**（`url_launcher`，已是依赖，零风险），webview 内嵌作为第二阶段。

---

## 3. webview 技术选型（桌面三平台真实支持度）

这是本方案唯一的硬技术风险。调研结论：

| 方案 | Windows | macOS | Linux | 内嵌/独窗 | 成熟度 |
|---|---|---|---|---|---|
| **webview_windows**（jnschulze, WebView2） | ✅ 成熟（benchmark 97） | ❌ | ❌ | 真·内嵌 widget | 高,Windows 唯一推荐 |
| **flutter_inappwebview** | ⚠️ 桌面支持实验性 | ⚠️ | ❌ | 内嵌 | 移动成熟、桌面不全 |
| **webview_cef**（CEF） | ✅ | ✅ | ✅ | 真·内嵌 | **跨三平台但 WIP**,需打包 CEF 二进制(体积大) |
| **desktop_webview_window**（MixinNetwork 系） | ✅ | ✅ | ✅ | **独立窗口**(非内嵌) | 较稳,但弹独立窗口非嵌入面板 |
| **url_launcher**（外部浏览器） | ✅ | ✅ | ✅ | 系统浏览器 | **已是依赖,零风险** |

**没有一个"跨三平台 + 真内嵌 + 成熟"的银弹。** 建议分阶段：

- **阶段一（推荐先做）**：网页产物用 `url_launcher` 外部浏览器打开。零风险、跨平台、立即可用。工作区面板对网页类产物显示"🌐 在浏览器打开 + URL/HTML 预览首屏文本"。
- **阶段二（可选增强）**：真内嵌 webview。两条路线供你定：
  - **A. 跨平台优先** → `webview_cef`：三平台真内嵌,但需打包 CEF 运行时(应用体积 +100MB 级)、WIP 需踩坑验证。
  - **B. 单平台先行** → `webview_windows`：Windows 先做到真内嵌(成熟),macOS/Linux 回退外部浏览器。体验不统一但风险低。

> 选型建议留给你定。我倾向阶段一先落地(无 webview),阶段二按你对"内嵌必要性 vs 体积/风险"的权衡选 A 或 B。

---

## 4. 数据联动（已有数据,无需改 core）

- 产物来源：`chatProvider` 的 `ExecutionStep`/`ToolCallStep`(D1 已采集,经 `/api/stream/events`)。
- 新增 client 侧：
  - `ArtifactModel`：从 ToolCall 归一化出产物(type: file/shell/web/step,path/url/content/output)。
  - `workspaceProvider`(Riverpod StateNotifier):当前选中产物 + 面板开合 + 时间线。
  - `WorkspacePanel` widget + 4 个子预览器(FilePreview / TerminalPreview / TimelineView / WebPreview)。
- **core 端零改动**(产物数据 D1 已经在传)。这是纯 client 改造。

---

## 5. 分阶段实施建议

**阶段一(本方案推荐先做,纯 Flutter 零 webview 风险)**:
1. chat_screen 三栏布局 + WorkspacePanel 骨架(可折叠/拖拽)
2. ArtifactModel + workspaceProvider(从 ExecutionStep 归一化产物)
3. 文件预览(代码高亮/markdown/图片)+ 终端输出预览 + 实时动作时间线
4. 消息内产物 chip → 点击右侧预览的联动
5. 网页产物 → url_launcher 外部打开
→ 已能达成 80% 的 Manus 观感(左对话/右实时工作区+产物),且全部可在桌面三平台跑、无新风险依赖。

**阶段二(已定选型:webview_cef,见下)**:
6. 内嵌 webview 网页预览 —— **webview_cef**(跨三平台真内嵌)。

### webview 选型决策（2026-06-07，用户定：桌面跨平台优先、体积不计、要稳定）
- **选 webview_cef**：唯一"跨 Win/mac/Linux + 真内嵌 widget"方案。底层 CEF（Chromium Embedded Framework）是工业级成熟引擎（Spotify/OBS 等在用），体积大（打包完整 CEF，+100MB 级）正是换三平台一致 + 真内嵌，符合用户"体积不在乎"取向。
- **风险定位**：库标 `[WIP]`，但风险在 **Flutter 绑定层**，不在 Chromium 引擎本身。绑定层稳定性**只能实测确认,不靠 README**。
- **落地策略**：阶段二第一步做 **webview_cef 三平台最小 POC**（各平台编译跑通一个内嵌网页 demo + 验证销毁/多实例/导航稳定性），通过再全量集成。
- **回退**：POC 若不稳 → Windows 用 `webview_windows`(WebView2,最成熟) + mac/Linux 用 `url_launcher` 外部浏览器（体验不统一但零风险）。
- 落选：`webview_windows`(仅 Windows,跨平台出局)、`desktop_webview_window`(弹独立窗非内嵌,不符 Manus 嵌入式预览)。


---

## 6. 待用户决策
1. 阶段一是否就按"文件+终端+时间线+外部浏览器"先落地?(我的推荐)
2. webview 内嵌(阶段二)走哪条:A 跨平台 webview_cef(体积大/WIP)还是 B Windows 先行 webview_windows?还是暂不做内嵌、长期用外部浏览器?
3. 代码高亮库选型(`flutter_highlight` vs `re_highlight`)——实施时定即可。

## 7. 验证边界
本文档为设计,无代码、无验证。reference 项目前端(paperclip/zeroclaw/Clawith web)因 subagent 渠道不可用未实地调研,Manus 范式基于公开认知——实施前可补充 reference 调研细化交互细节。

---

## 8. 阶段一实施结果（2026-06-07，已落地 + analyze 通过）

纯 client 改造，core 零改动；全部用现有依赖（flutter_highlight/markdown/url_launcher），无新增。

### 新增文件
- `lib/features/chat/workspace/artifact.dart`：`Artifact` 归一化模型（从 ToolCallStep 提取 file/terminal/web）+ `WorkspaceState`/`WorkspaceNotifier`/`workspaceProvider`（show/openTimeline/toggle/close/setTab）+ `artifactsFromMessages()`。纯 Dart class，不引 build_runner。
- `lib/features/chat/workspace/workspace_panel.dart`（855 行）：`WorkspacePanel(sessionId)` + 4 预览器：
  - `_FilePreview`：图片(Image.file) / markdown(MarkdownBody) / 代码(HighlightView, github 主题)
  - `_TerminalPreview`：等宽只读视图 + command/工具名/退出码徽标 + strip ANSI
  - `_TimelineView`：watch chatProvider 实时时间线，ThinkingStep + ToolCallStep，Running…/✓ 状态，点产物卡 → show()
  - `_WebPreview`：url_launcher 外部浏览器打开 + 抓取文本首屏

### 集成（chat_screen.dart）
- 外层 Row 二栏→三栏：`Session侧栏 | 对话区 | WorkspacePanel`（按 workspaceProvider.open 显隐，Expanded flex 1）
- `_TopBar` 加 `_WorkspaceToggle`：工作区开合按钮 + 产物数量徽标
- `_StepItem` 改 ConsumerWidget：toolCall 步骤可归一化为 Artifact 时，加"在工作区打开"按钮 → workspaceProvider.show()

### 验证
- `dart analyze lib`（全项目）→ **0 error**（仅 3 个预存 unnecessary_underscores info，非本次引入）
- **未做**：GUI 运行时实测（起 Flutter app 点击交互、各预览器实际渲染、时间线实时刷新）——需起 core+app 人工冒烟。

### 阶段二（webview 内嵌网页预览）未开始
选型已定 webview_cef（见 §7），第一步做三平台 POC。本次仅阶段一。

---

## 9. 工作区重构 + webview_cef 尝试结果（2026-06-07）

### 重构（按用户反馈：自动加载、去时间线、渲染文件）— 已落地，build exit 0
- **去掉时间线 tab**（用户："时间线没必要"）。
- **自动跟随最新产物**：`WorkspaceNotifier.followLatest()`，agent 写文件/生成网页 → 工作区自动展开并渲染最新（用户："工作区一般自己加载"）。`workspaceProvider` 去掉 tab、加 `following` 态。
- 顶栏改为：当前产物标题+路径 + 历史产物下拉（PopupMenu）+ 跟随按钮 + 关闭。删除 `_TabChip`/`_TimelineView`/`_TabEmptyState`/`_countByKind`。
- **`.html`/`.htm` 文件路由到 web 预览**（`Artifact.fromToolCall`），不再当源码高亮。
- 消息步骤手动按钮弱化为"查看"（回看历史产物）。
- 验证：`dart analyze lib` 0 error；`flutter build windows --debug` √ Built（31.8s）。

### webview_cef 内嵌渲染 — 尝试失败，已回退
目标：HTML 产物内嵌 webview 看渲染效果（用户："能渲染就自动渲染"）。
- 完成：pubspec 加 webview_cef 0.2.2（解析成功）、windows/runner/main.cpp 加 initCEFProcesses/handleWndProcForCEF、main.dart 加 WebviewManager().initialize()、_WebPreview 改 StatefulWidget 用 WebViewController 渲染。Dart analyze 全过。
- **构建失败（硬阻断）**：`flutter build windows` 中 CEF 200MB 下载成功，但编译报
  `webview_cef-0.2.2/lib/src/webview.dart:290: WebViewState (with WebeViewTextInput mixin) is missing implementations` —
  **包自身与 Flutter 3.44.1 不兼容**（WIP 包未跟上新 SDK 给 TextInput 接口加的必须成员）。无现成 fix。
- **决策**：暂移除 webview_cef（避免拖住已可用的重构），全部 4 处改动已回退，build 恢复 exit 0。内嵌 HTML 渲染的代码思路已验证（API 用法正确），待 webview_cef 出 Flutter 3.44 兼容版、或换包、或调整 Flutter 版本后即可启用。当前 `.html` 用"在浏览器打开"（file:// URI）看真实渲染效果兜底。

### 验证边界
- **GUI 未实测**：flutter-inspector MCP 本轮不可用，三栏视觉/自动加载/各预览器渲染/历史切换均未肉眼验证，仅 analyze + build 通过。需人工起 app 冒烟。
- webview 内嵌渲染未达成（包不兼容）；HTML 当前为外部浏览器查看。

### 内嵌 HTML 渲染后续选项（按稳定性排序）
1. 等 webview_cef 出 Flutter 3.44 兼容版（git 版可能已修，未试）。
2. 换 `desktop_webview_window`（跨平台较稳，但弹独立窗口非内嵌）。
3. `webview_windows`（Windows 内嵌最成熟，但仅 Windows，mac/Linux 回退外部浏览器）。
4. 轻量 `flutter_widget_from_html`（纯 Dart 无 native，渲染静态 HTML/CSS 不跑 JS）。

---

## 10. 自动加载死锁修复 + HTML 渲染换 flutter_widget_from_html_core（2026-06-07）

### 修复：工作区不自动展开（死锁 bug）
- 症状：agent 写文件后工作区面板不出现。
- 根因：`followLatest()`（自动跟随最新产物）放在 `WorkspacePanel.build` 里，但面板 `open=false` 时不渲染 → build 不跑 → followLatest 不触发 → 永远不开（鸡生蛋死锁）。
- 修复：把 followLatest 移到常驻渲染的 `ChatScreen.build`（watch 产物列表，postFrame 调）。WorkspacePanel 内的重复调用删除。
- 验证（VM Service render tree）：发消息→agent 写 html→`WorkspacePanel` 自动渲染，实测 size 543×848（屏宽 1389 占 ~39%），内容为 `_WebPreview` 非 EmptyState。

### HTML 渲染：webview_cef → flutter_widget_from_html_core
- webview_cef 构建失败已回退（§9）。改用 `flutter_widget_from_html_core ^0.17.2`：
  - 轻量纯 Dart（仅 +3 依赖，无 native），零构建风险（区别于 webview_cef 的 50 依赖/CEF 200MB/构建失败）。
  - `_WebPreview` 对 HTML 内容（`_isHtml`：.html 后缀或含 `<html>/<body>/<div>/<!DOCTYPE>`）用 `HtmlWidget(content)` 渲染为 Flutter widget；非 HTML（web_search 文本）走纯文本。
  - 限制：**不跑 JS**，渲染静态 HTML/CSS（标题/段落/列表/表格/样式）。带 JS 交互的页面看不到动态效果 → 保留"在浏览器打开"按钮兜底。
- 验证（VM Service widget tree，节点计数法）：demo.html（h1/p/ul/li×3）→ widget tree 中 "Hello" 出现 9 次 vs HTML 源码 1 次 → 证明 HTML 被解析渲染成多个 widget 节点，非源码文本。build exit 0，analyze 0 error。

### 验证手段说明
- **本环境 Read 工具读本地图片文件无法返回图像内容**（Claude Code 已知行为，见 issue #18588；贴进对话的图可看，主动 Read 的不可看）。
- 故本轮视觉验证改用 **VM Service 的 widget tree / render tree 文本数据**（ext.flutter.debugDumpApp / debugDumpRenderTree + 节点计数），定位了死锁 bug 并证明 HTML 渲染生效。
- **视觉细节（布局美观度/配色/比例）未经人眼确认**，需用户查看 app 窗口或贴图反馈。

### 当前状态
工作区：自动跟随最新产物、文件/代码/markdown/终端预览、HTML 静态渲染（flutter_widget_from_html_core）+ 外部浏览器兜底。全部 build 通过、widget 树验证功能正确。视觉待用户确认。
