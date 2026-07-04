---
date: 2026-03-19
topic: unified-js-plugin-system
---

# 统一 JS/TS 插件系统

## 我们要构建什么

将 WEFT-core 现有的 WASM 插件系统（Extism）完全替换为统一的 JS/TS 插件系统。每个插件是一个目录，包含后端逻辑（TypeScript，运行在 Deno runtime 中）和 UI 界面（HTML/JS/CSS，由 WEFT-core 提供静态服务）。插件通过 WebSocket 双向通道与 UI 通信，通过 Host API 访问 WEFT-core 的能力。

## 为什么选择这个方案

### 考虑过的方案

1. **保留 WASM + 新增 JS/TS 插件** — 维护两套插件系统，复杂度高，放弃
2. **纯 WASM 插件** — 开发门槛高（需要 Rust/Go 编译），不适合 UI 集成场景
3. **统一 JS/TS 插件（选定）** — 单一插件系统，开发门槛低，UI + 后端一体化

### 选择理由
- 现有 WASM 插件系统无实际使用（plugins 目录为空）
- 核心场景（weft-claw 集成）主要是 IO 操作，不需要 WASM 性能
- JS/TS 生态更成熟，插件开发更快
- 单一系统更易维护

## 关键决策

- **JS 运行时**: `deno_runtime`（完整 Deno 权限沙箱，内置 --allow-net 等权限控制）
- **隔离模型**: 共享 V8 isolate，通过模块隔离（节省内存）
- **Key 暴露**: 通过虚拟 key 系统，插件获取虚拟 key 而非原始 key，可自行管理和修改
- **异常处理**: pipeline 调用中 JS 抛异常时，重试一次，仍失败则回退到 Rust 默认实现
- **权限系统**: 声明制，插件在 plugin.toml 中声明所需权限，未声明的能力被拒绝
- **打包方式**: 单一插件目录，同时包含 WASM 逻辑和 UI 资源
- **通信模型**: UI ←WebSocket→ WEFT-core ←Host API→ JS 插件

## 插件包结构

```
plugins/weft-claw/
  ├── plugin.toml       # 元数据 + 权限声明
  ├── backend.ts        # 后端逻辑（跑在 Deno runtime 里）
  └── ui/
      ├── index.html
      ├── app.js
      └── style.css
```

## 通信架构

```
浏览器/Desktop UI  ←—WebSocket—→  WEFT-core  ←—Host API—→  JS 插件(Deno runtime)
                                      ↕
                                  weft-claw 进程
```

## Host API 能力域

| 域 | 能力 |
|---|------|
| process | spawn/stop/restart 进程、健康检查、崩溃监听、环境变量注入、stdout/stderr 流 |
| network | HTTP 请求（fetch）、WebSocket 客户端/服务端、TCP/UDP socket |
| auth | 获取虚拟 key、创建/吊销虚拟 key |
| storage | 插件私有 KV 存储、读写插件目录文件、访问 data/ 目录 |
| pipeline | 注册/覆盖 Router/KeySelector/Transform/ErrorHandler |
| routes | 注册自定义 HTTP 端点（/api/plugins/{name}/*） |
| events | 订阅核心事件（请求完成、provider 切换、key 耗尽、服务状态变化等） |
| log | 写入 WEFT-core 统一日志、设置日志级别 |
| ui | 注册 UI 页面路由、注册侧边栏/导航入口、推送通知到 UI |
| config | 读取 WEFT-core 全局配置（只读）、读写插件自身配置 |
| scheduler | 注册定时任务（cron/interval） |

## plugin.toml 示例

```toml
[plugin]
name = "weft-claw"
version = "0.1.0"
description = "Weft Claw AI agent orchestration"
entry = "backend.ts"

[permissions]
process = true
network = true
auth = true
storage = true
pipeline = false
routes = true
events = true
log = true
ui = true
config = true
scheduler = true

[ui]
title = "Weft Claw"
icon = "robot"
entry = "ui/index.html"
```

## 开放问题（留给实现阶段）

- WebSocket 消息协议格式（JSON envelope 结构）
- 插件间通信机制（v1 暂不支持）
- 热重载（v1 暂不支持，需重启）
- 插件依赖声明（v1 暂不支持）
- 插件 SDK TypeScript 类型包

## 下一步

→ 进入 `/workflows:plan` 制定详细实现计划
