# WEFT-Code 蓝图：Client Surface

## 为什么需要这篇说明

Task 9 已经把 `runtime` 作为唯一 client-facing control plane 固定下来，但还需要一份更短、更直接的说明，明确未来 CLI / TUI / GUI 的分工，不把它们重新塞回 core 或 runtime 文档里。

这份说明的作用不是定义完整实现，而是提前固定 client surface 的演进方向，避免后续在实现 CLI/TUI 时又回到“是不是应该直接碰 core”这类已经定过的问题上。

## 未来 client 分层

### CLI

CLI 用于 command-oriented 和 automation-friendly control。

它更适合：

- 单次命令触发
- shell / script / CI 集成
- 明确输入、明确输出的控制入口

CLI 应该是 runtime 的薄客户端，而不是重新拥有 session/task/team 的业务真相。

在当前 package-managed 架构里，`weft-core` 拥有 `/api/weft-code/*` HTTP compatibility bridge，并通过 `weft_code.runtime` capability 把请求 dispatch 到已安装的 WASM package：`packages/installed/weft-code-runtime`。`registry-server` 不再拥有 WEFT-Code runtime 或 API；它不应再作为 weft-code 的 client-facing control plane。`clients/weft-code-tui` 仍然只是 CLI / TUI client，不是 provider，也不是另一层 runtime，默认连接本机 `weft-core` 的 `3004` 端口。

当前最小 execution checkpoint 也固定在这个边界内：`core.execution/run` 只允许 host-enforced dry-run 请求 `{"mode":"dry_run","command":"weft-core-version"}`，不接受来自 client、prompt 或 TUI 的任意 command/action/capability。`read_only_mode` analyze 与 approval 后的 retry completion 会返回同一个只读 execution record，同时保持 `execution = null`、`workflow_steps = []`，并且不产生文件、shell、git 或 registry-server mutation。最小响应形状如下：

```json
{
  "execution": null,
  "workflow_steps": [],
  "execution_record": {
    "status": "host_capability_called",
    "engine": "host_mediated",
    "capability": "core.execution",
    "action": "run",
    "would_execute": false,
    "would_mutate": false,
    "request": {
      "capability": "core.execution",
      "action": "run",
      "data": {
        "mode": "dry_run",
        "command": "weft-core-version"
      },
      "provider": "core",
      "app": "weft-claw"
    },
    "response": {
      "response": {
        "dry_run": true,
        "would_execute": false
      }
    }
  }
}
```

当前最小 live CLI shell 已经存在于 `clients/weft-code-tui` 中：它不是完整交互式 shell，但已经能把自然语言 prompt 直接发到 runtime，提供一个最小 approval follow-up 命令，支持基础 runtime inspection 命令：`sessions`、`teams`、`team-tasks <TEAM_ID>`，可以通过 `mode <coding|plan|team>` 调用 runtime 切换当前 session mode，并提供 `status` 聚合当前 policy / sessions / teams / approvals / tasks。

当前最小 live TUI path 已经进入 full-screen foundation 阶段：非 watch 的 `tui` 现在会进入 `crossterm` raw mode + alternate screen，并通过 `ratatui` 渲染一个 runtime-backed 的 full-screen shell。当前骨架已经具备 top bar / sidebar / main pane / detail pane / composer 五个区域，实时展示 live policy / sessions / teams / approvals / tasks 数据，并额外通过只读 `/api/weft-code/events` snapshot 展示 Timeline。支持 `1/2/3/4/5` 切换 route，其中 `5` 进入 Timeline；`Tab` / `Shift+Tab` 切换 focus，`j/k` 或方向键移动选中项，`PgUp` / `PgDn` 以 5 条为固定步长跳转，`Home` / `End` 跳到当前 route 的首尾项，`f` 在 Timeline route 上按 `all → runtime → policy → session → approval → all` 循环只读 kind filter，`/` 在 Timeline route 上进入 timeline-search input mode，`Enter` 应用搜索，`Esc` 取消搜索草稿，`Backspace` 编辑搜索草稿，`:` 在 navigate mode 打开只读 command palette help overlay；`i` 进入 composer 输入模式，`Enter` 提交 prompt，`a` 在 approvals route 上批准当前选中的 live approval，`r` 刷新，`q` / `Esc` 退出或返回导航态。`tui --script` 同样支持 `route timeline`、`search "..."` 与 `clear-search`，`tui --watch --frames <N> --interval-ms <MS>` 仍保留为有限次数刷新路径，便于验证 live status refresh。

这意味着当前 TUI 已经跨过了 `stdin.read_line()` 伪 TUI 阶段，形成了一个真正 runtime-backed 的 full-screen terminal application foundation。Timeline 当前只是 read-only snapshot surface：main pane 显示 event rows，detail pane 显示选中 event 的 kind/id/source/resource/summary/sequence，并额外显示 deterministic `data` section；空或 null data 显示为 `none`，object data 按 key 排序渲染，scalar/array data 使用 compact JSON。当前支持列表级 jump navigation、in-memory kind filter 与 in-memory text search；filter/search 只筛选已经刷新到本地的 snapshot events，不发起新的 runtime 查询，也不改变 runtime state。搜索既可通过脚本 `search "..."` 设置，也可在 Timeline route 上按 `/` 进入 timeline-search input mode，用 composer pane 编辑草稿，`Enter` 应用、`Esc` 取消。`:` command palette 目前是 local UI accelerator，并额外允许一个固定诊断命令 `execution probe`。本地 UI allowlist 包括 `route timeline`、`route sessions`、`filter timeline`、`search timeline`、`clear timeline search`；这些 command 只切换 route、循环本地 Timeline filter、进入本地 timeline-search mode，或清空本地 Timeline search。`execution probe` 是唯一 runtime-backed palette action，但它只调用已存在的 `GET /api/weft-code/execution-probe`，不接受用户提供的 command/action/capability，后端仍固定为 host-mediated dry-run `core.execution/run` + `weft-core-version`。TUI 只保存并展示 probe metadata；它不会刷新、批准、提交 prompt、触发任意 execution，或造成 package/server mutation。未知或空 palette command 只关闭 palette 并显示 no-match feedback。Timeline main pane 会按当前 main pane 高度计算 visible rows，并用 `filter=<label> search=<none|query> showing <start>-<end>/<total>` 这样的文本 header 暴露当前位置；测试与非布局调用仍保留固定 5-row fallback。它仍然没有接入其他 runtime-backed palette actions、SSE/live event stream 或更丰富的 tool details，但主交互壳已经不再依赖文本壳循环，也不再以 `ShellSnapshot::sample()` 作为 live path。

Command palette 已经开始向 OpenCode-like 的 command registry 结构迁移：palette command 现在由集中 metadata allowlist 描述，包含 command id、title、category、description、keybind、slash alias 与 safety class。palette UI 会按 deterministic category section 展示 visible commands，当前顺序固定为 `navigation → timeline → diagnostics`；每条 command 直接展示 metadata，例如 `route timeline [navigation] key:5 slash:- safety:local`，下一行展示 description，从而让 command palette 更像可发现的命令表，而不是隐藏字符串列表。category grouping 只是 display-only organization：title-based filtering 与 command dispatch 保持不变，filter 后没有 visible command 的 category 不显示，no-match 仍显示 `no matching commands`。当前 safety class 只有两类：`LocalOnly` 与 `RuntimeReadOnlyDiagnostic`；其中只有 `execution probe` 属于 `RuntimeReadOnlyDiagnostic`，其他 palette command 都必须保持 `LocalOnly`。这个 registry 目前只作为 palette discovery/filter/dispatch 的单一真相源；metadata/category rendering 只是 display-only 增强，不代表已经支持通用 slash command、用户自定义 command、任意 runtime action 或 shell/file/git execution。

### TUI

TUI 是默认 interactive shell。

它更适合：

- 持续交互式工作
- 查看 session / task / team / approval 状态
- 消费 event stream
- 承载 command palette、composer、history、panel 这些交互壳

TUI 的职责是把 runtime 资源与事件以交互式方式呈现出来，而不是重新发明 workflow / task / approval 的语义。

### TUI 技术路线

WEFT-Code 的下一阶段 TUI 重建应明确采用：

- `ratatui` 作为布局、widgets 与 frame rendering 层
- `crossterm` 作为跨平台 terminal backend、raw mode、alternate screen、键盘/鼠标事件层

选择这条路线的原因不是“Rust 社区流行什么就用什么”，而是当前目标本身决定了需要一套能稳定承载下面这些能力的栈：

- alternate-screen 全屏交互
- 多面板布局
- 焦点与选中态
- 非阻塞键盘事件循环
- composer 输入框
- 可扩展 keybind 系统
- 后续 event stream / scrolling / palette / modal 能力

当前 `clients/weft-code-tui` 已经切换到 `ratatui + crossterm` 的基础架构，但 richer event stream / scrolling / palette / modal 能力仍是后续迭代任务。继续把新能力塞回 legacy text renderer 或 one-shot shell 输出路径，仍然会把过渡层重新变成长期负担。

## 目标 TUI 架构

### 1. Runtime 仍然是唯一真相源

TUI 继续作为 runtime 的富客户端，而不是拥有 session / task / team / approval 真相的第二套系统。客户端本地只维护 UI 必需的临时状态，例如：

- 当前 route / view
- focus target
- selected index
- composer buffer
- scroll position
- loading / error / flash message

任何正式的业务真相仍然来自 runtime resources 与后续 event stream。

### 2. TUI 应是 event-driven app，而不是命令行循环

重建后的 TUI 应由三层组成：

- terminal lifecycle：进入/退出 alternate screen、raw mode、cursor 恢复
- app loop：消费键盘、resize、tick、runtime refresh 结果
- frame renderer：根据 `AppState` 重绘整帧 UI

也就是说，`tui` 不应再是“打印字符串 -> 等待整行输入 -> 再打印字符串”的循环，而应是焦点敏感的事件驱动应用。

### 3. 推荐的本地分层

建议未来 `clients/weft-code-tui` 至少拆出以下职责层：

- `app/`：`AppState`、`Route`、`FocusTarget`、`InputMode`、`Action`
- `runtime/`：对现有 HTTP client 的 UI 友好封装
- `tui/terminal`：raw mode / alternate screen 生命周期
- `tui/events`：键盘、鼠标、resize、tick
- `tui/render`：frame-level 渲染入口
- `tui/widgets`：top bar、sidebar、main pane、detail pane、composer、modal

这里的目标不是为了“好看地分文件”，而是为了避免再次把业务逻辑、输入逻辑、字符串 render 和 runtime client 混成一个文件里的命令循环。

## 目标交互骨架

未来默认 interactive shell 的最小形态应包含：

- **Top bar**：project / session / mode / policy / connection status
- **Sidebar**：sessions / teams / tasks / approvals 的导航入口
- **Main pane**：当前 route 的核心列表或消息流
- **Detail pane**：当前选中项的详细信息与 next actions
- **Composer**：用于提交自然语言 prompt 的输入区

这个骨架比当前 panel-style 文本视图更接近 OpenCode 一类现代 coding-agent TUI，也更适合作为未来 command palette、slash command、tool details、event stream timeline 的承载底盘。

## 分阶段边界

### P1：Full-screen TUI Foundation

P1 的目标不再是 mock scaffold，而是建立一个真实的 full-screen TUI foundation：

- alternate screen + raw mode
- sidebar / main / detail / composer 布局
- focus 与 selection
- runtime-backed status 展示
- prompt submit
- approval action

P1 仍然不追求完整产品体验，但应彻底摆脱当前 `read_line()` 驱动的伪 TUI。

### P2：OpenCode-like 交互增强

P2 再追加：

- command palette
- slash commands
- keybind config
- scrolling / paging
- event stream consumption
- richer session switching / task timeline / tool details

### GUI

GUI 是 future reuse target of the same runtime。

它不是第二套系统，也不是绕开 runtime 的特例入口。无论未来是桌面 GUI、IDE extension，还是 remote client，前提都应保持一致：

- 复用同一 runtime resource model
- 复用同一 event stream
- 复用同一 task / team / approval 真相

## 非目标

P0 不实现完整 CLI/TUI，只先固定 runtime 与资源模型。

## 当前实现与下一阶段的关系

当前 `clients/weft-code-tui` crate 里仍然保留了 `ShellSnapshot::sample()` 与 legacy text renderer，供 scaffold/test 与 one-shot CLI 输出使用；但 live CLI/TUI 路径已经改为通过 runtime HTTP API 获取真实数据。full-screen TUI 本地只维护 UI 状态（route / focus / selected index / composer / loading / flash），不复制 runtime 的业务真相；当前 detail/composer 反馈已经可交互，并已有最小 Timeline snapshot surface、palette 本地辅助与 last-submit detail metadata。Detail pane 会从已经加载到本地 state 的 `last_response` 只读展示 task/result/interpretation/lifecycle/approval、action kind/task kind/status，以及 execution intent 的 status/engine/capability/approval/mutation/reason/requested capabilities/step lines；它不额外调用 runtime，也不触发 approval、refresh 或 execution。当前仍没有 live event-stream surface 或 runtime-backed tool record expansion。

因此，现在的 client surface 应视为：

- CLI 已经是最小可用的 live runtime shell
- TUI 已经是最小可用的 live runtime path
- TUI 已经完成 full-screen framework foundation 重构，并进入 P2 的最小 Timeline snapshot surface、列表级 jump navigation、in-memory kind filter、interactive in-memory Timeline search、allowlisted local command palette actions、固定 dry-run `execution probe` 诊断、last-submit action/execution-intent detail metadata 与基于 main pane 高度的 dynamic visible window；但尚未进入通用 runtime-backed palette actions、live event stream 或 runtime-backed richer tool record 阶段

下一轮 client work 的重点不应再是“把 full-screen shell 再退回文本壳补丁”，而应是在当前 foundation 上继续补齐 scrolling / palette / event stream / richer session timeline 等增强能力，同时保持 runtime 作为唯一控制面与真相源。

在 package-first 路径上，runtime 现在还额外暴露了一个最小自然语言入口：`POST /api/weft-code/session/{session_id}/natural-language-task`。它接收 `{"prompt":"..."}`，将请求解释为一个 runtime task，并返回 session / task / interpretation / result / next_steps 的结构化结果。

这个入口目前仍然是最小闭环：它证明 WEFT-Code package 已经能通过 runtime 接住自然语言请求并产出任务结果。

当前自然语言响应还会额外返回一个统一的 `action` 摘要对象，最小字段为：

- `kind`：当前动作语义，例如 `analyze` / `patch` / `write_note` / `plan` / `team_kickoff`
- `task_kind`：底层 runtime task 类型，例如 `coding_task` / `plan_task` / `workflow_task`
- `status`：该动作当前状态，例如 `waiting_approval` / `queued` / `completed` / `blocked`

这个 `action` 对象是给 CLI / TUI / 未来 GUI 直接消费的统一摘要层，不替代已有的 `task`、`approval`、`patch_record`、`created_team` 等详细对象。

与此同时，响应还会返回一个统一的 `lifecycle` 对象，最小字段为：

- `state`：当前 task state
- `transition`：当前状态在 runtime 语义上的归类，例如 `accepted` / `approval_pending` / `policy_blocked` / `completed`
- `record`：最小 transition record，目前包含 `from`、`to` 与 `reason`

这个 `lifecycle` 对象是共享的 runtime lifecycle/transition contract，不是完整工作流图或事件历史。

当前已打通的 runtime 动作边界包括：

- 普通 coding prompt → 先生成一个 approval gate；未批准时返回 `waiting_approval` 的 `coding_task`。对 coding 请求，这里的 approval metadata 与 coding action model 对齐，并暴露同一个 `action_kind`。批准后再次提交同一类 prompt，会返回 completed 的兼容响应，并附带固定 dry-run execution record；它不会执行 shell、不会运行任意命令、不会写文件。
- `read_only_mode` 下的 mutating-looking prompt（例如包含 `target_id`，或文本意图涉及 patch/write/note/file）会在 approval 与 execution 之前被阻止，返回 `blocked` 的 task、`policy_blocked` lifecycle、`approval = null`。
- `read_only_mode` 下的非 mutating analyze prompt 会返回 completed 的只读分析响应，包含 `tool_preview` 与同一固定 dry-run execution record；它只调用 host-mediated `core.execution/run` 的 dry-run `weft-core-version`，不会触发真实 workflow steps。
- `GET /api/weft-code/execution-probe` 是显式 probe route，用于验证 WASM package → core capability 的 host-mediated dry-run path；它同样固定为 `core.execution/run` + `weft-core-version`，不接受来自 client 的任意 command/action/capability。
- TUI palette 中的 `execution probe` 只是上述 route 的 UI diagnostic wrapper；它不扩展后端能力、不接受任意 command，也不会把 palette 变成通用 runtime action surface。结果只作为 `last_execution_probe` metadata 展示在 Detail pane 中。
- TUI command palette 当前使用 registry metadata 渲染命令，并按 category 分组展示；这些 metadata 只用于 discoverability。过滤可以是 title-based partial match，但提交执行必须仍是 normalized exact title lookup，不得把 partial filter result、category、description、keybind 或 slash metadata 当作 dispatch alias。
- Command registry 的 ordered metadata 是一个受测试保护的 client contract：新增命令、重排命令、修改 category / description / keybind / slash / safety 都应显式更新 registry snapshot test，并重新确认 dispatch 仍是 exact title lookup、runtime-backed 命令仍未扩展。
- 后续修改 command registry 时必须按以下检查清单处理：先确认变更是否只是 discoverability metadata，还是会改变 dispatch / runtime / safety 语义；更新 ordered metadata snapshot test，并保留 title 唯一性与 category 顺序检查；为 exact normalized title dispatch 补充或更新测试，确认 partial filter result、category、description、keybind、slash 都不会成为隐式 dispatch alias；若引入任何 alias / slash-command / keybind dispatch，必须单独定义 scope、测试与安全边界，不能借 metadata 字段自动生效；若新增 runtime-backed 命令或修改 `execution probe` safety，必须先重新审查 fixed diagnostic 边界、approval / allowlist / sandbox 需求，以及是否仍禁止任意 command/action/capability；验证至少运行 `cargo test -p weft-code-tui command_registry`、`cargo test -p weft-code-tui palette`、`cargo test -p weft-code-tui probe`、`cargo check -p weft-code-tui` 与 `cargo check -p weft-core -p weft-code-tui`。
- 当前 palette 中唯一 runtime-backed 命令仍是 `execution probe`，且只允许调用固定 `GET /api/weft-code/execution-probe` 诊断路径。`slash` 字段目前只是显示 metadata，不代表 slash-command mode，也不能绕过 exact title dispatch。
- `GET /api/weft-code/events` 当前是只读 timeline snapshot route，通过 WASM runtime 的 `list_events` action 返回 deterministic event array：`runtime.bootstrap`、`policy.current`、当前 `session.current` 与 `approval.current`。它不是 SSE/long-polling/live stream，不推进 approval、不创建 task、不触发 execution，也不修改 runtime state。
- 包含 `plan` 的 prompt → 当前最小支持路径仍是通过 `POST /api/weft-code/session/{session_id}/mode` 切换 mode，并返回 TUI 可消费的 session/mode 状态；它不是完整 planning workflow。
- `teams` 与 `team/{team_id}/tasks` 当前提供 bootstrap parity 数据，证明 core bridge 与 WASM runtime 能返回 TUI 所需 shape；它不是完整 team workflow orchestration。

它仍然没有接入真实 workflow orchestration 或完整 analyze→plan→execute→verify 编排；当前 deliberately 不提供源码 patch、runtime note 写入、任意 shell、git、文件系统 mutation，也不再引用 `server/src/weft_code/*` 作为 patch target。下一阶段若要接入真实 execution engine，必须先定义单独的 allowlist、approval、sandbox 与审计 contract；不能把当前 dry-run record 解释为真实执行能力。
