import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'canvas_node_widget.dart';
import 'canvas_state.dart';
import 'edge_painter.dart';
import 'models/canvas_models.dart';
import 'video_player_dialog.dart';

/// 无限画布 — InteractiveViewer 提供平移/缩放，内部 Stack 承载连线层与节点层。
/// 节点拖拽通过节点自身的 onPanUpdate 回写 position（需除以当前缩放比）。
class InfiniteCanvas extends ConsumerStatefulWidget {
  const InfiniteCanvas({super.key});

  @override
  ConsumerState<InfiniteCanvas> createState() => _InfiniteCanvasState();
}

class _InfiniteCanvasState extends ConsumerState<InfiniteCanvas> {
  final _viewer = TransformationController();

  /// 画布逻辑尺寸 — 给一个足够大的固定区域当“无限”画布。
  static const _canvasSize = Size(6000, 6000);

  /// 框选矩形（画布坐标）。非 null 时正在框选。
  Offset? _marqueeStart;
  Offset? _marqueeEnd;

  /// 框选起手的画布坐标 + 屏幕坐标 + 是否已移动（区分点击与框选）。
  Offset? _pointerDownScene;
  Offset _downGlobal = Offset.zero;
  bool _pointerMoved = false;

  /// 中键/右键拖动平移：记录上一帧屏幕坐标，非 null 时正在平移。
  Offset? _panLastGlobal;

  bool get _isMarqueeing => _marqueeStart != null && _marqueeEnd != null;

  /// 是否按下空格（临时进入平移模式）。按住空格拖动 = 平移画布；
  /// 否则空白拖动 = 框选。中键拖动也用于平移。
  bool get _spaceDown =>
      HardwareKeyboard.instance.logicalKeysPressed.contains(LogicalKeyboardKey.space);

  Rect? get _marqueeRect => _isMarqueeing
      ? Rect.fromPoints(_marqueeStart!, _marqueeEnd!)
      : null;

  double get _scale => _viewer.value.getMaxScaleOnAxis();

  /// 全局屏幕坐标 → 画布逻辑坐标。
  Offset _toCanvas(Offset globalPos) {
    final box = context.findRenderObject() as RenderBox?;
    if (box == null) return globalPos;
    final local = box.globalToLocal(globalPos);
    return _viewer.toScene(local);
  }

  /// 最近一次指针位置（全局坐标），用于在右键处弹菜单。
  Offset _lastPointer = Offset.zero;

  /// 节点浮出工具条的动作分发。
  void _onNodeAction(String action, String nodeId) {
    final notifier = ref.read(canvasProvider.notifier);
    notifier.select(nodeId);
    switch (action) {
      case 'regen':
      case 'variant':
        final node = ref.read(canvasProvider).nodes[nodeId];
        if (node == null) return;
        notifier.setNodeStatus(nodeId, NodeStatus.proposed);
        if (node.kind == CanvasNodeKind.video) {
          notifier.confirmAndGenerateVideo(nodeId);
        } else {
          notifier.confirmAndGenerateImage(nodeId);
        }
      case 'play':
        final node = ref.read(canvasProvider).nodes[nodeId];
        if (node?.assetPath != null) {
          VideoPlayerDialog.show(context, node!.assetPath!);
        }
      case 'export':
        final node = ref.read(canvasProvider).nodes[nodeId];
        if (node?.assetPath != null) _exportVideo(context, node!.assetPath!);
      case 'duplicate':
        final id = notifier.duplicateNode(nodeId);
        if (id != null) notifier.select(id);
      case 'delete':
        notifier.deleteSelected();
    }
  }

  Future<void> _exportVideo(BuildContext context, String srcPath) async {
    final src = File(srcPath);
    if (!src.existsSync()) return;
    final dir = await FilePicker.getDirectoryPath(dialogTitle: '选择导出目录');
    if (dir == null) return;
    try {
      final fileName = srcPath.split(RegExp(r'[\\/]')).last;
      await src.copy('$dir${Platform.pathSeparator}$fileName');
      if (context.mounted) {
        ScaffoldMessenger.maybeOf(context)?.showSnackBar(
          SnackBar(content: Text('已导出到 $dir')),
        );
      }
    } catch (_) {}
  }

  /// 拖线到空白处松手 → 弹菜单选新节点类型，在落点建节点并连线。
  Future<void> _showLinkDropMenu(Offset canvasPos) async {
    final notifier = ref.read(canvasProvider.notifier);
    final overlay = Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) {
      notifier.cancelLink();
      return;
    }
    final pos = RelativeRect.fromLTRB(
      _lastPointer.dx,
      _lastPointer.dy,
      overlay.size.width - _lastPointer.dx,
      overlay.size.height - _lastPointer.dy,
    );
    final choice = await showMenu<CanvasNodeKind>(
      context: context,
      position: pos,
      items: const [
        PopupMenuItem(value: CanvasNodeKind.image, child: Text('＋ 图像节点')),
        PopupMenuItem(value: CanvasNodeKind.video, child: Text('＋ 视频节点')),
      ],
    );
    if (choice != null) {
      notifier.endLinkAtEmpty(canvasPos, kind: choice);
    } else {
      notifier.cancelLink();
    }
  }

  Future<void> _showNodeMenu(BuildContext context, String nodeId) async {
    final notifier = ref.read(canvasProvider.notifier);
    final overlay = Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final pos = RelativeRect.fromLTRB(
      _lastPointer.dx,
      _lastPointer.dy,
      overlay.size.width - _lastPointer.dx,
      overlay.size.height - _lastPointer.dy,
    );
    final selectedCount = ref.read(canvasProvider).selectedIds.length;
    final choice = await showMenu<String>(
      context: context,
      position: pos,
      items: [
        const PopupMenuItem(value: 'rename', child: Text('重命名')),
        const PopupMenuItem(value: 'duplicate', child: Text('复制')),
        PopupMenuItem(
          value: 'delete',
          child: Text(selectedCount > 1 ? '删除选中 ($selectedCount)' : '删除'),
        ),
      ],
    );
    if (!context.mounted) return;
    if (choice == 'rename') {
      await _renameNode(context, nodeId);
    } else if (choice == 'duplicate') {
      final id = notifier.duplicateNode(nodeId);
      if (id != null) notifier.select(id);
    } else if (choice == 'delete') {
      notifier.deleteSelected();
    }
  }

  Future<void> _renameNode(BuildContext context, String nodeId) async {
    final node = ref.read(canvasProvider).nodes[nodeId];
    if (node == null) return;
    final controller = TextEditingController(text: node.title);
    final name = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('重命名节点'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(hintText: '节点标题'),
          onSubmitted: (v) => Navigator.pop(ctx, v),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, controller.text),
            child: const Text('确定'),
          ),
        ],
      ),
    );
    if (name != null && name.trim().isNotEmpty) {
      ref.read(canvasProvider.notifier).renameNode(nodeId, name.trim());
    }
  }

  @override
  void dispose() {
    _viewer.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final state = ref.watch(canvasProvider);
    final notifier = ref.read(canvasProvider.notifier);
    final edgeColor = theme.colorScheme.primary.withValues(alpha: 0.55);

    return Listener(
      onPointerSignal: (e) {
        // 滚轮缩放（以光标为中心）。scaleEnabled:false 后由这里接管。
        if (e is PointerScrollEvent) {
          final box = context.findRenderObject() as RenderBox?;
          if (box == null) return;
          final local = box.globalToLocal(e.position);
          final scenePt = _viewer.toScene(local);
          final factor = e.scrollDelta.dy < 0 ? 1.1 : 1 / 1.1;
          final cur = _viewer.value.getMaxScaleOnAxis();
          final next = (cur * factor).clamp(0.2, 4.0);
          final applied = next / cur;
          if (applied == 1.0) return;
          final m = _viewer.value.clone()
            ..translateByDouble(scenePt.dx, scenePt.dy, 0, 1)
            ..scaleByDouble(applied, applied, 1, 1)
            ..translateByDouble(-scenePt.dx, -scenePt.dy, 0, 1);
          _viewer.value = m;
          setState(() {});
        }
      },
      onPointerDown: (e) {
        _lastPointer = e.position;
        // 中键按下 = 平移画布（业界惯例，无需按空格）。右键保留给菜单。
        if (e.buttons == kMiddleMouseButton) {
          _panLastGlobal = e.position;
          return;
        }
        // 用原始指针事件实现框选/平移取消，不进 GestureArena，
        // 因此不会抢上层工具栏/节点的点击（修复"按钮点不动"）。
        // 仅左键、且不在平移模式时，在空白处起手准备框选。
        if (e.buttons == kPrimaryMouseButton && !_spaceDown) {
          _pointerDownScene = _toCanvas(e.position);
          _downGlobal = e.position;
          _pointerMoved = false;
        }
      },
      onPointerMove: (e) {
        // 中键/右键平移：直接按屏幕位移平移变换矩阵。
        if (_panLastGlobal != null) {
          final delta = e.position - _panLastGlobal!;
          _panLastGlobal = e.position;
          final m = _viewer.value.clone()
            ..translateByDouble(delta.dx, delta.dy, 0, 1);
          _viewer.value = m;
          setState(() {});
          return;
        }
        if (_pointerDownScene == null || _spaceDown) return;
        final scene = _toCanvas(e.position);
        // 超过阈值才算框选拖动（避免手抖把点击变框选）。
        if (!_pointerMoved && (e.position - _downGlobal).distance < 4) return;
        _pointerMoved = true;
        setState(() {
          _marqueeStart ??= _pointerDownScene;
          _marqueeEnd = scene;
        });
      },
      onPointerUp: (e) {
        if (_panLastGlobal != null) {
          _panLastGlobal = null;
          return;
        }
        if (_pointerDownScene != null && !_pointerMoved) {
          // 没拖动 = 在空白处点击 → 取消选中。
          notifier.select(null);
        } else if (_isMarqueeing) {
          final rect = _marqueeRect;
          if (rect != null) {
            notifier.selectMany(notifier.nodesInRect(rect));
          }
        }
        setState(() {
          _marqueeStart = null;
          _marqueeEnd = null;
          _pointerDownScene = null;
          _pointerMoved = false;
        });
      },
      child: Stack(
        children: [
          ColoredBox(
        color: theme.colorScheme.surfaceContainerLowest,
        child: InteractiveViewer(
          transformationController: _viewer,
          constrained: false,
          boundaryMargin: const EdgeInsets.all(double.infinity),
          minScale: 0.2,
          maxScale: 4.0,
          // 默认空白拖动=框选（由外层 Listener 处理）；按住空格时 InteractiveViewer 接管=平移。
          panEnabled: _spaceDown,
          // 关键：禁掉 scale 手势识别器。它是 eager 的，会在 GestureArena 抢走工具栏
          // IconButton 的单击（导致按钮"点不动"）。缩放改由滚轮 + 视图控件按钮实现。
          scaleEnabled: false,
        child: SizedBox(
            width: _canvasSize.width,
            height: _canvasSize.height,
            child: Stack(
              clipBehavior: Clip.none,
              children: [
                // 网格背景
                Positioned.fill(
                  child: CustomPaint(painter: _GridPainter(
                    color: theme.colorScheme.outline.withValues(alpha: 0.08),
                  )),
                ),
                // 连线层
                Positioned.fill(
                  child: CustomPaint(
                    painter: EdgePainter(
                      nodes: state.nodes,
                      edges: state.edges,
                      color: edgeColor,
                      linkFrom: state.linkingFromId != null
                          ? state.nodes[state.linkingFromId]?.outPort
                          : null,
                      linkTo: state.linkCursor,
                    ),
                  ),
                ),
                // 节点层
                for (final node in state.nodes.values)
                  Positioned(
                    left: node.position.dx,
                    top: node.position.dy,
                    child: CanvasNodeWidget(
                      node: node,
                      selected: state.isSelected(node.id),
                      onTap: () {
                        // Ctrl+点击 = 多选切换；普通点击 = 单选。
                        if (HardwareKeyboard.instance.isControlPressed) {
                          notifier.toggleSelect(node.id);
                        } else {
                          notifier.select(node.id);
                        }
                      },
                      onSecondaryTap: () {
                        if (!state.isSelected(node.id)) notifier.select(node.id);
                        _showNodeMenu(context, node.id);
                      },
                      onPanStart: () => notifier.beginMove(),
                      onPanUpdate: (delta) {
                        // delta 是屏幕像素，换算回画布逻辑坐标。
                        notifier.moveNode(node.id, delta / _scale);
                      },
                      onPanEnd: () => notifier.endMove(node.id),
                      onLinkStart: (globalPos) {
                        notifier.startLink(node.id, _toCanvas(globalPos));
                      },
                      onLinkUpdate: (globalPos) {
                        notifier.updateLink(_toCanvas(globalPos));
                      },
                      onLinkEnd: () {
                        // 用拖拽过程中记录的画布坐标做命中测试。
                        final cursor = ref.read(canvasProvider).linkCursor;
                        final target = cursor == null ? null : notifier.nodeAt(cursor);
                        if (target != null) {
                          notifier.endLink(target);
                        } else if (cursor != null) {
                          // 落在空白处 → 弹菜单选新节点类型并建节点连线。
                          _showLinkDropMenu(cursor);
                        } else {
                          notifier.cancelLink();
                        }
                      },
                      onResizeStart: () => notifier.beginResize(),
                      onResizeUpdate: (delta) => notifier.resizeNode(node.id, delta / _scale),
                      onToolbarAction: (action) => _onNodeAction(action, node.id),
                    ),
                  ),
                // 框选矩形
                if (_marqueeRect != null)
                  Positioned.fromRect(
                    rect: _marqueeRect!,
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        color: theme.colorScheme.primary.withValues(alpha: 0.12),
                        border: Border.all(color: theme.colorScheme.primary, width: 1),
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ),
          ),
          // 右下角小地图（视图控件上方）
          Positioned(
            right: 12,
            bottom: 56,
            child: IgnorePointer(
              child: Container(
                width: 160,
                height: 110,
                decoration: BoxDecoration(
                  color: theme.colorScheme.surface.withValues(alpha: 0.85),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(color: theme.colorScheme.outline.withValues(alpha: 0.2)),
                ),
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(6),
                  child: CustomPaint(
                    painter: _MinimapPainter(
                      nodes: state.nodes,
                      nodeColor: theme.colorScheme.primary.withValues(alpha: 0.7),
                      selColor: theme.colorScheme.tertiary,
                      selectedIds: state.selectedIds,
                    ),
                  ),
                ),
              ),
            ),
          ),
          // 右下角视图控件
          Positioned(
            right: 12,
            bottom: 12,
            child: _ViewControls(
              scale: _scale,
              onZoomIn: () => _zoom(1.2),
              onZoomOut: () => _zoom(1 / 1.2),
              onReset: _resetView,
              onFit: _fitToContent,
            ),
          ),
        ],
      ),
    );
  }

  void _zoom(double factor) {
    final m = _viewer.value.clone();
    m.scaleByDouble(factor, factor, 1, 1);
    _viewer.value = m;
    setState(() {});
  }

  void _resetView() {
    _viewer.value = Matrix4.identity();
    setState(() {});
  }

  /// 把所有节点缩放/平移到可见区域。
  void _fitToContent() {
    final nodes = ref.read(canvasProvider).nodes.values;
    if (nodes.isEmpty) {
      _resetView();
      return;
    }
    var minX = double.infinity, minY = double.infinity;
    var maxX = -double.infinity, maxY = -double.infinity;
    for (final n in nodes) {
      minX = n.position.dx < minX ? n.position.dx : minX;
      minY = n.position.dy < minY ? n.position.dy : minY;
      final r = n.position.dx + n.size.width;
      final b = n.position.dy + n.size.height;
      maxX = r > maxX ? r : maxX;
      maxY = b > maxY ? b : maxY;
    }
    const pad = 80.0;
    final contentW = (maxX - minX) + pad * 2;
    final contentH = (maxY - minY) + pad * 2;
    final box = context.findRenderObject() as RenderBox?;
    if (box == null) return;
    final vw = box.size.width, vh = box.size.height;
    final scale = (vw / contentW).clamp(0.2, 2.0) < (vh / contentH).clamp(0.2, 2.0)
        ? (vw / contentW)
        : (vh / contentH);
    final s = scale.clamp(0.2, 2.0);
    final m = Matrix4.identity()
      ..scaleByDouble(s, s, 1, 1)
      ..setTranslationRaw(
        -(minX - pad) * s + (vw - contentW * s) / 2,
        -(minY - pad) * s + (vh - contentH * s) / 2,
        0,
      );
    _viewer.value = m;
    setState(() {});
  }
}

/// 小地图：把所有节点的包围盒缩放进小窗口，画出节点缩略块。
class _MinimapPainter extends CustomPainter {
  _MinimapPainter({
    required this.nodes,
    required this.nodeColor,
    required this.selColor,
    required this.selectedIds,
  });

  final Map<String, CanvasNode> nodes;
  final Color nodeColor;
  final Color selColor;
  final Set<String> selectedIds;

  @override
  void paint(Canvas canvas, Size size) {
    if (nodes.isEmpty) return;
    // 计算所有节点的包围盒。
    var minX = double.infinity, minY = double.infinity;
    var maxX = -double.infinity, maxY = -double.infinity;
    for (final n in nodes.values) {
      minX = n.position.dx < minX ? n.position.dx : minX;
      minY = n.position.dy < minY ? n.position.dy : minY;
      final r = n.position.dx + n.size.width;
      final b = n.position.dy + n.size.height;
      maxX = r > maxX ? r : maxX;
      maxY = b > maxY ? b : maxY;
    }
    const pad = 40.0;
    final contentW = (maxX - minX) + pad * 2;
    final contentH = (maxY - minY) + pad * 2;
    if (contentW <= 0 || contentH <= 0) return;
    final scale = (size.width / contentW) < (size.height / contentH)
        ? size.width / contentW
        : size.height / contentH;
    // 居中偏移。
    final offX = (size.width - contentW * scale) / 2;
    final offY = (size.height - contentH * scale) / 2;

    Offset map(Offset p) => Offset(
          offX + (p.dx - minX + pad) * scale,
          offY + (p.dy - minY + pad) * scale,
        );

    final paint = Paint()..style = PaintingStyle.fill;
    for (final n in nodes.values) {
      final tl = map(n.position);
      final rect = Rect.fromLTWH(
        tl.dx,
        tl.dy,
        (n.size.width * scale).clamp(2.0, size.width),
        (n.size.height * scale).clamp(2.0, size.height),
      );
      paint.color = selectedIds.contains(n.id) ? selColor : nodeColor;
      canvas.drawRRect(
        RRect.fromRectAndRadius(rect, const Radius.circular(1.5)),
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _MinimapPainter old) =>
      old.nodes != nodes || old.selectedIds != selectedIds;
}

/// 画布网格背景。
class _GridPainter extends CustomPainter {
  _GridPainter({required this.color});

  final Color color;  static const double step = 40;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..color = color..strokeWidth = 1;
    for (double x = 0; x < size.width; x += step) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), paint);
    }
    for (double y = 0; y < size.height; y += step) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paint);
    }
  }

  @override
  bool shouldRepaint(covariant _GridPainter old) => old.color != color;
}

/// 画布右下角视图控件：缩放百分比 + 放大/缩小/适应/回原点。
class _ViewControls extends StatelessWidget {
  const _ViewControls({
    required this.scale,
    required this.onZoomIn,
    required this.onZoomOut,
    required this.onReset,
    required this.onFit,
  });

  final double scale;
  final VoidCallback onZoomIn;
  final VoidCallback onZoomOut;
  final VoidCallback onReset;
  final VoidCallback onFit;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: theme.colorScheme.surface,
      borderRadius: BorderRadius.circular(10),
      elevation: 2,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            IconButton(
              tooltip: '适应内容',
              visualDensity: VisualDensity.compact,
              icon: const Icon(Icons.fit_screen_outlined, size: 18),
              onPressed: onFit,
            ),
            IconButton(
              tooltip: '回到原点',
              visualDensity: VisualDensity.compact,
              icon: const Icon(Icons.center_focus_strong_outlined, size: 18),
              onPressed: onReset,
            ),
            IconButton(
              tooltip: '缩小',
              visualDensity: VisualDensity.compact,
              icon: const Icon(Icons.remove, size: 18),
              onPressed: onZoomOut,
            ),
            SizedBox(
              width: 44,
              child: Text(
                '${(scale * 100).round()}%',
                textAlign: TextAlign.center,
                style: theme.textTheme.bodySmall,
              ),
            ),
            IconButton(
              tooltip: '放大',
              visualDensity: VisualDensity.compact,
              icon: const Icon(Icons.add, size: 18),
              onPressed: onZoomIn,
            ),
          ],
        ),
      ),
    );
  }
}
