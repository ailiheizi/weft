import 'package:flutter/material.dart';

import '../../../core/models/app.dart';
import '../../../shared/theme/spacing.dart';
import '../../../shared/widgets/empty_state.dart';
import 'native_registry.dart';

/// Dispatches an app's functional UI by local native registry.
class SurfaceRenderer extends StatelessWidget {
  const SurfaceRenderer({super.key, required this.app});

  final ResolvedApp app;

  @override
  Widget build(BuildContext context) {
    final builder = nativeSurfaceRegistry[app.name];
    if (builder != null) {
      return builder(context, app);
    }
    return const _Notice(
      icon: Icons.apps_outlined,
      title: '无功能界面',
      subtitle: '当前应用仅提供诊断信息，未注册原生功能页面。',
    );
  }
}

class _Notice extends StatelessWidget {
  const _Notice({
    required this.icon,
    required this.title,
    required this.subtitle,
  });

  final IconData icon;
  final String title;
  final String subtitle;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(Spacing.lg),
      child: EmptyState(icon: icon, title: title, subtitle: subtitle),
    );
  }
}
