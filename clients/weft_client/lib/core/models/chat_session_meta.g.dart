// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'chat_session_meta.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

_ChatSessionMeta _$ChatSessionMetaFromJson(Map<String, dynamic> json) =>
    _ChatSessionMeta(
      id: json['id'] as String,
      title: json['title'] as String,
      createdAt: DateTime.parse(json['createdAt'] as String),
      updatedAt: DateTime.parse(json['updatedAt'] as String),
      messageCount: (json['messageCount'] as num?)?.toInt() ?? 0,
      provider: json['provider'] as String?,
      model: json['model'] as String?,
    );

Map<String, dynamic> _$ChatSessionMetaToJson(_ChatSessionMeta instance) =>
    <String, dynamic>{
      'id': instance.id,
      'title': instance.title,
      'createdAt': instance.createdAt.toIso8601String(),
      'updatedAt': instance.updatedAt.toIso8601String(),
      'messageCount': instance.messageCount,
      'provider': instance.provider,
      'model': instance.model,
    };
