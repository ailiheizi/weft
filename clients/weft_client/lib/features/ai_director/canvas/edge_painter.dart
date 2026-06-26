import 'package:flutter/material.dart';

import 'models/canvas_models.dart';

/// 画连线层 — 节点 outPort → inPort 的贝塞尔曲线。
/// 坐标为画布逻辑坐标，绘制在与节点同一个 Stack 坐标系内
/// （由外层 Transform/InteractiveViewer 统一变换）。
class EdgePainter extends CustomPainter {
  EdgePainter({
    required this.nodes,
    required this.edges,
    required this.color,
    this.strokeWidth = 2.0,
    this.linkFrom,
    this.linkTo,
  });

  final Map<String, CanvasNode> nodes;
  final List<CanvasEdge> edges;
  final Color color;
  final double strokeWidth;

  /// 拖拽中的临时连线起点/终点（画布坐标）。两者皆非空时绘制虚线预览。
  final Offset? linkFrom;
  final Offset? linkTo;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = strokeWidth
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;

    final dot = Paint()
      ..color = color
      ..style = PaintingStyle.fill;

    for (final edge in edges) {
      final from = nodes[edge.fromNodeId];
      final to = nodes[edge.toNodeId];
      if (from == null || to == null) continue;

      final start = from.outPort;
      final end = to.inPort;

      // 水平张力的三次贝塞尔，营造平滑的流向感。
      final dx = (end.dx - start.dx).abs();
      final tension = (dx * 0.5).clamp(40.0, 160.0);
      final c1 = Offset(start.dx + tension, start.dy);
      final c2 = Offset(end.dx - tension, end.dy);

      final path = Path()
        ..moveTo(start.dx, start.dy)
        ..cubicTo(c1.dx, c1.dy, c2.dx, c2.dy, end.dx, end.dy);
      canvas.drawPath(path, paint);

      // 起点小圆点。
      canvas.drawCircle(start, strokeWidth + 1.5, dot);
      // 终点方向箭头（沿曲线末端切线 end-c2 方向），表达数据流向。
      _drawArrow(canvas, end, end - c2, dot);
    }

    // 拖拽中的临时连线（实心曲线 + 终点圆点）。
    if (linkFrom != null && linkTo != null) {
      final tempPaint = Paint()
        ..color = color.withValues(alpha: 0.9)
        ..strokeWidth = strokeWidth
        ..style = PaintingStyle.stroke
        ..strokeCap = StrokeCap.round;
      final dx = (linkTo!.dx - linkFrom!.dx).abs();
      final tension = (dx * 0.5).clamp(40.0, 160.0);
      final path = Path()
        ..moveTo(linkFrom!.dx, linkFrom!.dy)
        ..cubicTo(
          linkFrom!.dx + tension, linkFrom!.dy,
          linkTo!.dx - tension, linkTo!.dy,
          linkTo!.dx, linkTo!.dy,
        );
      canvas.drawPath(path, tempPaint);
      canvas.drawCircle(linkTo!, strokeWidth + 2, dot);
    }
  }

  /// 在 [tip] 处画一个朝 [dir] 方向的实心三角箭头。
  void _drawArrow(Canvas canvas, Offset tip, Offset dir, Paint fill) {
    final len = dir.distance;
    if (len < 0.001) return;
    final ux = dir.dx / len;
    final uy = dir.dy / len;
    const size = 9.0; // 箭头长度
    const half = 5.0; // 箭头半宽
    // 底边中点 = tip 沿反方向退 size。
    final bx = tip.dx - ux * size;
    final by = tip.dy - uy * size;
    // 垂直方向。
    final px = -uy;
    final py = ux;
    final p1 = Offset(bx + px * half, by + py * half);
    final p2 = Offset(bx - px * half, by - py * half);
    final path = Path()
      ..moveTo(tip.dx, tip.dy)
      ..lineTo(p1.dx, p1.dy)
      ..lineTo(p2.dx, p2.dy)
      ..close();
    canvas.drawPath(path, fill);
  }

  @override
  bool shouldRepaint(covariant EdgePainter old) {
    return old.nodes != nodes ||
        old.edges != edges ||
        old.color != color ||
        old.strokeWidth != strokeWidth ||
        old.linkFrom != linkFrom ||
        old.linkTo != linkTo;
  }
}
