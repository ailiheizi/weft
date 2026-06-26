import 'dart:convert';
import 'dart:typed_data';

import 'package:dio/dio.dart';

import '../transport/frb_generated/ffi.dart' as frb;

/// 自定义 Dio HttpClientAdapter:把所有 HTTP 请求透明转为 FFI rpcCall 调用。
/// 上层 API client 零改动,Dio 请求不走网络,直接经 FRB → Rust router.oneshot。
class FfiDioAdapter implements HttpClientAdapter {
  /// FFI 是否就绪(RustLib 初始化成功)。未就绪时 fallback 到默认 HTTP。
  static bool enabled = false;

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    if (!enabled) {
      // FFI 未就绪,走默认 HTTP adapter。
      return HttpClientAdapter().fetch(options, requestStream, cancelFuture);
    }

    // 把 Dio 请求转为 RequestEnvelope JSON。
    final method = options.method.toUpperCase();
    final path = options.uri.path +
        (options.uri.query.isNotEmpty ? '?${options.uri.query}' : '');

    // 收集 body
    Map<String, dynamic>? body;
    if (options.data != null) {
      if (options.data is Map) {
        body = Map<String, dynamic>.from(options.data as Map);
      } else if (options.data is String) {
        try {
          body = jsonDecode(options.data as String) as Map<String, dynamic>?;
        } catch (_) {
          body = {'_raw': options.data};
        }
      }
    }

    // 转 headers(取 authorization)
    final headers = <String, String>{};
    final auth = options.headers['Authorization'] ?? options.headers['authorization'];
    if (auth != null) headers['authorization'] = auth.toString();

    final envelope = jsonEncode({
      'id': DateTime.now().microsecondsSinceEpoch.toString(),
      'method': (method == 'GET' || method == 'HEAD') ? 'QUERY' : method == 'DELETE' ? 'DELETE' : method == 'PUT' ? 'PUT' : 'CALL',
      'path': path,
      'headers': headers,
      if (body != null) 'body': body,
    });

    // 调 FFI
    final respJson = await frb.rpcCall(requestJson: envelope);
    final resp = jsonDecode(respJson) as Map<String, dynamic>;

    final status = (resp['status'] as num?)?.toInt() ?? 500;
    final respBody = resp['body'];
    final bodyBytes = utf8.encode(jsonEncode(respBody));

    return ResponseBody.fromBytes(
      bodyBytes,
      status,
      headers: {
        'content-type': ['application/json'],
      },
      statusMessage: status < 400 ? 'OK' : 'Error',
    );
  }

  @override
  void close({bool force = false}) {}
}
