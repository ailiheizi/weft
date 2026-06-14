import 'package:dio/dio.dart';

/// 多 agent 编排能力的封装。
///
/// 走通用能力端点 `POST /api/capabilities/{capability}/call`
/// （body: `{action, data}`），不同于 weft-claw 的 `/api/apps/{app}/run`。
/// 该端点对 team.*/workflow.* 无 policy 限制，Developer profile 直接可用。
///
/// 响应包络：`{capability, provider, response: {status, data, error}, status:"executed"}`。
/// 这里统一解包到 `response.data`。
class OrchestrationApi {
  OrchestrationApi(this._dio);

  final Dio _dio;

  static const capTaskboard = 'team.taskboard';
  static const capHandoff = 'team.handoff';
  static const capRuntime = 'team.runtime';
  static const capOrchestration = 'workflow.orchestration';

  /// 调用任意能力的某个 action，返回包内 `response.data`（已解包）。
  Future<Map<String, dynamic>> call(
    String capability,
    String action,
    Map<String, dynamic> data, {
    Duration timeout = const Duration(seconds: 20),
  }) async {
    final resp = await _dio.post<Map<String, dynamic>>(
      '/api/capabilities/$capability/call',
      data: {'action': action, 'data': data},
      options: Options(receiveTimeout: timeout),
    );
    final response = resp.data?['response'] as Map<String, dynamic>?;
    // PackageResult: {status, data, error}
    final inner = response?['data'];
    if (inner is Map<String, dynamic>) return inner;
    return <String, dynamic>{};
  }

  // ── team.taskboard ─────────────────────────────────────────────────────────

  /// 创建看板。board_id 会注册进 `:boards` KV 索引，orchestrator 据此发现它。
  Future<void> createBoard({
    required String boardId,
    required String sessionId,
    required String title,
  }) async {
    await call(capTaskboard, 'create_board', {
      'board_id': boardId,
      'session_id': sessionId,
      'title': title,
      'status': 'active',
    });
  }

  /// 新建/更新任务。metadata.phase 决定工作流阶段（默认 intake）。
  Future<void> saveTask({
    required String boardId,
    required String taskId,
    required String title,
    String description = '',
    String kind = 'feature',
    String status = 'ready_for_plan',
    String ownerMemberId = 'planner',
    String phase = 'intake',
  }) async {
    await call(capTaskboard, 'save_task', {
      'board_id': boardId,
      'task_id': taskId,
      'title': title,
      'description': description,
      'kind': kind,
      'status': status,
      'owner_member_id': ownerMemberId,
      'metadata': {'phase': phase},
    });
  }

  Future<List<Map<String, dynamic>>> listTasks(String boardId) async {
    final data = await call(capTaskboard, 'list_tasks', {'board_id': boardId});
    return _asList(data['tasks']);
  }

  // ── team.handoff ───────────────────────────────────────────────────────────

  /// 创建一个 pending handoff，后台 dispatch 循环会自动执行（调真 agent）。
  Future<void> createHandoff({
    required String boardId,
    required String handoffId,
    required String taskId,
    String fromMemberId = 'planner',
    required String toMemberId,
    String reason = '',
    String expectedOutcome = '',
  }) async {
    await call(capHandoff, 'create_handoff', {
      'board_id': boardId,
      'handoff_id': handoffId,
      'task_id': taskId,
      'from_member_id': fromMemberId,
      'to_member_id': toMemberId,
      'reason': reason,
      'expected_outcome': expectedOutcome,
      'status': 'pending',
    });
  }

  Future<List<Map<String, dynamic>>> listHandoffs(String boardId) async {
    final data =
        await call(capHandoff, 'list_handoffs', {'board_id': boardId});
    return _asList(data['handoffs']);
  }

  Future<List<Map<String, dynamic>>> listActivity(String boardId) async {
    final data =
        await call(capHandoff, 'list_activity', {'board_id': boardId});
    return _asList(data['activity'] ?? data['entries'] ?? data['items']);
  }

  // ── team.runtime ───────────────────────────────────────────────────────────

  /// 角色目录（planner / implementer / reviewer / integrator）。
  Future<List<Map<String, dynamic>>> getRoleCatalog() async {
    final data = await call(capRuntime, 'get_catalog', {});
    return _asList(data['roles'] ?? data['catalog']);
  }

  List<Map<String, dynamic>> _asList(dynamic v) {
    if (v is List) {
      return v.whereType<Map<String, dynamic>>().toList();
    }
    return const [];
  }
}
