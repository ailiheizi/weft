import 'dart:io';

import 'package:flutter/material.dart';

import '../../../shared/theme/app_theme.dart';
import 'models/canvas_models.dart';

/// 画布上的单个节点卡片。按 kind 渲染不同内容，带选中态与生成状态指示。
class CanvasNodeWidget extends StatelessWidget {
  const CanvasNodeWidget({
    super.key,
    required this.node,
    required this.selected,
    required this.onTap,
    required this.onPanUpdate,
  });

  final CanvasNode node;
  final bool selected;
  final VoidCallback onTap;
  final void Function(Offset delta) onPanUpdate;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final surfaces = theme.extension<AppSurfaces>();
    final accent = theme.colorScheme.primary;

    return GestureDetector(
      onTap: onTap,
      onPanUpdate: (d) => onPanUpdate(d.delta),
      child: Container(
        width: node.size.width,
        height: node.size.height,
        decoration: BoxDecoration(
          color: theme.colorScheme.surface,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(
            color: selected ? accent : theme.colorScheme.outline.withValues(alpha: 0.3),
            width: selected ? 2 : 1,
          ),
          boxShadow: selected
              ? [BoxShadow(color: accent.withValues(alpha: 0.25), blurRadius: 16, spreadRadius: 1)]
              : const [BoxShadow(color: Colors.black26, blurRadius: 8)],
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Expanded(child: _buildBody(context, theme, surfaces)),
            _buildFooter(context, theme),
          ],
        ),
      ),
    );
  }

  Widget _buildBody(BuildContext context, ThemeData theme, AppSurfaces? surfaces) {
    // 生成中 / 提议态覆盖层优先。
    if (node.status == NodeStatus.generating) {
      return _statusOverlay(theme, const _Spinner(), '生成中…');
    }
    if (node.status == NodeStatus.proposed) {
      return _statusOverlay(theme, Icon(Icons.bolt, color: theme.colorScheme.primary), '待确认');
    }
    if (node.status == NodeStatus.failed) {
      return _statusOverlay(
        theme,
        Icon(Icons.error_outline, color: surfaces?.statusError ?? Colors.red),
        node.errorMessage ?? '生成失败',
      );
    }

    // 有素材：图片直接显示；视频/音乐显示占位图标。
    if (node.kind == CanvasNodeKind.image &&
        node.assetPath != null &&
        File(node.assetPath!).existsSync()) {
      return Image.file(File(node.assetPath!), fit: BoxFit.cover);
    }

    return _placeholder(theme);
  }

  Widget _placeholder(ThemeData theme) {
    final (icon, label) = switch (node.kind) {
      CanvasNodeKind.image => (Icons.image_outlined, '图像'),
      CanvasNodeKind.video => (Icons.movie_outlined, '视频'),
      CanvasNodeKind.music => (Icons.music_note_outlined, '音乐'),
      CanvasNodeKind.text => (Icons.notes_outlined, '文本'),
    };
    return Container(
      color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.4),
      child: Center(
        child: Icon(icon, size: 40, color: theme.colorScheme.onSurfaceVariant),
      ),
    );
  }

  Widget _statusOverlay(ThemeData theme, Widget icon, String label) {
    return Container(
      color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            icon,
            const SizedBox(height: 6),
            Text(label, style: theme.textTheme.bodySmall),
          ],
        ),
      ),
    );
  }

  Widget _buildFooter(BuildContext context, ThemeData theme) {
    final (icon, _) = switch (node.kind) {
      CanvasNodeKind.image => (Icons.image_outlined, '图像'),
      CanvasNodeKind.video => (Icons.movie_outlined, '视频'),
      CanvasNodeKind.music => (Icons.music_note_outlined, '音乐'),
      CanvasNodeKind.text => (Icons.notes_outlined, '文本'),
    };
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      color: theme.colorScheme.surface,
      child: Row(
        children: [
          Icon(icon, size: 14, color: theme.colorScheme.onSurfaceVariant),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              node.title.isEmpty ? '未命名' : node.title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall,
            ),
          ),
        ],
      ),
    );
  }
}

class _Spinner extends StatelessWidget {
  const _Spinner();
  @override
  Widget build(BuildContext context) => const SizedBox(
        width: 24,
        height: 24,
        child: CircularProgressIndicator(strokeWidth: 2.5),
      );
}
