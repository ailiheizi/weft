import 'package:flutter/material.dart';
import 'package:vyuh_node_flow/vyuh_node_flow.dart';

/// POC：用 vyuh_node_flow 验证「图像节点连线汇聚成视频」的画布形态、
/// 自定义卡片(nodeBuilder)、拖拽连线、平移缩放手感、Windows 桌面运行。
///
/// 仅供评估，不接入正式流程。验证通过后再决定是否迁移正式画布。
class VyuhPocPage extends StatefulWidget {
  const VyuhPocPage({super.key});

  @override
  State<VyuhPocPage> createState() => _VyuhPocPageState();
}

/// 节点业务数据：标题 + 类型 + 状态（模拟我们的 CanvasNode）。
class PocNodeData {
  PocNodeData({required this.title, required this.kind, required this.status});
  final String title;
  final String kind; // image / video
  final String status; // ready / generating / proposed
}

class _VyuhPocPageState extends State<VyuhPocPage> {
  late final controller = NodeFlowController<PocNodeData, dynamic>(
    nodes: [
      Node<PocNodeData>(
        id: 'img-1',
        type: 'image',
        position: const Offset(120, 120),
        data: PocNodeData(title: '分镜1 · 霓虹街道', kind: 'image', status: 'ready'),
        size: const Size(220, 180),
        ports: [Port(id: 'out', name: '', position: PortPosition.right)],
      ),
      Node<PocNodeData>(
        id: 'img-2',
        type: 'image',
        position: const Offset(120, 340),
        data: PocNodeData(title: '分镜2 · 雨夜追逐', kind: 'image', status: 'generating'),
        size: const Size(220, 180),
        ports: [Port(id: 'out', name: '', position: PortPosition.right)],
      ),
      Node<PocNodeData>(
        id: 'vid-1',
        type: 'video',
        position: const Offset(520, 230),
        data: PocNodeData(title: '汇聚成片', kind: 'video', status: 'proposed'),
        size: const Size(220, 180),
        ports: [Port(id: 'in', name: '', position: PortPosition.left)],
      ),
    ],
    connections: [
      Connection(
          id: 'c1', sourceNodeId: 'img-1', sourcePortId: 'out', targetNodeId: 'vid-1', targetPortId: 'in'),
      Connection(
          id: 'c2', sourceNodeId: 'img-2', sourcePortId: 'out', targetNodeId: 'vid-1', targetPortId: 'in'),
    ],
  );

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('vyuh_node_flow POC（评估）')),
      body: NodeFlowEditor<PocNodeData, dynamic>(
        controller: controller,
        theme: NodeFlowTheme.dark,
        nodeBuilder: (context, node) => _card(node.data),
      ),
    );
  }

  /// 仿我们的节点卡片：缩略图区 + 状态边框色 + 底部标题。
  Widget _card(PocNodeData d) {
    final (border, badge) = switch (d.status) {
      'generating' => (const Color(0xFF4A9EFF), '生成中'),
      'ready' => (const Color(0xFF4CB782), '完成'),
      _ => (const Color(0xFFB0853A), '待确认'),
    };
    return Container(
      width: 220,
      height: 180,
      decoration: BoxDecoration(
        color: const Color(0xFF1E1E22),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: border, width: 1.5),
        boxShadow: [BoxShadow(color: border.withValues(alpha: 0.25), blurRadius: 14)],
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        children: [
          Expanded(
            child: Container(
              color: const Color(0xFF2A2A30),
              child: Center(
                child: Icon(
                  d.kind == 'video' ? Icons.movie_outlined : Icons.image_outlined,
                  size: 40,
                  color: Colors.white24,
                ),
              ),
            ),
          ),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            color: const Color(0xFF1E1E22),
            child: Row(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                      color: border.withValues(alpha: 0.2), borderRadius: BorderRadius.circular(4)),
                  child: Text(badge, style: TextStyle(color: border, fontSize: 10)),
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(d.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(color: Colors.white70, fontSize: 12)),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
