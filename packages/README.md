# Packages 目录说明

`packages/` 是 WEFT vNext 的内部能力实现层目录。

这里的对象不是给普通用户直接操作的，而是给 Core、Resolver、AI 维护流程使用的内部实现单元。

## 当前阶段

当前仓库中真正可构建的官方 Package 源码仍然位于：

- `core/plugins/official/`

因此当前 `packages/` 目录先承担两个作用：

1. 提前建立 vNext 结构
2. 通过索引文件表达“未来 Package 层”和“当前源码位置”的映射关系

## 迁移原则

- 先建立目录结构和索引
- 再逐步把源码迁移出 `core/plugins/official/`
- 在 Core 与构建链路支持新路径之前，不做破坏性移动
