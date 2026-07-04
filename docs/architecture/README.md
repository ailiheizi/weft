# Architecture

主线架构文档。**当前主线 = `vNext/`**。

## 入口

| 我要看什么 | 看哪 |
|---|---|
| 当前主线总览 | [`vNext/00-overview.md`](./vNext/00-overview.md) |
| 不可违反的架构决策（ADR） | [`vNext/adr/`](./vNext/adr/) |
| 包模型 / 产品-包-实例 关系 | [`vNext/foundations/`](./vNext/foundations/) |
| Runtime（声明 / Lock / Generation / Profile）| [`vNext/runtime/`](./vNext/runtime/) |
| 产品蓝图（weft-claw / weft-code）| [`vNext/products/`](./vNext/products/) |
| 工作流 | [`vNext/workflows/`](./vNext/workflows/) |
| 早期演进参考（已替代）| [`legacy/`](./legacy/) |

## 目录结构

```
architecture/
├── README.md                        本文件
├── vNext/                           ★ 当前主线
│   ├── 00-overview.md               主线入口
│   ├── adr/                         不可违反的决策
│   │   ├── 0001-product-package-server-authority.md
│   │   ├── 0002-package-server-review-publication-boundary.md
│   │   └── 0003-package-server-client-experience-boundary.md
│   ├── foundations/                 基础模型
│   │   ├── 01-core-kernel.md
│   │   ├── 03-package-model.md
│   │   └── 13-product-package-instance-model.md
│   ├── runtime/                     运行时（lock / binding / profile / generation）
│   │   ├── 04-declaration-config-lock.md
│   │   ├── 05-binding-and-resolution.md
│   │   ├── 06-profiles-and-policy.md
│   │   ├── 07-execution-and-files.md
│   │   ├── 16-package-service.md
│   │   ├── 17-package-service-implementation-plan.md
│   │   ├── 18-self-hosted-package-server-blueprint.md
│   │   ├── 19-generation-manager.md
│   │   └── 20-product-package-server-detailed-plan.md
│   ├── products/                    产品蓝图
│   │   ├── weft-claw/
│   │   └── weft-code/
│   ├── workflows/                   工作流
│   │   └── 08-ai-change-workflow.md
│   └── legacy/                      vNext 内部废弃模型
│       └── 02-app-model.md
└── legacy/                          整体早期设计（已被 vNext 替代）
    ├── README.md
    ├── design-v1.md
    ├── design-v2.md
    └── overview.md
```

## 维护原则

- 新增架构文档进 `vNext/` 对应子目录
- 已被替代的早期文档进 `legacy/`，不再维护
- 重大决策必须写 ADR 进 `vNext/adr/`
- ADR 不能违反，违反需先废弃旧 ADR
