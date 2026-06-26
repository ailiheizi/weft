# Hub 形态画布工作台 — 实现设计

对标 MiniMax Hub 的三栏 + 无限画布形态。计费/积分跳过。

## 1. 目标形态

```
┌─────────────────────────────────────────────────────────────┐
│ Header: 产品名 / 工程标题 / 视图切换                              │
├──────────┬──────────────────────────────────┬───────────────┤
│ 左栏      │ 中栏 无限画布 (核心)               │ 右栏 Agent     │
│ Shot资产库 │  - 平移/缩放                      │ - 对话流        │
│ Shot-01   │  - 节点(图/视频/音乐卡片)          │ - director.turn │
│  ├ img    │  - 节点连线(派生/分镜流)            │ - ask_user      │
│  └ video  │  - 选中/拖拽/就地生成               │ - 生成提议确认   │
│ Shot-02   │                                  │ - 「/」skills   │
└──────────┴──────────────────────────────────┴───────────────┘
```

## 2. 技术选型 (来自 #47)

- 画布:**自研** InteractiveViewer(平移缩放) + Stack(节点绝对定位) + CustomPainter(连线层)
- 状态:Riverpod (canvas state notifier)
- 模型:freezed
- 复用:现有对话逻辑(_sendDirectorMessage/_extractDirectorReply/ask_user)、主题(AppTheme 冷近黑)、Spacing
- 入口:native_registry.dart 已映射 ai-director → AiDirectorWorkbenchScreen

## 3. 数据模型 (lib/features/ai_director/canvas/models/)

```dart
enum CanvasNodeKind { image, video, music, text }

@freezed CanvasNode:
  id: String
  kind: CanvasNodeKind
  position: Offset (画布坐标)
  size: Size
  shotId: String?            // 归属哪个 Shot
  assetPath: String?         // 落地文件路径(图/视频)
  prompt: String?            // 生成用的提示词
  params: GenParams?         // 模型/比例/分辨率/时长
  status: NodeStatus         // idle/generating/ready/failed
  thumbnailPath: String?

@freezed CanvasEdge:
  id, fromNodeId, toNodeId   // 派生/分镜流连线

@freezed Shot:
  id, title, order, nodeIds  // 分镜分组

@freezed GenParams:
  model, aspectRatio, resolution, durationSec

@freezed CanvasState:
  nodes: Map<String,CanvasNode>
  edges: List<CanvasEdge>
  shots: List<Shot>
  selectedNodeId: String?
  viewport: (scale, offset)
```

## 4. 组件结构 (lib/features/ai_director/canvas/)

- `canvas_state.dart` — Riverpod notifier(增删节点/连线/选中/视口/生成状态机)
- `infinite_canvas.dart` — InteractiveViewer + Stack 容器,手势处理
- `canvas_node_widget.dart` — 单节点卡片(按 kind 渲染图/视频缩略/音乐波形)
- `edge_painter.dart` — CustomPainter 画连线(贝塞尔)
- `node_param_panel.dart` — 选中节点的就地生成参数面板
- `shot_library_panel.dart` — 左栏 Shot 资产库
- `director_chat_panel.dart` — 右栏对话(抽取现有对话逻辑)

## 5. 分阶段实现

**#49 画布骨架(中栏核心)**
1. CanvasState 模型 + notifier
2. InfiniteCanvas 平移/缩放
3. CanvasNodeWidget 节点渲染 + 拖拽移动
4. EdgePainter 连线
5. 选中态
→ 用假数据(2-3个节点)验证交互

**#50 左右栏**
1. ShotLibraryPanel 左栏(Shot 分组 + 缩略图,点击聚焦画布节点)
2. DirectorChatPanel 右栏(抽取 _sendDirectorMessage 等,接画布上下文)
3. 三栏布局组装(替换现有 Row 布局)

**#51 接线就地生成**
1. 选中节点 → NodeParamPanel(模型/比例/分辨率/时长)
2. 调 generate_image/render_video(走 coreRepository.runApp,已验证通)
3. 生成结果落新节点 + 自动连线
4. 提议→确认→执行闭环(状态机:proposed→confirmed→generating→ready)
5. 「/」唤起 skills

## 6. 关键复用点

- 后端能力全通(死锁已修):generate_image/render_video/director.turn 经 /api/apps/ai-director/run
- 对话:现有 _sendDirectorMessage/_extractDirectorReply/_extractAskUser* 直接搬到 DirectorChatPanel
- 视频节点播放:media_kit(已有依赖)
- 现有 timeline painter 经验可迁移到 edge_painter

## 7. 保留与替换

- 保留:Header、主题、Spacing、对话逻辑、媒体导入(_importVideo)
- 替换:中栏 mock timeline → 无限画布;mock 资产卡片 → Shot 节点
- 渐进:先并存(新画布作为新视图),验证后再替换默认视图
