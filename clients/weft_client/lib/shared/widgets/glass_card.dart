import 'package:flutter/material.dart';

/// Linear 风立面令牌——集中定义，供 [GlassCard]、HoverCard 等复用。
///
/// 核心原则：卡片**完全不透明**且**明显比背景亮**（强图底对比 → 卡片真正"浮"
/// 起来），靠提亮 + hairline 描边 + 阴影造层次。零 BackdropFilter。
class GlassTokens {
  GlassTokens._();

  /// 卡片统一圆角（Linear 偏小）。
  static const double radius = 10;

  /// 内层面板 / 图标 tile 圆角。
  static const double radiusInner = 8;

  /// 卡片填充：明显比 #0E0F13 背景亮，制造图底对比。
  static const Color fill = Color(0xFF181A20);

  /// 抬升档（hover / 英雄卡）。
  static const Color fillRaised = Color(0xFF21242D);

  /// 列表行平铺档。
  static const Color fillFlat = Color(0xFF161820);

  /// hairline 描边 冷白@0.09。
  static const Color borderIdle = Color(0x17FFFFFF);

  /// hover 边框 冷白@0.15。
  static const Color borderHover = Color(0x26FFFFFF);

  /// 顶边方向高光 冷白@0.06 → 0。
  static const Gradient specular = LinearGradient(
    begin: Alignment.topCenter,
    end: Alignment.bottomCenter,
    colors: [Color(0x0FFFFFFF), Color(0x00FFFFFF)],
  );

  /// 卡片内嵌小块（图标 tile / 代码块）内凹填充。
  static const Color innerTileFill = Color(0xFF0A0B0E);

  // ── 主题感知解析(亮/暗自动取对应 ColorScheme 令牌) ──────────────────────
  // 暗色常量与暗色 ColorScheme 的 surface 令牌一致;亮色走 ColorScheme 自然变浅。
  static Color fillOf(BuildContext c) =>
      Theme.of(c).colorScheme.surfaceContainer;
  static Color fillRaisedOf(BuildContext c) =>
      Theme.of(c).colorScheme.surfaceContainerHigh;
  static Color fillFlatOf(BuildContext c) =>
      Theme.of(c).colorScheme.surface;
  static Color innerTileFillOf(BuildContext c) =>
      Theme.of(c).colorScheme.surfaceContainerLowest;
  static Color borderIdleOf(BuildContext c) =>
      Theme.of(c).colorScheme.outline;
  static Color borderHoverOf(BuildContext c) =>
      Theme.of(c).colorScheme.outlineVariant == Theme.of(c).colorScheme.outline
          ? Theme.of(c).colorScheme.outline
          : Theme.of(c).colorScheme.onSurfaceVariant.withValues(alpha: 0.3);

  /// 双层阴影：贴地接触阴影 + 远投柔影（深底上更明显）。
  static const List<BoxShadow> shadows = [
    BoxShadow(color: Color(0x66000000), blurRadius: 1, offset: Offset(0, 1)),
    BoxShadow(
      color: Color(0x4D000000),
      blurRadius: 16,
      offset: Offset(0, 6),
      spreadRadius: -2,
    ),
  ];
}

/// 不透明立面卡：纯色填充（明显亮于背景）+ 1px 冷白边 + 顶部高光 + 双层阴影。
///
/// 永不透明 → 永不发灰；明显亮于背景 → 真正浮起。零 [BackdropFilter]。
class GlassCard extends StatelessWidget {
  const GlassCard({
    super.key,
    required this.child,
    this.radius = GlassTokens.radius,
    this.padding,
    this.borderColor,
    this.elevated = false,
  });

  final Widget child;
  final double radius;
  final EdgeInsetsGeometry? padding;

  /// 覆盖边框色（如 hover 时传 borderHover）。
  final Color? borderColor;

  /// 是否抬升一档亮度（英雄统计卡）。
  final bool elevated;

  @override
  Widget build(BuildContext context) {
    final r = BorderRadius.circular(radius);

    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: r,
        color: elevated ? GlassTokens.fillRaisedOf(context) : GlassTokens.fillOf(context),
        border: Border.all(
          color: borderColor ?? GlassTokens.borderIdleOf(context),
          width: 1,
        ),
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
              height: radius * 1.5,
              child: const DecoratedBox(
                decoration: BoxDecoration(gradient: GlassTokens.specular),
              ),
            ),
            Padding(padding: padding ?? EdgeInsets.zero, child: child),
          ],
        ),
      ),
    );
  }
}
