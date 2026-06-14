import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

// ---------------------------------------------------------------------------
// 本地 UI 偏好 — 通过 shared_preferences 持久化（不跨设备同步）。
// ---------------------------------------------------------------------------

class AppPreferences {
  const AppPreferences({
    this.showSparkline = true,
    this.enableAnimations = true,
    this.coreBaseUrl = 'http://127.0.0.1:3004',
    this.onboardingCompleted = false,
  });

  /// Dashboard 统计卡是否显示 sparkline 趋势图。
  final bool showSparkline;

  /// 是否启用页面过渡 / 列表进场等动画。
  final bool enableAnimations;

  /// weft-core 的连接地址。默认本地 sidecar；高级用户可指向远程 core。
  final String coreBaseUrl;

  /// 是否已完成首次启动引导 (OOBE)。
  final bool onboardingCompleted;

  AppPreferences copyWith({
    bool? showSparkline,
    bool? enableAnimations,
    String? coreBaseUrl,
    bool? onboardingCompleted,
  }) {
    return AppPreferences(
      showSparkline: showSparkline ?? this.showSparkline,
      enableAnimations: enableAnimations ?? this.enableAnimations,
      coreBaseUrl: coreBaseUrl ?? this.coreBaseUrl,
      onboardingCompleted: onboardingCompleted ?? this.onboardingCompleted,
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
  static const _defaultCoreBaseUrl = 'http://127.0.0.1:3004';

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    state = AppPreferences(
      showSparkline: prefs.getBool(_kSparkline) ?? true,
      enableAnimations: prefs.getBool(_kAnimations) ?? true,
      coreBaseUrl: prefs.getString(_kCoreBaseUrl) ?? _defaultCoreBaseUrl,
      onboardingCompleted: prefs.getBool(_kOnboardingCompleted) ?? false,
    );
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
}

final preferencesProvider =
    StateNotifierProvider<PreferencesNotifier, AppPreferences>(
  (ref) => PreferencesNotifier(),
);
