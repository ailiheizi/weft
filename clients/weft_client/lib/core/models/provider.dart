import 'package:freezed_annotation/freezed_annotation.dart';

part 'provider.freezed.dart';
part 'provider.g.dart';

@freezed
abstract class ProviderConfig with _$ProviderConfig {
  const factory ProviderConfig({
    required String name,
    @JsonKey(name: 'base_url') required String baseUrl,
    @Default('openai') String format,
    @Default([]) List<String> models,
    @Default([]) List<ApiKeyConfig> keys,
  }) = _ProviderConfig;

  factory ProviderConfig.fromJson(Map<String, dynamic> json) =>
      _$ProviderConfigFromJson(json);
}

@freezed
abstract class ApiKeyConfig with _$ApiKeyConfig {
  const factory ApiKeyConfig({
    // The core API serializes the secret under "value", not "key".
    @JsonKey(name: 'value') required String key,
    String? label,
    @Default(true) bool enabled,
  }) = _ApiKeyConfig;

  factory ApiKeyConfig.fromJson(Map<String, dynamic> json) =>
      _$ApiKeyConfigFromJson(json);
}
