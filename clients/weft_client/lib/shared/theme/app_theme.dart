import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

/// 状态色 + 等宽数字令牌，挂在 ThemeExtension 上供组件读取。
@immutable
class AppSurfaces extends ThemeExtension<AppSurfaces> {
  const AppSurfaces({
    required this.statusOk,
    required this.statusWarn,
    required this.statusError,
    required this.mono,
  });

  /// 去饱和状态色。
  final Color statusOk;
  final Color statusWarn;
  final Color statusError;

  /// 等宽数字样式（tabular figures）。
  final TextStyle mono;

  @override
  AppSurfaces copyWith({
    Color? statusOk,
    Color? statusWarn,
    Color? statusError,
    TextStyle? mono,
  }) {
    return AppSurfaces(
      statusOk: statusOk ?? this.statusOk,
      statusWarn: statusWarn ?? this.statusWarn,
      statusError: statusError ?? this.statusError,
      mono: mono ?? this.mono,
    );
  }

  @override
  AppSurfaces lerp(ThemeExtension<AppSurfaces>? other, double t) {
    if (other is! AppSurfaces) return this;
    return AppSurfaces(
      statusOk: Color.lerp(statusOk, other.statusOk, t)!,
      statusWarn: Color.lerp(statusWarn, other.statusWarn, t)!,
      statusError: Color.lerp(statusError, other.statusError, t)!,
      mono: TextStyle.lerp(mono, other.mono, t)!,
    );
  }
}

class AppTheme {
  AppTheme._();

  // ── Linear 冷调深蓝黑调色板 ────────────────────────────────────────────
  static const _baseBg = Color(0xFF0E0F13); // 冷近黑画布
  static const _surface = Color(0xFF181A20); // 卡片：明显比底亮，强图底对比
  static const _surfaceRaised = Color(0xFF21242D); // hover / 抬升
  static const _surfaceInset = Color(0xFF0A0B0E); // 输入框 / 代码块内凹
  static const _textPrimary = Color(0xFFF7F8F8); // 冷近白
  static const _textMuted = Color(0xFF8A8F98); // Linear 同款次级灰
  static const _textTertiary = Color(0xFF6E7178); // 三级
  static const _border = Color(0x17FFFFFF); // hairline @0.09 冷白
  static const _divider = Color(0x0FFFFFFF); // @0.06
  static const _accent = Color(0xFF5E6AD2); // Linear 品牌蓝紫
  static const _accentHover = Color(0xFF6E7AE0);
  static const _onAccent = Color(0xFFFFFFFF);
  static const _accentRowTint = Color(0x1F5E6AD2); // 当前行底 @0.12
  static const _statusOk = Color(0xFF4CB782); // Linear 绿
  static const _statusWarn = Color(0xFFF2C94C);
  static const _statusError = Color(0xFFEB5757);

  static ThemeData get dark {
    const scheme = ColorScheme.dark(
      brightness: Brightness.dark,
      primary: _accent,
      onPrimary: _onAccent,
      primaryContainer: _accentRowTint,
      onPrimaryContainer: _accent,
      secondary: _accentHover,
      onSecondary: _onAccent,
      surface: _surface,
      onSurface: _textPrimary,
      onSurfaceVariant: _textMuted,
      surfaceContainerLowest: _surfaceInset,
      surfaceContainerLow: _baseBg,
      surfaceContainer: _surface,
      surfaceContainerHigh: _surfaceRaised,
      surfaceContainerHighest: _surfaceRaised,
      surfaceDim: _baseBg,
      outline: _border,
      outlineVariant: _divider,
      error: _statusError,
      onError: _textPrimary,
    );

    final base = ThemeData(
      useMaterial3: true,
      colorScheme: scheme,
      scaffoldBackgroundColor: _baseBg,
    );

    // ── 全 Inter（Linear 风：无衬线，紧凑），数字用等宽 tabular ────────────
    final inter = GoogleFonts.interTextTheme(base.textTheme);
    final mono = GoogleFonts.jetBrainsMono(
      fontSize: 13,
      fontWeight: FontWeight.w500,
      color: _textPrimary,
      fontFeatures: const [FontFeature.tabularFigures()],
    );

    TextStyle h(double size, {double spacing = -0.4}) => GoogleFonts.inter(
          fontWeight: FontWeight.w600,
          fontSize: size,
          letterSpacing: spacing,
          color: _textPrimary,
          height: 1.2,
        );

    final textTheme = inter
        .copyWith(
          displayLarge: h(30, spacing: -0.6),
          displaySmall: h(24, spacing: -0.5),
          headlineSmall: h(21, spacing: -0.5),
          titleMedium: h(16, spacing: -0.3),
          titleSmall: GoogleFonts.inter(
            fontSize: 13,
            fontWeight: FontWeight.w600,
            letterSpacing: -0.1,
            color: _textPrimary,
          ),
          // SECTION LABEL: 小型大写标签。
          labelLarge: GoogleFonts.inter(
            fontSize: 11,
            fontWeight: FontWeight.w600,
            letterSpacing: 0.6,
            color: _textMuted,
          ),
          bodyMedium: GoogleFonts.inter(
            fontSize: 14,
            fontWeight: FontWeight.w400,
            height: 1.5,
            letterSpacing: -0.08,
            color: _textPrimary,
          ),
          bodySmall: GoogleFonts.inter(
            fontSize: 12,
            fontWeight: FontWeight.w400,
            height: 1.45,
            color: _textTertiary,
          ),
        )
        .apply(bodyColor: _textPrimary, displayColor: _textPrimary);

    return base.copyWith(
      textTheme: textTheme,
      extensions: <ThemeExtension<dynamic>>[
        AppSurfaces(
          statusOk: _statusOk,
          statusWarn: _statusWarn,
          statusError: _statusError,
          mono: mono,
        ),
      ],
      dividerTheme: const DividerThemeData(color: _divider, space: 1),
      cardTheme: CardThemeData(
        color: _surface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
          side: const BorderSide(color: _border),
        ),
      ),
      navigationRailTheme: const NavigationRailThemeData(
        backgroundColor: Colors.transparent,
        indicatorColor: _accentRowTint,
        selectedIconTheme: IconThemeData(color: _accent, size: 20),
        unselectedIconTheme: IconThemeData(color: _textMuted, size: 20),
        labelType: NavigationRailLabelType.none,
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: _surfaceInset,
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: _border),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: _border),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: _accent, width: 2),
        ),
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        isDense: true,
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: _accent,
          foregroundColor: _onAccent,
          elevation: 0,
          textStyle: const TextStyle(fontWeight: FontWeight.w600),
          shape:
              RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: _accent,
          foregroundColor: _onAccent,
          textStyle: const TextStyle(fontWeight: FontWeight.w600),
          shape:
              RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: _textPrimary,
          side: const BorderSide(color: _border),
          shape:
              RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        ),
      ),
      chipTheme: ChipThemeData(
        backgroundColor: _surfaceRaised,
        side: const BorderSide(color: _border),
        shape:
            RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
        labelStyle: const TextStyle(color: _textPrimary, fontSize: 12),
      ),
      listTileTheme: const ListTileThemeData(
        tileColor: Colors.transparent,
        selectedTileColor: _accentRowTint,
      ),
    );
  }
}
