import 'package:dio/dio.dart';

/// Skill 管理 API。对应 skills-runtime 两类能力:
/// - `ext.skills`:内置/文件式技能的浏览与启停(list_skills / read_skill / enable / disable)。
/// - `skills.governance`:agent 自动演化技能的治理(list_evolved / review_evolved)。
/// 均走通用能力端点 `/api/capabilities/{capability}/call`。
class SkillsApi {
  SkillsApi(this._dio);

  final Dio _dio;
  static const _capExt = 'ext.skills';
  static const _capGov = 'skills.governance';

  Future<Map<String, dynamic>> _call(
    String cap,
    String action,
    Map<String, dynamic> data,
  ) async {
    final resp = await _dio.post<Map<String, dynamic>>(
      '/api/capabilities/$cap/call',
      data: {'action': action, 'data': data},
    );
    final response = resp.data?['response'] as Map<String, dynamic>?;
    final inner = response?['data'];
    return inner is Map<String, dynamic> ? inner : <String, dynamic>{};
  }

  // ── ext.skills:内置/文件式技能管理 ──────────────────────────────────────

  /// 列出所有可用技能(builtin + 文件式)。
  Future<List<SkillInfo>> listSkills(String agent) async {
    final data = await _call(_capExt, 'list_skills', {'agent': agent});
    final enabled =
        (data['enabled'] as List? ?? []).whereType<String>().toSet();
    return (data['skills'] as List? ?? [])
        .whereType<Map<String, dynamic>>()
        .map((j) => SkillInfo.fromJson(j, enabled))
        .toList();
  }

  /// 读取单个技能详情(含完整内容)。
  Future<SkillDetail> readSkill(String name) async {
    final data = await _call(_capExt, 'read_skill', {'name': name});
    return SkillDetail.fromJson(data);
  }

  /// 为 agent 启用一个技能。
  Future<void> enable(String agent, String skill) async {
    await _call(_capExt, 'enable', {'agent': agent, 'skill': skill});
  }

  /// 为 agent 禁用一个技能。
  Future<void> disable(String agent, String skill) async {
    await _call(_capExt, 'disable', {'agent': agent, 'skill': skill});
  }

  // ── skills.governance:演化技能治理 ─────────────────────────────────────

  /// 列出某 agent 的演化技能。
  Future<List<EvolvedSkill>> listEvolved(String agent) async {
    final data = await _call(_capGov, 'list_evolved', {'agent': agent});
    return (data['skills'] as List? ?? [])
        .whereType<Map<String, dynamic>>()
        .map(EvolvedSkill.fromJson)
        .toList();
  }

  /// 审批(approve=true)或拒绝(approve=false)一个待审技能。
  Future<void> review(String agent, String skillId,
      {required bool approve, String notes = ''}) async {
    await _call(_capGov, 'review_evolved', {
      'agent': agent,
      'skill_id': skillId,
      'approve': approve,
      'notes': notes,
    });
  }
}

/// 内置/文件式技能的摘要信息(来自 list_skills)。
class SkillInfo {
  const SkillInfo({
    required this.name,
    required this.description,
    required this.enabled,
  });

  final String name;
  final String description;
  final bool enabled;

  factory SkillInfo.fromJson(Map<String, dynamic> j, Set<String> enabledSet) {
    final name = j['name'] as String? ?? '';
    // 后端可能在每条记录里带 enabled 字段,也可能只返回 enabled 名单。
    // 名单为空时视为全部启用(默认全开)。
    final localEnabled = j['enabled'];
    final isEnabled = localEnabled is bool
        ? localEnabled
        : (enabledSet.isEmpty || enabledSet.contains(name));
    return SkillInfo(
      name: name,
      description: j['description'] as String? ?? '',
      enabled: isEnabled,
    );
  }

  SkillInfo copyWith({bool? enabled}) => SkillInfo(
        name: name,
        description: description,
        enabled: enabled ?? this.enabled,
      );
}

/// 单个技能的完整详情(来自 read_skill)。
class SkillDetail {
  const SkillDetail({
    required this.name,
    required this.description,
    required this.source,
    required this.content,
  });

  final String name;
  final String description;
  final String source;
  final String content;

  factory SkillDetail.fromJson(Map<String, dynamic> j) => SkillDetail(
        name: j['name'] as String? ?? '',
        description: j['description'] as String? ?? '',
        source: j['source'] as String? ?? '',
        content: j['content'] as String? ?? '',
      );
}

class EvolvedSkill {
  const EvolvedSkill({
    required this.id,
    required this.title,
    required this.description,
    required this.status,
    required this.riskLevel,
    required this.qualityScore,
    required this.confidence,
    required this.reviewRequired,
    required this.successfulUses,
    required this.failedUses,
    required this.triggers,
  });

  final String id;
  final String title;
  final String description;
  final String status;
  final String riskLevel;
  final double qualityScore;
  final double confidence;
  final bool reviewRequired;
  final int successfulUses;
  final int failedUses;
  final List<String> triggers;

  factory EvolvedSkill.fromJson(Map<String, dynamic> j) {
    double d(dynamic v) => (v as num?)?.toDouble() ?? 0;
    int i(dynamic v) => (v as num?)?.toInt() ?? 0;
    return EvolvedSkill(
      id: j['id'] as String? ?? '',
      title: j['title'] as String? ?? '',
      description: j['description'] as String? ?? '',
      status: j['status'] as String? ?? '',
      riskLevel: j['risk_level'] as String? ?? '',
      qualityScore: d(j['quality_score']),
      confidence: d(j['confidence']),
      reviewRequired: j['review_required'] as bool? ?? false,
      successfulUses: i(j['successful_uses']),
      failedUses: i(j['failed_uses']),
      triggers: (j['triggers'] as List? ?? []).whereType<String>().toList(),
    );
  }
}
