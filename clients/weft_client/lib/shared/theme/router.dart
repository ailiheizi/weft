import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter/widgets.dart';
import 'package:go_router/go_router.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import '../../features/ai_director/canvas/vyuh_poc_page.dart';
import '../../features/apps/package_surface_screen.dart';
import '../../features/chat/chat_screen.dart';
import '../../features/dashboard/dashboard_screen.dart';
import '../../features/management/management_screens.dart';
import '../../features/onboarding/onboarding_screen.dart';
import '../../features/orchestration/orchestration_screen.dart';
import '../../features/packages/packages_screen.dart';
import '../../features/providers_config/providers_screen.dart';
import '../../features/settings/settings_screen.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/preferences_provider.dart';
import '../../main.dart' show navigatorKey;
import '../widgets/app_shell.dart';

part 'router.g.dart';

/// 统一的页面过渡：淡入 + 轻微上滑（Linear 风短时长）。
CustomTransitionPage<void> _fade(Widget child) {
  return CustomTransitionPage<void>(
    child: child,
    transitionDuration: const Duration(milliseconds: 180),
    reverseTransitionDuration: const Duration(milliseconds: 120),
    transitionsBuilder: (context, anim, sec, child) {
      final curved = CurvedAnimation(parent: anim, curve: Curves.easeOutCubic);
      return FadeTransition(
        opacity: curved,
        child: SlideTransition(
          position: Tween(begin: const Offset(0, 0.012), end: Offset.zero)
              .animate(curved),
          child: child,
        ),
      );
    },
  );
}

/// 桥接 Riverpod → GoRouter.refreshListenable:当 providers 列表或
/// onboarding 完成状态变化时,通知 GoRouter 重新评估 redirect。
/// 这样首启时 providers 异步加载完(为空)后能自动跳到 onboarding。
class _RouterRefresh extends ChangeNotifier {
  _RouterRefresh(Ref ref) {
    ref.listen(providersProvider, (_, _) => notifyListeners());
    ref.listen(preferencesProvider, (_, _) => notifyListeners());
  }
}

@riverpod
GoRouter router(Ref ref) {
  final refresh = _RouterRefresh(ref);
  ref.onDispose(refresh.dispose);

  return GoRouter(
    navigatorKey: navigatorKey,
    initialLocation: '/dashboard',
    refreshListenable: refresh,
    redirect: (context, state) {
      final prefs = ref.read(preferencesProvider);
      // 已完成/跳过引导:不再拦截。
      if (prefs.onboardingCompleted) {
        // 已完成后若仍停在 onboarding,送回 dashboard。
        return state.matchedLocation == '/onboarding' ? '/dashboard' : null;
      }

      final providersAsync = ref.read(providersProvider);
      // 加载中或出错时不重定向,避免闪烁(dashboard 自身会显示加载态)。
      final isEmpty = providersAsync.maybeWhen(
        data: (list) => list.isEmpty,
        orElse: () => false,
      );

      final atOnboarding = state.matchedLocation == '/onboarding';
      if (isEmpty && !atOnboarding) return '/onboarding';
      // providers 非空但用户手动进了 onboarding:放行到 dashboard。
      if (!isEmpty && atOnboarding) return '/dashboard';
      return null;
    },
    routes: [
      GoRoute(
        path: '/onboarding',
        pageBuilder: (context, state) => _fade(const OnboardingScreen()),
      ),
      ShellRoute(
        builder: (context, state, child) => AppShell(child: child),
        routes: [
          GoRoute(
            path: '/dashboard',
            pageBuilder: (context, state) =>
                _fade(const DashboardScreen()),
          ),
          GoRoute(
            path: '/apps/:name',
            pageBuilder: (context, state) {
              final name = state.pathParameters['name']!;
              return _fade(PackageSurfaceScreen(appName: name));
            },
          ),
          GoRoute(
            path: '/chat',
            pageBuilder: (context, state) =>
                _fade(const ChatScreen()),
          ),
          GoRoute(
            path: '/orchestration',
            pageBuilder: (context, state) =>
                _fade(const OrchestrationScreen()),
          ),
          GoRoute(
            path: '/packages',
            pageBuilder: (context, state) =>
                _fade(const PackagesScreen()),
          ),
          GoRoute(
            path: '/providers',
            pageBuilder: (context, state) =>
                _fade(const ProvidersScreen()),
          ),
          GoRoute(
            path: '/vyuh-poc',
            pageBuilder: (context, state) => _fade(const VyuhPocPage()),
          ),
          GoRoute(
            path: '/settings',
            pageBuilder: (context, state) =>
                _fade(const SettingsScreen()),
          ),
          GoRoute(
            path: '/scenes',
            pageBuilder: (context, state) => _fade(const ScenesScreen()),
          ),
          GoRoute(
            path: '/skills',
            pageBuilder: (context, state) => _fade(const SkillsScreen()),
          ),
          GoRoute(
            path: '/mcp',
            pageBuilder: (context, state) => _fade(const McpScreen()),
          ),
        ],
      ),
    ],
  );
}
