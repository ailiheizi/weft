import 'package:freezed_annotation/freezed_annotation.dart';

part 'package.freezed.dart';
part 'package.g.dart';

// API returns overrides as either a string or a list
class _OverridesConverter implements JsonConverter<List<String>, Object?> {
  const _OverridesConverter();

  @override
  List<String> fromJson(Object? json) {
    if (json == null) return [];
    if (json is List) return json.cast<String>();
    if (json is String) {
      if (json.isEmpty) return [];
      return [json];
    }
    return [];
  }

  @override
  Object? toJson(List<String> list) => list;
}

@freezed
abstract class PackageInfo with _$PackageInfo {
  const factory PackageInfo({
    required String name,
    String? version,
    @_OverridesConverter() @Default([]) List<String> overrides,
    @Default(true) bool enabled,
    @JsonKey(name: 'has_ui') @Default(false) bool hasUi,
    String? description,
    // runtime field is optional — not always returned by API
    @Default(PackageRuntime.unknown) PackageRuntime runtime,
  }) = _PackageInfo;

  factory PackageInfo.fromJson(Map<String, dynamic> json) =>
      _$PackageInfoFromJson(json);
}

enum PackageRuntime {
  @JsonValue('wasm') wasm,
  @JsonValue('native') native,
  @JsonValue('service') service,
  @JsonValue('embedded') embedded,
  @JsonValue('remote') remote,
  @JsonValue('unknown') unknown,
}
