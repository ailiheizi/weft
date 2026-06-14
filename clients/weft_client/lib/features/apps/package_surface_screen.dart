import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/models/app.dart';
import '../../core/providers/data_providers.dart';
import '../../shared/theme/spacing.dart';
import '../../shared/widgets/app_error_widget.dart';
import 'app_detail_screen.dart';
import 'surface/native_registry.dart';
import 'surface/surface_renderer.dart';

/// Unified host for a product package's surface.
///
/// When a native functional UI exists, shows a two-tab shell: 「功能 | 诊断」.
/// Diagnostics always remain reachable, reusing [AppDetailScreen].
class PackageSurfaceScreen extends ConsumerWidget {
  const PackageSurfaceScreen({super.key, required this.appName});

  final String appName;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final appAsync = ref.watch(appDetailProvider(appName));

    return appAsync.when(
      loading: () => const Scaffold(
        backgroundColor: Colors.transparent,
        body: Center(child: CircularProgressIndicator()),
      ),
      error: (e, _) => Scaffold(
        backgroundColor: Colors.transparent,
        body: AppErrorWidget(
          error: e,
          onRetry: () => ref.invalidate(appDetailProvider(appName)),
        ),
      ),
      data: (app) {
        if (!nativeSurfaceRegistry.containsKey(app.name)) {
          return AppDetailScreen(appName: appName);
        }
        return _SurfaceShell(app: app);
      },
    );
  }
}

class _SurfaceShell extends StatelessWidget {
  const _SurfaceShell({required this.app});

  final ResolvedApp app;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return DefaultTabController(
      length: 2,
      child: Scaffold(
        backgroundColor: Colors.transparent,
        appBar: AppBar(
          backgroundColor: Colors.transparent,
          titleSpacing: Spacing.lg,
          title: Text(
            app.displayName,
            style: theme.textTheme.titleMedium
                ?.copyWith(fontWeight: FontWeight.w600),
          ),
          bottom: const TabBar(
            tabs: [
              Tab(text: '功能'),
              Tab(text: '诊断'),
            ],
          ),
        ),
        body: TabBarView(
          children: [
            SurfaceRenderer(app: app),
            AppDetailScreen(appName: app.name),
          ],
        ),
      ),
    );
  }
}
