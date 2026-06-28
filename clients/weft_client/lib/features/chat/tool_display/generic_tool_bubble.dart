import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/models/chat.dart';
import 'bubbles/_bubble_utils.dart';
import 'kv_table.dart';
import 'tool_bubble_chrome.dart';
import '../workspace/artifact.dart';

/// 通用工具气泡：未命中专属气泡的工具（含绝大多数 MCP 工具）的统一兜底展示。
///
/// 套 [ToolBubbleChrome] 外壳（状态点 + 友好名 + 折叠 body），与专属气泡视觉一致。
/// body：
///   · 入参段：能 JSON 解析为 Map → [KvTable] 键值表；否则 [RawOutput] raw 文本。
///   · 结果段：能 JSON 解析为 Map → [KvTable]；否则 [RawOutput]（带折叠+滚动）。
/// 若该工具可归一化为产物（[Artifact]），表头右侧给一个"查看"链接（沿用旧兜底行为）。
class GenericToolBubble extends ConsumerWidget {
  const GenericToolBubble({super.key, required this.step});

  final ToolCallStep step;

  /// 注册表/调用方用的构建器。
  static Widget create(ToolCallStep step) => GenericToolBubble(step: step);

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final artifact = Artifact.fromToolCall(step);
    return ToolBubbleChrome(
      step: step,
      trailing: artifact != null ? _viewButton(context, ref, artifact) : null,
      child: _body(context),
    );
  }

  Widget _body(BuildContext context) {
    final theme = Theme.of(context);
    final status = toolStatusOf(step);
    final argsMap = tryJsonMap(step.arguments);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // ── 入参段 ──
        if (step.arguments.trim().isNotEmpty) ...[
          _sectionLabel(theme, '参数'),
          const SizedBox(height: 4),
          if (argsMap != null)
            KvTable(data: argsMap)
          else
            RawOutput(text: step.arguments, maxHeight: 160),
        ],
        // ── 运行中 ──
        if (status == ToolStatus.pending) ...[
          if (step.arguments.trim().isNotEmpty) const SizedBox(height: 8),
          _runningRow(theme),
        ],
        // ── 结果段 ──
        if (step.result != null && step.result!.trim().isNotEmpty) ...[
          if (step.arguments.trim().isNotEmpty) const SizedBox(height: 8),
          _sectionLabel(theme, '结果'),
          const SizedBox(height: 4),
          _result(context, step.result!),
        ],
      ],
    );
  }

  /// 结果渲染：能解析为 JSON Map → KvTable；否则 RawOutput。
  Widget _result(BuildContext context, String raw) {
    final m = tryJsonMap(raw);
    if (m != null) return KvTable(data: m);
    return RawOutput(text: raw);
  }

  Widget _sectionLabel(ThemeData theme, String text) {
    return Text(
      text,
      style: TextStyle(
        fontSize: 10,
        fontWeight: FontWeight.w700,
        letterSpacing: 0.4,
        color: theme.colorScheme.onSurfaceVariant,
      ),
    );
  }

  Widget _runningRow(ThemeData theme) {
    return Row(
      children: [
        SizedBox(
          width: 12,
          height: 12,
          child: CircularProgressIndicator(
              strokeWidth: 1.5, color: theme.colorScheme.secondary),
        ),
        const SizedBox(width: 8),
        Text('运行中…',
            style: TextStyle(
                fontSize: 12, color: theme.colorScheme.onSurfaceVariant)),
      ],
    );
  }

  /// 表头右侧"查看"链接：点开右侧工作区预览该产物（沿用旧通用兜底行为）。
  Widget _viewButton(BuildContext context, WidgetRef ref, Artifact artifact) {
    final theme = Theme.of(context);
    return InkWell(
      borderRadius: BorderRadius.circular(8),
      onTap: () => ref.read(workspaceProvider.notifier).show(artifact),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(_artifactIcon(artifact.kind),
                size: 11, color: theme.colorScheme.primary),
            const SizedBox(width: 3),
            Text('查看',
                style: TextStyle(
                    fontSize: 10, color: theme.colorScheme.primary)),
          ],
        ),
      ),
    );
  }

  static IconData _artifactIcon(ArtifactKind kind) {
    switch (kind) {
      case ArtifactKind.file:
        return Icons.insert_drive_file_outlined;
      case ArtifactKind.terminal:
        return Icons.terminal_outlined;
      case ArtifactKind.web:
        return Icons.language_outlined;
      case ArtifactKind.step:
        return Icons.timeline_outlined;
      case ArtifactKind.orchestration:
        return Icons.account_tree_outlined;
    }
  }
}
