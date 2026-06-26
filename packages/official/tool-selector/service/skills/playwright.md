# Playwright 浏览器 领域知识

## 可用工具
| 工具 | 功能 | 备注 |
|------|------|------|
| `browser_navigate` | 导航到指定 URL | 新会话第一步 |
| `browser_snapshot` | 抓取页面无障碍树快照 | 返回可交互元素的 `ref` 引用；优于截图，是定位元素的基础 |
| `browser_click` | 点击元素 | 需 `target`(ref/选择器)+`element`(人类可读描述)；支持双击、右键、修饰键 |
| `browser_type` | 向可编辑元素输入文本 | `slowly` 逐字输入可触发按键事件；`submit` 输入后回车 |
| `browser_fill_form` | 一次性批量填写表单字段 | 支持 textbox/checkbox/radio/combobox/slider；比逐个 type 高效 |
| `browser_select_option` | 在下拉框中选择选项 | 支持单选或多选(传 values 数组) |
| `browser_press_key` | 按下键盘按键 | 如 `ArrowLeft`、`Enter`、单字符 |
| `browser_take_screenshot` | 截取页面或元素图像 | 仅用于视觉确认，不能据此定位元素；可截全页或指定元素 |
| `browser_wait_for` | 等待文本出现/消失或固定时长 | 处理异步加载、跳转后内容渲染 |
| `browser_tabs` | 管理标签页 | list/new/close/select |
| `browser_navigate_back` | 返回上一页 | 等价浏览器后退 |

## 工具选择指南
- 进入站点 → `browser_navigate`
- 想知道页面有什么、要操作哪个元素 → `browser_snapshot`(拿 ref，不要靠猜)
- 点按钮/链接 → `browser_click`
- 单个输入框填值 → `browser_type`；多字段表单 → `browser_fill_form`
- 下拉选择 → `browser_select_option`；键盘交互 → `browser_press_key`
- 内容异步出现/页面跳转后 → `browser_wait_for` 再继续
- 多页面切换 → `browser_tabs`；回退 → `browser_navigate_back`
- 给人看效果/留存证据 → `browser_take_screenshot`(注意:截图不能用于后续操作定位)

## 技能协作流程
- 标准操作循环:`browser_navigate` → `browser_snapshot` 取 ref → `browser_click`/`browser_type` 操作 → 再次 `browser_snapshot` 确认结果。
- 表单提交:`browser_snapshot` 定位字段 → `browser_fill_form` 批量填充 → `browser_click` 提交按钮 → `browser_wait_for` 等待结果/跳转。
- 异步内容:任何点击或导航后若内容需加载,先 `browser_wait_for` 指定文本,再 `browser_snapshot` 抓取最新状态,避免对旧 ref 操作。
- 验证产出:关键节点用 `browser_take_screenshot` 留证,但所有交互定位一律基于 `browser_snapshot`。

## 常见陷阱
- `browser_click`/`browser_type` 等交互工具的 `target` 必须来自最近一次 `browser_snapshot` 的 ref;页面变化后旧 ref 失效,需重新 snapshot。
- 截图与快照职责互斥:`browser_take_screenshot` 只能看不能点,定位元素必须用 `browser_snapshot`。
- 交互工具普遍需要同时提供 `element`(人类可读描述,用于授权)与 `target`(实际引用),缺一会失败。
- `browser_fill_form` 中 checkbox 值用 `true`/`false`,combobox 值要填选项文本而非 value。
- 不要在导航/点击后立即抓取或断言,先 `browser_wait_for` 等待目标文本,防止读到加载中的中间态。
- 节省上下文:优先用 `browser_snapshot`(文本结构)而非截图;大页面可用 snapshot 的 `depth`/`target` 限制范围,避免一次性吐出整棵树。
