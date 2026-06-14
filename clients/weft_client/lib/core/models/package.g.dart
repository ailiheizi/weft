// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'package.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

_PackageInfo _$PackageInfoFromJson(Map<String, dynamic> json) => _PackageInfo(
  name: json['name'] as String,
  version: json['version'] as String?,
  overrides: json['overrides'] == null
      ? const []
      : const _OverridesConverter().fromJson(json['overrides']),
  enabled: json['enabled'] as bool? ?? true,
  hasUi: json['has_ui'] as bool? ?? false,
  description: json['description'] as String?,
  runtime:
      $enumDecodeNullable(_$PackageRuntimeEnumMap, json['runtime']) ??
      PackageRuntime.unknown,
);

Map<String, dynamic> _$PackageInfoToJson(_PackageInfo instance) =>
    <String, dynamic>{
      'name': instance.name,
      'version': instance.version,
      'overrides': const _OverridesConverter().toJson(instance.overrides),
      'enabled': instance.enabled,
      'has_ui': instance.hasUi,
      'description': instance.description,
      'runtime': _$PackageRuntimeEnumMap[instance.runtime]!,
    };

const _$PackageRuntimeEnumMap = {
  PackageRuntime.wasm: 'wasm',
  PackageRuntime.native: 'native',
  PackageRuntime.service: 'service',
  PackageRuntime.embedded: 'embedded',
  PackageRuntime.remote: 'remote',
  PackageRuntime.unknown: 'unknown',
};
