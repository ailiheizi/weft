// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'app.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

_ResolvedApp _$ResolvedAppFromJson(Map<String, dynamic> json) => _ResolvedApp(
  name: json['name'] as String,
  version: json['version'] as String,
  displayName: json['display_name'] as String,
  description: json['description'] as String,
  capabilities:
      (json['capabilities'] as List<dynamic>?)
          ?.map((e) => e as String)
          .toList() ??
      const [],
  enabledFeatures:
      (json['enabled_features'] as List<dynamic>?)
          ?.map((e) => e as String)
          .toList() ??
      const [],
  bindings:
      (json['bindings'] as List<dynamic>?)
          ?.map((e) => AppBindingResolution.fromJson(e as Map<String, dynamic>))
          .toList() ??
      const [],
  validationChecks:
      (json['validation_checks'] as List<dynamic>?)
          ?.map((e) => e as String)
          .toList() ??
      const [],
  errors:
      (json['errors'] as List<dynamic>?)?.map((e) => e as String).toList() ??
      const [],
  status: $enumDecode(_$ResolvedAppStatusEnumMap, json['status']),
);

Map<String, dynamic> _$ResolvedAppToJson(_ResolvedApp instance) =>
    <String, dynamic>{
      'name': instance.name,
      'version': instance.version,
      'display_name': instance.displayName,
      'description': instance.description,
      'capabilities': instance.capabilities,
      'enabled_features': instance.enabledFeatures,
      'bindings': instance.bindings,
      'validation_checks': instance.validationChecks,
      'errors': instance.errors,
      'status': _$ResolvedAppStatusEnumMap[instance.status]!,
    };

const _$ResolvedAppStatusEnumMap = {
  ResolvedAppStatus.ok: 'ok',
  ResolvedAppStatus.degraded: 'degraded',
  ResolvedAppStatus.error: 'error',
  ResolvedAppStatus.unknown: 'unknown',
  ResolvedAppStatus.resolved: 'Resolved',
  ResolvedAppStatus.partial: 'Partial',
  ResolvedAppStatus.failed: 'Failed',
};

_AppBindingResolution _$AppBindingResolutionFromJson(
  Map<String, dynamic> json,
) => _AppBindingResolution(
  capability: json['capability'] as String,
  provider: json['provider'] as String,
  mutable: json['mutable'] as bool? ?? false,
  source: json['source'] as String? ?? '',
);

Map<String, dynamic> _$AppBindingResolutionToJson(
  _AppBindingResolution instance,
) => <String, dynamic>{
  'capability': instance.capability,
  'provider': instance.provider,
  'mutable': instance.mutable,
  'source': instance.source,
};
