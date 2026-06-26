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
    return DragTarget<String>(
      onAcceptWithDetails: (details) => notifier.moveNodeToShot(details.data, shot.id),
      builder: (context, candidate, _) {
        final highlight = candidate.isNotEmpty;
        return Container(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            color: highlight
                ? theme.colorScheme.primary.withValues(alpha: 0.08)
                : Colors.transparent,
          ),
          child: _Group(
            title: shot.title,
            count: nodes.length,
            onRename: () => _renameShot(context, ref, shot),
            onDelete: () => notifier.removeShot(shot.id),
            child: Wrap(
              spacing: Spacing.xs,
              runSpacing: Spacing.xs,
              children: nodes
                  .map((n) => _thumb(theme, n, selected: state.selectedNodeId == n.id, onTap: () => notifier.select(n.id)))
                  .toList(),
            ),
          ),
        );
      },
    );
  }

  Future<void> _renameShot(BuildContext context, WidgetRef ref, Shot shot) async {
    final controller = TextEditingController(text: shot.title);
    final name = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('重命名分镜'),
        content: TextField(
          controller: controller,
          autofocus: true,
          onSubmitted: (v) => Navigator.pop(ctx, v),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, controller.text),
            child: const Text('确定'),
          ),
        ],
      ),
    );
    if (name != null && name.trim().isNotEmpty) {
      ref.read(canvasProvider.notifier).renameShot(shot.id, name.trim());
    }
  }

  Widget _looseGroup(
    BuildContext context,
    WidgetRef ref,
    ThemeData theme,
    CanvasNotifier notifier,
    List<CanvasNode> nodes,
  ) {
    return DragTarget<String>(
      onAcceptWithDetails: (details) => notifier.moveNodeToShot(details.data, null),
      builder: (context, candidate, _) {
        final highlight = candidate.isNotEmpty;
        return Container(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            color: highlight
                ? theme.colorScheme.primary.withValues(alpha: 0.08)
                : Colors.transparent,
          ),
          child: _Group(
            title: '未分组',
            count: nodes.length,
            child: Wrap(
              spacing: Spacing.xs,
              runSpacing: Spacing.xs,
              children: nodes
                  .map((n) => _thumb(theme, n, selected: false, onTap: () => notifier.select(n.id)))
                  .toList(),
            ),
          ),
        );
      },
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
    final tile = GestureDetector(
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
    // 可拖拽到别的 Shot 分组归类。
    return Draggable<String>(
      data: node.id,
      feedback: Opacity(opacity: 0.8, child: tile),
      childWhenDragging: Opacity(opacity: 0.3, child: tile),
      child: tile,
    );
  }
}

class _Group extends StatelessWidget {
  const _Group({
    required this.title,
    required this.count,
    required this.child,
    this.onRename,
    this.onDelete,
  });
  final String title;
  final int count;
  final Widget child;
  final VoidCallback? onRename;
  final VoidCallback? onDelete;

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
                if (onRename != null || onDelete != null) ...[
                  const Spacer(),
                  PopupMenuButton<String>(
                    icon: Icon(Icons.more_horiz, size: 16, color: theme.colorScheme.onSurfaceVariant),
                    padding: EdgeInsets.zero,
                    iconSize: 16,
                    onSelected: (v) {
                      if (v == 'rename') onRename?.call();
                      if (v == 'delete') onDelete?.call();
                    },
                    itemBuilder: (_) => [
                      if (onRename != null)
                        const PopupMenuItem(value: 'rename', child: Text('重命名')),
                      if (onDelete != null)
                        const PopupMenuItem(value: 'delete', child: Text('删除分镜')),
                    ],
                  ),
                ],
              ],
            ),
          ),
          child,
        ],
      ),
    );
  }
}
