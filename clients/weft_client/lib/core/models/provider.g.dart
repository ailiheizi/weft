// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'provider.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

_ProviderConfig _$ProviderConfigFromJson(Map<String, dynamic> json) =>
    _ProviderConfig(
      name: json['name'] as String,
      baseUrl: json['base_url'] as String,
      format: json['format'] as String? ?? 'openai',
      models:
          (json['models'] as List<dynamic>?)
              ?.map((e) => e as String)
              .toList() ??
          const [],
      keys:
          (json['keys'] as List<dynamic>?)
              ?.map((e) => ApiKeyConfig.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
    );

Map<String, dynamic> _$ProviderConfigToJson(_ProviderConfig instance) =>
    <String, dynamic>{
      'name': instance.name,
      'base_url': instance.baseUrl,
      'format': instance.format,
      'models': instance.models,
      'keys': instance.keys,
    };

_ApiKeyConfig _$ApiKeyConfigFromJson(Map<String, dynamic> json) =>
    _ApiKeyConfig(
      key: json['key'] as String,
      label: json['label'] as String?,
      enabled: json['enabled'] as bool? ?? true,
    );

Map<String, dynamic> _$ApiKeyConfigToJson(_ApiKeyConfig instance) =>
    <String, dynamic>{
      'key': instance.key,
      'label': instance.label,
      'enabled': instance.enabled,
    };
