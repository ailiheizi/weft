import 'dart:math' as math;
import 'package:flutter/material.dart';

/// 轻量 sparkline 迷你趋势图（CustomPainter，无第三方依赖）。
///
/// 用渐变描线 + 末点高亮 + 线下极淡填充。数据由调用方提供；当无真实历史
/// 序列时，用 [seeded] 基于一个种子值生成确定性平滑曲线作视觉点缀
/// （不随重绘抖动）。
class Sparkline extends StatelessWidget {
  const Sparkline({
    super.key,
    required this.data,
    required this.color,
    this.height = 28,
  });

  final List<double> data;
  final Color color;
  final double height;

  /// 基于种子生成确定性的平滑波形（8 个点）。仅作装饰，非真实趋势。
  factory Sparkline.seeded({
    Key? key,
    required int seed,
    required Color color,
    double height = 28,
  }) {
    final pts = <double>[];
    for (var i = 0; i < 8; i++) {
      // 确定性：正弦叠加 + 种子相位，范围 0.25~0.95。
      final v = 0.6 +
          0.32 * math.sin(i * 0.9 + seed * 0.7) +
          0.10 * math.sin(i * 2.1 + seed);
      pts.add(v.clamp(0.18, 0.98));
    }
    return Sparkline(key: key, data: pts, color: color, height: height);
  }

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: height,
      width: double.infinity,
      child: CustomPaint(painter: _SparklinePainter(data, color)),
    );
  }
}

class _SparklinePainter extends CustomPainter {
  _SparklinePainter(this.data, this.color);
  final List<double> data;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    if (data.length < 2) return;
    final maxV = data.reduce(math.max);
    final minV = data.reduce(math.min);
    final range = (maxV - minV).abs() < 1e-6 ? 1.0 : (maxV - minV);

    final dx = size.width / (data.length - 1);
    Offset pointAt(int i) {
      final norm = (data[i] - minV) / range;
      return Offset(dx * i, size.height - norm * size.height);
    }

    // 平滑路径（Catmull-Rom 近似为二次贝塞尔）。
    final path = Path()..moveTo(pointAt(0).dx, pointAt(0).dy);
    for (var i = 0; i < data.length - 1; i++) {
      final p0 = pointAt(i);
      final p1 = pointAt(i + 1);
      final mid = Offset((p0.dx + p1.dx) / 2, (p0.dy + p1.dy) / 2);
      path.quadraticBezierTo(p0.dx, p0.dy, mid.dx, mid.dy);
    }
    final last = pointAt(data.length - 1);
    path.lineTo(last.dx, last.dy);

    // 线下填充。
    final fill = Path.from(path)
      ..lineTo(size.width, size.height)
      ..lineTo(0, size.height)
      ..close();
    canvas.drawPath(
      fill,
      Paint()
        ..shader = LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [color.withValues(alpha: 0.18), color.withValues(alpha: 0)],
        ).createShader(Offset.zero & size),
    );

    // 描线。
    canvas.drawPath(
      path,
      Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.6
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round
        ..color = color.withValues(alpha: 0.9),
    );

    // 末点高亮。
    canvas.drawCircle(last, 2.2, Paint()..color = color);
  }

  @override
  bool shouldRepaint(_SparklinePainter old) =>
      old.data != data || old.color != color;
}
