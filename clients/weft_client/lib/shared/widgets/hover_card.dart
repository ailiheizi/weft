import 'package:flutter/material.dart';
import 'glass_card.dart';

/// 带 hover 效果的立面卡片容器（Ember Editorial）：
/// - 不透明竖向渐变填充 + 顶部方向高光 + 双层阴影
/// - idle 暖白 hairline 边；hover 整面抬升一档亮度 + 边框提亮
/// - 仅 140ms 换色，无位移、无 scale、无辉光
class HoverCard extends StatefulWidget {
  const HoverCard({
    super.key,
    required this.child,
    this.margin = const EdgeInsets.only(bottom: 8),
    this.onTap,
    this.flat = false,
  });

  final Widget child;
  final EdgeInsetsGeometry margin;
  final VoidCallback? onTap;

  /// 列表行平铺档：用最低立面单色，而非渐变（减弱卡片感）。
  final bool flat;

  @override
  State<HoverCard> createState() => _HoverCardState();
}

class _HoverCardState extends State<HoverCard> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    final r = BorderRadius.circular(GlassTokens.radius);
    final borderColor =
        _hovered ? GlassTokens.borderHover : GlassTokens.borderIdle;

    // idle: flat 用最低档，否则默认档；hover: 抬升档。
    final Color color = _hovered
        ? GlassTokens.fillRaised
        : (widget.flat ? GlassTokens.fillFlat : GlassTokens.fill);

    return MouseRegion(
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      cursor: widget.onTap != null
          ? SystemMouseCursors.click
          : SystemMouseCursors.basic,
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 140),
          curve: Curves.easeOut,
          margin: widget.margin,
          decoration: BoxDecoration(
            color: color,
            borderRadius: r,
            border: Border.all(color: borderColor, width: 1),
            boxShadow: GlassTokens.shadows,
          ),
          child: ClipRRect(
            borderRadius: r,
            child: Stack(
              children: [
                // 顶部方向高光。
                Positioned(
                  top: 0,
                  left: 0,
                  right: 0,
                  height: GlassTokens.radius * 1.5,
                  child: const DecoratedBox(
                    decoration: BoxDecoration(gradient: GlassTokens.specular),
                  ),
                ),
                widget.child,
              ],
            ),
          ),
        ),
      ),
    );
  }
}
