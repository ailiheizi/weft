import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:path_provider/path_provider.dart';

import '../transport/frb_generated/ffi.dart' as frb;

/// 临时诊断:把 FFI adapter 的关键事件写到 data-dir/ffi-adapter.log。
void _ffiLog(String msg) {
  () async {
    try {
      final dir = await getApplicationSupportDirectory();
      final f = File('${dir.path}/weft-core-data/ffi-adapter.log');
      await f.writeAsString(
        '${DateTime.now().toIso8601String()} $msg\n',
        mode: FileMode.append,
        flush: true,
      );
    } catch (_) {}
  }();
}

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
    _ffiLog('fetch ${options.method} ${options.uri.path} enabled=$enabled');
    if (!enabled) {
      // FFI 未就绪,等一小段时间(core 可能正在启动),避免立即走 HTTP 失败。
      for (var i = 0; i < 50 && !enabled; i++) {
        await Future.delayed(const Duration(milliseconds: 200));
      }
      if (!enabled) {
        // 超时仍未就绪,走默认 HTTP adapter(sidecar 模式)。
        _ffiLog('fallback HTTP after wait: ${options.uri.path}');
        return HttpClientAdapter().fetch(options, requestStream, cancelFuture);
      }
      _ffiLog('enabled after wait: ${options.uri.path}');
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
