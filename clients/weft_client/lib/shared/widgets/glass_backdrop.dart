import 'package:flutter/material.dart';

/// 全局静态背景：在 [MaterialApp.builder] 处包裹整个 app 内容一次。
///
/// 用 [RepaintBoundary] 缓存——零逐帧成本。
///
/// Linear 风：几乎纯冷近黑画布，仅顶部一抹极淡的冷光增加纵深，绝不抢戏。
/// 真正的层次靠"明显更亮的卡片"提供，而非背景花哨。
class GlassBackdrop extends StatelessWidget {
  const GlassBackdrop({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        const Positioned.fill(
          child: RepaintBoundary(child: _Ambience()),
        ),
        child,
      ],
    );
  }
}

class _Ambience extends StatelessWidget {
  const _Ambience();

  @override
  Widget build(BuildContext context) {
    return const DecoratedBox(
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [Color(0xFF13151B), Color(0xFF0E0F13)],
          stops: [0.0, 0.5],
        ),
      ),
    );
  }
}
