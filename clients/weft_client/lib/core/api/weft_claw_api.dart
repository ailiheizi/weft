import 'package:dio/dio.dart';

/// weft-claw capability 的轻量封装
class WeftClawApi {
  WeftClawApi(this._dio);

  final Dio _dio;

  static const _app = 'weft-claw';
  static const _capability = 'weft_claw.turn';

  Future<Map<String, dynamic>> _call(
    String action,
    Map<String, dynamic> data, {
    Duration timeout = const Duration(seconds: 15),
    CancelToken? cancelToken,
  }) async {
    final resp = await _dio.post<Map<String, dynamic>>(
      '/api/apps/$_app/run',
      data: {
        'capability': _capability,
        'action': action,
        'app': _app,
        'data': data,
      },
      options: Options(receiveTimeout: timeout),
      cancelToken: cancelToken,
    );
    final result = resp.data?['result'] as Map<String, dynamic>?;
    final response = result?['response'] as Map<String, dynamic>?;
    return response?['data'] as Map<String, dynamic>? ?? {};
  }

  // ── Sessions ──────────────────────────────────────────────────────────────

  Future<List<WeftClawSession>> listSessions() async {
    final data = await _call('list_sessions', {});
    final items = data['sessions'] as List? ?? [];
    return items
        .whereType<Map<String, dynamic>>()
        .map(WeftClawSession.fromJson)
        .toList();
  }

  Future<void> deleteSession(String sessionId) async {
    await _call('delete_session', {'session_id': sessionId});
  }

  Future<void> resetSession(String sessionId) async {
    await _call('reset_session', {'session_id': sessionId});
  }

  // ── Messages ──────────────────────────────────────────────────────────────

  /// 返回 session 的历史消息（role/content 对）
  Future<List<WeftClawMessage>> getSessionMessages(String sessionId) async {
    final data = await _call('get_session_messages', {'session_id': sessionId});
    final items = data['messages'] as List? ?? [];
    return items
        .whereType<Map<String, dynamic>>()
        .map(WeftClawMessage.fromJson)
        .toList();
  }

  // ── Events ────────────────────────────────────────────────────────────────

  Future<WeftClawEventsResult> getSessionEvents(
    String sessionId, {
    int afterSeq = 0,
    int? limit,
  }) async {
    final data = await _call('get_session_events', {
      'session_id': sessionId,
      'after_seq': afterSeq,
      if (limit != null) 'limit': limit,
    });
    final events = (data['events'] as List? ?? [])
        .whereType<Map<String, dynamic>>()
        .toList();
    return WeftClawEventsResult(
      events: events,
      latestSeq: data['latest_seq'] as int? ?? 0,
    );
  }

  // ── Send ──────────────────────────────────────────────────────────────────

  Future<String> sendMessage(
    String sessionId,
    String content, {
    String? model,
    Duration timeout = const Duration(minutes: 5),
    CancelToken? cancelToken,
  }) async {
    final data = await _call(
      'send_message',
      {
        'session_id': sessionId,
        'content': content,
        if (model != null) 'model': model,
      },
      timeout: timeout,
      cancelToken: cancelToken,
    );
    final agent = data['agent'] as Map<String, dynamic>?;
    return agent?['reply'] as String? ?? data['reply'] as String? ?? '';
  }

  /// 从 native stream buffer 拉取待消费的 token 列表（消费后清空）
  Future<List<String>> getStreamTokens(
    String sessionId, {
    CancelToken? cancelToken,
  }) async {
    final resp = await _dio.get<Map<String, dynamic>>(
      '/api/stream/tokens',
      queryParameters: {'session_id': sessionId},
      options: Options(receiveTimeout: const Duration(seconds: 5)),
      cancelToken: cancelToken,
    );
    final tokens = resp.data?['tokens'] as List?;
    return tokens?.map((e) => e.toString()).toList() ?? [];
  }

  /// 直接读 SQLite，绕过 WASM 锁，send_message 执行期间也能实时拉取
  Future<WeftClawEventsResult> getStreamEvents(
    String sessionId, {
    int afterSeq = 0,
    CancelToken? cancelToken,
  }) async {
    final resp = await _dio.get<Map<String, dynamic>>(
      '/api/stream/events',
      queryParameters: {'session_id': sessionId, 'after_seq': afterSeq},
      options: Options(receiveTimeout: const Duration(seconds: 5)),
      cancelToken: cancelToken,
    );
    final data = resp.data ?? {};
    final events = (data['events'] as List? ?? [])
        .whereType<Map<String, dynamic>>()
        .toList();
    return WeftClawEventsResult(
      events: events,
      latestSeq: data['latest_seq'] as int? ?? afterSeq,
    );
  }
}

// ── Data classes ─────────────────────────────────────────────────────────────

class WeftClawSession {
  const WeftClawSession({
    required this.id,
    required this.title,
    required this.createdAt,
    required this.updatedAt,
  });

  final String id;
  final String title;
  final DateTime createdAt;
  final DateTime updatedAt;

  factory WeftClawSession.fromJson(Map<String, dynamic> json) {
    return WeftClawSession(
      id: json['id'] as String? ?? '',
      title: json['title'] as String? ?? 'Chat',
      createdAt: DateTime.fromMillisecondsSinceEpoch(
          json['created_at'] as int? ?? 0),
      updatedAt: DateTime.fromMillisecondsSinceEpoch(
          json['updated_at'] as int? ?? 0),
    );
  }
}

class WeftClawMessage {
  const WeftClawMessage({required this.role, required this.content});

  final String role;
  final String content;

  factory WeftClawMessage.fromJson(Map<String, dynamic> json) {
    // payload 可能是 Map 或直接字段
    final payload = json['payload'];
    if (payload is Map<String, dynamic>) {
      return WeftClawMessage(
        role: payload['role'] as String? ?? json['role'] as String? ?? 'user',
        content: payload['content'] as String? ?? '',
      );
    }
    return WeftClawMessage(
      role: json['role'] as String? ?? 'user',
      content: json['content'] as String? ?? '',
    );
  }
}

class WeftClawEventsResult {
  const WeftClawEventsResult({
    required this.events,
    required this.latestSeq,
  });

  final List<Map<String, dynamic>> events;
  final int latestSeq;
}
