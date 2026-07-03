import 'dart:async';
import 'dart:developer' as developer;
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:go_router/go_router.dart';
import 'package:media_kit/media_kit.dart';
import 'package:path_provider/path_provider.dart';
import 'package:window_manager/window_manager.dart';
import 'core/api/client.dart' show runtimeTokenPathOverride;
import 'core/api/ffi_dio_adapter.dart';
import 'core/providers/chat_provider.dart';
import 'core/runtime/core_process_manager.dart';
import 'core/providers/preferences_provider.dart';
import 'core/providers/sessions_provider.dart';
import 'core/transport/frb_generated/ffi.dart' as frb;
import 'core/transport/frb_generated/frb_generated.dart';
import 'features/startup/core_startup_screen.dart';
import 'shared/theme/app_theme.dart';
import 'shared/theme/router.dart';
import 'shared/widgets/glass_backdrop.dart';

// Global navigator key so VM Service extensions can access the router
final navigatorKey = GlobalKey<NavigatorState>();

// Bundled weft-core sidecar; started before the UI, stopped on exit.
// Only used if FFI mode fails.
final coreManager = CoreProcessManager();

// Whether core is running in FFI mode (DLL in-process) vs sidecar mode.
bool _ffiMode = false;

// Global ProviderContainer reference for VM extensions
ProviderContainer? _container;

void _registerVmExtensions() {
  // ext.weft.navigate — navigate to a route
  developer.registerExtension('ext.weft.navigate', (method, params) async {
    final route = params['route'] ?? '/';
    final ctx = navigatorKey.currentContext;
    if (ctx != null) {
      ctx.go(route);
      return developer.ServiceExtensionResponse.result('{"navigated": "$route"}');
    }
    return developer.ServiceExtensionResponse.error(
        developer.ServiceExtensionResponse.extensionError, 'No context');
  });

  // ext.weft.currentRoute — get current route
  developer.registerExtension('ext.weft.currentRoute', (method, params) async {
    final ctx = navigatorKey.currentContext;
    if (ctx != null) {
      return developer.ServiceExtensionResponse.result('{"route": "${GoRouterState.of(ctx).uri}"}');
    }
    return developer.ServiceExtensionResponse.error(
        developer.ServiceExtensionResponse.extensionError, 'No context');
  });

  // ext.weft.sendMessage — send a chat message
  developer.registerExtension('ext.weft.sendMessage', (method, params) async {
    final content = params['content'];
    if (content == null || content.isEmpty) {
      return developer.ServiceExtensionResponse.error(
          developer.ServiceExtensionResponse.extensionError, 'content required');
    }
    final container = _container;
    if (container == null) {
      return developer.ServiceExtensionResponse.error(
          developer.ServiceExtensionResponse.extensionError, 'container not ready');
    }
    final sessionId = container.read(activeSessionIdProvider);
    if (sessionId == null) {
      final meta = await container.read(sessionsProvider.notifier).createSession();
      final newId = meta.id;
      container.read(activeSessionIdProvider.notifier).state = newId;
      await container.read(chatProvider(newId).notifier).sendMessage(content);
      return developer.ServiceExtensionResponse.result('{"sent": true, "sessionId": "$newId"}');
    }
    await container.read(chatProvider(sessionId).notifier).sendMessage(content);
    return developer.ServiceExtensionResponse.result('{"sent": true, "sessionId": "$sessionId"}');
  });
}

/// Stops the core when the window is closed, then lets the app exit.
class _CoreShutdownListener extends WindowListener {
  bool _closing = false;

  @override
  void onWindowClose() async {
    if (_closing) return;
    _closing = true;
    try {
      if (_ffiMode) {
        // FFI mode: stop core via in-process dispatch.
        await frb.stopCore();
      } else {
        // Sidecar mode: shut down the process.
        await coreManager.dispose();
      }
    } finally {
      await windowManager.setPreventClose(false);
      await windowManager.destroy();
    }
    exit(0);
  }
}

/// Join path segments with the platform separator.
String _joinPath(List<String> parts) {
  return parts.join(Platform.pathSeparator);
}

/// Attempt to start core via FFI (DLL in-process).
/// Returns true if successful, false if we should fall back to sidecar.
Future<bool> _tryStartCoreFfi() async {
  try {
    // Locate weft_core DLL next to the app executable.
    final appDir = File(Platform.resolvedExecutable).parent.path;
    final dllName = Platform.isWindows ? 'weft_core.dll' : 'libweft_core.so';

    // Search order for the DLL:
    final candidates = [
      _joinPath([appDir, dllName]),
      _joinPath([appDir, 'weft-core', dllName]),
      // Dev mode: relative to repo
      _joinPath([appDir, '..', '..', 'core', 'target', 'release', dllName]),
    ];

    String? dllPath;
    for (final path in candidates) {
      if (File(path).existsSync()) {
        dllPath = path;
        break;
      }
    }

    if (dllPath == null) {
      debugPrint('[weft-ffi] DLL not found, falling back to sidecar');
      return false;
    }

    // Initialize flutter_rust_bridge with the located DLL.
    await RustLib.init(
      externalLibrary: ExternalLibrary.open(dllPath),
    );

    // Resolve paths:
    // - projectRoot = exe directory (where packages/ lives in bundled app)
    // - dataDir = OS app-support directory for persistent data
    // - configDir = same as dataDir (writable location for config)
    final support = await getApplicationSupportDirectory();
    final dataDir = _joinPath([support.path, 'weft-core-data']);
    await Directory(dataDir).create(recursive: true);
    final configDir = dataDir;

    // Use appDir as project root (bundled packages/ lives there).
    final projectRoot = appDir;

    debugPrint('[weft-ffi] starting core: root=$projectRoot, data=$dataDir');

    StartupController.current?.setStatus('正在启动核心运行时…');
    final error = await frb.startCoreWithOptions(
      projectRoot: projectRoot,
      dataDir: dataDir,
      configDir: configDir,
    );

    if (error.isNotEmpty) {
      debugPrint('[weft-ffi] start_core_with_options failed: $error');
      return false;
    }

    // FFI mode active: set up the adapter and token path.
    FfiDioAdapter.enabled = true;
    final tokenPath = _joinPath([dataDir, 'runtime-token']);
    runtimeTokenPathOverride = tokenPath;

    // core 在后台线程异步写 token 文件,router_ready 时它可能还没落盘。
    // 等 token 文件真正存在(最多 10s)再返回,避免首个 API 请求 401。
    StartupController.current?.setStatus('正在准备鉴权…');
    final tokenFile = File(tokenPath);
    for (var i = 0; i < 100; i++) {
      if (await tokenFile.exists() && (await tokenFile.length()) > 0) break;
      await Future.delayed(const Duration(milliseconds: 100));
    }

    // 等 HTTP listener 真正可达(部分 UI/webview 走 HTTP,不走 FFI dispatch)。
    // 探测 /api/health 直到成功(最多 15s),避免首个请求 connection refused。
    StartupController.current?.setStatus('正在加载插件与服务…');
    final token = (await tokenFile.readAsString()).trim();
    final probe = HttpClient()..connectionTimeout = const Duration(seconds: 1);
    for (var i = 0; i < 150; i++) {
      try {
        final req = await probe.getUrl(Uri.parse('http://127.0.0.1:17830/api/health'));
        req.headers.set('Authorization', 'Bearer $token');
        final resp = await req.close().timeout(const Duration(seconds: 1));
        if (resp.statusCode == 200) {
          await resp.drain();
          break;
        }
        await resp.drain();
      } catch (_) {
        // 还没起来,继续等
      }
      await Future.delayed(const Duration(milliseconds: 100));
    }
    probe.close(force: true);

    debugPrint('[weft-ffi] core started successfully in FFI mode');
    return true;
  } catch (e, stack) {
    debugPrint('[weft-ffi] FFI init failed: $e\n$stack');
    return false;
  }
}

/// Phases of the app-level startup gate. Drives [_StartupGate].
enum StartupPhase { initializing, ready, failed }

/// Holds the current startup phase + failure reason so the UI can render a
/// loading screen while core spins up, instead of a frozen/blank window.
///
/// Core init (FFI DLL or sidecar) is intentionally kicked off *after* `runApp`
/// so the first frame — the loading screen — paints immediately.
class StartupController extends ChangeNotifier {
  StartupPhase phase = StartupPhase.initializing;
  String? errorMessage;

  /// 当前启动阶段的中文文案,显示在 loading 屏上让用户知道进度。
  String statusText = '正在初始化…';

  /// 全局单例,供 _tryStartCoreFfi 等函数更新启动状态文案。
  static StartupController? current;

  void setStatus(String text) {
    statusText = text;
    notifyListeners();
  }

  /// Run (or re-run) core initialization. Safe to call again from a Retry.
  Future<void> initCore() async {
    current = this;
    phase = StartupPhase.initializing;
    errorMessage = null;
    statusText = '正在启动核心…';
    notifyListeners();

    // Try FFI mode first (DLL in-process, no sidecar).
    _ffiMode = await _tryStartCoreFfi();

    if (!_ffiMode) {
      // Fallback: start sidecar core process.
      setStatus('正在启动核心进程(sidecar)…');
      try {
        await coreManager.ensureRunning();
        runtimeTokenPathOverride = coreManager.tokenFilePath;
      } catch (e) {
        debugPrint('[weft] Core start failed: $e');
        errorMessage = e is CoreLaunchException ? e.message : e.toString();
        phase = StartupPhase.failed;
        notifyListeners();
        return;
      }
    }

    statusText = '就绪';
    phase = StartupPhase.ready;
    notifyListeners();
  }

  /// Give up on the pre-flight gate and enter the app anyway. The in-app
  /// connection poller keeps trying to reach core, so this is not a dead end.
  void skip() {
    phase = StartupPhase.ready;
    notifyListeners();
  }
}

final startupController = StartupController();

void main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();

  _registerVmExtensions();
  await windowManager.ensureInitialized();

  // Intercept window close so we can stop the core before exiting.
  await windowManager.setPreventClose(true);
  windowManager.addListener(_CoreShutdownListener());

  windowManager.waitUntilReadyToShow(
    const WindowOptions(
      minimumSize: Size(900, 600),
      size: Size(1200, 800),
      title: 'WEFT',
    ),
    () async {
      await windowManager.show();
      await windowManager.focus();
    },
  );

  final container = ProviderContainer();
  _container = container;
  // Paint the loading screen first, then kick off the blocking core init so the
  // window is never frozen/blank while FFI/sidecar spins up.
  runApp(UncontrolledProviderScope(container: container, child: const WeftApp()));
  unawaited(startupController.initCore());
}

class WeftApp extends ConsumerWidget {
  const WeftApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(routerProvider);
    final prefs = ref.watch(preferencesProvider);
    final themeMode = prefs.themeMode;

    return MaterialApp.router(
      title: 'WEFT',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.light,
      darkTheme: AppTheme.dark,
      themeMode: themeMode,
      routerConfig: router,
      builder: (context, child) => GlassBackdrop(
        child: _StartupGate(child: child ?? const SizedBox()),
      ),
    );
  }
}

/// Overlays the [CoreStartupScreen] on top of the routed content until core
/// initialization finishes (or the user chooses to skip). This keeps the router
/// mounted underneath so navigation state is preserved, while the window shows a
/// spinner instead of a blank/frozen frame during the blocking FFI/sidecar boot.
class _StartupGate extends StatefulWidget {
  const _StartupGate({required this.child});
  final Widget child;

  @override
  State<_StartupGate> createState() => _StartupGateState();
}

class _StartupGateState extends State<_StartupGate> {
  @override
  void initState() {
    super.initState();
    startupController.addListener(_onChange);
  }

  @override
  void dispose() {
    startupController.removeListener(_onChange);
    super.dispose();
  }

  void _onChange() {
    if (mounted) setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final phase = startupController.phase;
    if (phase == StartupPhase.ready) return widget.child;

    return CoreStartupScreen(
      failed: phase == StartupPhase.failed,
      message: startupController.errorMessage,
      statusText: startupController.statusText,
      onRetry: () => startupController.initCore(),
      onSkip: startupController.skip,
    );
  }
}
