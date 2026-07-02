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
    runtimeTokenPathOverride = _joinPath([dataDir, 'runtime-token']);
    debugPrint('[weft-ffi] core started successfully in FFI mode');
    return true;
  } catch (e, stack) {
    debugPrint('[weft-ffi] FFI init failed: $e\n$stack');
    return false;
  }
}

void main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();

  // Try FFI mode first (DLL in-process, no sidecar).
  _ffiMode = await _tryStartCoreFfi();

  if (!_ffiMode) {
    // Fallback: start sidecar core process.
    try {
      await coreManager.ensureRunning();
      runtimeTokenPathOverride = coreManager.tokenFilePath;
    } catch (e) {
      debugPrint('[weft] Core start failed: $e');
    }
  }

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
  runApp(UncontrolledProviderScope(container: container, child: const WeftApp()));
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
      builder: (context, child) => GlassBackdrop(child: child ?? const SizedBox()),
    );
  }
}
