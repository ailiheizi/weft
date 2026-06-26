import 'package:flutter/material.dart';

import '../chat/workspace/management_tabs.dart';

/// 场景管理页(侧栏导航进入)。
class ScenesScreen extends StatelessWidget {
  const ScenesScreen({super.key});
  @override
  Widget build(BuildContext context) => const _Page(child: ScenesPanel());
}

/// 技能管理页。
class SkillsScreen extends StatelessWidget {
  const SkillsScreen({super.key});
  @override
  Widget build(BuildContext context) => const _Page(child: SkillsPanel());
}

/// MCP server 管理页。
class McpScreen extends StatelessWidget {
  const McpScreen({super.key});
  @override
  Widget build(BuildContext context) => const _Page(child: McpPanel());
}

/// 管理页统一容器:居中限宽,留白,圆角卡片包裹面板。
class _Page extends StatelessWidget {
  const _Page({required this.child});
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 820),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(16),
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: theme.colorScheme.surface,
                borderRadius: BorderRadius.circular(16),
                border: Border.all(
                    color: theme.colorScheme.outlineVariant
                        .withValues(alpha: 0.5)),
              ),
              child: child,
            ),
          ),
        ),
      ),
    );
  }
}
