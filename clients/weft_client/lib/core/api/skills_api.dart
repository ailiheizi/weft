import 'package:dio/dio.dart';

/// Skill 管理 API。对应 skills-runtime 的能力动作:
/// list_evolved(列出 agent 的演化技能)、review_evolved(审批/拒绝)、maintenance。
/// 走通用能力端点 `/api/capabilities/{capability}/call`。
class SkillsApi {
  SkillsApi(this._dio);

  final Dio _dio;
  static const _cap = 'skills.governance';

  Future<Map<String, dynamic>> _call(
    String action,
    Map<String, dynamic> data,
  ) async {
    final resp = await _dio.post<Map<String, dynamic>>(
      '/api/capabilities/$_cap/call',
      data: {'action': action, 'data': data},
    );
    final response = resp.data?['response'] as Map<String, dynamic>?;
    final inner = response?['data'];
    return inner is Map<String, dynamic> ? inner : <String, dynamic>{};
  }

  /// 列出某 agent 的演化技能。
  Future<List<EvolvedSkill>> listEvolved(String agent) async {
    final data = await _call('list_evolved', {'agent': agent});
    return (data['skills'] as List? ?? [])
        .whereType<Map<String, dynamic>>()
        .map(EvolvedSkill.fromJson)
        .toList();
  }

  /// 审批(approve=true)或拒绝(approve=false)一个待审技能。
  Future<void> review(String agent, String skillId,
      {required bool approve, String notes = ''}) async {
    await _call('review_evolved', {
      'agent': agent,
      'skill_id': skillId,
      'approve': approve,
      'notes': notes,
    });
  }
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
