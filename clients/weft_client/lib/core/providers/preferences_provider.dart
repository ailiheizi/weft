import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

// ---------------------------------------------------------------------------
// 本地 UI 偏好 — 通过 shared_preferences 持久化（不跨设备同步）。
// ---------------------------------------------------------------------------

class AppPreferences {
  const AppPreferences({
    this.showSparkline = true,
    this.enableAnimations = true,
  });

  /// Dashboard 统计卡是否显示 sparkline 趋势图。
  final bool showSparkline;

  /// 是否启用页面过渡 / 列表进场等动画。
  final bool enableAnimations;

  AppPreferences copyWith({bool? showSparkline, bool? enableAnimations}) {
    return AppPreferences(
      showSparkline: showSparkline ?? this.showSparkline,
      enableAnimations: enableAnimations ?? this.enableAnimations,
    );
  }
}

class PreferencesNotifier extends StateNotifier<AppPreferences> {
  PreferencesNotifier() : super(const AppPreferences()) {
    _load();
  }

  static const _kSparkline = 'pref_show_sparkline';
  static const _kAnimations = 'pref_enable_animations';

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    state = AppPreferences(
      showSparkline: prefs.getBool(_kSparkline) ?? true,
      enableAnimations: prefs.getBool(_kAnimations) ?? true,
    );
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
}

final preferencesProvider =
    StateNotifierProvider<PreferencesNotifier, AppPreferences>(
  (ref) => PreferencesNotifier(),
);
