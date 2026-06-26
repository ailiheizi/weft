import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../shared/theme/spacing.dart';
import 'canvas_state.dart';
import 'models/canvas_models.dart';
import 'video_player_dialog.dart';

/// 选中节点的就地生成参数面板（浮在画布右上角）。
/// 提供 prompt、比例选择，以及「确认生成 / 重新生成」按钮，驱动生成闭环。
class NodeParamPanel extends ConsumerStatefulWidget {
  const NodeParamPanel({super.key, required this.node});

  final CanvasNode node;

  @override
  ConsumerState<NodeParamPanel> createState() => _NodeParamPanelState();
}

class _NodeParamPanelState extends ConsumerState<NodeParamPanel> {
  late final TextEditingController _prompt =
      TextEditingController(text: widget.node.prompt ?? '');
  late String _ratio = widget.node.params.aspectRatio;

  static const _ratios = ['1:1', '16:9', '9:16'];

  @override
  void didUpdateWidget(NodeParamPanel old) {
    super.didUpdateWidget(old);
    if (old.node.id != widget.node.id) {
      _prompt.text = widget.node.prompt ?? '';
      _ratio = widget.node.params.aspectRatio;
    }
  }

  @override
  void dispose() {
    _prompt.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final node = widget.node;
    final notifier = ref.read(canvasProvider.notifier);
    final busy = node.status == NodeStatus.generating;

    // 视频节点 ready：显示播放入口，不显示图像生成 UI。
    if (node.kind == CanvasNodeKind.video && node.status == NodeStatus.ready) {
      return _videoPanel(theme, node, notifier);
    }

    return Container(
      width: 280,
      padding: const EdgeInsets.all(Spacing.md),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: theme.colorScheme.outline.withValues(alpha: 0.2)),
        boxShadow: const [BoxShadow(color: Colors.black38, blurRadius: 16)],
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Icon(Icons.tune, size: 16, color: theme.colorScheme.primary),
              const SizedBox(width: Spacing.xs),
              Text('生成设置', style: theme.textTheme.titleSmall),
              const Spacer(),
              if (notifier.hasEdges(node.id))
                IconButton(
                  tooltip: '断开连线',
                  visualDensity: VisualDensity.compact,
                  icon: const Icon(Icons.link_off, size: 16),
                  onPressed: () => notifier.removeEdgesForNode(node.id),
                ),
              IconButton(
                visualDensity: VisualDensity.compact,
                icon: const Icon(Icons.close, size: 16),
                onPressed: () => notifier.select(null),
              ),
            ],
          ),
          const SizedBox(height: Spacing.sm),
          TextField(
            controller: _prompt,
            minLines: 2,
            maxLines: 4,
            enabled: !busy,
            style: theme.textTheme.bodySmall,
            decoration: InputDecoration(
              labelText: '提示词',
              isDense: true,
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
            ),
          ),
          const SizedBox(height: Spacing.sm),
          Row(
            children: [
              Text('比例', style: theme.textTheme.bodySmall),
              const SizedBox(width: Spacing.sm),
              ..._ratios.map((r) => Padding(
                    padding: const EdgeInsets.only(right: Spacing.xs),
                    child: ChoiceChip(
                      label: Text(r, style: theme.textTheme.bodySmall),
                      selected: _ratio == r,
                      onSelected: busy ? null : (_) => setState(() => _ratio = r),
                    ),
                  )),
            ],
          ),
          const SizedBox(height: Spacing.md),
          if (node.status == NodeStatus.failed && node.errorMessage != null) ...[
            Text(
              node.errorMessage!,
              style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.error),
            ),
            const SizedBox(height: Spacing.sm),
          ],
          FilledButton.icon(
            onPressed: busy ? null : () => _generate(notifier),
            icon: busy
                ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 2))
                : Icon(node.status == NodeStatus.proposed ? Icons.check : Icons.refresh, size: 16),
            label: Text(_buttonLabel(node.status)),
          ),
        ],
      ),
    );
  }

  String _buttonLabel(NodeStatus status) {
    return switch (status) {
      NodeStatus.proposed => '确认生成',
      NodeStatus.generating => '生成中…',
      NodeStatus.failed => '重试',
      _ => '重新生成',
    };
  }

  void _generate(CanvasNotifier notifier) {
    // 回写最新 prompt / 比例后触发生成。
    notifier.updateNode(widget.node.id, (n) => n.copyWith(
          prompt: _prompt.text.trim(),
          params: n.params.copyWith(aspectRatio: _ratio),
          status: NodeStatus.proposed,
        ));
    notifier.confirmAndGenerateImage(widget.node.id);
  }

  /// 视频节点面板：播放 + 重新合成。
  Widget _videoPanel(ThemeData theme, CanvasNode node, CanvasNotifier notifier) {
    return Container(
      width: 280,
      padding: const EdgeInsets.all(Spacing.md),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: theme.colorScheme.outline.withValues(alpha: 0.2)),
        boxShadow: const [BoxShadow(color: Colors.black38, blurRadius: 16)],
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Icon(Icons.movie_outlined, size: 16, color: theme.colorScheme.primary),
              const SizedBox(width: Spacing.xs),
              Text('视频片段', style: theme.textTheme.titleSmall),
              const Spacer(),
              IconButton(
                visualDensity: VisualDensity.compact,
                icon: const Icon(Icons.close, size: 16),
                onPressed: () => notifier.select(null),
              ),
            ],
          ),
          const SizedBox(height: Spacing.sm),
          Text(
            node.assetPath ?? '',
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant),
          ),
          const SizedBox(height: Spacing.md),
          FilledButton.icon(
            onPressed: node.assetPath == null
                ? null
                : () => VideoPlayerDialog.show(context, node.assetPath!),
            icon: const Icon(Icons.play_arrow, size: 18),
            label: const Text('播放'),
          ),
          const SizedBox(height: Spacing.sm),
          OutlinedButton.icon(
            onPressed: () {
              notifier.setNodeStatus(node.id, NodeStatus.proposed);
              notifier.confirmAndGenerateVideo(node.id);
            },
            icon: const Icon(Icons.refresh, size: 16),
            label: const Text('重新合成'),
          ),
          const SizedBox(height: Spacing.sm),
          OutlinedButton.icon(
            onPressed: node.assetPath == null
                ? null
                : () => _exportVideo(context, node.assetPath!),
            icon: const Icon(Icons.download_outlined, size: 16),
            label: const Text('导出成片'),
          ),
        ],
      ),
    );
  }

  /// 导出视频成片到用户选择的目录。
  Future<void> _exportVideo(BuildContext context, String srcPath) async {
    final src = File(srcPath);
    if (!src.existsSync()) {
      if (context.mounted) {
        ScaffoldMessenger.maybeOf(context)?.showSnackBar(
          const SnackBar(content: Text('源视频文件不存在')),
        );
      }
      return;
    }
    final dir = await FilePicker.getDirectoryPath(dialogTitle: '选择导出目录');
    if (dir == null) return;
    try {
      final fileName = srcPath.split(RegExp(r'[\\/]')).last;
      final dest = '$dir${Platform.pathSeparator}$fileName';
      await src.copy(dest);
      if (context.mounted) {
        ScaffoldMessenger.maybeOf(context)?.showSnackBar(
          SnackBar(content: Text('已导出到 $dest')),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.maybeOf(context)?.showSnackBar(
          const SnackBar(content: Text('导出失败')),
        );
      }
    }
  }
}
