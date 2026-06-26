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

/// 一套配色调色板(暗/亮各一份),喂给 _build 派生完整 ThemeData。
@immutable
class _Palette {
  const _Palette({
    required this.brightness,
    required this.baseBg,
    required this.surface,
    required this.surfaceRaised,
    required this.surfaceInset,
    required this.textPrimary,
    required this.textMuted,
    required this.textTertiary,
    required this.border,
    required this.divider,
    required this.accent,
    required this.accentHover,
    required this.onAccent,
    required this.accentRowTint,
    required this.statusOk,
    required this.statusWarn,
    required this.statusError,
  });

  final Brightness brightness;
  final Color baseBg;
  final Color surface;
  final Color surfaceRaised;
  final Color surfaceInset;
  final Color textPrimary;
  final Color textMuted;
  final Color textTertiary;
  final Color border;
  final Color divider;
  final Color accent;
  final Color accentHover;
  final Color onAccent;
  final Color accentRowTint;
  final Color statusOk;
  final Color statusWarn;
  final Color statusError;
}

class AppTheme {
  AppTheme._();

  // ── Linear 冷调深蓝黑(暗色) ────────────────────────────────────────────
  static const _darkPalette = _Palette(
    brightness: Brightness.dark,
    baseBg: Color(0xFF0E0F13),
    surface: Color(0xFF181A20),
    surfaceRaised: Color(0xFF21242D),
    surfaceInset: Color(0xFF0A0B0E),
    textPrimary: Color(0xFFF7F8F8),
    textMuted: Color(0xFF8A8F98),
    textTertiary: Color(0xFF6E7178),
    border: Color(0x17FFFFFF),
    divider: Color(0x0FFFFFFF),
    accent: Color(0xFF5E6AD2),
    accentHover: Color(0xFF6E7AE0),
    onAccent: Color(0xFFFFFFFF),
    accentRowTint: Color(0x1F5E6AD2),
    statusOk: Color(0xFF4CB782),
    statusWarn: Color(0xFFF2C94C),
    statusError: Color(0xFFEB5757),
  );

  // ── 亮色(冷调浅灰,与暗色同品牌蓝紫) ──────────────────────────────────
  static const _lightPalette = _Palette(
    brightness: Brightness.light,
    baseBg: Color(0xFFFBFBFC), // 近白画布
    surface: Color(0xFFFFFFFF), // 卡片纯白
    surfaceRaised: Color(0xFFF1F2F5), // hover/抬升 浅灰
    surfaceInset: Color(0xFFF4F5F7), // 输入框/代码块内凹
    textPrimary: Color(0xFF1C1D21), // 冷近黑
    textMuted: Color(0xFF6B7079), // 次级灰
    textTertiary: Color(0xFF8A8F98), // 三级
    border: Color(0x14000000), // hairline @0.08 黑
    divider: Color(0x0A000000), // @0.04
    accent: Color(0xFF5E6AD2), // 同品牌蓝紫
    accentHover: Color(0xFF4E5AC0),
    onAccent: Color(0xFFFFFFFF),
    accentRowTint: Color(0x1A5E6AD2), // 当前行底 @0.10
    statusOk: Color(0xFF2E9E6B),
    statusWarn: Color(0xFFC79A1E),
    statusError: Color(0xFFD64545),
  );

  static ThemeData get dark => _build(_darkPalette);
  static ThemeData get light => _build(_lightPalette);

  static ThemeData _build(_Palette p) {
    final scheme = ColorScheme(
      brightness: p.brightness,
      primary: p.accent,
      onPrimary: p.onAccent,
      primaryContainer: p.accentRowTint,
      onPrimaryContainer: p.accent,
      secondary: p.accentHover,
      onSecondary: p.onAccent,
      surface: p.surface,
      onSurface: p.textPrimary,
      onSurfaceVariant: p.textMuted,
      surfaceContainerLowest: p.surfaceInset,
      surfaceContainerLow: p.baseBg,
      surfaceContainer: p.surface,
      surfaceContainerHigh: p.surfaceRaised,
      surfaceContainerHighest: p.surfaceRaised,
      surfaceDim: p.baseBg,
      outline: p.border,
      outlineVariant: p.divider,
      error: p.statusError,
      onError: p.onAccent,
    );

    final base = ThemeData(
      useMaterial3: true,
      colorScheme: scheme,
      scaffoldBackgroundColor: p.baseBg,
    );

    // ── 全 Inter（Linear 风：无衬线，紧凑），数字用等宽 tabular ────────────
    final inter = GoogleFonts.interTextTheme(base.textTheme);
    final mono = GoogleFonts.jetBrainsMono(
      fontSize: 13,
      fontWeight: FontWeight.w500,
      color: p.textPrimary,
      fontFeatures: const [FontFeature.tabularFigures()],
    );

    TextStyle h(double size, {double spacing = -0.4}) => GoogleFonts.inter(
          fontWeight: FontWeight.w600,
          fontSize: size,
          letterSpacing: spacing,
          color: p.textPrimary,
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
            color: p.textPrimary,
          ),
          // SECTION LABEL: 小型大写标签。
          labelLarge: GoogleFonts.inter(
            fontSize: 11,
            fontWeight: FontWeight.w600,
            letterSpacing: 0.6,
            color: p.textMuted,
          ),
          bodyMedium: GoogleFonts.inter(
            fontSize: 14,
            fontWeight: FontWeight.w400,
            height: 1.5,
            letterSpacing: -0.08,
            color: p.textPrimary,
          ),
          bodySmall: GoogleFonts.inter(
            fontSize: 12,
            fontWeight: FontWeight.w400,
            height: 1.45,
            color: p.textTertiary,
          ),
        )
        .apply(bodyColor: p.textPrimary, displayColor: p.textPrimary);

    return base.copyWith(
      textTheme: textTheme,
      extensions: <ThemeExtension<dynamic>>[
        AppSurfaces(
          statusOk: p.statusOk,
          statusWarn: p.statusWarn,
          statusError: p.statusError,
          mono: mono,
        ),
      ],
      dividerTheme: DividerThemeData(color: p.divider, space: 1),
      cardTheme: CardThemeData(
        color: p.surface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
          side: BorderSide(color: p.border),
        ),
      ),
      navigationRailTheme: NavigationRailThemeData(
        backgroundColor: Colors.transparent,
        indicatorColor: p.accentRowTint,
        selectedIconTheme: IconThemeData(color: p.accent, size: 20),
        unselectedIconTheme: IconThemeData(color: p.textMuted, size: 20),
        labelType: NavigationRailLabelType.none,
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: p.surfaceInset,
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: BorderSide(color: p.border),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: BorderSide(color: p.border),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: BorderSide(color: p.accent, width: 2),
        ),
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        isDense: true,
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: p.accent,
          foregroundColor: p.onAccent,
          elevation: 0,
          textStyle: const TextStyle(fontWeight: FontWeight.w600),
          shape:
              RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: p.accent,
          foregroundColor: p.onAccent,
          textStyle: const TextStyle(fontWeight: FontWeight.w600),
          shape:
              RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: p.textPrimary,
          side: BorderSide(color: p.border),
          shape:
              RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        ),
      ),
      chipTheme: ChipThemeData(
        backgroundColor: p.surfaceRaised,
        side: BorderSide(color: p.border),
        shape:
            RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
        labelStyle: TextStyle(color: p.textPrimary, fontSize: 12),
      ),
      listTileTheme: ListTileThemeData(
        tileColor: Colors.transparent,
        selectedTileColor: p.accentRowTint,
      ),
    );
  }
}
