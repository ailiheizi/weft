// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'app.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$ResolvedApp {

 String get name; String get version;@JsonKey(name: 'display_name') String get displayName; String get description; List<String> get capabilities;@JsonKey(name: 'enabled_features') List<String> get enabledFeatures; List<AppBindingResolution> get bindings;@JsonKey(name: 'validation_checks') List<String> get validationChecks; List<String> get errors; ResolvedAppStatus get status;
/// Create a copy of ResolvedApp
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ResolvedAppCopyWith<ResolvedApp> get copyWith => _$ResolvedAppCopyWithImpl<ResolvedApp>(this as ResolvedApp, _$identity);

  /// Serializes this ResolvedApp to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ResolvedApp&&(identical(other.name, name) || other.name == name)&&(identical(other.version, version) || other.version == version)&&(identical(other.displayName, displayName) || other.displayName == displayName)&&(identical(other.description, description) || other.description == description)&&const DeepCollectionEquality().equals(other.capabilities, capabilities)&&const DeepCollectionEquality().equals(other.enabledFeatures, enabledFeatures)&&const DeepCollectionEquality().equals(other.bindings, bindings)&&const DeepCollectionEquality().equals(other.validationChecks, validationChecks)&&const DeepCollectionEquality().equals(other.errors, errors)&&(identical(other.status, status) || other.status == status));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,name,version,displayName,description,const DeepCollectionEquality().hash(capabilities),const DeepCollectionEquality().hash(enabledFeatures),const DeepCollectionEquality().hash(bindings),const DeepCollectionEquality().hash(validationChecks),const DeepCollectionEquality().hash(errors),status);

@override
String toString() {
  return 'ResolvedApp(name: $name, version: $version, displayName: $displayName, description: $description, capabilities: $capabilities, enabledFeatures: $enabledFeatures, bindings: $bindings, validationChecks: $validationChecks, errors: $errors, status: $status)';
}


}

/// @nodoc
abstract mixin class $ResolvedAppCopyWith<$Res>  {
  factory $ResolvedAppCopyWith(ResolvedApp value, $Res Function(ResolvedApp) _then) = _$ResolvedAppCopyWithImpl;
@useResult
$Res call({
 String name, String version,@JsonKey(name: 'display_name') String displayName, String description, List<String> capabilities,@JsonKey(name: 'enabled_features') List<String> enabledFeatures, List<AppBindingResolution> bindings,@JsonKey(name: 'validation_checks') List<String> validationChecks, List<String> errors, ResolvedAppStatus status
});




}
/// @nodoc
class _$ResolvedAppCopyWithImpl<$Res>
    implements $ResolvedAppCopyWith<$Res> {
  _$ResolvedAppCopyWithImpl(this._self, this._then);

  final ResolvedApp _self;
  final $Res Function(ResolvedApp) _then;

/// Create a copy of ResolvedApp
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? name = null,Object? version = null,Object? displayName = null,Object? description = null,Object? capabilities = null,Object? enabledFeatures = null,Object? bindings = null,Object? validationChecks = null,Object? errors = null,Object? status = null,}) {
  return _then(_self.copyWith(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,version: null == version ? _self.version : version // ignore: cast_nullable_to_non_nullable
as String,displayName: null == displayName ? _self.displayName : displayName // ignore: cast_nullable_to_non_nullable
as String,description: null == description ? _self.description : description // ignore: cast_nullable_to_non_nullable
as String,capabilities: null == capabilities ? _self.capabilities : capabilities // ignore: cast_nullable_to_non_nullable
as List<String>,enabledFeatures: null == enabledFeatures ? _self.enabledFeatures : enabledFeatures // ignore: cast_nullable_to_non_nullable
as List<String>,bindings: null == bindings ? _self.bindings : bindings // ignore: cast_nullable_to_non_nullable
as List<AppBindingResolution>,validationChecks: null == validationChecks ? _self.validationChecks : validationChecks // ignore: cast_nullable_to_non_nullable
as List<String>,errors: null == errors ? _self.errors : errors // ignore: cast_nullable_to_non_nullable
as List<String>,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as ResolvedAppStatus,
  ));
}

}


/// Adds pattern-matching-related methods to [ResolvedApp].
extension ResolvedAppPatterns on ResolvedApp {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ResolvedApp value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ResolvedApp() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ResolvedApp value)  $default,){
final _that = this;
switch (_that) {
case _ResolvedApp():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ResolvedApp value)?  $default,){
final _that = this;
switch (_that) {
case _ResolvedApp() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String name,  String version, @JsonKey(name: 'display_name')  String displayName,  String description,  List<String> capabilities, @JsonKey(name: 'enabled_features')  List<String> enabledFeatures,  List<AppBindingResolution> bindings, @JsonKey(name: 'validation_checks')  List<String> validationChecks,  List<String> errors,  ResolvedAppStatus status)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ResolvedApp() when $default != null:
return $default(_that.name,_that.version,_that.displayName,_that.description,_that.capabilities,_that.enabledFeatures,_that.bindings,_that.validationChecks,_that.errors,_that.status);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String name,  String version, @JsonKey(name: 'display_name')  String displayName,  String description,  List<String> capabilities, @JsonKey(name: 'enabled_features')  List<String> enabledFeatures,  List<AppBindingResolution> bindings, @JsonKey(name: 'validation_checks')  List<String> validationChecks,  List<String> errors,  ResolvedAppStatus status)  $default,) {final _that = this;
switch (_that) {
case _ResolvedApp():
return $default(_that.name,_that.version,_that.displayName,_that.description,_that.capabilities,_that.enabledFeatures,_that.bindings,_that.validationChecks,_that.errors,_that.status);case _:
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String name,  String version, @JsonKey(name: 'display_name')  String displayName,  String description,  List<String> capabilities, @JsonKey(name: 'enabled_features')  List<String> enabledFeatures,  List<AppBindingResolution> bindings, @JsonKey(name: 'validation_checks')  List<String> validationChecks,  List<String> errors,  ResolvedAppStatus status)?  $default,) {final _that = this;
switch (_that) {
case _ResolvedApp() when $default != null:
return $default(_that.name,_that.version,_that.displayName,_that.description,_that.capabilities,_that.enabledFeatures,_that.bindings,_that.validationChecks,_that.errors,_that.status);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _ResolvedApp implements ResolvedApp {
  const _ResolvedApp({required this.name, required this.version, @JsonKey(name: 'display_name') required this.displayName, required this.description, final  List<String> capabilities = const [], @JsonKey(name: 'enabled_features') final  List<String> enabledFeatures = const [], final  List<AppBindingResolution> bindings = const [], @JsonKey(name: 'validation_checks') final  List<String> validationChecks = const [], final  List<String> errors = const [], required this.status}): _capabilities = capabilities,_enabledFeatures = enabledFeatures,_bindings = bindings,_validationChecks = validationChecks,_errors = errors;
  factory _ResolvedApp.fromJson(Map<String, dynamic> json) => _$ResolvedAppFromJson(json);

@override final  String name;
@override final  String version;
@override@JsonKey(name: 'display_name') final  String displayName;
@override final  String description;
 final  List<String> _capabilities;
@override@JsonKey() List<String> get capabilities {
  if (_capabilities is EqualUnmodifiableListView) return _capabilities;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_capabilities);
}

 final  List<String> _enabledFeatures;
@override@JsonKey(name: 'enabled_features') List<String> get enabledFeatures {
  if (_enabledFeatures is EqualUnmodifiableListView) return _enabledFeatures;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_enabledFeatures);
}

 final  List<AppBindingResolution> _bindings;
@override@JsonKey() List<AppBindingResolution> get bindings {
  if (_bindings is EqualUnmodifiableListView) return _bindings;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_bindings);
}

 final  List<String> _validationChecks;
@override@JsonKey(name: 'validation_checks') List<String> get validationChecks {
  if (_validationChecks is EqualUnmodifiableListView) return _validationChecks;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_validationChecks);
}

 final  List<String> _errors;
@override@JsonKey() List<String> get errors {
  if (_errors is EqualUnmodifiableListView) return _errors;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_errors);
}

@override final  ResolvedAppStatus status;

/// Create a copy of ResolvedApp
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ResolvedAppCopyWith<_ResolvedApp> get copyWith => __$ResolvedAppCopyWithImpl<_ResolvedApp>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$ResolvedAppToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ResolvedApp&&(identical(other.name, name) || other.name == name)&&(identical(other.version, version) || other.version == version)&&(identical(other.displayName, displayName) || other.displayName == displayName)&&(identical(other.description, description) || other.description == description)&&const DeepCollectionEquality().equals(other._capabilities, _capabilities)&&const DeepCollectionEquality().equals(other._enabledFeatures, _enabledFeatures)&&const DeepCollectionEquality().equals(other._bindings, _bindings)&&const DeepCollectionEquality().equals(other._validationChecks, _validationChecks)&&const DeepCollectionEquality().equals(other._errors, _errors)&&(identical(other.status, status) || other.status == status));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,name,version,displayName,description,const DeepCollectionEquality().hash(_capabilities),const DeepCollectionEquality().hash(_enabledFeatures),const DeepCollectionEquality().hash(_bindings),const DeepCollectionEquality().hash(_validationChecks),const DeepCollectionEquality().hash(_errors),status);

@override
String toString() {
  return 'ResolvedApp(name: $name, version: $version, displayName: $displayName, description: $description, capabilities: $capabilities, enabledFeatures: $enabledFeatures, bindings: $bindings, validationChecks: $validationChecks, errors: $errors, status: $status)';
}


}

/// @nodoc
abstract mixin class _$ResolvedAppCopyWith<$Res> implements $ResolvedAppCopyWith<$Res> {
  factory _$ResolvedAppCopyWith(_ResolvedApp value, $Res Function(_ResolvedApp) _then) = __$ResolvedAppCopyWithImpl;
@override @useResult
$Res call({
 String name, String version,@JsonKey(name: 'display_name') String displayName, String description, List<String> capabilities,@JsonKey(name: 'enabled_features') List<String> enabledFeatures, List<AppBindingResolution> bindings,@JsonKey(name: 'validation_checks') List<String> validationChecks, List<String> errors, ResolvedAppStatus status
});




}
/// @nodoc
class __$ResolvedAppCopyWithImpl<$Res>
    implements _$ResolvedAppCopyWith<$Res> {
  __$ResolvedAppCopyWithImpl(this._self, this._then);

  final _ResolvedApp _self;
  final $Res Function(_ResolvedApp) _then;

/// Create a copy of ResolvedApp
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? name = null,Object? version = null,Object? displayName = null,Object? description = null,Object? capabilities = null,Object? enabledFeatures = null,Object? bindings = null,Object? validationChecks = null,Object? errors = null,Object? status = null,}) {
  return _then(_ResolvedApp(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,version: null == version ? _self.version : version // ignore: cast_nullable_to_non_nullable
as String,displayName: null == displayName ? _self.displayName : displayName // ignore: cast_nullable_to_non_nullable
as String,description: null == description ? _self.description : description // ignore: cast_nullable_to_non_nullable
as String,capabilities: null == capabilities ? _self._capabilities : capabilities // ignore: cast_nullable_to_non_nullable
as List<String>,enabledFeatures: null == enabledFeatures ? _self._enabledFeatures : enabledFeatures // ignore: cast_nullable_to_non_nullable
as List<String>,bindings: null == bindings ? _self._bindings : bindings // ignore: cast_nullable_to_non_nullable
as List<AppBindingResolution>,validationChecks: null == validationChecks ? _self._validationChecks : validationChecks // ignore: cast_nullable_to_non_nullable
as List<String>,errors: null == errors ? _self._errors : errors // ignore: cast_nullable_to_non_nullable
as List<String>,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as ResolvedAppStatus,
  ));
}


}


/// @nodoc
mixin _$AppBindingResolution {

 String get capability; String get provider; bool get mutable; String get source;
/// Create a copy of AppBindingResolution
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AppBindingResolutionCopyWith<AppBindingResolution> get copyWith => _$AppBindingResolutionCopyWithImpl<AppBindingResolution>(this as AppBindingResolution, _$identity);

  /// Serializes this AppBindingResolution to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AppBindingResolution&&(identical(other.capability, capability) || other.capability == capability)&&(identical(other.provider, provider) || other.provider == provider)&&(identical(other.mutable, mutable) || other.mutable == mutable)&&(identical(other.source, source) || other.source == source));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,capability,provider,mutable,source);

@override
String toString() {
  return 'AppBindingResolution(capability: $capability, provider: $provider, mutable: $mutable, source: $source)';
}


}

/// @nodoc
abstract mixin class $AppBindingResolutionCopyWith<$Res>  {
  factory $AppBindingResolutionCopyWith(AppBindingResolution value, $Res Function(AppBindingResolution) _then) = _$AppBindingResolutionCopyWithImpl;
@useResult
$Res call({
 String capability, String provider, bool mutable, String source
});




}
/// @nodoc
class _$AppBindingResolutionCopyWithImpl<$Res>
    implements $AppBindingResolutionCopyWith<$Res> {
  _$AppBindingResolutionCopyWithImpl(this._self, this._then);

  final AppBindingResolution _self;
  final $Res Function(AppBindingResolution) _then;

/// Create a copy of AppBindingResolution
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? capability = null,Object? provider = null,Object? mutable = null,Object? source = null,}) {
  return _then(_self.copyWith(
capability: null == capability ? _self.capability : capability // ignore: cast_nullable_to_non_nullable
as String,provider: null == provider ? _self.provider : provider // ignore: cast_nullable_to_non_nullable
as String,mutable: null == mutable ? _self.mutable : mutable // ignore: cast_nullable_to_non_nullable
as bool,source: null == source ? _self.source : source // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [AppBindingResolution].
extension AppBindingResolutionPatterns on AppBindingResolution {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _AppBindingResolution value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _AppBindingResolution() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _AppBindingResolution value)  $default,){
final _that = this;
switch (_that) {
case _AppBindingResolution():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _AppBindingResolution value)?  $default,){
final _that = this;
switch (_that) {
case _AppBindingResolution() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String capability,  String provider,  bool mutable,  String source)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _AppBindingResolution() when $default != null:
return $default(_that.capability,_that.provider,_that.mutable,_that.source);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String capability,  String provider,  bool mutable,  String source)  $default,) {final _that = this;
switch (_that) {
case _AppBindingResolution():
return $default(_that.capability,_that.provider,_that.mutable,_that.source);case _:
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String capability,  String provider,  bool mutable,  String source)?  $default,) {final _that = this;
switch (_that) {
case _AppBindingResolution() when $default != null:
return $default(_that.capability,_that.provider,_that.mutable,_that.source);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _AppBindingResolution implements AppBindingResolution {
  const _AppBindingResolution({required this.capability, required this.provider, this.mutable = false, this.source = ''});
  factory _AppBindingResolution.fromJson(Map<String, dynamic> json) => _$AppBindingResolutionFromJson(json);

@override final  String capability;
@override final  String provider;
@override@JsonKey() final  bool mutable;
@override@JsonKey() final  String source;

/// Create a copy of AppBindingResolution
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$AppBindingResolutionCopyWith<_AppBindingResolution> get copyWith => __$AppBindingResolutionCopyWithImpl<_AppBindingResolution>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$AppBindingResolutionToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _AppBindingResolution&&(identical(other.capability, capability) || other.capability == capability)&&(identical(other.provider, provider) || other.provider == provider)&&(identical(other.mutable, mutable) || other.mutable == mutable)&&(identical(other.source, source) || other.source == source));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,capability,provider,mutable,source);

@override
String toString() {
  return 'AppBindingResolution(capability: $capability, provider: $provider, mutable: $mutable, source: $source)';
}


}

/// @nodoc
abstract mixin class _$AppBindingResolutionCopyWith<$Res> implements $AppBindingResolutionCopyWith<$Res> {
  factory _$AppBindingResolutionCopyWith(_AppBindingResolution value, $Res Function(_AppBindingResolution) _then) = __$AppBindingResolutionCopyWithImpl;
@override @useResult
$Res call({
 String capability, String provider, bool mutable, String source
});




}
/// @nodoc
class __$AppBindingResolutionCopyWithImpl<$Res>
    implements _$AppBindingResolutionCopyWith<$Res> {
  __$AppBindingResolutionCopyWithImpl(this._self, this._then);

  final _AppBindingResolution _self;
  final $Res Function(_AppBindingResolution) _then;

/// Create a copy of AppBindingResolution
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? capability = null,Object? provider = null,Object? mutable = null,Object? source = null,}) {
  return _then(_AppBindingResolution(
capability: null == capability ? _self.capability : capability // ignore: cast_nullable_to_non_nullable
as String,provider: null == provider ? _self.provider : provider // ignore: cast_nullable_to_non_nullable
as String,mutable: null == mutable ? _self.mutable : mutable // ignore: cast_nullable_to_non_nullable
as bool,source: null == source ? _self.source : source // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
