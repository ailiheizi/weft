# 网页抓取 领域知识

`web_fetch` 是 agent 内置的网页内容抓取工具，用于获取指定 URL 的正文。

## 可用工具
| 工具 | 功能 | 备注 |
|------|------|------|
| `web_fetch` | 抓取指定 URL 的内容 | 必填 `url`(完整 URL)；可选 `method`(默认 GET)、`body`(POST 时用) |

## 工具选择指南
- 用户给了具体链接要看内容 → `web_fetch`
- `web_search` 返回了有价值的 URL，要看全文 → `web_fetch`
- 调 REST API（GET/POST 拿 JSON） → `web_fetch`
- 还不知道 URL、只有话题 → 先 `web_search` 找链接
- 页面需 JS 渲染（SPA、动态加载） → 改用 `browser_navigate` + `browser_snapshot`

## 技能协作流程
- 搜索 → 抓取：`web_search` 定位 URL → `web_fetch` 取全文。
- 抓取 → 总结：抓回长文后直接总结给用户，无需先落盘。
- 抓取 → 保存：需要留存的内容用 `fs_write` 写本地文件。

## 常见陷阱
- `url` 必填且要完整（带 http(s)://），相对路径会失败。
- 返回纯文本/HTML，不是渲染后页面；动态站点用浏览器工具。
- 需登录的页面可能返回空或登录页，别误把登录页当目标内容。
- 默认 GET，只有调 API 才用 POST 并配 `body`。
- 不要抓取已知大文件（视频/压缩包），会浪费上下文且可能超时。
