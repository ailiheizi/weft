import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/weft_claw_api.dart';

// ── Provider ────────────────────────────────────────────────────────────────

final selectorApiProvider = Provider<SelectorApi>((ref) {
  return SelectorApi();
});

/// State for the tool-selector chips displayed above the input bar.
class ToolSelectorState {
  const ToolSelectorState({
    this.matches = const [],
    this.deselected = const {},
    this.autoSelect = true,
    this.loading = false,
    this.modelReady = false,
  });

  /// All matches returned by selector.
  final List<SelectorMatch> matches;

  /// IDs the user has manually deselected (✕).
  final Set<String> deselected;

  /// Whether auto-select is enabled (debounce on input change).
  final bool autoSelect;

  /// Whether a selector call is in progress.
  final bool loading;

  /// Whether the tool-selector service is available and models are loaded.
  final bool modelReady;

  /// Active (visible) matches = all minus deselected.
  List<SelectorMatch> get active =>
      matches.where((m) => !deselected.contains(m.id)).toList();

  /// Selected tool IDs to pass to agent-core.
  List<String> get selectedToolIds => active.map((m) => m.id).toList();

  ToolSelectorState copyWith({
    List<SelectorMatch>? matches,
    Set<String>? deselected,
    bool? autoSelect,
    bool? loading,
    bool? modelReady,
  }) {
    return ToolSelectorState(
      matches: matches ?? this.matches,
      deselected: deselected ?? this.deselected,
      autoSelect: autoSelect ?? this.autoSelect,
      loading: loading ?? this.loading,
      modelReady: modelReady ?? this.modelReady,
    );
  }
}

class ToolSelectorNotifier extends StateNotifier<ToolSelectorState> {
  ToolSelectorNotifier(this._api) : super(const ToolSelectorState()) {
    checkModelStatus();
  }

  final SelectorApi _api;
  Timer? _debounce;

  /// Check if the tool-selector service is available and models are loaded.
  Future<void> checkModelStatus() async {
    try {
      final available = await _api.checkStatus();
      if (!mounted) return;
      state = state.copyWith(modelReady: available);
    } catch (_) {
      if (!mounted) return;
      state = state.copyWith(modelReady: false);
    }
  }

  /// Called on text change (debounced). Triggers selector if autoSelect is on.
  void onTextChanged(String text) {
    _debounce?.cancel();
    if (!state.autoSelect || text.trim().isEmpty) {
      if (text.trim().isEmpty && state.matches.isNotEmpty) {
        state = state.copyWith(matches: [], deselected: {});
      }
      return;
    }
    _debounce = Timer(const Duration(milliseconds: 800), () {
      _runSelect(text.trim());
    });
  }

  /// Manually trigger selector (e.g. button press).
  Future<void> manualSelect(String text) async {
    if (text.trim().isEmpty) return;
    await _runSelect(text.trim());
  }

  Future<void> _runSelect(String query) async {
    if (!mounted) return;
    state = state.copyWith(loading: true);
    try {
      final results = await _api.select(query, library: 'tools', topK: 5);
      if (!mounted) return;
      // Only show matches with score > threshold.
      final filtered = results.where((m) => m.score > 0.15).toList();
      state = state.copyWith(
        matches: filtered,
        deselected: {},
        loading: false,
      );
    } catch (_) {
      if (!mounted) return;
      state = state.copyWith(loading: false);
    }
  }

  /// User tapped ✕ on a chip.
  void deselect(String id) {
    state = state.copyWith(deselected: {...state.deselected, id});
  }

  /// User re-selected a previously deselected tool.
  void reselect(String id) {
    final updated = Set<String>.from(state.deselected)..remove(id);
    state = state.copyWith(deselected: updated);
  }

  /// Toggle auto-select mode.
  void toggleAutoSelect() {
    state = state.copyWith(autoSelect: !state.autoSelect);
  }

  /// Clear all matches (e.g. after sending message).
  void clear() {
    _debounce?.cancel();
    state = const ToolSelectorState();
  }

  @override
  void dispose() {
    _debounce?.cancel();
    super.dispose();
  }
}

/// Per-session tool selector state.
final toolSelectorProvider = StateNotifierProvider.family<
    ToolSelectorNotifier, ToolSelectorState, String>((ref, sessionId) {
  final api = ref.watch(selectorApiProvider);
  return ToolSelectorNotifier(api);
});

/// Per-session current input text (updated by ChatScreen's text listener).
/// Used by _SelectorButton to get the current text for manual trigger.
final inputTextProvider = StateProvider.family<String, String>(
    (ref, sessionId) => '');
