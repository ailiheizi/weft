import 'package:freezed_annotation/freezed_annotation.dart';

part 'chat_session_meta.freezed.dart';
part 'chat_session_meta.g.dart';

@freezed
abstract class ChatSessionMeta with _$ChatSessionMeta {
  const factory ChatSessionMeta({
    required String id,
    required String title,
    required DateTime createdAt,
    required DateTime updatedAt,
    @Default(0) int messageCount,
    String? provider,
    String? model,
  }) = _ChatSessionMeta;

  factory ChatSessionMeta.fromJson(Map<String, dynamic> json) =>
      _$ChatSessionMetaFromJson(json);
}
