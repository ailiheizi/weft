import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:uuid/uuid.dart';

part 'chat.freezed.dart';

const _uuid = Uuid();

/// 单个执行步骤（thinking 或 tool_call）
@freezed
abstract class ExecutionStep with _$ExecutionStep {
  const factory ExecutionStep.thinking({
    required String content,
  }) = ThinkingStep;

  const factory ExecutionStep.toolCall({
    required String id,
    required String name,
    required String arguments,
    String? result,
  }) = ToolCallStep;

  /// AI 向用户提问并提供可点选项(ask_user 工具)。
  /// 用户点选项后作为新消息发送,继续对话。
  const factory ExecutionStep.askUser({
    required String question,
    @Default([]) List<String> options,
  }) = AskUserStep;
}

@freezed
abstract class ChatMessage with _$ChatMessage {
  const factory ChatMessage({
    required String id,
    required String role,
    required String content,
    @Default([]) List<ExecutionStep> steps,
  }) = _ChatMessage;

  factory ChatMessage.user(String content) => ChatMessage(
        id: _uuid.v4(),
        role: 'user',
        content: content,
      );

  factory ChatMessage.assistant(String content) => ChatMessage(
        id: _uuid.v4(),
        role: 'assistant',
        content: content,
      );
}

@freezed
abstract class ChatSession with _$ChatSession {
  const factory ChatSession({
    required String id,
    @Default([]) List<ChatMessage> messages,
    @Default(false) bool isStreaming,
    String? selectedProvider,
    String? selectedModel,
  }) = _ChatSession;

  factory ChatSession.empty() => ChatSession(id: _uuid.v4());
}
