import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

// ---------------------------------------------------------------------------
// Pinned apps — product packages the user pinned to the sidebar.
// Persisted locally via shared_preferences (not synced across devices).
// ---------------------------------------------------------------------------

const _prefsKey = 'pinned_apps';

class PinnedAppsNotifier extends StateNotifier<List<String>> {
  PinnedAppsNotifier() : super(const []) {
    _load();
  }

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    state = prefs.getStringList(_prefsKey) ?? const [];
  }

  Future<void> _persist() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setStringList(_prefsKey, state);
  }

  bool isPinned(String appName) => state.contains(appName);

  Future<void> pin(String appName) async {
    if (state.contains(appName)) return;
    state = [...state, appName];
    await _persist();
  }

  Future<void> unpin(String appName) async {
    if (!state.contains(appName)) return;
    state = state.where((n) => n != appName).toList();
    await _persist();
  }

  Future<void> toggle(String appName) =>
      isPinned(appName) ? unpin(appName) : pin(appName);
}

final pinnedAppsProvider =
    StateNotifierProvider<PinnedAppsNotifier, List<String>>(
  (ref) => PinnedAppsNotifier(),
);
