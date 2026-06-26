import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../shared/theme/spacing.dart';
import 'canvas_state.dart';
import 'models/canvas_models.dart';

/// 左栏 — 按 Shot（分镜）组织的资产库。
/// 每个 Shot 一组，组内列出节点缩略图。点击节点 → 选中画布上的它。
class ShotLibraryPanel extends ConsumerWidget {
  const ShotLibraryPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final state = ref.watch(canvasProvider);
    final notifier = ref.read(canvasProvider.notifier);

    // 散落节点（无 shot 归属）。
    final groupedIds = state.shots.expand((s) => s.nodeIds).toSet();
    final looseNodes =
        state.nodes.values.where((n) => !groupedIds.contains(n.id)).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(Spacing.md, Spacing.md, Spacing.md, Spacing.sm),
          child: Row(
            children: [
              Icon(Icons.movie_filter_outlined, size: 18, color: theme.colorScheme.primary),
              const SizedBox(width: Spacing.xs),
              Text('分镜资产', style: theme.textTheme.titleSmall),
              const Spacer(),
              IconButton(
                tooltip: '新建 Shot',
                visualDensity: VisualDensity.compact,
                icon: const Icon(Icons.add, size: 18),
                onPressed: () => notifier.addShot('Shot-${state.shots.length + 1}'),
              ),
            ],
          ),
        ),
        Expanded(
          child: (state.shots.isEmpty && looseNodes.isEmpty)
              ? _empty(theme)
              : ListView(
                  padding: const EdgeInsets.symmetric(horizontal: Spacing.sm),
                  children: [
                    for (final shot in state.shots)
                      _shotGroup(context, ref, theme, state, notifier, shot),
                    if (looseNodes.isNotEmpty)
                      _looseGroup(context, ref, theme, notifier, looseNodes),
                  ],
                ),
        ),
      ],
    );
  }

  Widget _empty(ThemeData theme) => Center(
        child: Padding(
          padding: const EdgeInsets.all(Spacing.lg),
          child: Text(
            '还没有素材。\n让导演生成，或导入媒体。',
            textAlign: TextAlign.center,
            style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant),
          ),
        ),
      );

  Widget _shotGroup(
    BuildContext context,
    WidgetRef ref,
    ThemeData theme,
    CanvasState state,
    CanvasNotifier notifier,
    Shot shot,
  ) {
    final nodes = shot.nodeIds.map((id) => state.nodes[id]).whereType<CanvasNode>().toList();
    return _Group(
      title: shot.title,
      count: nodes.length,
      child: Wrap(
        spacing: Spacing.xs,
        runSpacing: Spacing.xs,
        children: nodes
            .map((n) => _thumb(theme, n, selected: state.selectedNodeId == n.id, onTap: () => notifier.select(n.id)))
            .toList(),
      ),
    );
  }

  Widget _looseGroup(
    BuildContext context,
    WidgetRef ref,
    ThemeData theme,
    CanvasNotifier notifier,
    List<CanvasNode> nodes,
  ) {
    return _Group(
      title: '未分组',
      count: nodes.length,
      child: Wrap(
        spacing: Spacing.xs,
        runSpacing: Spacing.xs,
        children: nodes
            .map((n) => _thumb(theme, n, selected: false, onTap: () => notifier.select(n.id)))
            .toList(),
      ),
    );
  }

  Widget _thumb(ThemeData theme, CanvasNode node, {required bool selected, required VoidCallback onTap}) {
    final hasImg = node.kind == CanvasNodeKind.image &&
        node.thumbnailPath != null &&
        File(node.thumbnailPath!).existsSync();
    final icon = switch (node.kind) {
      CanvasNodeKind.image => Icons.image_outlined,
      CanvasNodeKind.video => Icons.movie_outlined,
      CanvasNodeKind.music => Icons.music_note_outlined,
      CanvasNodeKind.text => Icons.notes_outlined,
    };
    return GestureDetector(
      onTap: onTap,
      child: Container(
        width: 72,
        height: 72,
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.4),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: selected ? theme.colorScheme.primary : Colors.transparent,
            width: 2,
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: hasImg
            ? Image.file(File(node.thumbnailPath!), fit: BoxFit.cover)
            : Center(child: Icon(icon, size: 24, color: theme.colorScheme.onSurfaceVariant)),
      ),
    );
  }
}

class _Group extends StatelessWidget {
  const _Group({required this.title, required this.count, required this.child});
  final String title;
  final int count;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: Spacing.md),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: Spacing.xs, vertical: Spacing.xs),
            child: Row(
              children: [
                Text(title, style: theme.textTheme.labelMedium),
                const SizedBox(width: Spacing.xs),
                Text('$count', style: theme.textTheme.labelSmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
              ],
            ),
          ),
          child,
        ],
      ),
    );
  }
}
