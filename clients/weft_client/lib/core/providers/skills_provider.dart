import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/client.dart';
import '../api/skills_api.dart';

final skillsApiProvider = Provider<SkillsApi>((ref) {
  return SkillsApi(ref.watch(apiClientProvider));
});

class SkillsState {
  const SkillsState({
    this.agent = 'weft-claw',
    this.skills = const [],
    this.loading = false,
    this.error,
  });

  final String agent;
  final List<EvolvedSkill> skills;
  final bool loading;
  final String? error;

  SkillsState copyWith({
    String? agent,
    List<EvolvedSkill>? skills,
    bool? loading,
    String? error,
    bool clearError = false,
  }) {
    return SkillsState(
      agent: agent ?? this.agent,
      skills: skills ?? this.skills,
      loading: loading ?? this.loading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

class SkillsNotifier extends StateNotifier<SkillsState> {
  SkillsNotifier(this._api) : super(const SkillsState()) {
    load();
  }

  final SkillsApi _api;

  Future<void> load() async {
    state = state.copyWith(loading: true, clearError: true);
    try {
      final skills = await _api.listEvolved(state.agent);
      state = state.copyWith(skills: skills, loading: false, clearError: true);
    } catch (e) {
      state = state.copyWith(loading: false, error: _sanitize(e));
    }
  }

  Future<void> review(String skillId, bool approve) async {
    state = state.copyWith(loading: true, clearError: true);
    try {
      await _api.review(state.agent, skillId, approve: approve);
      await load();
    } catch (e) {
      state = state.copyWith(loading: false, error: _sanitize(e));
    }
  }

  static String _sanitize(Object e) {
    final s = e.toString();
    if (s.contains('SocketException') || s.contains('Connection')) {
      return '无法连接到 weft-core';
    }
    return s.length > 120 ? '${s.substring(0, 120)}…' : s;
  }
}

final skillsProvider =
    StateNotifierProvider<SkillsNotifier, SkillsState>((ref) {
  return SkillsNotifier(ref.watch(skillsApiProvider));
});
