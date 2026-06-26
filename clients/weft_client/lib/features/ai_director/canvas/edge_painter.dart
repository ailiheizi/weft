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
  });

  final Map<String, CanvasNode> nodes;
  final List<CanvasEdge> edges;
  final Color color;
  final double strokeWidth;

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

      // 端点小圆点。
      canvas.drawCircle(start, strokeWidth + 1.5, dot);
      canvas.drawCircle(end, strokeWidth + 1.5, dot);
    }
  }

  @override
  bool shouldRepaint(covariant EdgePainter old) {
    return old.nodes != nodes ||
        old.edges != edges ||
        old.color != color ||
        old.strokeWidth != strokeWidth;
  }
}
