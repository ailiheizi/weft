import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'models/canvas_models.dart';

/// 画布的完整状态快照（不可变）。
@immutable
class CanvasState {
  const CanvasState({
    this.nodes = const {},
    this.edges = const [],
    this.shots = const [],
    this.selectedNodeId,
  });

  final Map<String, CanvasNode> nodes;
  final List<CanvasEdge> edges;
  final List<Shot> shots;
  final String? selectedNodeId;

  CanvasNode? get selectedNode =>
      selectedNodeId == null ? null : nodes[selectedNodeId];

  CanvasState copyWith({
    Map<String, CanvasNode>? nodes,
    List<CanvasEdge>? edges,
    List<Shot>? shots,
    Object? selectedNodeId = _sentinel,
  }) {
    return CanvasState(
      nodes: nodes ?? this.nodes,
      edges: edges ?? this.edges,
      shots: shots ?? this.shots,
      selectedNodeId: selectedNodeId == _sentinel
          ? this.selectedNodeId
          : selectedNodeId as String?,
    );
  }
}

const Object _sentinel = Object();

/// 画布状态管理器。增删节点/连线/Shot，移动、选中、生成状态机。
class CanvasNotifier extends Notifier<CanvasState> {
  int _seq = 0;

  @override
  CanvasState build() => const CanvasState();

  String _newId(String prefix) => '$prefix-${_seq++}-${DateTime.now().microsecondsSinceEpoch}';

  // ── 节点 ──

  /// 新增节点，返回其 id。
  String addNode({
    required CanvasNodeKind kind,
    required Offset position,
    String? shotId,
    String title = '',
    String? assetPath,
    String? thumbnailPath,
    String? prompt,
    GenParams params = const GenParams(),
    NodeStatus status = NodeStatus.idle,
  }) {
    final id = _newId('node');
    final node = CanvasNode(
      id: id,
      kind: kind,
      position: position,
      shotId: shotId,
      title: title,
      assetPath: assetPath,
      thumbnailPath: thumbnailPath,
      prompt: prompt,
      params: params,
      status: status,
    );
    state = state.copyWith(nodes: {...state.nodes, id: node});
    if (shotId != null) _attachNodeToShot(id, shotId);
    return id;
  }

  void updateNode(String id, CanvasNode Function(CanvasNode) update) {
    final node = state.nodes[id];
    if (node == null) return;
    state = state.copyWith(nodes: {...state.nodes, id: update(node)});
  }

  void moveNode(String id, Offset delta) {
    updateNode(id, (n) => n.copyWith(position: n.position + delta));
  }

  void setNodePosition(String id, Offset position) {
    updateNode(id, (n) => n.copyWith(position: position));
  }

  void removeNode(String id) {
    final nodes = {...state.nodes}..remove(id);
    final edges = state.edges
        .where((e) => e.fromNodeId != id && e.toNodeId != id)
        .toList();
    final shots = state.shots
        .map((s) => s.copyWith(nodeIds: s.nodeIds.where((n) => n != id).toList()))
        .toList();
    state = state.copyWith(
      nodes: nodes,
      edges: edges,
      shots: shots,
      selectedNodeId: state.selectedNodeId == id ? null : state.selectedNodeId,
    );
  }

  // ── 选中 ──

  void select(String? id) => state = state.copyWith(selectedNodeId: id);

  // ── 连线 ──

  String addEdge(String fromNodeId, String toNodeId) {
    // 去重 + 防自连
    if (fromNodeId == toNodeId) return '';
    final exists = state.edges.any(
      (e) => e.fromNodeId == fromNodeId && e.toNodeId == toNodeId,
    );
    if (exists) return '';
    final id = _newId('edge');
    state = state.copyWith(
      edges: [...state.edges, CanvasEdge(id: id, fromNodeId: fromNodeId, toNodeId: toNodeId)],
    );
    return id;
  }

  void removeEdge(String id) {
    state = state.copyWith(edges: state.edges.where((e) => e.id != id).toList());
  }

  // ── Shot ──

  String addShot(String title) {
    final id = _newId('shot');
    final order = state.shots.length;
    state = state.copyWith(
      shots: [...state.shots, Shot(id: id, title: title, order: order)],
    );
    return id;
  }

  void _attachNodeToShot(String nodeId, String shotId) {
    final shots = state.shots.map((s) {
      if (s.id != shotId) return s;
      if (s.nodeIds.contains(nodeId)) return s;
      return s.copyWith(nodeIds: [...s.nodeIds, nodeId]);
    }).toList();
    state = state.copyWith(shots: shots);
  }

  // ── 生成状态机 ──

  void setNodeStatus(String id, NodeStatus status, {String? errorMessage}) {
    updateNode(id, (n) => n.copyWith(status: status, errorMessage: errorMessage));
  }

  void completeGeneration(String id, {required String assetPath, String? thumbnailPath}) {
    updateNode(
      id,
      (n) => n.copyWith(
        assetPath: assetPath,
        thumbnailPath: thumbnailPath ?? assetPath,
        status: NodeStatus.ready,
      ),
    );
  }

  /// 测试/演示用：填充几个示例节点验证画布交互。
  void seedDemo() {
    final s1 = addShot('Shot-01');
    final n1 = addNode(
      kind: CanvasNodeKind.image,
      position: const Offset(120, 120),
      shotId: s1,
      title: '霓虹咖啡馆',
    );
    final n2 = addNode(
      kind: CanvasNodeKind.image,
      position: const Offset(420, 120),
      shotId: s1,
      title: '雨夜街道',
    );
    final n3 = addNode(
      kind: CanvasNodeKind.video,
      position: const Offset(720, 220),
      shotId: s1,
      title: '合成短片',
      status: NodeStatus.idle,
    );
    addEdge(n1, n3);
    addEdge(n2, n3);
  }
}

final canvasProvider =
    NotifierProvider<CanvasNotifier, CanvasState>(CanvasNotifier.new);
