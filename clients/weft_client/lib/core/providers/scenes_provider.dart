import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/client.dart';
import '../api/scene_api.dart';

/// SceneApi 的 provider(复用核心 Dio)。
final sceneApiProvider = Provider<SceneApi>((ref) {
  return SceneApi(ref.watch(apiClientProvider));
});

/// 场景管理状态。
class ScenesState {
  const ScenesState({
    this.app = 'weft-claw',
    this.activeScene = '',
    this.scenes = const [],
    this.loading = false,
    this.error,
  });

  final String app;
  final String activeScene;
  final List<Scene> scenes;
  final bool loading;
  final String? error;

  ScenesState copyWith({
    String? app,
    String? activeScene,
    List<Scene>? scenes,
    bool? loading,
    String? error,
    bool clearError = false,
  }) {
    return ScenesState(
      app: app ?? this.app,
      activeScene: activeScene ?? this.activeScene,
      scenes: scenes ?? this.scenes,
      loading: loading ?? this.loading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

class ScenesNotifier extends StateNotifier<ScenesState> {
  ScenesNotifier(this._api, {String app = 'weft-claw'})
      : super(ScenesState(app: app)) {
    load();
  }

  final SceneApi _api;

  /// 拉取场景列表 + 当前激活场景。
  Future<void> load() async {
    state = state.copyWith(loading: true, clearError: true);
    try {
      final result = await _api.list(state.app);
      state = state.copyWith(
        activeScene: result.activeScene,
        scenes: result.scenes,
        loading: false,
        clearError: true,
      );
    } catch (e) {
      state = state.copyWith(loading: false, error: _sanitize(e));
    }
  }

  /// 激活一个场景,然后刷新。
  Future<void> bind(String sceneName) async {
    state = state.copyWith(loading: true, clearError: true);
    try {
      await _api.bind(state.app, sceneName);
      await load();
    } catch (e) {
      state = state.copyWith(loading: false, error: _sanitize(e));
    }
  }

  /// 删除一个场景,然后刷新。激活场景会被后端拒绝(409)。
  Future<void> delete(String sceneName) async {
    state = state.copyWith(loading: true, clearError: true);
    try {
      await _api.delete(state.app, sceneName);
      await load();
    } catch (e) {
      state = state.copyWith(loading: false, error: _sanitize(e));
    }
  }

  /// 创建场景,然后刷新。
  Future<void> create(Scene scene) async {
    state = state.copyWith(loading: true, clearError: true);
    try {
      await _api.create(state.app, scene);
      await load();
    } catch (e) {
      state = state.copyWith(loading: false, error: _sanitize(e));
    }
  }

  static String _sanitize(Object e) {
    final s = e.toString();
    if (s.contains('409')) return '不能删除当前激活的场景(请先切到别的场景)';
    if (s.contains('SocketException') || s.contains('Connection')) {
      return '无法连接到 weft-core';
    }
    return s.length > 120 ? '${s.substring(0, 120)}…' : s;
  }
}

/// weft-claw app 的场景管理 provider。
final scenesProvider =
    StateNotifierProvider<ScenesNotifier, ScenesState>((ref) {
  return ScenesNotifier(ref.watch(sceneApiProvider));
});
