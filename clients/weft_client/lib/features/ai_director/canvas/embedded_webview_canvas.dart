import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:webview_windows/webview_windows.dart';

import '../../../core/api/client.dart'
    show apiClientProvider, runtimeTokenPathOverride;
import '../../../core/transport/transport_provider.dart';
import '../../../core/transport/webview_bridge.dart';

/// 嵌入式 webview：加载 web-canvas UI（React Flow 画布）。
///
/// Windows 平台使用 webview_windows (WebView2/Edge 内核)。
/// 集成 FFI WebView Bridge：将 JS fetch(/api/*) 请求通过 postMessage
/// 转发给 Dart 层，由 FfiTransportAdapter 直接调用 core，消除 HTTP 中间层。
class EmbeddedWebviewCanvas extends ConsumerStatefulWidget {
  const EmbeddedWebviewCanvas({super.key, this.url});

  final String? url;

  @override
  ConsumerState<EmbeddedWebviewCanvas> createState() =>
      _EmbeddedWebviewCanvasState();
}

class _EmbeddedWebviewCanvasState extends ConsumerState<EmbeddedWebviewCanvas> {
  final _controller = WebviewController();
  WebViewBridge? _bridge;
  bool _ready = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _initWebview();
  }

  Future<void> _initWebview() async {
    try {
      await _controller.initialize();
      if (!mounted) return;

      // 设置 FFI bridge（注入 shim + 监听 postMessage）
      await _setupBridge();

      // 加载目标 URL
      final url = widget.url ?? await _buildDefaultUrl();
      await _controller.loadUrl(url);

      if (mounted) setState(() => _ready = true);
    } catch (e) {
      if (mounted) setState(() => _error = e.toString());
    }
  }

  /// 配置 WebView Bridge：注入 JS shim,监听消息,转发到 transport adapter。
  Future<void> _setupBridge() async {
    final transport = ref.read(transportSyncProvider);

    // 如果 transport 是 FFI,用它做 primary;否则直接用当前 adapter。
    // fallback 为 HTTP(当 FFI 不可用时)。
    final httpFallback = _isHttpAdapter(transport)
        ? null // 主适配已经是 HTTP,不需要 fallback
        : HttpTransportAdapter(ref.read(apiClientProvider));

    _bridge = WebViewBridge(
      controller: _controller,
      primaryAdapter: transport,
      fallbackAdapter: httpFallback,
      coreBase: 'http://127.0.0.1:17830',
    );

    await _bridge!.initialize();
  }

  bool _isHttpAdapter(TransportAdapter adapter) =>
      adapter is HttpTransportAdapter;

  /// 构建默认 URL（canvas 画布页面）。
  Future<String> _buildDefaultUrl() async {
    final token = await _readToken();
    final cacheBust = DateTime.now().millisecondsSinceEpoch;
    const coreBase = 'http://127.0.0.1:17830';
    return '$coreBase/app-ui/?core=$coreBase'
        '&token=${Uri.encodeComponent(token)}'
        '&_=$cacheBust';
  }

  /// 从 runtime-token 文件读取鉴权 token（和 Dio 拦截器走同样的候选路径）。
  Future<String> _readToken() async {
    final candidates = <String>[
      './data/runtime-token',
      '../data/runtime-token',
    ];
    final override = runtimeTokenPathOverride;
    if (override != null && override.isNotEmpty) {
      candidates.insert(0, override);
    }
    for (final path in candidates) {
      try {
        final f = File(path);
        if (await f.exists()) {
          final token = (await f.readAsString()).trim();
          if (token.isNotEmpty) return token;
        }
      } catch (_) {}
    }
    return '';
  }

  @override
  void dispose() {
    _bridge?.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.error_outline, size: 48, color: Colors.red),
            const SizedBox(height: 8),
            Text('WebView 初始化失败', style: Theme.of(context).textTheme.bodyLarge),
            const SizedBox(height: 4),
            Text(_error!, style: Theme.of(context).textTheme.bodySmall),
          ],
        ),
      );
    }
    if (!_ready) {
      return const Center(child: CircularProgressIndicator());
    }
    return Webview(_controller);
  }
}
