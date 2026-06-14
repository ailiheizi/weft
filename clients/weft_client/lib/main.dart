import 'dart:developer' as developer;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:media_kit/media_kit.dart';
import 'package:window_manager/window_manager.dart';
import 'core/providers/chat_provider.dart';
import 'core/providers/sessions_provider.dart';
import 'shared/theme/app_theme.dart';
import 'shared/theme/router.dart';
import 'shared/widgets/glass_backdrop.dart';

// Global navigator key so VM Service extensions can access the router
final navigatorKey = GlobalKey<NavigatorState>();

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
