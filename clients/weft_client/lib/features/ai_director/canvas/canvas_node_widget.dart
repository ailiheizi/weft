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
    this.onPanStart,
    this.onPanEnd,
    this.onSecondaryTap,
    this.onLinkStart,
    this.onLinkUpdate,
    this.onLinkEnd,
    this.onResizeStart,
    this.onResizeUpdate,
    this.onToolbarAction,
  });

  final CanvasNode node;
  final bool selected;
  final VoidCallback onTap;
  final void Function(Offset delta) onPanUpdate;
  final VoidCallback? onPanStart;
  final VoidCallback? onPanEnd;
  final VoidCallback? onSecondaryTap;

  /// 连线手柄拖拽：开始（出点全局坐标）、更新（全局坐标）、结束。
  final void Function(Offset globalPos)? onLinkStart;
  final void Function(Offset globalPos)? onLinkUpdate;
  final VoidCallback? onLinkEnd;

  /// 缩放手柄拖拽：开始、更新（屏幕像素 delta，调用方按缩放比换算）。
  final VoidCallback? onResizeStart;
  final void Function(Offset delta)? onResizeUpdate;

  /// 选中时浮出工具条的动作回调。action 取值见 _toolbarActions。
  final void Function(String action)? onToolbarAction;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final surfaces = theme.extension<AppSurfaces>();
    final accent = theme.colorScheme.primary;

    return SizedBox(
      width: node.size.width + 16,
      height: node.size.height,
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          GestureDetector(
            onTap: onTap,
            onSecondaryTap: onSecondaryTap,
            onPanStart: (_) => onPanStart?.call(),
            onPanUpdate: (d) => onPanUpdate(d.delta),
            onPanEnd: (_) => onPanEnd?.call(),
            child: _card(context, theme, surfaces, accent),
          ),
          // 右侧连线出点手柄
          Positioned(
            right: 0,
            top: node.size.height / 2 - 8,
            child: GestureDetector(
              onPanStart: (d) => onLinkStart?.call(d.globalPosition),
              onPanUpdate: (d) => onLinkUpdate?.call(d.globalPosition),
              onPanEnd: (_) => onLinkEnd?.call(),
              child: Container(
                width: 16,
                height: 16,
                decoration: BoxDecoration(
                  color: accent,
                  shape: BoxShape.circle,
                  border: Border.all(color: theme.colorScheme.surface, width: 2),
                ),
              ),
            ),
          ),
          // 右下角缩放手柄（选中时显示）
          if (selected)
            Positioned(
              right: 16,
              bottom: 0,
              child: MouseRegion(
                cursor: SystemMouseCursors.resizeDownRight,
                child: GestureDetector(
                  onPanStart: (_) => onResizeStart?.call(),
                  onPanUpdate: (d) => onResizeUpdate?.call(d.delta),
                  child: Container(
                    width: 16,
                    height: 16,
                    decoration: BoxDecoration(
                      color: theme.colorScheme.surface,
                      border: Border.all(color: accent, width: 2),
                      borderRadius: const BorderRadius.only(
                        bottomRight: Radius.circular(4),
                        topLeft: Radius.circular(4),
                      ),
                    ),
                    child: Icon(Icons.open_in_full, size: 9, color: accent),
                  ),
                ),
              ),
            ),
          // 缩放手柄已在上方处理。工具条改为卡片内顶部独立一行（见 _card），
          // 不再用覆盖式 Positioned——既避免盖住内容，又天然在 hitTest 范围内。
        ],
      ),
    );
  }

  Widget _card(BuildContext context, ThemeData theme, AppSurfaces? surfaces, Color accent) {
    // 状态色：generating 蓝 / ready 绿 / failed 红 / 其他默认描边。
    // 选中时一律用 accent 突出。
    final statusColor = switch (node.status) {
      NodeStatus.generating => const Color(0xFF4A9EFF),
      NodeStatus.ready => surfaces?.statusOk ?? const Color(0xFF4CB782),
      NodeStatus.failed => surfaces?.statusError ?? const Color(0xFFEB5757),
      _ => null,
    };
    final borderColor = selected
        ? accent
        : (statusColor ?? theme.colorScheme.outline.withValues(alpha: 0.3));
    final borderWidth = selected ? 2.0 : (statusColor != null ? 1.5 : 1.0);
    final glowColor = selected ? accent : statusColor;
    return Container(
        width: node.size.width,
        height: node.size.height,
        decoration: BoxDecoration(
          color: theme.colorScheme.surface,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: borderColor, width: borderWidth),
          boxShadow: glowColor != null
              ? [BoxShadow(color: glowColor.withValues(alpha: 0.25), blurRadius: 16, spreadRadius: 1)]
              : const [BoxShadow(color: Colors.black26, blurRadius: 8)],
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // 选中时顶部浮出工具条（独立一行，不覆盖内容）。
            if (selected && onToolbarAction != null)
              _Toolbar(node: node, theme: theme, onAction: onToolbarAction!),
            Expanded(child: _buildBody(context, theme, surfaces)),
            _buildFooter(context, theme),
          ],
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

    // 视频节点 ready：显示首帧缩略 + 播放角标。
    if (node.kind == CanvasNodeKind.video &&
        node.status == NodeStatus.ready &&
        node.thumbnailPath != null &&
        File(node.thumbnailPath!).existsSync()) {
      return Stack(
        fit: StackFit.expand,
        children: [
          Image.file(File(node.thumbnailPath!), fit: BoxFit.cover),
          Container(color: Colors.black.withValues(alpha: 0.25)),
          const Center(
            child: Icon(Icons.play_circle_fill, size: 40, color: Colors.white70),
          ),
        ],
      );
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

/// 节点选中时浮在上方的上下文工具条。按节点类型/状态显示不同动作。
class _Toolbar extends StatelessWidget {
  const _Toolbar({required this.node, required this.theme, required this.onAction});

  final CanvasNode node;
  final ThemeData theme;
  final void Function(String action) onAction;

  @override
  Widget build(BuildContext context) {
    // 动作集按节点类型/状态计算。
    final items = <(String, IconData, String)>[];
    if (node.kind == CanvasNodeKind.image) {
      if (node.status == NodeStatus.ready) {
        items.add(('regen', Icons.refresh, '重新生成'));
        items.add(('variant', Icons.auto_awesome, '变体'));
      } else {
        items.add(('regen', Icons.play_arrow, '生成'));
      }
    } else if (node.kind == CanvasNodeKind.video) {
      if (node.status == NodeStatus.ready) {
        items.add(('play', Icons.play_circle_outline, '播放'));
        items.add(('regen', Icons.refresh, '重新合成'));
        items.add(('export', Icons.download_outlined, '导出'));
      } else {
        items.add(('regen', Icons.movie_creation_outlined, '合成'));
      }
    }
    items.add(('duplicate', Icons.copy_outlined, '复制'));
    items.add(('delete', Icons.delete_outline, '删除'));

    // 卡片内顶部工具条：贴合卡片顶部的一条，accent 淡背景，不覆盖下方内容。
    return Container(
      height: 34,
      decoration: BoxDecoration(
        color: theme.colorScheme.primary.withValues(alpha: 0.10),
        border: Border(
          bottom: BorderSide(color: theme.colorScheme.primary.withValues(alpha: 0.25)),
        ),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        mainAxisSize: MainAxisSize.max,
        children: items
            .map((it) => IconButton(
                  tooltip: it.$3,
                  visualDensity: VisualDensity.compact,
                  iconSize: 16,
                  constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
                  padding: EdgeInsets.zero,
                  icon: Icon(it.$2,
                      color: it.$1 == 'delete'
                          ? theme.colorScheme.error
                          : theme.colorScheme.primary),
                  onPressed: () => onAction(it.$1),
                ))
            .toList(),
      ),
    );
  }
}
