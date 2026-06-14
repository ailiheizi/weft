import 'package:freezed_annotation/freezed_annotation.dart';

part 'app.freezed.dart';
part 'app.g.dart';

@freezed
abstract class ResolvedApp with _$ResolvedApp {
  const factory ResolvedApp({
    required String name,
    required String version,
    @JsonKey(name: 'display_name') required String displayName,
    required String description,
    @Default([]) List<String> capabilities,
    @Default([]) @JsonKey(name: 'enabled_features') List<String> enabledFeatures,
    @Default([]) List<AppBindingResolution> bindings,
    @Default([]) @JsonKey(name: 'validation_checks') List<String> validationChecks,
    @Default([]) List<String> errors,
    required ResolvedAppStatus status,
  }) = _ResolvedApp;

  factory ResolvedApp.fromJson(Map<String, dynamic> json) =>
      _$ResolvedAppFromJson(json);
}

@freezed
abstract class AppBindingResolution with _$AppBindingResolution {
  const factory AppBindingResolution({
    required String capability,
    required String provider,
    @Default(false) bool mutable,
    @Default('') String source,
  }) = _AppBindingResolution;

  factory AppBindingResolution.fromJson(Map<String, dynamic> json) =>
      _$AppBindingResolutionFromJson(json);
}

enum ResolvedAppStatus {
  @JsonValue('ok') ok,
  @JsonValue('degraded') degraded,
  @JsonValue('error') error,
  @JsonValue('unknown') unknown,
  @JsonValue('Resolved') resolved,
  @JsonValue('Partial') partial,
  @JsonValue('Failed') failed,
}
