import 'package:freezed_annotation/freezed_annotation.dart';

part 'service.freezed.dart';
part 'service.g.dart';

@freezed
abstract class ServiceInfo with _$ServiceInfo {
  const factory ServiceInfo({
    required String name,
    required ServiceStatus status,
    String? description,
  }) = _ServiceInfo;

  factory ServiceInfo.fromJson(Map<String, dynamic> json) =>
      _$ServiceInfoFromJson(json);
}

enum ServiceStatus {
  @JsonValue('running') running,
  @JsonValue('stopped') stopped,
  @JsonValue('error') error,
  @JsonValue('unknown') unknown,
}
