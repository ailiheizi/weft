// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'provider.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$ProviderConfig {

 String get name;@JsonKey(name: 'base_url') String get baseUrl; String get format; List<String> get models; List<ApiKeyConfig> get keys;
/// Create a copy of ProviderConfig
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ProviderConfigCopyWith<ProviderConfig> get copyWith => _$ProviderConfigCopyWithImpl<ProviderConfig>(this as ProviderConfig, _$identity);

  /// Serializes this ProviderConfig to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ProviderConfig&&(identical(other.name, name) || other.name == name)&&(identical(other.baseUrl, baseUrl) || other.baseUrl == baseUrl)&&(identical(other.format, format) || other.format == format)&&const DeepCollectionEquality().equals(other.models, models)&&const DeepCollectionEquality().equals(other.keys, keys));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,name,baseUrl,format,const DeepCollectionEquality().hash(models),const DeepCollectionEquality().hash(keys));

@override
String toString() {
  return 'ProviderConfig(name: $name, baseUrl: $baseUrl, format: $format, models: $models, keys: $keys)';
}


}

/// @nodoc
abstract mixin class $ProviderConfigCopyWith<$Res>  {
  factory $ProviderConfigCopyWith(ProviderConfig value, $Res Function(ProviderConfig) _then) = _$ProviderConfigCopyWithImpl;
@useResult
$Res call({
 String name,@JsonKey(name: 'base_url') String baseUrl, String format, List<String> models, List<ApiKeyConfig> keys
});




}
/// @nodoc
class _$ProviderConfigCopyWithImpl<$Res>
    implements $ProviderConfigCopyWith<$Res> {
  _$ProviderConfigCopyWithImpl(this._self, this._then);

  final ProviderConfig _self;
  final $Res Function(ProviderConfig) _then;

/// Create a copy of ProviderConfig
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? name = null,Object? baseUrl = null,Object? format = null,Object? models = null,Object? keys = null,}) {
  return _then(_self.copyWith(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,baseUrl: null == baseUrl ? _self.baseUrl : baseUrl // ignore: cast_nullable_to_non_nullable
as String,format: null == format ? _self.format : format // ignore: cast_nullable_to_non_nullable
as String,models: null == models ? _self.models : models // ignore: cast_nullable_to_non_nullable
as List<String>,keys: null == keys ? _self.keys : keys // ignore: cast_nullable_to_non_nullable
as List<ApiKeyConfig>,
  ));
}

}


/// Adds pattern-matching-related methods to [ProviderConfig].
extension ProviderConfigPatterns on ProviderConfig {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ProviderConfig value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ProviderConfig() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ProviderConfig value)  $default,){
final _that = this;
switch (_that) {
case _ProviderConfig():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ProviderConfig value)?  $default,){
final _that = this;
switch (_that) {
case _ProviderConfig() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String name, @JsonKey(name: 'base_url')  String baseUrl,  String format,  List<String> models,  List<ApiKeyConfig> keys)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ProviderConfig() when $default != null:
return $default(_that.name,_that.baseUrl,_that.format,_that.models,_that.keys);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String name, @JsonKey(name: 'base_url')  String baseUrl,  String format,  List<String> models,  List<ApiKeyConfig> keys)  $default,) {final _that = this;
switch (_that) {
case _ProviderConfig():
return $default(_that.name,_that.baseUrl,_that.format,_that.models,_that.keys);case _:
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String name, @JsonKey(name: 'base_url')  String baseUrl,  String format,  List<String> models,  List<ApiKeyConfig> keys)?  $default,) {final _that = this;
switch (_that) {
case _ProviderConfig() when $default != null:
return $default(_that.name,_that.baseUrl,_that.format,_that.models,_that.keys);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _ProviderConfig implements ProviderConfig {
  const _ProviderConfig({required this.name, @JsonKey(name: 'base_url') required this.baseUrl, this.format = 'openai', final  List<String> models = const [], final  List<ApiKeyConfig> keys = const []}): _models = models,_keys = keys;
  factory _ProviderConfig.fromJson(Map<String, dynamic> json) => _$ProviderConfigFromJson(json);

@override final  String name;
@override@JsonKey(name: 'base_url') final  String baseUrl;
@override@JsonKey() final  String format;
 final  List<String> _models;
@override@JsonKey() List<String> get models {
  if (_models is EqualUnmodifiableListView) return _models;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_models);
}

 final  List<ApiKeyConfig> _keys;
@override@JsonKey() List<ApiKeyConfig> get keys {
  if (_keys is EqualUnmodifiableListView) return _keys;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_keys);
}


/// Create a copy of ProviderConfig
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ProviderConfigCopyWith<_ProviderConfig> get copyWith => __$ProviderConfigCopyWithImpl<_ProviderConfig>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$ProviderConfigToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ProviderConfig&&(identical(other.name, name) || other.name == name)&&(identical(other.baseUrl, baseUrl) || other.baseUrl == baseUrl)&&(identical(other.format, format) || other.format == format)&&const DeepCollectionEquality().equals(other._models, _models)&&const DeepCollectionEquality().equals(other._keys, _keys));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,name,baseUrl,format,const DeepCollectionEquality().hash(_models),const DeepCollectionEquality().hash(_keys));

@override
String toString() {
  return 'ProviderConfig(name: $name, baseUrl: $baseUrl, format: $format, models: $models, keys: $keys)';
}


}

/// @nodoc
abstract mixin class _$ProviderConfigCopyWith<$Res> implements $ProviderConfigCopyWith<$Res> {
  factory _$ProviderConfigCopyWith(_ProviderConfig value, $Res Function(_ProviderConfig) _then) = __$ProviderConfigCopyWithImpl;
@override @useResult
$Res call({
 String name,@JsonKey(name: 'base_url') String baseUrl, String format, List<String> models, List<ApiKeyConfig> keys
});




}
/// @nodoc
class __$ProviderConfigCopyWithImpl<$Res>
    implements _$ProviderConfigCopyWith<$Res> {
  __$ProviderConfigCopyWithImpl(this._self, this._then);

  final _ProviderConfig _self;
  final $Res Function(_ProviderConfig) _then;

/// Create a copy of ProviderConfig
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? name = null,Object? baseUrl = null,Object? format = null,Object? models = null,Object? keys = null,}) {
  return _then(_ProviderConfig(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,baseUrl: null == baseUrl ? _self.baseUrl : baseUrl // ignore: cast_nullable_to_non_nullable
as String,format: null == format ? _self.format : format // ignore: cast_nullable_to_non_nullable
as String,models: null == models ? _self._models : models // ignore: cast_nullable_to_non_nullable
as List<String>,keys: null == keys ? _self._keys : keys // ignore: cast_nullable_to_non_nullable
as List<ApiKeyConfig>,
  ));
}


}


/// @nodoc
mixin _$ApiKeyConfig {

 String get key; String? get label; bool get enabled;
/// Create a copy of ApiKeyConfig
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ApiKeyConfigCopyWith<ApiKeyConfig> get copyWith => _$ApiKeyConfigCopyWithImpl<ApiKeyConfig>(this as ApiKeyConfig, _$identity);

  /// Serializes this ApiKeyConfig to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ApiKeyConfig&&(identical(other.key, key) || other.key == key)&&(identical(other.label, label) || other.label == label)&&(identical(other.enabled, enabled) || other.enabled == enabled));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,key,label,enabled);

@override
String toString() {
  return 'ApiKeyConfig(key: $key, label: $label, enabled: $enabled)';
}


}

/// @nodoc
abstract mixin class $ApiKeyConfigCopyWith<$Res>  {
  factory $ApiKeyConfigCopyWith(ApiKeyConfig value, $Res Function(ApiKeyConfig) _then) = _$ApiKeyConfigCopyWithImpl;
@useResult
$Res call({
 String key, String? label, bool enabled
});




}
/// @nodoc
class _$ApiKeyConfigCopyWithImpl<$Res>
    implements $ApiKeyConfigCopyWith<$Res> {
  _$ApiKeyConfigCopyWithImpl(this._self, this._then);

  final ApiKeyConfig _self;
  final $Res Function(ApiKeyConfig) _then;

/// Create a copy of ApiKeyConfig
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? key = null,Object? label = freezed,Object? enabled = null,}) {
  return _then(_self.copyWith(
key: null == key ? _self.key : key // ignore: cast_nullable_to_non_nullable
as String,label: freezed == label ? _self.label : label // ignore: cast_nullable_to_non_nullable
as String?,enabled: null == enabled ? _self.enabled : enabled // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}

}


/// Adds pattern-matching-related methods to [ApiKeyConfig].
extension ApiKeyConfigPatterns on ApiKeyConfig {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ApiKeyConfig value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ApiKeyConfig() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ApiKeyConfig value)  $default,){
final _that = this;
switch (_that) {
case _ApiKeyConfig():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ApiKeyConfig value)?  $default,){
final _that = this;
switch (_that) {
case _ApiKeyConfig() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String key,  String? label,  bool enabled)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ApiKeyConfig() when $default != null:
return $default(_that.key,_that.label,_that.enabled);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String key,  String? label,  bool enabled)  $default,) {final _that = this;
switch (_that) {
case _ApiKeyConfig():
return $default(_that.key,_that.label,_that.enabled);case _:
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String key,  String? label,  bool enabled)?  $default,) {final _that = this;
switch (_that) {
case _ApiKeyConfig() when $default != null:
return $default(_that.key,_that.label,_that.enabled);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _ApiKeyConfig implements ApiKeyConfig {
  const _ApiKeyConfig({required this.key, this.label, this.enabled = true});
  factory _ApiKeyConfig.fromJson(Map<String, dynamic> json) => _$ApiKeyConfigFromJson(json);

@override final  String key;
@override final  String? label;
@override@JsonKey() final  bool enabled;

/// Create a copy of ApiKeyConfig
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ApiKeyConfigCopyWith<_ApiKeyConfig> get copyWith => __$ApiKeyConfigCopyWithImpl<_ApiKeyConfig>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$ApiKeyConfigToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ApiKeyConfig&&(identical(other.key, key) || other.key == key)&&(identical(other.label, label) || other.label == label)&&(identical(other.enabled, enabled) || other.enabled == enabled));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,key,label,enabled);

@override
String toString() {
  return 'ApiKeyConfig(key: $key, label: $label, enabled: $enabled)';
}


}

/// @nodoc
abstract mixin class _$ApiKeyConfigCopyWith<$Res> implements $ApiKeyConfigCopyWith<$Res> {
  factory _$ApiKeyConfigCopyWith(_ApiKeyConfig value, $Res Function(_ApiKeyConfig) _then) = __$ApiKeyConfigCopyWithImpl;
@override @useResult
$Res call({
 String key, String? label, bool enabled
});




}
/// @nodoc
class __$ApiKeyConfigCopyWithImpl<$Res>
    implements _$ApiKeyConfigCopyWith<$Res> {
  __$ApiKeyConfigCopyWithImpl(this._self, this._then);

  final _ApiKeyConfig _self;
  final $Res Function(_ApiKeyConfig) _then;

/// Create a copy of ApiKeyConfig
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? key = null,Object? label = freezed,Object? enabled = null,}) {
  return _then(_ApiKeyConfig(
key: null == key ? _self.key : key // ignore: cast_nullable_to_non_nullable
as String,label: freezed == label ? _self.label : label // ignore: cast_nullable_to_non_nullable
as String?,enabled: null == enabled ? _self.enabled : enabled // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

// dart format on
