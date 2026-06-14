// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'chat_session_meta.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$ChatSessionMeta {

 String get id; String get title; DateTime get createdAt; DateTime get updatedAt; int get messageCount; String? get provider; String? get model;
/// Create a copy of ChatSessionMeta
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ChatSessionMetaCopyWith<ChatSessionMeta> get copyWith => _$ChatSessionMetaCopyWithImpl<ChatSessionMeta>(this as ChatSessionMeta, _$identity);

  /// Serializes this ChatSessionMeta to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ChatSessionMeta&&(identical(other.id, id) || other.id == id)&&(identical(other.title, title) || other.title == title)&&(identical(other.createdAt, createdAt) || other.createdAt == createdAt)&&(identical(other.updatedAt, updatedAt) || other.updatedAt == updatedAt)&&(identical(other.messageCount, messageCount) || other.messageCount == messageCount)&&(identical(other.provider, provider) || other.provider == provider)&&(identical(other.model, model) || other.model == model));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,id,title,createdAt,updatedAt,messageCount,provider,model);

@override
String toString() {
  return 'ChatSessionMeta(id: $id, title: $title, createdAt: $createdAt, updatedAt: $updatedAt, messageCount: $messageCount, provider: $provider, model: $model)';
}


}

/// @nodoc
abstract mixin class $ChatSessionMetaCopyWith<$Res>  {
  factory $ChatSessionMetaCopyWith(ChatSessionMeta value, $Res Function(ChatSessionMeta) _then) = _$ChatSessionMetaCopyWithImpl;
@useResult
$Res call({
 String id, String title, DateTime createdAt, DateTime updatedAt, int messageCount, String? provider, String? model
});




}
/// @nodoc
class _$ChatSessionMetaCopyWithImpl<$Res>
    implements $ChatSessionMetaCopyWith<$Res> {
  _$ChatSessionMetaCopyWithImpl(this._self, this._then);

  final ChatSessionMeta _self;
  final $Res Function(ChatSessionMeta) _then;

/// Create a copy of ChatSessionMeta
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? id = null,Object? title = null,Object? createdAt = null,Object? updatedAt = null,Object? messageCount = null,Object? provider = freezed,Object? model = freezed,}) {
  return _then(_self.copyWith(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,title: null == title ? _self.title : title // ignore: cast_nullable_to_non_nullable
as String,createdAt: null == createdAt ? _self.createdAt : createdAt // ignore: cast_nullable_to_non_nullable
as DateTime,updatedAt: null == updatedAt ? _self.updatedAt : updatedAt // ignore: cast_nullable_to_non_nullable
as DateTime,messageCount: null == messageCount ? _self.messageCount : messageCount // ignore: cast_nullable_to_non_nullable
as int,provider: freezed == provider ? _self.provider : provider // ignore: cast_nullable_to_non_nullable
as String?,model: freezed == model ? _self.model : model // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}

}


/// Adds pattern-matching-related methods to [ChatSessionMeta].
extension ChatSessionMetaPatterns on ChatSessionMeta {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ChatSessionMeta value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ChatSessionMeta() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ChatSessionMeta value)  $default,){
final _that = this;
switch (_that) {
case _ChatSessionMeta():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ChatSessionMeta value)?  $default,){
final _that = this;
switch (_that) {
case _ChatSessionMeta() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String id,  String title,  DateTime createdAt,  DateTime updatedAt,  int messageCount,  String? provider,  String? model)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ChatSessionMeta() when $default != null:
return $default(_that.id,_that.title,_that.createdAt,_that.updatedAt,_that.messageCount,_that.provider,_that.model);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String id,  String title,  DateTime createdAt,  DateTime updatedAt,  int messageCount,  String? provider,  String? model)  $default,) {final _that = this;
switch (_that) {
case _ChatSessionMeta():
return $default(_that.id,_that.title,_that.createdAt,_that.updatedAt,_that.messageCount,_that.provider,_that.model);case _:
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String id,  String title,  DateTime createdAt,  DateTime updatedAt,  int messageCount,  String? provider,  String? model)?  $default,) {final _that = this;
switch (_that) {
case _ChatSessionMeta() when $default != null:
return $default(_that.id,_that.title,_that.createdAt,_that.updatedAt,_that.messageCount,_that.provider,_that.model);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _ChatSessionMeta implements ChatSessionMeta {
  const _ChatSessionMeta({required this.id, required this.title, required this.createdAt, required this.updatedAt, this.messageCount = 0, this.provider, this.model});
  factory _ChatSessionMeta.fromJson(Map<String, dynamic> json) => _$ChatSessionMetaFromJson(json);

@override final  String id;
@override final  String title;
@override final  DateTime createdAt;
@override final  DateTime updatedAt;
@override@JsonKey() final  int messageCount;
@override final  String? provider;
@override final  String? model;

/// Create a copy of ChatSessionMeta
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ChatSessionMetaCopyWith<_ChatSessionMeta> get copyWith => __$ChatSessionMetaCopyWithImpl<_ChatSessionMeta>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$ChatSessionMetaToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ChatSessionMeta&&(identical(other.id, id) || other.id == id)&&(identical(other.title, title) || other.title == title)&&(identical(other.createdAt, createdAt) || other.createdAt == createdAt)&&(identical(other.updatedAt, updatedAt) || other.updatedAt == updatedAt)&&(identical(other.messageCount, messageCount) || other.messageCount == messageCount)&&(identical(other.provider, provider) || other.provider == provider)&&(identical(other.model, model) || other.model == model));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,id,title,createdAt,updatedAt,messageCount,provider,model);

@override
String toString() {
  return 'ChatSessionMeta(id: $id, title: $title, createdAt: $createdAt, updatedAt: $updatedAt, messageCount: $messageCount, provider: $provider, model: $model)';
}


}

/// @nodoc
abstract mixin class _$ChatSessionMetaCopyWith<$Res> implements $ChatSessionMetaCopyWith<$Res> {
  factory _$ChatSessionMetaCopyWith(_ChatSessionMeta value, $Res Function(_ChatSessionMeta) _then) = __$ChatSessionMetaCopyWithImpl;
@override @useResult
$Res call({
 String id, String title, DateTime createdAt, DateTime updatedAt, int messageCount, String? provider, String? model
});




}
/// @nodoc
class __$ChatSessionMetaCopyWithImpl<$Res>
    implements _$ChatSessionMetaCopyWith<$Res> {
  __$ChatSessionMetaCopyWithImpl(this._self, this._then);

  final _ChatSessionMeta _self;
  final $Res Function(_ChatSessionMeta) _then;

/// Create a copy of ChatSessionMeta
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? id = null,Object? title = null,Object? createdAt = null,Object? updatedAt = null,Object? messageCount = null,Object? provider = freezed,Object? model = freezed,}) {
  return _then(_ChatSessionMeta(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,title: null == title ? _self.title : title // ignore: cast_nullable_to_non_nullable
as String,createdAt: null == createdAt ? _self.createdAt : createdAt // ignore: cast_nullable_to_non_nullable
as DateTime,updatedAt: null == updatedAt ? _self.updatedAt : updatedAt // ignore: cast_nullable_to_non_nullable
as DateTime,messageCount: null == messageCount ? _self.messageCount : messageCount // ignore: cast_nullable_to_non_nullable
as int,provider: freezed == provider ? _self.provider : provider // ignore: cast_nullable_to_non_nullable
as String?,model: freezed == model ? _self.model : model // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

// dart format on
