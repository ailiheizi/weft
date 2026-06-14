import 'dart:developer' as developer;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:media_kit/media_kit.dart';
import 'package:window_manager/window_manager.dart';
import 'core/api/client.dart' show runtimeTokenPathOverride;
import 'core/providers/chat_provider.dart';
import 'core/providers/sessions_provider.dart';
import 'core/runtime/core_process_manager.dart';
import 'shared/theme/app_theme.dart';
import 'shared/theme/router.dart';
import 'shared/widgets/glass_backdrop.dart';

// Global navigator key so VM Service extensions can access the router
final navigatorKey = GlobalKey<NavigatorState>();

// Bundled weft-core sidecar; started before the UI, stopped on exit.
final coreManager = CoreProcessManager();

/// Stops the bundled core when the window is closed, then lets the app exit.
class _CoreShutdownListener extends WindowListener {
  bool _closing = false;

  @override
  void onWindowClose() async {
    if (_closing) return;
    _closing = true;
    try {
      await coreManager.dispose();
    } finally {
      await windowManager.setPreventClose(false);
      await windowManager.destroy();
    }
  }
}

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
      final location = GoRouterState.of(ctx).uri.toString();
      return developer.ServiceExtensionResponse.result('{"route": "$location"}');
    }
    return developer.ServiceExtensionResponse.result('{"route": "unknown"}');
  });

  // ext.weft.sendMessage — send a chat message in the active session
  developer.registerExtension('ext.weft.sendMessage', (method, params) async {
    final content = params['content'] ?? '';
    if (content.isEmpty) {
      return developer.ServiceExtensionResponse.error(
          developer.ServiceExtensionResponse.extensionError, 'content is required');
    }
    final container = _container;
    if (container == null) {
      return developer.ServiceExtensionResponse.error(
          developer.ServiceExtensionResponse.extensionError, 'container not ready');
    }
    final sessionId = container.read(activeSessionIdProvider);
    if (sessionId == null) {
      // Create a new session first
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

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();
  _registerVmExtensions();
  await windowManager.ensureInitialized();

  // Intercept window close so we can stop the sidecar core before exiting.
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
  runApp(UncontrolledProviderScope(
    container: container,
    child: const CoreStartupGate(child: WeftApp()),
  ));
}

/// Starts the bundled weft-core before showing the app. While the core is
/// coming up a splash is shown; if it fails to start, an error with a retry
/// button is shown instead. A core already running externally is reused.
class CoreStartupGate extends StatefulWidget {
  const CoreStartupGate({super.key, required this.child});

  final Widget child;

  @override
  State<CoreStartupGate> createState() => _CoreStartupGateState();
}

class _CoreStartupGateState extends State<CoreStartupGate> {
  late Future<void> _startup;

  @override
  void initState() {
    super.initState();
    _startup = _startCore();
  }

  Future<void> _startCore() async {
    await coreManager.ensureRunning();
    // Point the API client at the exact runtime-token the core just wrote,
    // so authenticated requests don't 401 due to a path mismatch.
    runtimeTokenPathOverride = coreManager.tokenFilePath;
  }

  void _retry() {
    setState(() => _startup = _startCore());
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<void>(
      future: _startup,
      builder: (context, snapshot) {
        if (snapshot.connectionState == ConnectionState.done) {
          if (snapshot.hasError) {
            return _StartupScaffold(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.error_outline, size: 48),
                  const SizedBox(height: 16),
                  const Text('Failed to start weft-core'),
                  const SizedBox(height: 8),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 32),
                    child: Text(
                      '${snapshot.error}',
                      textAlign: TextAlign.center,
                      style: const TextStyle(fontSize: 12, color: Colors.grey),
                    ),
                  ),
                  const SizedBox(height: 16),
                  FilledButton(onPressed: _retry, child: const Text('Retry')),
                ],
              ),
            );
          }
          return widget.child;
        }
        return const _StartupScaffold(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              SizedBox(
                width: 36,
                height: 36,
                child: CircularProgressIndicator(strokeWidth: 3),
              ),
              SizedBox(height: 20),
              Text('Starting weft-core…'),
            ],
          ),
        );
      },
    );
  }
}

class _StartupScaffold extends StatelessWidget {
  const _StartupScaffold({required this.child});
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      home: Scaffold(body: Center(child: child)),
    );
  }
}

class WeftApp extends ConsumerWidget {
  const WeftApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(routerProvider);
    return MaterialApp.router(
      title: 'WEFT',
      theme: AppTheme.dark,
      routerConfig: router,
      debugShowCheckedModeBanner: false,
      builder: (context, child) =>
          GlassBackdrop(child: child ?? const SizedBox.shrink()),
    );
  }
}
