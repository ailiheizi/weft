import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';

import '../api/weft_claw_api.dart';
import '../models/chat.dart';
import '../services/chat_storage.dart';
import 'data_providers.dart';
import 'sessions_provider.dart';

const _uuid = Uuid();

class ChatNotifier extends StateNotifier<ChatSession> {
  ChatNotifier(this._api, this._storage, this._ref, String sessionId)
      : super(ChatSession(id: sessionId)) {
    _loadHistory(sessionId);
    _autoSelectProvider();
  }

  final WeftClawApi _api;
  final ChatStorage _storage;
  final Ref _ref;

  /// 当前进行中的轮询 timer 与请求取消令牌（供 stopStreaming 中断）
  Timer? _pollTimer;
  CancelToken? _cancelToken;
  bool _stopRequested = false;

  Future<void> _loadHistory(String sessionId) async {
    try {
      // 优先从 weft-claw 加载历史消息
      final rcMessages = await _api.getSessionMessages(sessionId);
      if (rcMessages.isNotEmpty) {
        var messages = rcMessages
            .where((m) => m.role == 'user' || m.role == 'assistant')
            .map((m) => ChatMessage(
                  id: _uuid.v4(),
                  role: m.role,
                  content: m.content,
                ))
            .toList();
        // 恢复历史工具产物：从 stream/events 重建 steps，挂到最后一条 assistant 消息。
        // 否则切换会话回来后工作区/时间线产物会丢失（消息历史不含 steps）。
        try {
          final history = await _api.getStreamEvents(sessionId, afterSeq: 0);
          if (history.events.isNotEmpty) {
            final steps = <ExecutionStep>[];
            final started = <String, Map<String, dynamic>>{};
            _applyEvents(history.events, steps, started);
            if (steps.isNotEmpty) {
              final lastAssistant =
                  messages.lastIndexWhere((m) => m.role == 'assistant');
              if (lastAssistant >= 0) {
                messages[lastAssistant] = messages[lastAssistant]
                    .copyWith(steps: List.unmodifiable(steps));
              }
            }
          }
        } catch (_) {}
        if (mounted) state = state.copyWith(messages: messages);
        return;
      }
    } catch (_) {}

    // 降级：从本地缓存加载
    try {
      final messages = await _storage.loadSession(sessionId);
      if (messages.isNotEmpty && mounted) {
        state = state.copyWith(messages: messages);
      }
    } catch (_) {}
  }

  Future<void> _autoSelectProvider() async {
    if (state.selectedProvider != null) return;
    try {
      final providers = await _ref.read(providersProvider.future);
      if (providers.isNotEmpty && mounted) {
        final p = providers.first;
        state = state.copyWith(
          selectedProvider: p.name,
          selectedModel: p.models.isNotEmpty ? p.models.first : null,
        );
      }
    } catch (_) {}
  }

  Future<void> sendMessage(String content) async {
    if (content.trim().isEmpty) return;
    if (state.isStreaming) return;

    _stopRequested = false;
    _cancelToken = CancelToken();

    // 斜杠指令 /team：强制走多 agent 团队编排。UI 仍显示用户原文(去掉指令前缀),
    // 但发给后端的内容包成强指令,让 AI 务必调用 delegate_to_team 而非自己直接做。
    final trimmed = content.trim();
    final isTeamCommand = trimmed.toLowerCase().startsWith('/team ') ||
        trimmed.toLowerCase() == '/team';
    final displayContent = isTeamCommand
        ? trimmed.replaceFirst(RegExp(r'^/team\s*', caseSensitive: false), '').trim()
        : trimmed;
    final backendContent = isTeamCommand
        ? '这是需要团队协作的复杂任务，请务必调用 delegate_to_team 组建团队（planner→implementer→reviewer→integrator）来完成，不要自己直接动手。任务：$displayContent'
        : trimmed;
    if (isTeamCommand && displayContent.isEmpty) return;

    final userMsg = ChatMessage.user(displayContent);
    final isFirstMessage = state.messages.isEmpty;
    state = state.copyWith(
      messages: [...state.messages, userMsg],
      isStreaming: true,
    );

    await _ref.read(sessionsProvider.notifier).onMessageAdded(
          state.id,
          messageCount: state.messages.length,
          firstUserContent: isFirstMessage ? displayContent : null,
        );

    final assistantId = _uuid.v4();
    state = state.copyWith(
      messages: [
        ...state.messages,
        ChatMessage(id: assistantId, role: 'assistant', content: ''),
      ],
    );

    final sessionId = state.id;
    // 先拿当前 latest_seq，跳过历史事件，只处理本次消息产生的新事件
    int latestSeq = 0;
    try {
      final baseline = await _api.getSessionEvents(sessionId, afterSeq: 0);
      latestSeq = baseline.latestSeq;
    } catch (_) {}
    // 增量追加用的可变 steps 列表和 started 索引
    final liveSteps = <ExecutionStep>[];
    final liveStarted = <String, Map<String, dynamic>>{};
    // token 级流式：累积助手正文（来自 /api/stream/tokens 的 native buffer）
    final replyBuffer = StringBuffer();

    // 增量轮询：走 lock-free 的 /api/stream/events + /api/stream/tokens，
    // send_message 执行期间也能实时拉取事件与正文 token
    _pollTimer = Timer.periodic(const Duration(milliseconds: 200), (_) async {
      if (!mounted || _stopRequested) return;
      try {
        var changed = false;

        // 1) 正文 token 增量（消费后 native buffer 清空）
        final tokens =
            await _api.getStreamTokens(sessionId, cancelToken: _cancelToken);
        if (tokens.isNotEmpty) {
          replyBuffer.writeAll(tokens);
          changed = true;
        }

        // 2) 工具/思考事件增量
        final eventsResult = await _api.getStreamEvents(sessionId,
            afterSeq: latestSeq, cancelToken: _cancelToken);
        if (eventsResult.events.isNotEmpty) {
          latestSeq = eventsResult.latestSeq;
          _applyEvents(eventsResult.events, liveSteps, liveStarted);
          changed = true;
        }

        if (!changed || !mounted || _stopRequested) return;
        final partial = _extractDisplayText(replyBuffer.toString());
        final updatedMessages = state.messages.map((m) {
          if (m.id != assistantId) return m;
          return m.copyWith(
            content: partial.isNotEmpty ? partial : m.content,
            steps: List.unmodifiable(liveSteps),
          );
        }).toList();
        state = state.copyWith(messages: updatedMessages);
      } on DioException catch (e) {
        // 取消导致的异常吞掉，其余忽略（下一拍重试）
        if (CancelToken.isCancel(e)) return;
      } catch (_) {}
    });

    try {
      final reply = await _api.sendMessage(
        sessionId,
        backendContent,
        model: state.selectedModel,
        cancelToken: _cancelToken,
      );

      _pollTimer?.cancel();
      _pollTimer = null;

      // 最终补全：用 stream/events 拉完所有剩余事件（绕过 WASM 锁）
      final finalEvents =
          await _api.getStreamEvents(sessionId, afterSeq: latestSeq);
      if (finalEvents.events.isNotEmpty) {
        _applyEvents(finalEvents.events, liveSteps, liveStarted);
      }

      // 把所有仍在 Running… 的工具标记为完成（无结果）
      for (var i = 0; i < liveSteps.length; i++) {
        final s = liveSteps[i];
        if (s is ToolCallStep && s.result == null) {
          liveSteps[i] = ExecutionStep.toolCall(
            id: s.id,
            name: s.name,
            arguments: s.arguments,
            result: '—',
          );
        }
      }

      // 最终正文：优先用 send_message 的完整 reply，回退到流式累积值。
      // 两者都经协议清洗（_extractDisplayText），避免把原始
      // {"mode":"reply","assistant":...} 协议 JSON 直接显示给用户。
      final finalContent = reply.isNotEmpty
          ? _extractDisplayText(reply)
          : _extractDisplayText(replyBuffer.toString());

      if (mounted) {
        final updatedMessages = state.messages.map((m) {
          if (m.id != assistantId) return m;
          return m.copyWith(
              content: finalContent, steps: List.unmodifiable(liveSteps));
        }).toList();
        state = state.copyWith(messages: updatedMessages);
      }

      // 本地缓存最终消息（离线备份）
      final finalMsg =
          state.messages.where((m) => m.id == assistantId).firstOrNull;
      if (finalMsg != null && finalMsg.content.isNotEmpty) {
        await _storage.saveMessage(state.id, userMsg);
        await _storage.saveMessage(state.id, finalMsg);
        await _ref.read(sessionsProvider.notifier).onMessageAdded(
              state.id,
              messageCount: state.messages.length,
            );
      }
    } on DioException catch (e) {
      _pollTimer?.cancel();
      _pollTimer = null;
      if (CancelToken.isCancel(e)) {
        // 用户主动停止：保留已流式出的部分正文，标注中断
        if (mounted) _markAssistantStopped(assistantId, replyBuffer.toString());
      } else if (mounted) {
        _setAssistantError(assistantId, '[Error: $e]');
      }
    } catch (e) {
      _pollTimer?.cancel();
      _pollTimer = null;
      if (mounted) _setAssistantError(assistantId, '[Error: $e]');
    } finally {
      _cancelToken = null;
      if (mounted) state = state.copyWith(isStreaming: false);
    }
  }

  void _applyEvents(
    List<Map<String, dynamic>> events,
    List<ExecutionStep> steps,
    Map<String, Map<String, dynamic>> started,
  ) {
    for (final e in events) {
      final type = e['type'] as String? ?? '';
      final payload = e['payload'];
      final p =
          payload is Map<String, dynamic> ? payload : <String, dynamic>{};

      switch (type) {
        case 'tool.started':
          final id = p['tool_call_id'] as String? ??
              e['event_id'] as String? ??
              _uuid.v4();
          started[id] = p;
          final toolName = p['tool'] as String? ?? '';
          final argsRaw = p['args'];
          final args = argsRaw == null
              ? ''
              : argsRaw is String
                  ? argsRaw
                  : jsonEncode(argsRaw);
          if (toolName.isNotEmpty) {
            steps.add(ExecutionStep.toolCall(
              id: id,
              name: toolName,
              arguments: args,
              result: null,
            ));
          }

        case 'tool.finished':
          final id = p['tool_call_id'] as String? ?? '';
          final startPayload = started[id] ?? p;
          final toolName =
              startPayload['tool'] as String? ?? p['tool'] as String? ?? '';
          final argsRaw = startPayload['args'];
          final args = argsRaw == null
              ? ''
              : argsRaw is String
                  ? argsRaw
                  : jsonEncode(argsRaw);
          final preview = p['output_preview'] as String?;
          String? result;
          if (preview != null && preview.isNotEmpty) {
            try {
              final parsed = jsonDecode(preview) as Map<String, dynamic>;
              final data = parsed['data'];
              if (data is Map) {
                if (data['entries'] is List) {
                  result = (data['entries'] as List)
                      .map((e) {
                        final em = e as Map<String, dynamic>;
                        return '${em['is_dir'] == true ? '📁' : '📄'} ${em['name']}';
                      })
                      .take(20)
                      .join('\n');
                } else if (data['stdout'] is String) {
                  result = (data['stdout'] as String).trim();
                  if (result.length > 400) result = '${result.substring(0, 400)}…';
                } else {
                  result = jsonEncode(data);
                  if (result.length > 400) result = '${result.substring(0, 400)}…';
                }
              } else {
                result = preview.length > 400
                    ? '${preview.substring(0, 400)}…'
                    : preview;
              }
            } catch (_) {
              result = preview.length > 400
                  ? '${preview.substring(0, 400)}…'
                  : preview;
            }
          }
          final finishedStep = ExecutionStep.toolCall(
            id: id.isEmpty ? _uuid.v4() : id,
            name: toolName,
            arguments: args,
            result: result,
          );
          final existingIdx =
              steps.indexWhere((s) => s is ToolCallStep && s.id == id);
          if (existingIdx >= 0) {
            steps[existingIdx] = finishedStep;
          } else {
            steps.add(finishedStep);
          }

        case 'thinking':
          final thought =
              p['content'] as String? ?? p['thinking'] as String? ?? '';
          if (thought.isNotEmpty) {
            steps.add(ExecutionStep.thinking(content: thought));
          }

        case 'ask_user':
          // AI 用 ask_user 工具向用户提问 + 给选项;渲染成可点按钮。
          final question = p['question'] as String? ?? '';
          final opts = (p['options'] is List)
              ? (p['options'] as List).whereType<String>().toList()
              : <String>[];
          if (question.isNotEmpty) {
            steps.add(ExecutionStep.askUser(question: question, options: opts));
          }
      }
    }
  }

  /// 清洗流式/最终正文：agent-core 期望 LLM 返回结构化协议
  /// `{"mode":"reply|tool","assistant":"...","tool_calls":[...]}`。
  /// 正常时 sendMessage 已解析出干净 reply，但当原始协议 JSON 漏进 token 流
  /// （解析失败/流式直出）时，避免把整坨 JSON 显示给用户——提取 assistant 字段。
  static String _extractDisplayText(String raw) {
    final s = raw.trimLeft();
    // 不是协议 JSON（不以 { 开头且不含 "mode"），原样返回。
    if (!s.startsWith('{') || !s.contains('"mode"')) return raw;
    // 先尝试完整 JSON 解析。
    try {
      final obj = jsonDecode(s);
      if (obj is Map && obj['assistant'] is String) {
        return (obj['assistant'] as String);
      }
    } catch (_) {
      // 流式中途 JSON 不完整：用正则尽量提取 "assistant":"..." 的值。
    }
    // 匹配 "assistant":"..."，值里允许转义序列（\" \\ 等）。
    final m = RegExp('"assistant"\\s*:\\s*"((?:\\\\.|[^"\\\\])*)"').firstMatch(s);
    if (m != null) {
      // 反转义常见序列。
      return m
          .group(1)!
          .replaceAll(r'\n', '\n')
          .replaceAll(r'\"', '"')
          .replaceAll(r'\\', '\\');
    }
    // 协议 JSON 但还没流式到 assistant 字段：暂不显示原始 JSON（显示空，等下一拍）。
    return '';
  }

  void _setAssistantError(String assistantId, String error) {
    final updatedMessages = state.messages.map((m) {
      if (m.id == assistantId && m.content.isEmpty) {
        return m.copyWith(content: error);
      }
      return m;
    }).toList();
    state = state.copyWith(messages: updatedMessages);
  }

  /// 用户主动停止：保留已流式出的部分正文，空内容则标注已中断
  void _markAssistantStopped(String assistantId, String partial) {
    final updatedMessages = state.messages.map((m) {
      if (m.id != assistantId) return m;
      final text = partial.trim().isNotEmpty ? partial : '[已停止]';
      return m.copyWith(content: text);
    }).toList();
    state = state.copyWith(messages: updatedMessages);
  }

  /// 真正中断生成：取消进行中的请求与轮询 timer，并复位流式状态
  void stopStreaming() {
    if (!state.isStreaming) return;
    _stopRequested = true;
    _pollTimer?.cancel();
    _pollTimer = null;
    _cancelToken?.cancel('stopped_by_user');
    // 不在此处复位正文/状态：交给 sendMessage 的 DioException(cancel) 分支与 finally 收尾
  }

  @override
  void dispose() {
    _stopRequested = true;
    _pollTimer?.cancel();
    _pollTimer = null;
    if (_cancelToken?.isCancelled == false) {
      _cancelToken?.cancel('disposed');
    }
    super.dispose();
  }

  void clearMessages() {
    if (mounted) {
      state = ChatSession(
        id: state.id,
        selectedProvider: state.selectedProvider,
        selectedModel: state.selectedModel,
      );
    }
  }

  void setProvider(String? provider) {
    if (mounted) {
      state = state.copyWith(selectedProvider: provider, selectedModel: null);
    }
  }

  void setModel(String? model) {
    if (mounted) state = state.copyWith(selectedModel: model);
  }
}

final activeSessionIdProvider = StateProvider<String?>((ref) => null);

final _chatStorageProvider = Provider<ChatStorage>((ref) => ChatStorage());

final chatProvider =
    StateNotifierProvider.family<ChatNotifier, ChatSession, String>(
  (ref, sessionId) {
    final api = ref.watch(weftClawApiProvider);
    final storage = ref.watch(_chatStorageProvider);
    return ChatNotifier(api, storage, ref, sessionId);
  },
);
