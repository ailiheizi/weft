# 顺序推理 (sequential-thinking) 领域知识

## 可用工具
| 工具 | 功能 | 备注 |
|------|------|------|
| `sequentialthinking` | 把复杂问题拆成有编号的思考步骤，逐步推进、可回溯修正、可分支探索 | 单一工具，需多次循环调用直至 `nextThoughtNeeded=false` |

## 工具选择指南
- 多步骤规划 / 设计分解 → `sequentialthinking` 逐步展开
- 推理中途发现前面想错了 → `sequentialthinking` 配 `isRevision=true` + `revisesThought` 修正
- 需要并行比较多个方案 → `sequentialthinking` 配 `branchFromThought` + `branchId` 开分支
- 思考过程中发现步数不够 → 调大 `totalThoughts` 继续
- 简单、可一次性给出答案的问题 → 不要用本工具，直接回答更省上下文

## 技能协作流程
- 先估总步数：首次调用设一个合理 `totalThoughts`（可后续动态调整），`thoughtNumber` 从 1 递增。
- 循环推进：每步把当前 `thought` 写清楚，只要还需继续就保持 `nextThoughtNeeded=true`，到收敛才置 `false` 结束。
- 修正而非重来：判断早先步骤有误时，用 `isRevision=true` 并指明 `revisesThought` 指向被修订的步号，而不是重开一轮。
- 分支再汇合：探索 A/B 方案时从某步 `branchFromThought` 拉出 `branchId`，比较后回主线综合结论。

## 常见陷阱
- `thought`、`nextThoughtNeeded`、`thoughtNumber`、`totalThoughts` 为每次调用必填；遗漏会调用失败。
- 工具本身不产出最终答案，只组织思考过程；收尾必须由你自己给出结论，别把中间 `thought` 当成回答交付。
- `nextThoughtNeeded` 忘记置 `false` 会导致推理无限循环、白白消耗上下文。
- `isRevision`/`revisesThought` 与 `branchFromThought`/`branchId` 是不同语义：修正用前者，方案探索用后者，别混用。
- `totalThoughts` 只是预估，可中途加大；不要为了凑数硬塞空步骤。
- 对简单问题套用本工具是过度设计，会显著拖慢响应并浪费 token，按需启用。
