// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'chat.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$ExecutionStep {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ExecutionStep);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ExecutionStep()';
}


}

/// @nodoc
class $ExecutionStepCopyWith<$Res>  {
$ExecutionStepCopyWith(ExecutionStep _, $Res Function(ExecutionStep) __);
}


/// Adds pattern-matching-related methods to [ExecutionStep].
extension ExecutionStepPatterns on ExecutionStep {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ThinkingStep value)?  thinking,TResult Function( ToolCallStep value)?  toolCall,TResult Function( AskUserStep value)?  askUser,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ThinkingStep() when thinking != null:
return thinking(_that);case ToolCallStep() when toolCall != null:
return toolCall(_that);case AskUserStep() when askUser != null:
return askUser(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ThinkingStep value)  thinking,required TResult Function( ToolCallStep value)  toolCall,required TResult Function( AskUserStep value)  askUser,}){
final _that = this;
switch (_that) {
case ThinkingStep():
return thinking(_that);case ToolCallStep():
return toolCall(_that);case AskUserStep():
return askUser(_that);case _:
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ThinkingStep value)?  thinking,TResult? Function( ToolCallStep value)?  toolCall,TResult? Function( AskUserStep value)?  askUser,}){
final _that = this;
switch (_that) {
case ThinkingStep() when thinking != null:
return thinking(_that);case ToolCallStep() when toolCall != null:
return toolCall(_that);case AskUserStep() when askUser != null:
return askUser(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String content)?  thinking,TResult Function( String id,  String name,  String arguments,  String? result)?  toolCall,TResult Function( String question,  List<String> options)?  askUser,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ThinkingStep() when thinking != null:
return thinking(_that.content);case ToolCallStep() when toolCall != null:
return toolCall(_that.id,_that.name,_that.arguments,_that.result);case AskUserStep() when askUser != null:
return askUser(_that.question,_that.options);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String content)  thinking,required TResult Function( String id,  String name,  String arguments,  String? result)  toolCall,required TResult Function( String question,  List<String> options)  askUser,}) {final _that = this;
switch (_that) {
case ThinkingStep():
return thinking(_that.content);case ToolCallStep():
return toolCall(_that.id,_that.name,_that.arguments,_that.result);case AskUserStep():
return askUser(_that.question,_that.options);case _:
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String content)?  thinking,TResult? Function( String id,  String name,  String arguments,  String? result)?  toolCall,TResult? Function( String question,  List<String> options)?  askUser,}) {final _that = this;
switch (_that) {
case ThinkingStep() when thinking != null:
return thinking(_that.content);case ToolCallStep() when toolCall != null:
return toolCall(_that.id,_that.name,_that.arguments,_that.result);case AskUserStep() when askUser != null:
return askUser(_that.question,_that.options);case _:
  return null;

}
}

}

/// @nodoc


class ThinkingStep implements ExecutionStep {
  const ThinkingStep({required this.content});
  

 final  String content;

/// Create a copy of ExecutionStep
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ThinkingStepCopyWith<ThinkingStep> get copyWith => _$ThinkingStepCopyWithImpl<ThinkingStep>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ThinkingStep&&(identical(other.content, content) || other.content == content));
}


@override
int get hashCode => Object.hash(runtimeType,content);

@override
String toString() {
  return 'ExecutionStep.thinking(content: $content)';
}


}

/// @nodoc
abstract mixin class $ThinkingStepCopyWith<$Res> implements $ExecutionStepCopyWith<$Res> {
  factory $ThinkingStepCopyWith(ThinkingStep value, $Res Function(ThinkingStep) _then) = _$ThinkingStepCopyWithImpl;
@useResult
$Res call({
 String content
});




}
/// @nodoc
class _$ThinkingStepCopyWithImpl<$Res>
    implements $ThinkingStepCopyWith<$Res> {
  _$ThinkingStepCopyWithImpl(this._self, this._then);

  final ThinkingStep _self;
  final $Res Function(ThinkingStep) _then;

/// Create a copy of ExecutionStep
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? content = null,}) {
  return _then(ThinkingStep(
content: null == content ? _self.content : content // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class ToolCallStep implements ExecutionStep {
  const ToolCallStep({required this.id, required this.name, required this.arguments, this.result});
  

 final  String id;
 final  String name;
 final  String arguments;
 final  String? result;

/// Create a copy of ExecutionStep
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ToolCallStepCopyWith<ToolCallStep> get copyWith => _$ToolCallStepCopyWithImpl<ToolCallStep>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ToolCallStep&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name)&&(identical(other.arguments, arguments) || other.arguments == arguments)&&(identical(other.result, result) || other.result == result));
}


@override
int get hashCode => Object.hash(runtimeType,id,name,arguments,result);

@override
String toString() {
  return 'ExecutionStep.toolCall(id: $id, name: $name, arguments: $arguments, result: $result)';
}


}

/// @nodoc
abstract mixin class $ToolCallStepCopyWith<$Res> implements $ExecutionStepCopyWith<$Res> {
  factory $ToolCallStepCopyWith(ToolCallStep value, $Res Function(ToolCallStep) _then) = _$ToolCallStepCopyWithImpl;
@useResult
$Res call({
 String id, String name, String arguments, String? result
});




}
/// @nodoc
class _$ToolCallStepCopyWithImpl<$Res>
    implements $ToolCallStepCopyWith<$Res> {
  _$ToolCallStepCopyWithImpl(this._self, this._then);

  final ToolCallStep _self;
  final $Res Function(ToolCallStep) _then;

/// Create a copy of ExecutionStep
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? id = null,Object? name = null,Object? arguments = null,Object? result = freezed,}) {
  return _then(ToolCallStep(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,arguments: null == arguments ? _self.arguments : arguments // ignore: cast_nullable_to_non_nullable
as String,result: freezed == result ? _self.result : result // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc


class AskUserStep implements ExecutionStep {
  const AskUserStep({required this.question, final  List<String> options = const []}): _options = options;
  

 final  String question;
 final  List<String> _options;
@JsonKey() List<String> get options {
  if (_options is EqualUnmodifiableListView) return _options;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_options);
}


/// Create a copy of ExecutionStep
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AskUserStepCopyWith<AskUserStep> get copyWith => _$AskUserStepCopyWithImpl<AskUserStep>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AskUserStep&&(identical(other.question, question) || other.question == question)&&const DeepCollectionEquality().equals(other._options, _options));
}


@override
int get hashCode => Object.hash(runtimeType,question,const DeepCollectionEquality().hash(_options));

@override
String toString() {
  return 'ExecutionStep.askUser(question: $question, options: $options)';
}


}

/// @nodoc
abstract mixin class $AskUserStepCopyWith<$Res> implements $ExecutionStepCopyWith<$Res> {
  factory $AskUserStepCopyWith(AskUserStep value, $Res Function(AskUserStep) _then) = _$AskUserStepCopyWithImpl;
@useResult
$Res call({
 String question, List<String> options
});




}
/// @nodoc
class _$AskUserStepCopyWithImpl<$Res>
    implements $AskUserStepCopyWith<$Res> {
  _$AskUserStepCopyWithImpl(this._self, this._then);

  final AskUserStep _self;
  final $Res Function(AskUserStep) _then;

/// Create a copy of ExecutionStep
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? question = null,Object? options = null,}) {
  return _then(AskUserStep(
question: null == question ? _self.question : question // ignore: cast_nullable_to_non_nullable
as String,options: null == options ? _self._options : options // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc
mixin _$ChatMessage {

 String get id; String get role; String get content; List<ExecutionStep> get steps;
/// Create a copy of ChatMessage
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ChatMessageCopyWith<ChatMessage> get copyWith => _$ChatMessageCopyWithImpl<ChatMessage>(this as ChatMessage, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ChatMessage&&(identical(other.id, id) || other.id == id)&&(identical(other.role, role) || other.role == role)&&(identical(other.content, content) || other.content == content)&&const DeepCollectionEquality().equals(other.steps, steps));
}


@override
int get hashCode => Object.hash(runtimeType,id,role,content,const DeepCollectionEquality().hash(steps));

@override
String toString() {
  return 'ChatMessage(id: $id, role: $role, content: $content, steps: $steps)';
}


}

/// @nodoc
abstract mixin class $ChatMessageCopyWith<$Res>  {
  factory $ChatMessageCopyWith(ChatMessage value, $Res Function(ChatMessage) _then) = _$ChatMessageCopyWithImpl;
@useResult
$Res call({
 String id, String role, String content, List<ExecutionStep> steps
});




}
/// @nodoc
class _$ChatMessageCopyWithImpl<$Res>
    implements $ChatMessageCopyWith<$Res> {
  _$ChatMessageCopyWithImpl(this._self, this._then);

  final ChatMessage _self;
  final $Res Function(ChatMessage) _then;

/// Create a copy of ChatMessage
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? id = null,Object? role = null,Object? content = null,Object? steps = null,}) {
  return _then(_self.copyWith(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,role: null == role ? _self.role : role // ignore: cast_nullable_to_non_nullable
as String,content: null == content ? _self.content : content // ignore: cast_nullable_to_non_nullable
as String,steps: null == steps ? _self.steps : steps // ignore: cast_nullable_to_non_nullable
as List<ExecutionStep>,
  ));
}

}


/// Adds pattern-matching-related methods to [ChatMessage].
extension ChatMessagePatterns on ChatMessage {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ChatMessage value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ChatMessage() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ChatMessage value)  $default,){
final _that = this;
switch (_that) {
case _ChatMessage():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ChatMessage value)?  $default,){
final _that = this;
switch (_that) {
case _ChatMessage() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String id,  String role,  String content,  List<ExecutionStep> steps)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ChatMessage() when $default != null:
return $default(_that.id,_that.role,_that.content,_that.steps);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String id,  String role,  String content,  List<ExecutionStep> steps)  $default,) {final _that = this;
switch (_that) {
case _ChatMessage():
return $default(_that.id,_that.role,_that.content,_that.steps);case _:
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String id,  String role,  String content,  List<ExecutionStep> steps)?  $default,) {final _that = this;
switch (_that) {
case _ChatMessage() when $default != null:
return $default(_that.id,_that.role,_that.content,_that.steps);case _:
  return null;

}
}

}

/// @nodoc


class _ChatMessage implements ChatMessage {
  const _ChatMessage({required this.id, required this.role, required this.content, final  List<ExecutionStep> steps = const []}): _steps = steps;
  

@override final  String id;
@override final  String role;
@override final  String content;
 final  List<ExecutionStep> _steps;
@override@JsonKey() List<ExecutionStep> get steps {
  if (_steps is EqualUnmodifiableListView) return _steps;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_steps);
}


/// Create a copy of ChatMessage
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ChatMessageCopyWith<_ChatMessage> get copyWith => __$ChatMessageCopyWithImpl<_ChatMessage>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ChatMessage&&(identical(other.id, id) || other.id == id)&&(identical(other.role, role) || other.role == role)&&(identical(other.content, content) || other.content == content)&&const DeepCollectionEquality().equals(other._steps, _steps));
}


@override
int get hashCode => Object.hash(runtimeType,id,role,content,const DeepCollectionEquality().hash(_steps));

@override
String toString() {
  return 'ChatMessage(id: $id, role: $role, content: $content, steps: $steps)';
}


}

/// @nodoc
abstract mixin class _$ChatMessageCopyWith<$Res> implements $ChatMessageCopyWith<$Res> {
  factory _$ChatMessageCopyWith(_ChatMessage value, $Res Function(_ChatMessage) _then) = __$ChatMessageCopyWithImpl;
@override @useResult
$Res call({
 String id, String role, String content, List<ExecutionStep> steps
});




}
/// @nodoc
class __$ChatMessageCopyWithImpl<$Res>
    implements _$ChatMessageCopyWith<$Res> {
  __$ChatMessageCopyWithImpl(this._self, this._then);

  final _ChatMessage _self;
  final $Res Function(_ChatMessage) _then;

/// Create a copy of ChatMessage
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? id = null,Object? role = null,Object? content = null,Object? steps = null,}) {
  return _then(_ChatMessage(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,role: null == role ? _self.role : role // ignore: cast_nullable_to_non_nullable
as String,content: null == content ? _self.content : content // ignore: cast_nullable_to_non_nullable
as String,steps: null == steps ? _self._steps : steps // ignore: cast_nullable_to_non_nullable
as List<ExecutionStep>,
  ));
}


}

/// @nodoc
mixin _$ChatSession {

 String get id; List<ChatMessage> get messages; bool get isStreaming; String? get selectedProvider; String? get selectedModel;
/// Create a copy of ChatSession
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ChatSessionCopyWith<ChatSession> get copyWith => _$ChatSessionCopyWithImpl<ChatSession>(this as ChatSession, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ChatSession&&(identical(other.id, id) || other.id == id)&&const DeepCollectionEquality().equals(other.messages, messages)&&(identical(other.isStreaming, isStreaming) || other.isStreaming == isStreaming)&&(identical(other.selectedProvider, selectedProvider) || other.selectedProvider == selectedProvider)&&(identical(other.selectedModel, selectedModel) || other.selectedModel == selectedModel));
}


@override
int get hashCode => Object.hash(runtimeType,id,const DeepCollectionEquality().hash(messages),isStreaming,selectedProvider,selectedModel);

@override
String toString() {
  return 'ChatSession(id: $id, messages: $messages, isStreaming: $isStreaming, selectedProvider: $selectedProvider, selectedModel: $selectedModel)';
}


}

/// @nodoc
abstract mixin class $ChatSessionCopyWith<$Res>  {
  factory $ChatSessionCopyWith(ChatSession value, $Res Function(ChatSession) _then) = _$ChatSessionCopyWithImpl;
@useResult
$Res call({
 String id, List<ChatMessage> messages, bool isStreaming, String? selectedProvider, String? selectedModel
});




}
/// @nodoc
class _$ChatSessionCopyWithImpl<$Res>
    implements $ChatSessionCopyWith<$Res> {
  _$ChatSessionCopyWithImpl(this._self, this._then);

  final ChatSession _self;
  final $Res Function(ChatSession) _then;

/// Create a copy of ChatSession
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? id = null,Object? messages = null,Object? isStreaming = null,Object? selectedProvider = freezed,Object? selectedModel = freezed,}) {
  return _then(_self.copyWith(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,messages: null == messages ? _self.messages : messages // ignore: cast_nullable_to_non_nullable
as List<ChatMessage>,isStreaming: null == isStreaming ? _self.isStreaming : isStreaming // ignore: cast_nullable_to_non_nullable
as bool,selectedProvider: freezed == selectedProvider ? _self.selectedProvider : selectedProvider // ignore: cast_nullable_to_non_nullable
as String?,selectedModel: freezed == selectedModel ? _self.selectedModel : selectedModel // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}

}


/// Adds pattern-matching-related methods to [ChatSession].
extension ChatSessionPatterns on ChatSession {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ChatSession value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ChatSession() when $default != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ChatSession value)  $default,){
final _that = this;
switch (_that) {
case _ChatSession():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ChatSession value)?  $default,){
final _that = this;
switch (_that) {
case _ChatSession() when $default != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String id,  List<ChatMessage> messages,  bool isStreaming,  String? selectedProvider,  String? selectedModel)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ChatSession() when $default != null:
return $default(_that.id,_that.messages,_that.isStreaming,_that.selectedProvider,_that.selectedModel);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String id,  List<ChatMessage> messages,  bool isStreaming,  String? selectedProvider,  String? selectedModel)  $default,) {final _that = this;
switch (_that) {
case _ChatSession():
return $default(_that.id,_that.messages,_that.isStreaming,_that.selectedProvider,_that.selectedModel);case _:
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String id,  List<ChatMessage> messages,  bool isStreaming,  String? selectedProvider,  String? selectedModel)?  $default,) {final _that = this;
switch (_that) {
case _ChatSession() when $default != null:
return $default(_that.id,_that.messages,_that.isStreaming,_that.selectedProvider,_that.selectedModel);case _:
  return null;

}
}

}

/// @nodoc


class _ChatSession implements ChatSession {
  const _ChatSession({required this.id, final  List<ChatMessage> messages = const [], this.isStreaming = false, this.selectedProvider, this.selectedModel}): _messages = messages;
  

@override final  String id;
 final  List<ChatMessage> _messages;
@override@JsonKey() List<ChatMessage> get messages {
  if (_messages is EqualUnmodifiableListView) return _messages;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_messages);
}

@override@JsonKey() final  bool isStreaming;
@override final  String? selectedProvider;
@override final  String? selectedModel;

/// Create a copy of ChatSession
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ChatSessionCopyWith<_ChatSession> get copyWith => __$ChatSessionCopyWithImpl<_ChatSession>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ChatSession&&(identical(other.id, id) || other.id == id)&&const DeepCollectionEquality().equals(other._messages, _messages)&&(identical(other.isStreaming, isStreaming) || other.isStreaming == isStreaming)&&(identical(other.selectedProvider, selectedProvider) || other.selectedProvider == selectedProvider)&&(identical(other.selectedModel, selectedModel) || other.selectedModel == selectedModel));
}


@override
int get hashCode => Object.hash(runtimeType,id,const DeepCollectionEquality().hash(_messages),isStreaming,selectedProvider,selectedModel);

@override
String toString() {
  return 'ChatSession(id: $id, messages: $messages, isStreaming: $isStreaming, selectedProvider: $selectedProvider, selectedModel: $selectedModel)';
}


}

/// @nodoc
abstract mixin class _$ChatSessionCopyWith<$Res> implements $ChatSessionCopyWith<$Res> {
  factory _$ChatSessionCopyWith(_ChatSession value, $Res Function(_ChatSession) _then) = __$ChatSessionCopyWithImpl;
@override @useResult
$Res call({
 String id, List<ChatMessage> messages, bool isStreaming, String? selectedProvider, String? selectedModel
});




}
/// @nodoc
class __$ChatSessionCopyWithImpl<$Res>
    implements _$ChatSessionCopyWith<$Res> {
  __$ChatSessionCopyWithImpl(this._self, this._then);

  final _ChatSession _self;
  final $Res Function(_ChatSession) _then;

/// Create a copy of ChatSession
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? id = null,Object? messages = null,Object? isStreaming = null,Object? selectedProvider = freezed,Object? selectedModel = freezed,}) {
  return _then(_ChatSession(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,messages: null == messages ? _self._messages : messages // ignore: cast_nullable_to_non_nullable
as List<ChatMessage>,isStreaming: null == isStreaming ? _self.isStreaming : isStreaming // ignore: cast_nullable_to_non_nullable
as bool,selectedProvider: freezed == selectedProvider ? _self.selectedProvider : selectedProvider // ignore: cast_nullable_to_non_nullable
as String?,selectedModel: freezed == selectedModel ? _self.selectedModel : selectedModel // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

// dart format on
