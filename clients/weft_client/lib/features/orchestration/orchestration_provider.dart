import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/api/client.dart';
import '../../core/api/orchestration_api.dart';
import '../../core/models/orchestration.dart';

/// OrchestrationApi 的 provider（复用核心 Dio）。
final orchestrationApiProvider = Provider<OrchestrationApi>((ref) {
  final dio = ref.watch(apiClientProvider);
  return OrchestrationApi(dio);
});

/// 编排面板状态。
class OrchestrationState {
  const OrchestrationState({
    this.boardId,
    this.goal = '',
    this.tasks = const [],
    this.handoffs = const [],
    this.activity = const [],
    this.running = false,
    this.error,
  });

  final String? boardId;
  final String goal;
  final List<TeamTask> tasks;
  final List<Handoff> handoffs;
  final List<ActivityEntry> activity;

  /// 是否有进行中的编排（board 已建、仍在轮询）。
  final bool running;
  final String? error;

  OrchestrationState copyWith({
    String? boardId,
    String? goal,
    List<TeamTask>? tasks,
    List<Handoff>? handoffs,
    List<ActivityEntry>? activity,
    bool? running,
    String? error,
    bool clearError = false,
  }) {
    return OrchestrationState(
      boardId: boardId ?? this.boardId,
      goal: goal ?? this.goal,
      tasks: tasks ?? this.tasks,
      handoffs: handoffs ?? this.handoffs,
      activity: activity ?? this.activity,
      running: running ?? this.running,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

class OrchestrationNotifier extends StateNotifier<OrchestrationState> {
  OrchestrationNotifier(this._api) : super(const OrchestrationState());

  final OrchestrationApi _api;
  Timer? _poll;

  /// 启动一次多 agent 编排：建看板 → 建任务(intake) → 建 handoff 给 planner(pending)。
  /// 后台 dispatch 循环会自动执行 handoff（调真 agent），并随 tick 推进阶段。
  Future<void> startDemo(String goal) async {
    final trimmed = goal.trim();
    if (trimmed.isEmpty) return;
    final ts = DateTime.now().millisecondsSinceEpoch;
    final boardId = 'board-$ts';
    final sessionId = 'orch-$ts';
    final taskId = 'task-$ts';

    state = state.copyWith(
      running: true,
      goal: trimmed,
      boardId: boardId,
      tasks: const [],
      handoffs: const [],
      activity: const [],
      clearError: true,
    );

    try {
      await _api.createBoard(
        boardId: boardId,
        sessionId: sessionId,
        title: trimmed,
      );
      await _api.saveTask(
        boardId: boardId,
        taskId: taskId,
        title: trimmed,
        description: trimmed,
        status: 'ready_for_plan',
        phase: 'intake',
      );
      // 不手动建 handoff：后台 tick 循环会按阶段图自动推进
      // （intake→plan→execute…）并在跨角色处自动创建 handoff，
      // dispatch 再执行（调真 agent）。手动建 planner→planner 会被委托规则拒绝。
      _startPolling();
      await refresh();
    } catch (e) {
      state = state.copyWith(running: false, error: e.toString());
    }
  }

  /// 拉取当前看板的任务/handoff/活动流。
  Future<void> refresh() async {
    final boardId = state.boardId;
    if (boardId == null) return;
    try {
      final results = await Future.wait([
        _api.listTasks(boardId),
        _api.listHandoffs(boardId),
        _api.listActivity(boardId),
      ]);
      final tasks =
          (results[0]).map(TeamTask.fromJson).toList();
      final handoffs =
          (results[1]).map(Handoff.fromJson).toList();
      final activity =
          (results[2]).map(ActivityEntry.fromJson).toList()
            ..sort((a, b) => a.timestamp.compareTo(b.timestamp));
      state = state.copyWith(
        tasks: tasks,
        handoffs: handoffs,
        activity: activity,
        clearError: true,
      );
    } catch (e) {
      state = state.copyWith(error: e.toString());
    }
  }

  /// 附着到一个已存在的 board(AI 从聊天创建的),只轮询不新建。
  /// 用于聊天工作区内嵌展示。
  void attach(String boardId) {
    if (state.boardId == boardId && _poll != null) return;
    state = state.copyWith(boardId: boardId, running: true, clearError: true);
    _startPolling();
    refresh();
  }

  void _startPolling() {
    _poll?.cancel();
    _poll = Timer.periodic(const Duration(seconds: 2), (_) => refresh());
  }

  /// 停止轮询（任务都完成或用户离开）。
  void stop() {
    _poll?.cancel();
    _poll = null;
    state = state.copyWith(running: false);
  }

  @override
  void dispose() {
    _poll?.cancel();
    super.dispose();
  }
}

final orchestrationProvider =
    StateNotifierProvider<OrchestrationNotifier, OrchestrationState>((ref) {
  final api = ref.watch(orchestrationApiProvider);
  return OrchestrationNotifier(api);
});

/// 按 boardId 隔离的编排状态(聊天工作区内嵌用)。
/// 每个 board 独立轮询,不与独立"团队"页的全局 provider 冲突。
final boardWatchProvider = StateNotifierProvider.family<OrchestrationNotifier,
    OrchestrationState, String>((ref, boardId) {
  final n = OrchestrationNotifier(ref.watch(orchestrationApiProvider));
  n.attach(boardId);
  return n;
});
