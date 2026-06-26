import 'package:flutter/foundation.dart';

import 'canvas_models.dart';

/// 工作流蓝图 —— 导演 Agent 输出的 DAG 描述（节点 + 连线）。
/// 前端用 [CanvasNotifier.applyBlueprint] 把它渲染成画布上的节点和连线。
///
/// 形态对齐 TapNow / Krea / ComfyUI 的"节点+边"结构，但节点是粗粒度创作操作
/// （图像生成 / 视频合成），提示词作为节点内参数，普通用户可读可改。
@immutable
class BlueprintNode {
  const BlueprintNode({
    required this.id,
    required this.kind,
    this.title = '',
    this.prompt,
    this.params = const GenParams(),
  });

  /// 蓝图内的临时 id（用于 edges 引用），与画布真实节点 id 不同。
  final String id;
  final CanvasNodeKind kind;
  final String title;
  final String? prompt;
  final GenParams params;

  factory BlueprintNode.fromJson(Map<String, dynamic> json) => BlueprintNode(
        id: json['id'] as String,
        kind: CanvasNodeKind.values.firstWhere(
          (k) => k.name == json['kind'],
          orElse: () => CanvasNodeKind.image,
        ),
        title: json['title'] as String? ?? '',
        prompt: json['prompt'] as String?,
        params: json['params'] is Map
            ? GenParams.fromJson(Map<String, dynamic>.from(json['params'] as Map))
            : const GenParams(),
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'kind': kind.name,
        'title': title,
        'prompt': prompt,
        'params': params.toJson(),
      };
}

@immutable
class BlueprintEdge {
  const BlueprintEdge({required this.from, required this.to});

  /// 引用 BlueprintNode.id。语义：from 的产物作为 to 的输入（图像→视频汇聚）。
  final String from;
  final String to;

  factory BlueprintEdge.fromJson(Map<String, dynamic> json) => BlueprintEdge(
        from: json['from'] as String,
        to: json['to'] as String,
      );

  Map<String, dynamic> toJson() => {'from': from, 'to': to};
}

/// 一张完整的工作流蓝图。
@immutable
class WorkflowBlueprint {
  const WorkflowBlueprint({
    this.title = '',
    this.nodes = const [],
    this.edges = const [],
  });

  final String title;
  final List<BlueprintNode> nodes;
  final List<BlueprintEdge> edges;

  factory WorkflowBlueprint.fromJson(Map<String, dynamic> json) => WorkflowBlueprint(
        title: json['title'] as String? ?? '',
        nodes: (json['nodes'] as List? ?? [])
            .map((e) => BlueprintNode.fromJson(Map<String, dynamic>.from(e as Map)))
            .toList(),
        edges: (json['edges'] as List? ?? [])
            .map((e) => BlueprintEdge.fromJson(Map<String, dynamic>.from(e as Map)))
            .toList(),
      );

  Map<String, dynamic> toJson() => {
        'title': title,
        'nodes': nodes.map((n) => n.toJson()).toList(),
        'edges': edges.map((e) => e.toJson()).toList(),
      };

  /// 一个演示蓝图：3 张图像（各带提示词）汇聚成一段视频。用于纯前端验证。
  static WorkflowBlueprint demo() => const WorkflowBlueprint(
        title: '赛博朋克咖啡馆短片',
        nodes: [
          BlueprintNode(
            id: 'img1',
            kind: CanvasNodeKind.image,
            title: '霓虹街道',
            prompt: 'cyberpunk neon street at night, rain, cinematic',
          ),
          BlueprintNode(
            id: 'img2',
            kind: CanvasNodeKind.image,
            title: '咖啡馆内景',
            prompt: 'cozy cyberpunk cafe interior, warm neon glow, cinematic',
          ),
          BlueprintNode(
            id: 'img3',
            kind: CanvasNodeKind.image,
            title: '主角特写',
            prompt: 'close-up of a programmer in cyberpunk cafe, moody lighting',
          ),
          BlueprintNode(
            id: 'vid1',
            kind: CanvasNodeKind.video,
            title: '合成短片',
          ),
        ],
        edges: [
          BlueprintEdge(from: 'img1', to: 'vid1'),
          BlueprintEdge(from: 'img2', to: 'vid1'),
          BlueprintEdge(from: 'img3', to: 'vid1'),
        ],
      );
}
