import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'canvas_node_widget.dart';
import 'canvas_state.dart';
import 'edge_painter.dart';

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

  double get _scale => _viewer.value.getMaxScaleOnAxis();

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

    return ColoredBox(
      color: theme.colorScheme.surfaceContainerLowest,
      child: InteractiveViewer(
        transformationController: _viewer,
        constrained: false,
        boundaryMargin: const EdgeInsets.all(double.infinity),
        minScale: 0.2,
        maxScale: 4.0,
        child: GestureDetector(
          behavior: HitTestBehavior.translucent,
          onTap: () => notifier.select(null),
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
                      selected: state.selectedNodeId == node.id,
                      onTap: () => notifier.select(node.id),
                      onPanUpdate: (delta) {
                        // delta 是屏幕像素，换算回画布逻辑坐标。
                        notifier.moveNode(node.id, delta / _scale);
                      },
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// 画布网格背景。
class _GridPainter extends CustomPainter {
  _GridPainter({required this.color});

  final Color color;
  static const double step = 40;

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
