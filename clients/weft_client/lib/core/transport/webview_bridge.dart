import 'dart:async';
import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;
import 'package:webview_windows/webview_windows.dart' as wv;

import 'transport_adapter.dart';

/// FFI WebView Bridge — 连接 WebView JS 层与 Dart FFI 传输层。
///
/// 职责:
/// 1. 向 WebView 注入 bridge shim (fetch 拦截)
/// 2. 监听 WebView postMessage (JS → Dart)
/// 3. 将请求转发给 [TransportAdapter] (FFI 或 HTTP fallback)
/// 4. 将响应回传给 WebView (Dart → JS)
///
/// 使用 webview_windows 的 `WebviewController`:
/// - `webMessage` -> `Stream<dynamic>` (已被 JSON decode)
/// - `postWebMessage(String)` -> 向 JS 发送消息
/// - `addScriptToExecuteOnDocumentCreated(String)` -> 页面加载前注入 JS
class WebViewBridge {
  WebViewBridge({
    required this.controller,
    required this.primaryAdapter,
    this.fallbackAdapter,
    this.coreBase = '',
  });

  final wv.WebviewController controller;

  /// 主传输(通常是 FfiTransportAdapter)。
  final TransportAdapter primaryAdapter;

  /// 降级传输(通常是 HttpTransportAdapter),FFI 失败时使用。
  final TransportAdapter? fallbackAdapter;

  /// Core API base URL(用于 shim 中的 CORE_BASE 占位符替换)。
  final String coreBase;

  StreamSubscription<dynamic>? _messageSubscription;
  bool _initialized = false;

  /// 初始化 bridge:注入 shim + 开始监听。
  ///
  /// 必须在 controller.initialize() 之后、loadUrl() 之前调用。
  Future<void> initialize() async {
    if (_initialized) return;

    // 加载并注入 JS shim(在每次文档创建前自动执行)
    final shimSource = await _loadShim();
    await controller.addScriptToExecuteOnDocumentCreated(shimSource);

    // 开始监听 WebView 消息
    // webview_windows 的 webMessage 是 Stream<dynamic>,
    // 内部已对 JSON 字符串做了 jsonDecode,所以收到的可能是 Map 或其他类型。
    _messageSubscription = controller.webMessage.listen(
      _handleMessage,
      onError: (e) {
        // WebView 通道错误,静默处理(页面可能已销毁)
      },
    );

    _initialized = true;
  }

  /// 加载 JS shim 并替换占位符。
  Future<String> _loadShim() async {
    final raw = await rootBundle.loadString(
      'assets/webview-bridge-shim.js',
    );
    return raw.replaceAll('{{CORE_BASE}}', coreBase);
  }

  /// 处理来自 WebView 的消息。
  ///
  /// webview_windows 内部会对收到的 string 做 json.decode,
  /// 所以 [rawMessage] 通常已经是 `Map<dynamic, dynamic>`。
  /// 如果 decode 失败(非 JSON),会作为 error 推送到 stream,不会到这里。
  Future<void> _handleMessage(dynamic rawMessage) async {
    Map<String, dynamic> msg;
    try {
      if (rawMessage is Map) {
        msg = Map<String, dynamic>.from(rawMessage);
      } else if (rawMessage is String) {
        msg = jsonDecode(rawMessage) as Map<String, dynamic>;
      } else {
        return; // 未知类型,忽略
      }
    } catch (_) {
      return; // 解析失败,忽略
    }

    final type = msg['type'] as String?;
    if (type != 'request') return;

    final id = msg['id'] as String?;
    if (id == null) return;

    try {
      final response = await _dispatch(msg);
      await _sendResponse(id, response);
    } catch (e) {
      await _sendResponse(
        id,
        const ResponseEnvelope(id: '', status: 500, body: null),
        errorOverride: {'error': e.toString()},
      );
    }
  }

  /// 将 JS 请求分发到 transport adapter。
  Future<ResponseEnvelope> _dispatch(Map<String, dynamic> msg) async {
    final envelope = _buildRequestEnvelope(msg);

    try {
      return await primaryAdapter.send(envelope);
    } catch (e) {
      // FFI 失败,尝试 fallback
      if (fallbackAdapter != null) {
        return await fallbackAdapter!.send(envelope);
      }
      rethrow;
    }
  }

  /// 从 JS 消息构建 RequestEnvelope。
  RequestEnvelope _buildRequestEnvelope(Map<String, dynamic> msg) {
    final method = _mapHttpMethod(msg['method'] as String? ?? 'GET');
    final path = msg['url'] as String? ?? '/';
    final headers = (msg['headers'] as Map<dynamic, dynamic>?)
            ?.map((k, v) => MapEntry(k.toString(), v.toString())) ??
        const <String, String>{};
    final body = (msg['body'] as Map<dynamic, dynamic>?)
        ?.map((k, v) => MapEntry(k.toString(), v));

    return RequestEnvelope(
      id: msg['id'] as String,
      method: method,
      path: path,
      headers: headers,
      body: body,
    );
  }

  /// HTTP method 字符串 → RpcMethod 枚举。
  RpcMethod _mapHttpMethod(String method) {
    switch (method.toUpperCase()) {
      case 'GET':
        return RpcMethod.query;
      case 'POST':
      case 'PUT':
      case 'DELETE':
      case 'PATCH':
        return RpcMethod.call;
      default:
        return RpcMethod.call;
    }
  }

  /// 将响应编码并发回 WebView。
  Future<void> _sendResponse(
    String requestId,
    ResponseEnvelope response, {
    Map<String, dynamic>? errorOverride,
  }) async {
    final payload = jsonEncode({
      'id': requestId,
      'type': 'response',
      'status': response.status,
      'headers': response.headers,
      'body': errorOverride ?? response.body,
    });

    try {
      await controller.postWebMessage(payload);
    } catch (_) {
      // WebView 已销毁,静默丢弃
    }
  }

  /// 释放资源。
  Future<void> dispose() async {
    await _messageSubscription?.cancel();
    _messageSubscription = null;
    _initialized = false;
  }
}
