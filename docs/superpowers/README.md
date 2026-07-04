# Superpowers — 设计稿与实施计划

`superpowers/` 目录沉淀两类文档：

| 子目录 | 用途 |
|---|---|
| [`specs/`](./specs/) | **设计稿**：功能/模块的设计与契约（"要做什么 / 为什么"）|
| [`plans/`](./plans/) | **实施计划**：分阶段、checkbox 风格的执行清单（"怎么做 / 进度到哪"）|

## 命名约定

```
YYYY-MM-DD-<主题>[-design].md
```

- **specs/** 一律 `-design.md` 后缀
- **plans/** 不带 `-design`

## 维护原则

- 这里的文档**按主题归档**，不按"还活着 / 已完成"分类
- 即使某个 plan 已落地，文档保留作历史参考
- 不主动删除，只在 [`specs/README.md`](./specs/README.md) 和 [`plans/README.md`](./plans/README.md) 中标记状态
- 新设计稿优先放 [`specs/`](./specs/)；实施细节放 [`plans/`](./plans/)
- 如果某主题被新文档完全替代（如被 [`../server/`](../server/) 替代），可在两个 README 索引中标 `superseded`

## 与其他目录的关系

- 比 [`../architecture/`](../architecture/) 更细：架构定方向，superpowers 落主题
- 比 [`../.plans/`](../../.plans/) 更稳定：当前活跃任务在 `.plans/`，定稿后可来这里归档
- 与 [`../saas-features/`](../saas-features/) / [`../server/`](../server/) / [`../client/`](../client/) 互补：那三组是 SaaS 商业化全景，本目录是逐主题的工程设计

## 索引

- [设计稿索引（specs/）](./specs/README.md)
- [实施计划索引（plans/）](./plans/README.md)
