import 'package:flutter/material.dart' show ThemeMode;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

// ---------------------------------------------------------------------------
// 本地 UI 偏好 — 通过 shared_preferences 持久化（不跨设备同步）。
// ---------------------------------------------------------------------------

class AppPreferences {
  const AppPreferences({
    this.showSparkline = true,
    this.enableAnimations = true,
    this.coreBaseUrl = 'http://127.0.0.1:17830',
    this.onboardingCompleted = false,
    this.themeMode = ThemeMode.dark,
    this.workspaceDir = '',
  });

  /// Dashboard 统计卡是否显示 sparkline 趋势图。
  final bool showSparkline;

  /// 是否启用页面过渡 / 列表进场等动画。
  final bool enableAnimations;

  /// weft-core 的连接地址。默认本地 sidecar；高级用户可指向远程 core。
  final String coreBaseUrl;

  /// 是否已完成首次启动引导 (OOBE)。
  final bool onboardingCompleted;

  /// 主题模式：暗 / 亮 / 跟随系统。默认暗色(保持原体验)。
  final ThemeMode themeMode;

  /// AI 文件操作的工作目录。为空时使用默认 data/workspaces/<session_id>。
  final String workspaceDir;

  AppPreferences copyWith({
    bool? showSparkline,
    bool? enableAnimations,
    String? coreBaseUrl,
    bool? onboardingCompleted,
    ThemeMode? themeMode,
    String? workspaceDir,
  }) {
    return AppPreferences(
      showSparkline: showSparkline ?? this.showSparkline,
      enableAnimations: enableAnimations ?? this.enableAnimations,
      coreBaseUrl: coreBaseUrl ?? this.coreBaseUrl,
      onboardingCompleted: onboardingCompleted ?? this.onboardingCompleted,
      themeMode: themeMode ?? this.themeMode,
      workspaceDir: workspaceDir ?? this.workspaceDir,
    );
  }
}

class PreferencesNotifier extends StateNotifier<AppPreferences> {
  PreferencesNotifier() : super(const AppPreferences()) {
    _load();
  }

  static const _kSparkline = 'pref_show_sparkline';
  static const _kAnimations = 'pref_enable_animations';
  static const _kCoreBaseUrl = 'pref_core_base_url';
  static const _kOnboardingCompleted = 'pref_onboarding_completed';
  static const _kThemeMode = 'pref_theme_mode';
  static const _kWorkspaceDir = 'pref_workspace_dir';
  static const _defaultCoreBaseUrl = 'http://127.0.0.1:17830';

  static ThemeMode _parseThemeMode(String? v) {
    switch (v) {
      case 'light':
        return ThemeMode.light;
      case 'system':
        return ThemeMode.system;
      default:
        return ThemeMode.dark;
    }
  }

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    state = AppPreferences(
      showSparkline: prefs.getBool(_kSparkline) ?? true,
      enableAnimations: prefs.getBool(_kAnimations) ?? true,
      coreBaseUrl: prefs.getString(_kCoreBaseUrl) ?? _defaultCoreBaseUrl,
      onboardingCompleted: prefs.getBool(_kOnboardingCompleted) ?? false,
      themeMode: _parseThemeMode(prefs.getString(_kThemeMode)),
      workspaceDir: prefs.getString(_kWorkspaceDir) ?? '',
    );
  }

  /// 设置主题模式并持久化。
  Future<void> setThemeMode(ThemeMode mode) async {
    state = state.copyWith(themeMode: mode);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_kThemeMode, mode.name);
  }

  /// 标记首次启动引导已完成（或重置为未完成）。
  Future<void> setOnboardingCompleted(bool value) async {
    state = state.copyWith(onboardingCompleted: value);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_kOnboardingCompleted, value);
  }

  Future<void> setShowSparkline(bool value) async {
    state = state.copyWith(showSparkline: value);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_kSparkline, value);
  }

  Future<void> setEnableAnimations(bool value) async {
    state = state.copyWith(enableAnimations: value);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_kAnimations, value);
  }

  /// 更新 core 连接地址；空值回退到默认本地地址。
  Future<void> setCoreBaseUrl(String value) async {
    final url = value.trim().isEmpty ? _defaultCoreBaseUrl : value.trim();
    state = state.copyWith(coreBaseUrl: url);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_kCoreBaseUrl, url);
  }

  /// 设置 AI 文件操作的工作目录；空值表示使用默认(data/workspaces/<sid>)。
  Future<void> setWorkspaceDir(String value) async {
    state = state.copyWith(workspaceDir: value.trim());
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_kWorkspaceDir, value.trim());
  }
}

final preferencesProvider =
    StateNotifierProvider<PreferencesNotifier, AppPreferences>(
  (ref) => PreferencesNotifier(),
);
