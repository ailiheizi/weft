import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/api/client.dart';
import '../../core/providers/data_providers.dart';
import '../../shared/widgets/app_error_widget.dart';
import '../ai_director/canvas/embedded_webview_canvas.dart';
import 'app_detail_screen.dart';

/// Host for a product package's surface.
///
/// 动态判断 package 是否有 web UI(通过探测多个候选地址)。
/// 有 → 嵌 webview 加载;没有 → fallback 到诊断页(AppDetailScreen)。
/// 探测顺序：本地 file:// → core 托管路径 → 本地 dev server → fallback。
class PackageSurfaceScreen extends ConsumerStatefulWidget {
  const PackageSurfaceScreen({super.key, required this.appName});

  final String appName;

  @override
  ConsumerState<PackageSurfaceScreen> createState() =>
      _PackageSurfaceScreenState();
}

class _PackageSurfaceScreenState extends ConsumerState<PackageSurfaceScreen> {
  /// null=还在探测, true=有 web UI, false=没有
  bool? _hasWebUi;
  /// 探测命中的 webview URL
  String? _webUiUrl;

  @override
  void initState() {
    super.initState();
    _probeWebUi();
  }

  Future<void> _probeWebUi() async {
    final dio = ref.read(apiClientProvider);
    final coreBase = 'http://127.0.0.1:17830';

    // 读 loopback token（webview 内 JS 调 API 需要它）
    final token = await readLoopbackToken();

    String buildUrl(String base) {
      final params = ['core=$coreBase'];
      if (token.isNotEmpty) params.add('token=$token');
      return '$base${base.contains('?') ? '&' : '?'}${params.join('&')}';
    }

    // 候选1 跳过: file:// 在 WebView2 下有 CORS/JS 限制,改用 DLL 内 HTTP serve。
    // final localUiPath = await _findLocalUiPath();
    // if (localUiPath != null) {
    //   _webUiUrl = buildUrl(Uri.file(localUiPath).toString());
    //   if (mounted) setState(() => _hasWebUi = true);
    //   return;
    // }

    // 候选2：core HTTP 托管路径 /packages/{name}/ui/index.html
    // FFI 模式下 HTTP listener 在后台启动,可能有延迟,重试几次。
    for (var attempt = 0; attempt < 5; attempt++) {
      try {
        final resp = await dio.get<void>(
          '/packages/${widget.appName}/ui/index.html',
          options: Options(receiveTimeout: const Duration(seconds: 3)),
        );
        if (resp.statusCode == 200) {
          _webUiUrl = buildUrl(
              '$coreBase/packages/${Uri.encodeComponent(widget.appName)}/ui/index.html');
          if (mounted) setState(() => _hasWebUi = true);
          return;
        }
      } catch (_) {}
      if (attempt < 4) await Future.delayed(const Duration(seconds: 2));
    }

    // 候选3：包的本地 Vite dev server（开发模式）
    final devPorts = <String, int>{
      'rss-reader': 5173,
    };
    final devPort = devPorts[widget.appName];
    if (devPort != null) {
      try {
        final devDio = Dio(BaseOptions(
          connectTimeout: const Duration(seconds: 2),
          receiveTimeout: const Duration(seconds: 2),
        ));
        final resp = await devDio.head<void>('http://localhost:$devPort/');
        if (resp.statusCode == 200) {
          _webUiUrl = buildUrl('http://localhost:$devPort');
          if (mounted) setState(() => _hasWebUi = true);
          return;
        }
      } catch (_) {}
    }

    if (mounted) setState(() => _hasWebUi = false);
  }

  /// 搜索本地打包的 UI 文件。
  /// 按优先级检查：
  /// 1. exe 同级 bundle/packages/official/{name}/ui/dist/index.html
  /// 2. 项目仓库 packages/official/{name}/ui/dist/index.html
  Future<String?> _findLocalUiPath() async {
    final name = widget.appName;
    final exeDir = File(Platform.resolvedExecutable).parent.path;

    final candidates = [
      // Release 打包路径（exe 同级 packages 目录）
      '$exeDir/packages/official/$name/ui/dist/index.html',
      '$exeDir/packages/official/$name/ui/index.html',
      // 旧路径兼容（bundle 子目录）
      '$exeDir/bundle/packages/official/$name/ui/dist/index.html',
      // 开发模式（项目根）
      'packages/official/$name/ui/dist/index.html',
      '../packages/official/$name/ui/dist/index.html',
      '../../packages/official/$name/ui/dist/index.html',
    ];

    for (final path in candidates) {
      try {
        final file = File(path);
        if (await file.exists()) {
          // 返回绝对路径
          return file.absolute.path;
        }
      } catch (_) {}
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final appAsync = ref.watch(appDetailProvider(widget.appName));

    return appAsync.when(
      loading: () => const Scaffold(
        backgroundColor: Colors.transparent,
        body: Center(child: CircularProgressIndicator()),
      ),
      error: (e, _) => Scaffold(
        backgroundColor: Colors.transparent,
        body: AppErrorWidget(
          error: e,
          onRetry: () => ref.invalidate(appDetailProvider(widget.appName)),
        ),
      ),
      data: (_) {
        // 探测中显示加载
        if (_hasWebUi == null) {
          return const Scaffold(
            backgroundColor: Colors.transparent,
            body: Center(child: CircularProgressIndicator()),
          );
        }
        return Scaffold(
          backgroundColor: Colors.transparent,
          appBar: AppBar(
            backgroundColor: Colors.transparent,
            elevation: 0,
            leading: IconButton(
              icon: const Icon(Icons.arrow_back),
              tooltip: '返回',
              onPressed: () => context.go('/dashboard'),
            ),
            title: Text(widget.appName),
          ),
          body: _hasWebUi!
              ? EmbeddedWebviewCanvas(url: _webUiUrl)
              : AppDetailScreen(appName: widget.appName),
        );
      },
    );
  }
}
