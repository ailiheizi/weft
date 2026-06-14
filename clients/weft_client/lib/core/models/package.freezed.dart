// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'package.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$PackageInfo {

 String get name; String? get version;@_OverridesConverter() List<String> get overrides; bool get enabled;@JsonKey(name: 'has_ui') bool get hasUi; String? get description;// runtime field is optional — not always returned by API
 PackageRuntime get runtime;
/// Create a copy of PackageInfo
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PackageInfoCopyWith<PackageInfo> get copyWith => _$PackageInfoCopyWithImpl<PackageInfo>(this as PackageInfo, _$identity);

  /// Serializes this PackageInfo to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PackageInfo&&(identical(other.name, name) || other.name == name)&&(identical(other.version, version) || other.version == version)&&const DeepCollectionEquality().equals(other.overrides, overrides)&&(identical(other.enabled, enabled) || other.enabled == enabled)&&(identical(other.hasUi, hasUi) || other.hasUi == hasUi)&&(identical(other.description, description) || other.description == description)&&(identical(other.runtime, runtime) || other.runtime == runtime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,name,version,const DeepCollectionEquality().hash(overrides),enabled,hasUi,description,runtime);

@override
String toString() {
  return 'PackageInfo(name: $name, version: $version, overrides: $overrides, enabled: $enabled, hasUi: $hasUi, description: $description, runtime: $runtime)';
}


}

/// @nodoc
abstract mixin class $PackageInfoCopyWith<$Res>  {
  factory $PackageInfoCopyWith(PackageInfo value, $Res Function(PackageInfo) _then) = _$PackageInfoCopyWithImpl;
@useResult
$Res call({
 String name, String? version,@_OverridesConverter() List<String> overrides, bool enabled,@JsonKey(name: 'has_ui') bool hasUi, String? description, PackageRuntime runtime
});




}
/// @nodoc
class _$PackageInfoCopyWithImpl<$Res>
    implements $PackageInfoCopyWith<$Res> {
  _$PackageInfoCopyWithImpl(this._self, this._then);

  final PackageInfo _self;
  final $Res Function(PackageInfo) _then;

/// Create a copy of PackageInfo
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? name = null,Object? version = freezed,Object? overrides = null,Object? enabled = null,Object? hasUi = null,Object? description = freezed,Object? runtime = null,}) {
  return _then(_self.copyWith(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,version: freezed == version ? _self.version : version // ignore: cast_nullable_to_non_nullable
as String?,overrides: null == overrides ? _self.overrides : overrides // ignore: cast_nullable_to_non_nullable
as List<String>,enabled: null == enabled ? _self.enabled : enabled // ignore: cast_nullable_to_non_nullable
as bool,hasUi: null == hasUi ? _self.hasUi : hasUi // ignore: cast_nullable_to_non_nullable
as bool,description: freezed == description ? _self.description : description // ignore: cast_nullable_to_non_nullable
as String?,runtime: null == runtime ? _self.runtime : runtime // ignore: cast_nullable_to_non_nullable
as PackageRuntime,
  ));
}

}


/// Adds pattern-matching-related methods to [PackageInfo].
extension PackageInfoPatterns on PackageInfo {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _PackageInfo value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _PackageInfo() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _PackageInfo value)  $default,){
final _that = this;
switch (_that) {
case _PackageInfo():
return $default(_that);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _PackageInfo value)?  $default,){
final _that = this;
switch (_that) {
case _PackageInfo() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String name,  String? version, @_OverridesConverter()  List<String> overrides,  bool enabled, @JsonKey(name: 'has_ui')  bool hasUi,  String? description,  PackageRuntime runtime)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _PackageInfo() when $default != null:
return $default(_that.name,_that.version,_that.overrides,_that.enabled,_that.hasUi,_that.description,_that.runtime);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String name,  String? version, @_OverridesConverter()  List<String> overrides,  bool enabled, @JsonKey(name: 'has_ui')  bool hasUi,  String? description,  PackageRuntime runtime)  $default,) {final _that = this;
switch (_that) {
case _PackageInfo():
return $default(_that.name,_that.version,_that.overrides,_that.enabled,_that.hasUi,_that.description,_that.runtime);case _:
  throw StateError('Unexpected subclass');

}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String name,  String? version, @_OverridesConverter()  List<String> overrides,  bool enabled, @JsonKey(name: 'has_ui')  bool hasUi,  String? description,  PackageRuntime runtime)?  $default,) {final _that = this;
switch (_that) {
case _PackageInfo() when $default != null:
return $default(_that.name,_that.version,_that.overrides,_that.enabled,_that.hasUi,_that.description,_that.runtime);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _PackageInfo implements PackageInfo {
  const _PackageInfo({required this.name, this.version, @_OverridesConverter() final  List<String> overrides = const [], this.enabled = true, @JsonKey(name: 'has_ui') this.hasUi = false, this.description, this.runtime = PackageRuntime.unknown}): _overrides = overrides;
  factory _PackageInfo.fromJson(Map<String, dynamic> json) => _$PackageInfoFromJson(json);

@override final  String name;
@override final  String? version;
 final  List<String> _overrides;
@override@JsonKey()@_OverridesConverter() List<String> get overrides {
  if (_overrides is EqualUnmodifiableListView) return _overrides;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_overrides);
}

@override@JsonKey() final  bool enabled;
@override@JsonKey(name: 'has_ui') final  bool hasUi;
@override final  String? description;
// runtime field is optional — not always returned by API
@override@JsonKey() final  PackageRuntime runtime;

/// Create a copy of PackageInfo
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$PackageInfoCopyWith<_PackageInfo> get copyWith => __$PackageInfoCopyWithImpl<_PackageInfo>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$PackageInfoToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _PackageInfo&&(identical(other.name, name) || other.name == name)&&(identical(other.version, version) || other.version == version)&&const DeepCollectionEquality().equals(other._overrides, _overrides)&&(identical(other.enabled, enabled) || other.enabled == enabled)&&(identical(other.hasUi, hasUi) || other.hasUi == hasUi)&&(identical(other.description, description) || other.description == description)&&(identical(other.runtime, runtime) || other.runtime == runtime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,name,version,const DeepCollectionEquality().hash(_overrides),enabled,hasUi,description,runtime);

@override
String toString() {
  return 'PackageInfo(name: $name, version: $version, overrides: $overrides, enabled: $enabled, hasUi: $hasUi, description: $description, runtime: $runtime)';
}


}

/// @nodoc
abstract mixin class _$PackageInfoCopyWith<$Res> implements $PackageInfoCopyWith<$Res> {
  factory _$PackageInfoCopyWith(_PackageInfo value, $Res Function(_PackageInfo) _then) = __$PackageInfoCopyWithImpl;
@override @useResult
$Res call({
 String name, String? version,@_OverridesConverter() List<String> overrides, bool enabled,@JsonKey(name: 'has_ui') bool hasUi, String? description, PackageRuntime runtime
});




}
/// @nodoc
class __$PackageInfoCopyWithImpl<$Res>
    implements _$PackageInfoCopyWith<$Res> {
  __$PackageInfoCopyWithImpl(this._self, this._then);

  final _PackageInfo _self;
  final $Res Function(_PackageInfo) _then;

/// Create a copy of PackageInfo
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? name = null,Object? version = freezed,Object? overrides = null,Object? enabled = null,Object? hasUi = null,Object? description = freezed,Object? runtime = null,}) {
  return _then(_PackageInfo(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,version: freezed == version ? _self.version : version // ignore: cast_nullable_to_non_nullable
as String?,overrides: null == overrides ? _self._overrides : overrides // ignore: cast_nullable_to_non_nullable
as List<String>,enabled: null == enabled ? _self.enabled : enabled // ignore: cast_nullable_to_non_nullable
as bool,hasUi: null == hasUi ? _self.hasUi : hasUi // ignore: cast_nullable_to_non_nullable
as bool,description: freezed == description ? _self.description : description // ignore: cast_nullable_to_non_nullable
as String?,runtime: null == runtime ? _self.runtime : runtime // ignore: cast_nullable_to_non_nullable
as PackageRuntime,
  ));
}


}

// dart format on
