import 'dart:async';
import 'package:uuid/uuid.dart';

const _uuid = Uuid();

/// 请求方法:CALL(写/动作) | QUERY(读) | SUBSCRIBE(流) | CANCEL(取消订阅)。
enum RpcMethod { call, query, subscribe, cancel }

/// 统一请求信封——传输无关,路径即接口。
class RequestEnvelope {
  RequestEnvelope({
    String? id,
    required this.method,
    required this.path,
    this.headers = const {},
    this.body,
  }) : id = id ?? _uuid.v4();

  final String id;
  final RpcMethod method;
  final String path;
  final Map<String, String> headers;
  final Map<String, dynamic>? body;

  Map<String, dynamic> toJson() => {
        'id': id,
        'method': method.name.toUpperCase(),
        'path': path,
        if (headers.isNotEmpty) 'headers': headers,
        if (body != null) 'body': body,
      };
}

/// 统一响应信封。
class ResponseEnvelope {
  const ResponseEnvelope({
    required this.id,
    required this.status,
    this.headers = const {},
    this.body,
  });

  final String id;
  final int status;
  final Map<String, String> headers;
  final Map<String, dynamic>? body;

  bool get isOk => status >= 200 && status < 300;

  factory ResponseEnvelope.fromJson(Map<String, dynamic> json) {
    return ResponseEnvelope(
      id: json['id'] as String? ?? '',
      status: (json['status'] as num?)?.toInt() ?? 500,
      headers: (json['headers'] as Map<String, dynamic>?)
              ?.map((k, v) => MapEntry(k, v.toString())) ??
          const {},
      body: json['body'] as Map<String, dynamic>?,
    );
  }
}

/// 传输适配器抽象——底层可为 HTTP/WS/Pipe/FFI,调用方不感知。
abstract class TransportAdapter {
  /// 单次请求→响应。
  Future<ResponseEnvelope> send(RequestEnvelope request);

  /// 订阅流(如 stream/events):返回多条响应的 Stream。
  Stream<ResponseEnvelope> subscribe(RequestEnvelope request);

  /// 关闭连接/释放资源。
  Future<void> close();
}
