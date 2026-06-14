// 多 agent 编排数据模型。从 capability 返回的 JSON 解析。
// 字段尽量宽松（缺失给默认值），后端返回结构演进时不易崩。

class TeamTask {
  const TeamTask({
    required this.taskId,
    required this.title,
    this.description = '',
    this.status = '',
    this.phase = 'intake',
    this.ownerMemberId = '',
    this.dependsOn = const [],
  });

  final String taskId;
  final String title;
  final String description;
  final String status;
  final String phase;
  final String ownerMemberId;

  /// 依赖的任务 id（DAG 边）。fan-out 父任务依赖其并行子任务。
  final List<String> dependsOn;

  factory TeamTask.fromJson(Map<String, dynamic> j) {
    final meta = j['metadata'];
    final phase = (meta is Map && meta['phase'] is String)
        ? meta['phase'] as String
        : (j['phase'] as String? ?? 'intake');
    final deps = (j['depends_on'] is List)
        ? (j['depends_on'] as List).whereType<String>().toList()
        : <String>[];
    return TeamTask(
      taskId: j['task_id'] as String? ?? '',
      title: j['title'] as String? ?? '',
      description: j['description'] as String? ?? '',
      status: j['status'] as String? ?? '',
      phase: phase,
      ownerMemberId: j['owner_member_id'] as String? ?? '',
      dependsOn: deps,
    );
  }
}

class Handoff {
  const Handoff({
    required this.handoffId,
    required this.taskId,
    this.fromMemberId = '',
    this.toMemberId = '',
    this.status = '',
    this.reason = '',
  });

  final String handoffId;
  final String taskId;
  final String fromMemberId;
  final String toMemberId;
  final String status;
  final String reason;

  factory Handoff.fromJson(Map<String, dynamic> j) => Handoff(
        handoffId: j['handoff_id'] as String? ?? '',
        taskId: j['task_id'] as String? ?? '',
        fromMemberId: j['from_member_id'] as String? ?? '',
        toMemberId: j['to_member_id'] as String? ?? '',
        status: j['status'] as String? ?? '',
        reason: j['reason'] as String? ?? '',
      );
}

class ActivityEntry {
  const ActivityEntry({
    required this.eventId,
    this.eventType = '',
    this.actorMemberId = '',
    this.summary = '',
    this.taskId = '',
    this.timestamp = 0,
  });

  final String eventId;
  final String eventType;
  final String actorMemberId;
  final String summary;
  final String taskId;
  final int timestamp;

  factory ActivityEntry.fromJson(Map<String, dynamic> j) => ActivityEntry(
        eventId: j['event_id'] as String? ?? '',
        eventType: j['event_type'] as String? ?? '',
        actorMemberId: j['actor_member_id'] as String? ?? '',
        summary: j['summary'] as String? ?? '',
        taskId: j['task_id'] as String? ?? '',
        timestamp: (j['timestamp'] as num?)?.toInt() ?? 0,
      );
}

/// 工作流阶段顺序（与 workflow-template-devteam 的阶段图一致）。
const kWorkflowPhases = <String>[
  'intake',
  'plan',
  'execute',
  'review',
  'integrate',
  'done',
];

/// 阶段中文标签。
const kPhaseLabels = <String, String>{
  'intake': '接收',
  'plan': '规划',
  'execute': '执行',
  'review': '评审',
  'integrate': '集成',
  'done': '完成',
};
