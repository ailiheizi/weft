import 'dart:io';

import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';

import '../../../core/api/client.dart' show runtimeTokenPathOverride;

/// 嵌入式 webview：加载 web-canvas UI（React Flow 画布）。
/// Core 通过 /app-ui/ 端点 serve 静态资源，token 从 runtime-token 文件动态读取。
class EmbeddedWebviewCanvas extends StatefulWidget {
  const EmbeddedWebviewCanvas({super.key, this.url});

  final String? url;

  @override
  State<EmbeddedWebviewCanvas> createState() => _EmbeddedWebviewCanvasState();
}

class _EmbeddedWebviewCanvasState extends State<EmbeddedWebviewCanvas> {
  late final WebViewController _controller;
  bool _loaded = false;

  @override
  void initState() {
    super.initState();
    _controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted);
    _initUrl();
  }

  Future<void> _initUrl() async {
    final token = await _readToken();
    final cacheBust = DateTime.now().millisecondsSinceEpoch;
    final url = widget.url ??
        'http://127.0.0.1:17830/app-ui/?core=http://127.0.0.1:17830&token=${Uri.encodeComponent(token)}&_=$cacheBust';
    _controller.loadRequest(Uri.parse(url));
    if (mounted) setState(() => _loaded = true);
  }

  /// 从 runtime-token 文件读取鉴权 token（和 Dio 拦截器走同样的候选路径）。
  Future<String> _readToken() async {
    final candidates = <String>[
      // 仓库根/data/runtime-token（最常见）
      './data/runtime-token',
      '../data/runtime-token',
    ];
    // 如果有 override 路径，优先
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
  Widget build(BuildContext context) {
    if (!_loaded) {
      return const Center(child: CircularProgressIndicator());
    }
    return WebViewWidget(controller: _controller);
  }
}
