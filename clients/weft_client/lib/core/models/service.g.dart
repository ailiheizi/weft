// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'service.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

_ServiceInfo _$ServiceInfoFromJson(Map<String, dynamic> json) => _ServiceInfo(
  name: json['name'] as String,
  status: $enumDecode(_$ServiceStatusEnumMap, json['status']),
  description: json['description'] as String?,
);

Map<String, dynamic> _$ServiceInfoToJson(_ServiceInfo instance) =>
    <String, dynamic>{
      'name': instance.name,
      'status': _$ServiceStatusEnumMap[instance.status]!,
      'description': instance.description,
    };

const _$ServiceStatusEnumMap = {
  ServiceStatus.running: 'running',
  ServiceStatus.stopped: 'stopped',
  ServiceStatus.error: 'error',
  ServiceStatus.unknown: 'unknown',
};
