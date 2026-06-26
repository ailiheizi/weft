import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

import '../../../core/providers/core_repository.dart';
import 'models/canvas_models.dart';
import 'models/workflow_blueprint.dart';

/// 画布的完整状态快照（不可变）。
@immutable
class CanvasState {
  const CanvasState({
    this.nodes = const {},
    this.edges = const [],
    this.shots = const [],
    this.selectedNodeId,
    this.selectedIds = const {},
    this.linkingFromId,
    this.linkCursor,
  });

  final Map<String, CanvasNode> nodes;
  final List<CanvasEdge> edges;
  final List<Shot> shots;

  /// 主选中节点（参数面板针对它）。
  final String? selectedNodeId;

  /// 多选集合（含主选中）。Delete/批量操作针对它。
  final Set<String> selectedIds;

  /// 正在拖拽连线的起点节点 id（null = 未在连线）。
  final String? linkingFromId;

  /// 连线拖拽时的当前画布坐标（用于画临时线）。
  final Offset? linkCursor;

  CanvasNode? get selectedNode =>
      selectedNodeId == null ? null : nodes[selectedNodeId];

  bool get isLinking => linkingFromId != null;

  bool isSelected(String id) => id == selectedNodeId || selectedIds.contains(id);

  CanvasState copyWith({
    Map<String, CanvasNode>? nodes,
    List<CanvasEdge>? edges,
    List<Shot>? shots,
    Object? selectedNodeId = _sentinel,
    Set<String>? selectedIds,
    Object? linkingFromId = _sentinel,
    Object? linkCursor = _sentinel,
  }) {
    return CanvasState(
      nodes: nodes ?? this.nodes,
      edges: edges ?? this.edges,
      shots: shots ?? this.shots,
      selectedNodeId: selectedNodeId == _sentinel
          ? this.selectedNodeId
          : selectedNodeId as String?,
      selectedIds: selectedIds ?? this.selectedIds,
      linkingFromId:
          linkingFromId == _sentinel ? this.linkingFromId : linkingFromId as String?,
      linkCursor: linkCursor == _sentinel ? this.linkCursor : linkCursor as Offset?,
    );
  }

  /// 仅序列化持久数据（节点/连线/Shot），不含选中/拖拽等临时态。
  Map<String, dynamic> toJson() => {
        'version': 1,
        'nodes': nodes.values.map((n) => n.toJson()).toList(),
        'edges': edges.map((e) => e.toJson()).toList(),
        'shots': shots.map((s) => s.toJson()).toList(),
      };

  factory CanvasState.fromJson(Map<String, dynamic> json) {
    final nodeList = (json['nodes'] as List? ?? [])
        .map((e) => CanvasNode.fromJson(Map<String, dynamic>.from(e as Map)))
        .toList();
    return CanvasState(
      nodes: {for (final n in nodeList) n.id: n},
      edges: (json['edges'] as List? ?? [])
          .map((e) => CanvasEdge.fromJson(Map<String, dynamic>.from(e as Map)))
          .toList(),
      shots: (json['shots'] as List? ?? [])
          .map((e) => Shot.fromJson(Map<String, dynamic>.from(e as Map)))
          .toList(),
    );
  }
}

const Object _sentinel = Object();

/// 画布状态管理器。增删节点/连线/Shot，移动、选中、生成状态机。
class CanvasNotifier extends Notifier<CanvasState> {
  int _seq = 0;
  Timer? _saveTimer;
  bool _loaded = false;

  /// 撤销/重做栈，存持久数据快照（JSON 字符串，避免共享引用）。
  final List<String> _undoStack = [];
  final List<String> _redoStack = [];
  static const _maxHistory = 50;

  bool get canUndo => _undoStack.isNotEmpty;
  bool get canRedo => _redoStack.isNotEmpty;

  /// 在一次语义操作前调用：把当前持久状态压入 undo 栈，清空 redo。
  void _pushUndo() {
    _undoStack.add(jsonEncode(state.toJson()));
    if (_undoStack.length > _maxHistory) _undoStack.removeAt(0);
    _redoStack.clear();
  }

  void undo() {
    if (_undoStack.isEmpty) return;
    _redoStack.add(jsonEncode(state.toJson()));
    final snapshot = _undoStack.removeLast();
    _restoreFrom(snapshot);
  }

  void redo() {
    if (_redoStack.isEmpty) return;
    _undoStack.add(jsonEncode(state.toJson()));
    final snapshot = _redoStack.removeLast();
    _restoreFrom(snapshot);
  }

  void _restoreFrom(String snapshot) {
    final restored = CanvasState.fromJson(
      jsonDecode(snapshot) as Map<String, dynamic>,
    );
    // 保留当前选中（若仍存在），清拖拽态。
    final keepSel = state.selectedNodeId != null &&
            restored.nodes.containsKey(state.selectedNodeId)
        ? state.selectedNodeId
        : null;
    state = restored.copyWith(selectedNodeId: keepSel);
  }

  @override
  CanvasState build() {
    // 状态变化时 debounce 自动保存（仅在已加载后，避免覆盖未读取的存档）。
    listenSelf((_, _) {
      if (_loaded) _scheduleSave();
    });
    ref.onDispose(() => _saveTimer?.cancel());
    return const CanvasState();
  }

  String _newId(String prefix) => '$prefix-${_seq++}-${DateTime.now().microsecondsSinceEpoch}';

  // ── 持久化 ──

  Future<File> _projectFile() async {
    final dir = await getApplicationSupportDirectory();
    final canvasDir = Directory('${dir.path}${Platform.pathSeparator}ai_director');
    if (!await canvasDir.exists()) {
      await canvasDir.create(recursive: true);
    }
    return File('${canvasDir.path}${Platform.pathSeparator}canvas_project.json');
  }

  /// 启动时加载存档。无存档则返回 false（调用方可决定是否 seedDemo）。
  Future<bool> loadProject() async {
    try {
      final file = await _projectFile();
      if (!await file.exists()) {
        _loaded = true;
        return false;
      }
      final raw = await file.readAsString();
      if (raw.trim().isEmpty) {
        _loaded = true;
        return false;
      }
      final json = jsonDecode(raw) as Map<String, dynamic>;
      state = CanvasState.fromJson(json);
      _loaded = true;
      return state.nodes.isNotEmpty;
    } catch (_) {
      _loaded = true;
      return false;
    }
  }

  void _scheduleSave() {
    _saveTimer?.cancel();
    _saveTimer = Timer(const Duration(milliseconds: 600), saveProject);
  }

  Future<void> saveProject() async {
    try {
      final file = await _projectFile();
      await file.writeAsString(jsonEncode(state.toJson()));
    } catch (_) {
      // 保存失败静默降级，不阻断创作。
    }
  }

  /// 新建工程（清空画布并立即保存空白）。
  void newProject() {
    _pushUndo();
    state = const CanvasState();
    _scheduleSave();
  }

  // ── 节点 ──

  /// 新增节点，返回其 id。
  String addNode({
    required CanvasNodeKind kind,
    required Offset position,
    Size size = const Size(180, 180),
    String? shotId,
    String title = '',
    String? assetPath,
    String? thumbnailPath,
    String? prompt,
    GenParams params = const GenParams(),
    NodeStatus status = NodeStatus.idle,
  }) {
    _pushUndo();
    final id = _newId('node');
    final node = CanvasNode(
      id: id,
      kind: kind,
      position: position,
      size: size,
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

  /// 导入一张本地图片为 ready 图节点。返回节点 id。
  String importImage({
    required String path,
    required Offset position,
    String? shotId,
    String title = '',
  }) {
    final name = title.isNotEmpty
        ? title
        : path.split(RegExp(r'[\\/]')).last;
    return addNode(
      kind: CanvasNodeKind.image,
      position: position,
      shotId: shotId,
      title: name,
      assetPath: path,
      thumbnailPath: path,
      status: NodeStatus.ready,
    );
  }

  /// 重命名节点标题。
  void renameNode(String id, String title) {
    updateNode(id, (n) => n.copyWith(title: title));
  }

  void updateNode(String id, CanvasNode Function(CanvasNode) update) {
    final node = state.nodes[id];
    if (node == null) return;
    state = state.copyWith(nodes: {...state.nodes, id: update(node)});
  }

  /// 拖拽移动开始：记一次 undo 快照（整段拖拽算一步）。
  void beginMove() => _pushUndo();

  void moveNode(String id, Offset delta) {
    // 若拖动的节点属于多选集合，整组一起移动。
    final group = state.selectedIds.contains(id) && state.selectedIds.length > 1
        ? state.selectedIds
        : {id};
    final nodes = {...state.nodes};
    for (final nid in group) {
      final n = nodes[nid];
      if (n != null) nodes[nid] = n.copyWith(position: n.position + delta);
    }
    state = state.copyWith(nodes: nodes);
  }

  /// 网格吸附步长（拖动结束时节点左上角吸附到此网格）。
  static const double snapGrid = 20.0;

  /// 拖动结束：把被拖动（及同组）节点的位置吸附到最近网格点。
  void endMove(String id) {
    final group = state.selectedIds.contains(id) && state.selectedIds.length > 1
        ? state.selectedIds
        : {id};
    final nodes = {...state.nodes};
    var changed = false;
    for (final nid in group) {
      final n = nodes[nid];
      if (n == null) continue;
      final snapped = Offset(
        (n.position.dx / snapGrid).round() * snapGrid,
        (n.position.dy / snapGrid).round() * snapGrid,
      );
      if (snapped != n.position) {
        nodes[nid] = n.copyWith(position: snapped);
        changed = true;
      }
    }
    if (changed) state = state.copyWith(nodes: nodes);
  }

  void setNodePosition(String id, Offset position) {
    updateNode(id, (n) => n.copyWith(position: position));
  }

  /// 缩放开始：记一次 undo 快照（整段缩放算一步）。
  void beginResize() => _pushUndo();

  /// 按 delta 调整节点尺寸（带最小/最大限制）。
  void resizeNode(String id, Offset delta) {
    updateNode(id, (n) {
      final w = (n.size.width + delta.dx).clamp(100.0, 600.0);
      final h = (n.size.height + delta.dy).clamp(80.0, 600.0);
      return n.copyWith(size: Size(w, h));
    });
  }

  void removeNode(String id) {
    _pushUndo();
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

  /// 框选：选中一组节点 id。主选中取第一个。
  void selectMany(Set<String> ids) {
    state = state.copyWith(
      selectedIds: ids,
      selectedNodeId: ids.isEmpty ? null : ids.first,
    );
  }

  /// 返回与给定矩形（画布坐标）相交的所有节点 id。
  Set<String> nodesInRect(Rect rect) {
    return state.nodes.values
        .where((n) => n.rect.overlaps(rect))
        .map((n) => n.id)
        .toSet();
  }

  /// 单选：设主选中并把多选集合重置为它（null = 全清）。
  void select(String? id) => state = state.copyWith(
        selectedNodeId: id,
        selectedIds: id == null ? const {} : {id},
      );

  /// Ctrl+点击：切换某节点的选中态（多选）。主选中跟随最后操作的节点。
  void toggleSelect(String id) {
    final set = {...state.selectedIds};
    final nowSelected = !set.contains(id);
    if (nowSelected) {
      set.add(id);
    } else {
      set.remove(id);
    }
    state = state.copyWith(
      selectedIds: set,
      selectedNodeId: nowSelected ? id : (set.isEmpty ? null : set.first),
    );
  }

  void clearSelection() =>
      state = state.copyWith(selectedNodeId: null, selectedIds: const {});

  /// 当前选中的节点列表（≥1）。
  List<CanvasNode> get _selectedNodes => state.selectedIds
      .map((id) => state.nodes[id])
      .whereType<CanvasNode>()
      .toList();

  /// 批量对齐选中节点。edge: left/right/top/bottom/centerH(水平居中)/centerV(垂直居中)。
  void alignSelected(String edge) {
    final sel = _selectedNodes;
    if (sel.length < 2) return;
    _pushUndo();
    final nodes = {...state.nodes};
    double target;
    switch (edge) {
      case 'left':
        target = sel.map((n) => n.position.dx).reduce((a, b) => a < b ? a : b);
        for (final n in sel) {
          nodes[n.id] = n.copyWith(position: Offset(target, n.position.dy));
        }
      case 'right':
        target = sel.map((n) => n.position.dx + n.size.width).reduce((a, b) => a > b ? a : b);
        for (final n in sel) {
          nodes[n.id] = n.copyWith(position: Offset(target - n.size.width, n.position.dy));
        }
      case 'top':
        target = sel.map((n) => n.position.dy).reduce((a, b) => a < b ? a : b);
        for (final n in sel) {
          nodes[n.id] = n.copyWith(position: Offset(n.position.dx, target));
        }
      case 'bottom':
        target = sel.map((n) => n.position.dy + n.size.height).reduce((a, b) => a > b ? a : b);
        for (final n in sel) {
          nodes[n.id] = n.copyWith(position: Offset(n.position.dx, target - n.size.height));
        }
      case 'centerH': // 垂直方向上各节点中心 x 对齐到平均中心
        final cx = sel.map((n) => n.position.dx + n.size.width / 2).reduce((a, b) => a + b) / sel.length;
        for (final n in sel) {
          nodes[n.id] = n.copyWith(position: Offset(cx - n.size.width / 2, n.position.dy));
        }
      case 'centerV':
        final cy = sel.map((n) => n.position.dy + n.size.height / 2).reduce((a, b) => a + b) / sel.length;
        for (final n in sel) {
          nodes[n.id] = n.copyWith(position: Offset(n.position.dx, cy - n.size.height / 2));
        }
      default:
        return;
    }
    state = state.copyWith(nodes: nodes);
  }

  /// 等距分布选中节点。axis: 'h' 水平按 x、'v' 垂直按 y。
  void distributeSelected(String axis) {
    final sel = _selectedNodes;
    if (sel.length < 3) return;
    _pushUndo();
    final nodes = {...state.nodes};
    final horizontal = axis == 'h';
    sel.sort((a, b) => horizontal
        ? a.position.dx.compareTo(b.position.dx)
        : a.position.dy.compareTo(b.position.dy));
    final first = horizontal ? sel.first.position.dx : sel.first.position.dy;
    final last = horizontal ? sel.last.position.dx : sel.last.position.dy;
    final step = (last - first) / (sel.length - 1);
    for (var i = 0; i < sel.length; i++) {
      final n = sel[i];
      final pos = first + step * i;
      nodes[n.id] = n.copyWith(
        position: horizontal ? Offset(pos, n.position.dy) : Offset(n.position.dx, pos),
      );
    }
    state = state.copyWith(nodes: nodes);
  }

  /// 批量复制选中节点（各偏移一点），新副本成为新选区。
  void duplicateSelected() {
    final sel = _selectedNodes;
    if (sel.isEmpty) return;
    _pushUndo();
    final newIds = <String>{};
    for (final n in sel) {
      final id = duplicateNode(n.id);
      if (id != null) newIds.add(id);
    }
    if (newIds.isNotEmpty) selectMany(newIds);
  }

  /// 删除所有选中节点（连带其连线）。
  void deleteSelected() {
    final ids = state.selectedIds.isNotEmpty
        ? state.selectedIds
        : (state.selectedNodeId != null ? {state.selectedNodeId!} : <String>{});
    if (ids.isEmpty) return;
    _pushUndo();
    final nodes = {...state.nodes}..removeWhere((k, _) => ids.contains(k));
    final edges = state.edges
        .where((e) => !ids.contains(e.fromNodeId) && !ids.contains(e.toNodeId))
        .toList();
    final shots = state.shots
        .map((s) => s.copyWith(nodeIds: s.nodeIds.where((n) => !ids.contains(n)).toList()))
        .toList();
    state = state.copyWith(
      nodes: nodes,
      edges: edges,
      shots: shots,
      selectedNodeId: null,
      selectedIds: const {},
    );
  }

  /// 复制一个节点（偏移一点位置），返回新节点 id。
  String? duplicateNode(String id) {
    final node = state.nodes[id];
    if (node == null) return null;
    _pushUndo();
    final newId = _newId('node');
    final copy = node.copyWith(position: node.position + const Offset(40, 40));
    // copyWith 不改 id，手动构造带新 id 的副本。
    final dup = CanvasNode(
      id: newId,
      kind: copy.kind,
      position: copy.position,
      size: copy.size,
      shotId: copy.shotId,
      title: copy.title,
      assetPath: copy.assetPath,
      thumbnailPath: copy.thumbnailPath,
      prompt: copy.prompt,
      params: copy.params,
      status: copy.status,
    );
    state = state.copyWith(nodes: {...state.nodes, newId: dup});
    if (dup.shotId != null) _attachNodeToShot(newId, dup.shotId!);
    return newId;
  }

  // ── 连线 ──

  String addEdge(String fromNodeId, String toNodeId) {
    // 去重 + 防自连
    if (fromNodeId == toNodeId) return '';
    final exists = state.edges.any(
      (e) => e.fromNodeId == fromNodeId && e.toNodeId == toNodeId,
    );
    if (exists) return '';
    _pushUndo();
    final id = _newId('edge');
    state = state.copyWith(
      edges: [...state.edges, CanvasEdge(id: id, fromNodeId: fromNodeId, toNodeId: toNodeId)],
    );
    return id;
  }

  void removeEdge(String id) {
    _pushUndo();
    state = state.copyWith(edges: state.edges.where((e) => e.id != id).toList());
  }

  // ── 手动拖拽连线 ──

  /// 从某节点的出点开始拖拽连线。
  void startLink(String fromNodeId, Offset cursor) {
    state = state.copyWith(linkingFromId: fromNodeId, linkCursor: cursor);
  }

  /// 更新拖拽中的光标位置（画布坐标）。
  void updateLink(Offset cursor) {
    if (state.linkingFromId == null) return;
    state = state.copyWith(linkCursor: cursor);
  }

  /// 在目标节点上松开 → 建立连线。targetNodeId 为 null 表示落空，取消。
  void endLink(String? targetNodeId) {
    final from = state.linkingFromId;
    if (from != null && targetNodeId != null && targetNodeId != from) {
      addEdge(from, targetNodeId);
    }
    state = state.copyWith(linkingFromId: null, linkCursor: null);
  }

  void cancelLink() {
    state = state.copyWith(linkingFromId: null, linkCursor: null);
  }

  /// 拖线到空白处松手 → 在落点创建一个新的下游节点并连线。
  /// kind 决定新节点类型（默认图像）。返回新节点 id。
  String? endLinkAtEmpty(Offset position, {CanvasNodeKind kind = CanvasNodeKind.image}) {
    final from = state.linkingFromId;
    if (from == null) {
      return null;
    }
    final id = addNode(
      kind: kind,
      position: position,
      size: kind == CanvasNodeKind.video ? const Size(220, 160) : const Size(180, 180),
      title: kind == CanvasNodeKind.video ? '合成视频' : '新建图像',
      status: NodeStatus.proposed,
    );
    addEdge(from, id);
    state = state.copyWith(linkingFromId: null, linkCursor: null, selectedNodeId: id, selectedIds: {id});
    return id;
  }

  /// 删除与某节点相关的所有连线（入边+出边）。
  void removeEdgesForNode(String nodeId) {
    _pushUndo();
    state = state.copyWith(
      edges: state.edges
          .where((e) => e.fromNodeId != nodeId && e.toNodeId != nodeId)
          .toList(),
    );
  }

  /// 该节点是否有任何连线。
  bool hasEdges(String nodeId) {
    return state.edges.any((e) => e.fromNodeId == nodeId || e.toNodeId == nodeId);
  }

  /// 命中测试：给定画布坐标，返回落在其上的节点 id（用于连线落点）。
  String? nodeAt(Offset point) {
    for (final node in state.nodes.values) {
      if (node.rect.contains(point)) return node.id;
    }
    return null;
  }

  // ── Shot ──

  String addShot(String title) {
    _pushUndo();
    final id = _newId('shot');
    final order = state.shots.length;
    state = state.copyWith(
      shots: [...state.shots, Shot(id: id, title: title, order: order)],
    );
    return id;
  }

  /// 把节点移动到目标 Shot（从原 Shot 移除，加入新 Shot；shotId=null 表示移出所有 Shot）。
  void moveNodeToShot(String nodeId, String? targetShotId) {
    _pushUndo();
    // 从所有 Shot 移除该节点，再加入目标。
    final shots = state.shots.map((s) {
      final without = s.nodeIds.where((n) => n != nodeId).toList();
      if (s.id == targetShotId && !without.contains(nodeId)) {
        return s.copyWith(nodeIds: [...without, nodeId]);
      }
      return s.copyWith(nodeIds: without);
    }).toList();
    final node = state.nodes[nodeId];
    final nodes = node == null
        ? state.nodes
        : {...state.nodes, nodeId: node.copyWith(shotId: targetShotId)};
    state = state.copyWith(shots: shots, nodes: nodes);
  }

  void renameShot(String shotId, String title) {
    _pushUndo();
    final shots = state.shots
        .map((s) => s.id == shotId ? s.copyWith(title: title) : s)
        .toList();
    state = state.copyWith(shots: shots);
  }

  /// 删除 Shot（仅移除分组，节点变为未分组，不删节点）。
  void removeShot(String shotId) {
    _pushUndo();
    final shots = state.shots.where((s) => s.id != shotId).toList();
    // 该 Shot 下的节点 shotId 清空。
    final nodes = {...state.nodes};
    for (final entry in nodes.entries.toList()) {
      if (entry.value.shotId == shotId) {
        nodes[entry.key] = entry.value.copyWith(shotId: null);
      }
    }
    state = state.copyWith(shots: shots, nodes: nodes);
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

  // ── 就地生成（提议 → 确认 → 执行）──

  /// 提议生成一个图像节点（待用户确认）。返回节点 id。
  /// 可选 sourceNodeId：从某个节点派生（自动连线）。
  String proposeImageNode({
    required String prompt,
    required Offset position,
    String? shotId,
    String? sourceNodeId,
    GenParams params = const GenParams(),
    String title = '',
  }) {
    final id = addNode(
      kind: CanvasNodeKind.image,
      position: position,
      shotId: shotId,
      title: title.isEmpty ? '生成图像' : title,
      prompt: prompt,
      params: params,
      status: NodeStatus.proposed,
    );
    if (sourceNodeId != null) addEdge(sourceNodeId, id);
    return id;
  }

  /// 确认并执行图像生成：proposed → generating → ready/failed。
  /// 调 image.generate（api_key/base_url 由 Core 环境变量提供）。
  Future<void> confirmAndGenerateImage(String nodeId) async {
    final node = state.nodes[nodeId];
    if (node == null || node.prompt == null) return;

    setNodeStatus(nodeId, NodeStatus.generating);
    try {
      final result = await ref.read(coreRepositoryProvider).callCapability(
        'image.generate',
        'generate',
        {
          'prompt': node.prompt,
          'size': _sizeForRatio(node.params.aspectRatio),
          'model': node.params.model,
        },
      );
      final path = _extractOutputPath(result);
      if (path == null) {
        setNodeStatus(nodeId, NodeStatus.failed, errorMessage: '生成未返回图片路径');
        return;
      }
      completeGeneration(nodeId, assetPath: path);
    } catch (e) {
      setNodeStatus(nodeId, NodeStatus.failed, errorMessage: '生成失败：$e');
    }
  }

  /// 批量生成变体：从一个 prompt 一次建 [count] 个图像节点（横向排开），
  /// 各自独立并行生成，在画布上铺出多个变体供挑选。返回新建节点 id 列表。
  Future<List<String>> generateBatch({
    required String prompt,
    required Offset origin,
    GenParams params = const GenParams(),
    int count = 1,
    String? shotId,
    String? sourceNodeId,
  }) async {
    final n = count.clamp(1, 8);
    const stepX = 360.0;
    final ids = <String>[];
    for (var i = 0; i < n; i++) {
      final id = proposeImageNode(
        prompt: prompt,
        position: Offset(origin.dx + i * stepX, origin.dy),
        shotId: shotId,
        sourceNodeId: sourceNodeId,
        params: params,
        title: n > 1 ? '变体 ${i + 1}' : '生成图像',
      );
      ids.add(id);
    }
    // 并行触发各节点生成（每个内部 proposed→generating→ready/failed）。
    await Future.wait(ids.map(confirmAndGenerateImage));
    return ids;
  }

  String _sizeForRatio(String ratio) {
    return switch (ratio) {
      '1:1' => '1024x1024',
      '16:9' => '1280x720',
      '9:16' => '720x1280',
      _ => '1024x1024',
    };
  }

  String? _extractOutputPath(dynamic o) {
    if (o is Map) {
      final p = o['output_path'];
      if (p is String && p.isNotEmpty) return p;
      for (final v in o.values) {
        final r = _extractOutputPath(v);
        if (r != null) return r;
      }
    } else if (o is List) {
      for (final v in o) {
        final r = _extractOutputPath(v);
        if (r != null) return r;
      }
    }
    return null;
  }

  // ── 视频合成（多个图节点 → 一个视频节点）──

  /// 从若干源图节点提议一个视频节点（待确认）。源节点须已 ready 且有 assetPath。
  /// 返回视频节点 id；若无有效源图返回 null。
  String? proposeVideoNode({
    required List<String> sourceImageNodeIds,
    required Offset position,
    String? shotId,
    GenParams params = const GenParams(),
    String title = '合成视频',
  }) {
    final sources = sourceImageNodeIds
        .map((id) => state.nodes[id])
        .whereType<CanvasNode>()
        .where((n) => n.assetPath != null && n.assetPath!.isNotEmpty)
        .toList();
    if (sources.isEmpty) return null;

    final id = addNode(
      kind: CanvasNodeKind.video,
      position: position,
      size: const Size(220, 160),
      shotId: shotId,
      title: title,
      params: params,
      status: NodeStatus.proposed,
    );
    // 源图 → 视频节点连线。
    for (final s in sources) {
      addEdge(s.id, id);
    }
    return id;
  }

  // ── 工作流蓝图（导演 Agent 自动铺 DAG）──

  /// 把一张工作流蓝图渲染成画布上的节点 + 连线（全部 proposed 态，待执行）。
  /// 自动分层布局：图像/文本类节点排上层一行，视频类（汇聚）节点排下层居中。
  /// 返回 蓝图节点id → 画布节点id 的映射，供执行引擎引用。
  Map<String, String> applyBlueprint(WorkflowBlueprint blueprint) {
    if (blueprint.nodes.isEmpty) return const {};
    _pushUndo();

    // 入边计数：有入边的视为下游（视频/汇聚），无入边的视为上游（图像源）。
    final inDegree = <String, int>{for (final n in blueprint.nodes) n.id: 0};
    for (final e in blueprint.edges) {
      inDegree[e.to] = (inDegree[e.to] ?? 0) + 1;
    }
    final upstream = blueprint.nodes.where((n) => (inDegree[n.id] ?? 0) == 0).toList();
    final downstream = blueprint.nodes.where((n) => (inDegree[n.id] ?? 0) > 0).toList();

    // 布局参数。
    const originX = 200.0;
    const upY = 160.0;
    const downY = 520.0;
    const gapX = 280.0;

    final idMap = <String, String>{};

    // 上层：图像/源节点并排。
    for (var i = 0; i < upstream.length; i++) {
      final bn = upstream[i];
      idMap[bn.id] = addNode(
        kind: bn.kind,
        position: Offset(originX + i * gapX, upY),
        title: bn.title.isEmpty ? '生成图像' : bn.title,
        prompt: bn.prompt,
        params: bn.params,
        status: NodeStatus.proposed,
      );
    }

    // 下层：汇聚/视频节点居中于上层之下。
    final centerX = upstream.isEmpty
        ? originX
        : originX + (upstream.length - 1) * gapX / 2;
    for (var i = 0; i < downstream.length; i++) {
      final bn = downstream[i];
      idMap[bn.id] = addNode(
        kind: bn.kind,
        position: Offset(centerX + i * gapX, downY),
        size: bn.kind == CanvasNodeKind.video
            ? const Size(220, 160)
            : const Size(180, 180),
        title: bn.title.isEmpty ? '合成视频' : bn.title,
        prompt: bn.prompt,
        params: bn.params,
        status: NodeStatus.proposed,
      );
    }

    // 连线：按蓝图 edges，用映射后的真实节点 id。
    for (final e in blueprint.edges) {
      final from = idMap[e.from];
      final to = idMap[e.to];
      if (from != null && to != null) addEdge(from, to);
    }

    return idMap;
  }

  /// 确认并执行视频合成：proposed → generating → ready/failed。
  /// 调 video.render/slideshow，images 取自所有指向该视频节点的源图节点。
  Future<void> confirmAndGenerateVideo(String nodeId) async {
    final node = state.nodes[nodeId];
    if (node == null) return;

    // 收集指向该节点的源图（沿 edge 反查）。
    final sourceIds = state.edges
        .where((e) => e.toNodeId == nodeId)
        .map((e) => e.fromNodeId)
        .toList();
    final images = sourceIds
        .map((id) => state.nodes[id])
        .whereType<CanvasNode>()
        .where((n) => n.assetPath != null && n.assetPath!.isNotEmpty)
        .map((n) => n.assetPath!)
        .toList();
    if (images.isEmpty) {
      setNodeStatus(nodeId, NodeStatus.failed, errorMessage: '没有可用的源图片');
      return;
    }

    final durSec = node.params.durationSec.clamp(1, 60);
    final perImage = (durSec / images.length).clamp(1, 10).toDouble();
    final output = 'workspace/agent-test/canvas-${node.id}.mp4';

    setNodeStatus(nodeId, NodeStatus.generating);
    try {
      final result = await ref.read(coreRepositoryProvider).callCapability(
        'video.render',
        'slideshow',
        {
          'images': images,
          'durations': List.filled(images.length, perImage),
          'output': output,
          'size': _sizeForRatio(node.params.aspectRatio),
          'fps': 25,
        },
      );
      final path = _extractOutputPath(result) ?? output;
      completeGeneration(nodeId, assetPath: path, thumbnailPath: images.first);
    } catch (e) {
      setNodeStatus(nodeId, NodeStatus.failed, errorMessage: '视频合成失败：$e');
    }
  }

  // ── DAG 拓扑执行 ──

  /// 执行整张画布 DAG（一键出片）。
  /// 拓扑分层：无入边的图像节点先并行生成 → 它们 ready 后，其下游视频节点汇聚生成。
  /// 只处理 proposed/failed 态节点（已 ready 的跳过，支持部分重跑）。
  /// 返回执行是否整体成功。
  Future<bool> runWorkflow() async {
    // 取当前所有节点的依赖关系快照。
    final nodes = {...state.nodes};
    final edges = [...state.edges];

    bool needsRun(CanvasNode n) =>
        n.status == NodeStatus.proposed || n.status == NodeStatus.failed;

    // 1) 并行生成所有「无入边」且需要跑的图像节点。
    final hasIncoming = <String>{for (final e in edges) e.toNodeId};
    final imageNodes = nodes.values
        .where((n) =>
            n.kind == CanvasNodeKind.image &&
            !hasIncoming.contains(n.id) &&
            needsRun(n) &&
            (n.prompt?.trim().isNotEmpty ?? false))
        .toList();

    await Future.wait(imageNodes.map((n) => confirmAndGenerateImage(n.id)));

    // 2) 视频/汇聚节点：等其所有上游 ready 后再跑（按依赖顺序）。
    //    简单分层：反复扫描，把「所有入边源已 ready」的待跑视频节点生成，直到没有可推进的。
    var progressed = true;
    while (progressed) {
      progressed = false;
      // 用最新 state 取节点（生成会更新 status）。
      final cur = state.nodes;
      final downstream = cur.values
          .where((n) => n.kind == CanvasNodeKind.video && needsRun(n))
          .toList();
      for (final vid in downstream) {
        final srcIds = state.edges
            .where((e) => e.toNodeId == vid.id)
            .map((e) => e.fromNodeId)
            .toList();
        if (srcIds.isEmpty) continue;
        final allReady = srcIds.every((id) =>
            state.nodes[id]?.status == NodeStatus.ready &&
            (state.nodes[id]?.assetPath?.isNotEmpty ?? false));
        if (allReady) {
          await confirmAndGenerateVideo(vid.id);
          progressed = true;
        }
      }
    }

    // 整体成功 = 没有 failed 节点。
    return !state.nodes.values.any((n) => n.status == NodeStatus.failed);
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
