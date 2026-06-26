import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/providers/core_repository.dart';

/// 一条对话消息。
@immutable
class DirectorMessage {
  const DirectorMessage({
    required this.role, // 'user' | 'assistant'
    required this.content,
    this.askUserQuestion,
    this.askUserOptions = const [],
  });

  final String role;
  final String content;
  final String? askUserQuestion;
  final List<String> askUserOptions;

  bool get isUser => role == 'user';
}

/// 右栏导演对话的状态。
@immutable
class DirectorChatState {
  const DirectorChatState({
    this.sessionId,
    this.messages = const [],
    this.sending = false,
    this.error,
  });

  final String? sessionId;
  final List<DirectorMessage> messages;
  final bool sending;
  final String? error;

  DirectorChatState copyWith({
    Object? sessionId = _sentinel,
    List<DirectorMessage>? messages,
    bool? sending,
    Object? error = _sentinel,
  }) {
    return DirectorChatState(
      sessionId: sessionId == _sentinel ? this.sessionId : sessionId as String?,
      messages: messages ?? this.messages,
      sending: sending ?? this.sending,
      error: error == _sentinel ? this.error : error as String?,
    );
  }
}

const Object _sentinel = Object();

/// 导演对话管理器 — 维护多轮消息流，调 director.turn/send_message。
class DirectorChatNotifier extends Notifier<DirectorChatState> {
  @override
  DirectorChatState build() => const DirectorChatState();

  /// 发送一条用户消息，追加助手回复。可选 contextHint：把画布选中节点等
  /// 上下文拼进 content（生成参考）。
  Future<void> send(String text, {String? contextHint}) async {
    final content = text.trim();
    if (content.isEmpty || state.sending) return;

    final sessionId = state.sessionId ?? _genSessionId();
    final composed = contextHint == null || contextHint.isEmpty
        ? content
        : '$content\n\n[画布上下文] $contextHint';

    state = state.copyWith(
      sessionId: sessionId,
      sending: true,
      error: null,
      messages: [...state.messages, DirectorMessage(role: 'user', content: content)],
    );

    try {
      final result = await ref.read(coreRepositoryProvider).runApp(
            'ai-director',
            'director.turn',
            'send_message',
            {'content': composed, 'session_id': sessionId},
          );
      final reply = _extractReply(result);
      final question = _extractAskUserQuestion(result);
      final options = _extractAskUserOptions(result);
      state = state.copyWith(
        sending: false,
        messages: [
          ...state.messages,
          DirectorMessage(
            role: 'assistant',
            content: reply,
            askUserQuestion: question,
            askUserOptions: options,
          ),
        ],
      );
    } catch (error) {
      state = state.copyWith(
        sending: false,
        error: '主导演暂时没有响应，请稍后重试。',
      );
    }
  }

  String _genSessionId() =>
      'hub-${DateTime.now().microsecondsSinceEpoch}';

  // ── 响应解析（沿用既有 workbench 的钻取逻辑）──

  Map<String, dynamic>? _asMap(dynamic v) =>
      v is Map<String, dynamic> ? v : (v is Map ? Map<String, dynamic>.from(v) : null);

  String _extractReply(Map<String, dynamic> result) {
    final data = _asMap(_asMap(_asMap(result['result'])?['response'])?['data']);
    final reply = data?['reply'];
    if (reply is String && reply.isNotEmpty) return reply;
    // 兜底深搜
    final found = _deepFind(result, 'reply');
    return found is String ? found : '（导演没有返回文本）';
  }

  String? _extractAskUserQuestion(Map<String, dynamic> result) {
    final q = _deepFind(result, 'ask_user_question') ?? _deepFind(result, 'question');
    return q is String && q.isNotEmpty ? q : null;
  }

  List<String> _extractAskUserOptions(Map<String, dynamic> result) {
    final opts = _deepFind(result, 'ask_user_options') ?? _deepFind(result, 'options');
    if (opts is List) {
      return opts.map((e) => e.toString()).toList();
    }
    return const [];
  }

  dynamic _deepFind(dynamic o, String key) {
    if (o is Map) {
      if (o.containsKey(key) && o[key] != null) return o[key];
      for (final v in o.values) {
        final r = _deepFind(v, key);
        if (r != null) return r;
      }
    } else if (o is List) {
      for (final v in o) {
        final r = _deepFind(v, key);
        if (r != null) return r;
      }
    }
    return null;
  }
}

final directorChatProvider =
    NotifierProvider<DirectorChatNotifier, DirectorChatState>(DirectorChatNotifier.new);
