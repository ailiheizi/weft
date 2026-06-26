import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/api/mcp_api.dart' show McpServer;
import '../../../core/api/scene_api.dart' show Scene;
import '../../../core/api/weft_claw_api.dart' show SelectorApi, McpPreset;
import '../../../core/providers/scenes_provider.dart';
import '../../../core/providers/skills_provider.dart';
import '../../../core/providers/mcp_provider.dart';

/// 聊天工作区内的「场景」管理面板。列出/激活/删除 Scene。
class ScenesPanel extends ConsumerWidget {
  const ScenesPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final state = ref.watch(scenesProvider);
    final notifier = ref.read(scenesProvider.notifier);

    return _PanelScaffold(
      title: '场景偏好',
      subtitle: '一套场景 = 启用插件 / provider 替换 / 模型路由 / 工作区',
      loading: state.loading,
      error: state.error,
      onRefresh: () => notifier.load(),
      empty: state.scenes.isEmpty,
      emptyHint: '暂无场景。场景定义一套偏好,激活后切换团队模型路由等。',
      children: [
        for (final scene in state.scenes)
          _Row(
            leading: Icon(
              scene.name == state.activeScene
                  ? Icons.check_circle
                  : Icons.circle_outlined,
              size: 18,
              color: scene.name == state.activeScene
                  ? theme.colorScheme.primary
                  : theme.colorScheme.onSurfaceVariant,
            ),
            title: scene.name,
            active: scene.name == state.activeScene,
            subtitle: _sceneSummary(scene),
            trailing: Row(mainAxisSize: MainAxisSize.min, children: [
              if (scene.name != state.activeScene)
                TextButton(
                  onPressed: state.loading ? null : () => notifier.bind(scene.name),
                  child: const Text('激活'),
                ),
              IconButton(
                icon: const Icon(Icons.delete_outline, size: 18),
                tooltip: scene.name == state.activeScene ? '不能删除激活场景' : '删除',
                onPressed: (state.loading || scene.name == state.activeScene)
                    ? null
                    : () => notifier.delete(scene.name),
              ),
            ]),
          ),
      ],
    );
  }

  static String _sceneSummary(Scene scene) {
    final parts = <String>[];
    if (scene.profile.isNotEmpty) parts.add('profile=${scene.profile}');
    if (scene.roleRouting.isNotEmpty) {
      parts.add('${scene.roleRouting.length} 角色路由');
    }
    if (scene.enabledFeatures.isNotEmpty) {
      parts.add('${scene.enabledFeatures.length} 启用');
    }
    if (scene.workspace.isNotEmpty) parts.add('工作区:${scene.workspace}');
    return parts.isEmpty
        ? (scene.description.isEmpty ? '(无额外偏好)' : scene.description)
        : parts.join(' · ');
  }
}

/// 聊天工作区内的「演化技能」管理面板。
class SkillsPanel extends ConsumerWidget {
  const SkillsPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final state = ref.watch(skillsProvider);
    final notifier = ref.read(skillsProvider.notifier);

    return _PanelScaffold(
      title: '演化技能',
      subtitle: 'agent 从成功任务轨迹自动结晶的可复用流程',
      loading: state.loading,
      error: state.error,
      onRefresh: () => notifier.load(),
      empty: state.skills.isEmpty,
      emptyHint: '暂无技能。agent 积累成功经验后会在这里出现,可审批/查看。',
      children: [
        for (final skill in state.skills)
          _Row(
            leading: Icon(_skillIcon(skill.status),
                size: 18, color: _skillColor(skill.status, theme)),
            title: skill.title.isEmpty ? skill.id : skill.title,
            badges: [
              skill.status,
              if (skill.riskLevel.isNotEmpty && skill.riskLevel != 'low')
                'risk:${skill.riskLevel}',
            ],
            subtitle:
                '质量 ${(skill.qualityScore * 100).toStringAsFixed(0)}% · 用 ${skill.successfulUses}✓/${skill.failedUses}✗'
                '${skill.triggers.isNotEmpty ? ' · ${skill.triggers.length} 触发词' : ''}',
            trailing: (skill.reviewRequired || skill.status == 'pending_review')
                ? Row(mainAxisSize: MainAxisSize.min, children: [
                    TextButton(
                      onPressed: state.loading
                          ? null
                          : () => notifier.review(skill.id, true),
                      child: const Text('通过'),
                    ),
                    TextButton(
                      onPressed: state.loading
                          ? null
                          : () => notifier.review(skill.id, false),
                      style: TextButton.styleFrom(
                          foregroundColor: theme.colorScheme.error),
                      child: const Text('拒绝'),
                    ),
                  ])
                : null,
          ),
      ],
    );
  }

  static IconData _skillIcon(String s) => switch (s) {
        'active' => Icons.bolt,
        'pending_review' => Icons.rate_review_outlined,
        'archived' => Icons.archive_outlined,
        _ => Icons.auto_awesome_outlined,
      };
  static Color? _skillColor(String s, ThemeData t) => switch (s) {
        'active' => t.colorScheme.primary,
        'pending_review' => t.colorScheme.tertiary,
        'archived' => t.colorScheme.onSurfaceVariant,
        _ => null,
      };
}

/// 聊天工作区内的「MCP server」管理面板。
class McpPanel extends ConsumerWidget {
  const McpPanel({super.key});

  Future<void> _showAddDialog(BuildContext context, WidgetRef ref) async {
    final nameCtrl = TextEditingController();
    final cmdCtrl = TextEditingController();
    final argsCtrl = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('添加 MCP Server'),
        content: Column(mainAxisSize: MainAxisSize.min, children: [
          TextField(
              controller: nameCtrl,
              decoration: const InputDecoration(
                  labelText: '名称', hintText: 'e.g. filesystem')),
          TextField(
              controller: cmdCtrl,
              decoration:
                  const InputDecoration(labelText: '命令', hintText: 'e.g. npx')),
          TextField(
              controller: argsCtrl,
              decoration: const InputDecoration(
                  labelText: '参数(空格分隔)',
                  hintText: '-y @modelcontextprotocol/server-filesystem /path')),
        ]),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('取消')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('添加')),
        ],
      ),
    );
    if (ok == true && nameCtrl.text.trim().isNotEmpty) {
      final args = argsCtrl.text
          .trim()
          .split(RegExp(r'\s+'))
          .where((s) => s.isNotEmpty)
          .toList();
      await ref.read(mcpProvider.notifier).add(McpServer(
            name: nameCtrl.text.trim(),
            command: cmdCtrl.text.trim(),
            args: args,
          ));
    }
  }

  Future<void> _showPresetDialog(BuildContext context, WidgetRef ref) async {
    final presets = await SelectorApi().listPresets();
    if (!context.mounted) return;
    if (presets.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('无法获取预置清单(selector 服务未运行?)')),
      );
      return;
    }
    final picked = await showDialog<McpPreset>(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text('从预置添加 MCP Server'),
        children: [
          SizedBox(
            width: 420,
            height: 420,
            child: ListView.separated(
              shrinkWrap: true,
              itemCount: presets.length,
              separatorBuilder: (_, _) => const Divider(height: 1),
              itemBuilder: (context, i) {
                final p = presets[i];
                return ListTile(
                  dense: true,
                  title: Row(
                    children: [
                      Flexible(child: Text(p.name)),
                      const SizedBox(width: 6),
                      _PresetBadge(label: p.category),
                      if (p.needsKey) ...[
                        const SizedBox(width: 4),
                        const _PresetBadge(label: '需Key', warn: true),
                      ],
                    ],
                  ),
                  subtitle: Text(
                    '${p.description}\n${p.command} ${p.args.join(' ')}',
                    style: const TextStyle(fontSize: 11),
                  ),
                  isThreeLine: true,
                  onTap: () => Navigator.pop(ctx, p),
                );
              },
            ),
          ),
        ],
      ),
    );
    if (picked == null || !context.mounted) return;

    // Resolve ${WORKSPACE} placeholder to current dir marker; user can edit later.
    final resolvedArgs = picked.args
        .map((a) => a.replaceAll(r'${WORKSPACE}', '.'))
        .toList();

    await ref.read(mcpProvider.notifier).add(McpServer(
          name: picked.id,
          command: picked.command,
          args: resolvedArgs,
          transport: picked.transport,
        ));
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(picked.needsKey
              ? '已添加 ${picked.name}。需在 env 配置 ${picked.keyEnv} 后才能启动'
              : '已添加 ${picked.name}，点启动运行'),
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final state = ref.watch(mcpProvider);
    final notifier = ref.read(mcpProvider.notifier);

    return _PanelScaffold(
      title: 'MCP Servers',
      subtitle: '添加后 agent 可通过 MCP 协议调用外部工具',
      loading: state.loading,
      error: state.error,
      onRefresh: () => notifier.load(),
      onAdd: () => _showAddDialog(context, ref),
      onPreset: () => _showPresetDialog(context, ref),
      empty: state.servers.isEmpty,
      emptyHint: '暂无 MCP server。点右上 + 添加。',
      children: [
        for (final s in state.servers)
          _Row(
            leading: Icon(
              (s.status == 'running' || s.status == 'active')
                  ? Icons.cloud_done
                  : Icons.cloud_outlined,
              size: 18,
              color: (s.status == 'running' || s.status == 'active')
                  ? theme.colorScheme.primary
                  : null,
            ),
            title: s.name,
            badges: [s.transport],
            subtitle:
                '${s.command}${s.args.isNotEmpty ? ' ${s.args.join(' ')}' : ''} · ${s.status}',
            trailing: Row(mainAxisSize: MainAxisSize.min, children: [
              TextButton(
                onPressed: state.loading
                    ? null
                    : () => (s.status == 'running' || s.status == 'active')
                        ? notifier.stop(s.name)
                        : notifier.start(s.name),
                child: Text((s.status == 'running' || s.status == 'active')
                    ? '停止'
                    : '启动'),
              ),
              IconButton(
                icon: const Icon(Icons.delete_outline, size: 18),
                tooltip: '删除',
                onPressed: state.loading ? null : () => notifier.remove(s.name),
              ),
            ]),
          ),
      ],
    );
  }
}

// ── 共用脚手架 ────────────────────────────────────────────────────────────────

/// 预置项的小徽章(分类/需Key 标记)。
class _PresetBadge extends StatelessWidget {
  const _PresetBadge({required this.label, this.warn = false});
  final String label;
  final bool warn;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = warn ? Colors.orange : theme.colorScheme.primary;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(label,
          style: TextStyle(
              fontSize: 9, color: color, fontWeight: FontWeight.w600)),
    );
  }
}

class _PanelScaffold extends StatelessWidget {
  const _PanelScaffold({
    required this.title,
    required this.subtitle,
    required this.loading,
    required this.error,
    required this.onRefresh,
    required this.empty,
    required this.emptyHint,
    required this.children,
    this.onAdd,
    this.onPreset,
  });

  final String title;
  final String subtitle;
  final bool loading;
  final String? error;
  final VoidCallback onRefresh;
  final VoidCallback? onAdd;
  final VoidCallback? onPreset;
  final bool empty;
  final String emptyHint;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 8, 8),
          child: Row(children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: theme.textTheme.titleSmall),
                  const SizedBox(height: 2),
                  Text(subtitle,
                      style: theme.textTheme.bodySmall
                          ?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                ],
              ),
            ),
            if (onPreset != null)
              IconButton(
                icon: const Icon(Icons.auto_awesome_motion_outlined, size: 18),
                tooltip: '从预置添加',
                onPressed: loading ? null : onPreset,
              ),
            if (onAdd != null)
              IconButton(
                icon: const Icon(Icons.add, size: 18),
                tooltip: '添加',
                onPressed: loading ? null : onAdd,
              ),
            loading
                ? const Padding(
                    padding: EdgeInsets.all(8),
                    child: SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2)),
                  )
                : IconButton(
                    icon: const Icon(Icons.refresh, size: 18),
                    tooltip: '刷新',
                    onPressed: onRefresh,
                  ),
          ]),
        ),
        if (error != null && error!.trim().isNotEmpty)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            color: theme.colorScheme.errorContainer,
            child: Text(error!,
                style: theme.textTheme.bodySmall
                    ?.copyWith(color: theme.colorScheme.onErrorContainer)),
          ),
        const Divider(height: 1),
        Expanded(
          child: (empty && !loading)
              ? Center(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Text(emptyHint,
                        textAlign: TextAlign.center,
                        style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant)),
                  ),
                )
              : ListView.separated(
                  itemCount: children.length,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (_, i) => children[i],
                ),
        ),
      ],
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({
    required this.leading,
    required this.title,
    required this.subtitle,
    this.trailing,
    this.badges = const [],
    this.active = false,
  });

  final Widget leading;
  final String title;
  final String subtitle;
  final Widget? trailing;
  final List<String> badges;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListTile(
      dense: true,
      leading: leading,
      title: Row(children: [
        Flexible(
          child: Text(title,
              style: TextStyle(
                  fontSize: 14,
                  fontWeight: active ? FontWeight.w600 : FontWeight.normal),
              overflow: TextOverflow.ellipsis),
        ),
        for (final b in badges) ...[
          const SizedBox(width: 6),
          _Badge(text: b, theme: theme),
        ],
      ]),
      subtitle: Text(subtitle,
          style: const TextStyle(fontSize: 12),
          maxLines: 1,
          overflow: TextOverflow.ellipsis),
      trailing: trailing,
    );
  }
}

class _Badge extends StatelessWidget {
  const _Badge({required this.text, required this.theme});
  final String text;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(text,
          style: theme.textTheme.labelSmall
              ?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
    );
  }
}
