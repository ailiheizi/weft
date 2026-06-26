import 'package:flutter/material.dart';

import '../../../../core/models/chat.dart';
import '../tool_bubble_chrome.dart';
import '_bubble_utils.dart';

/// 终端/命令类工具气泡（shell_exec / run_command / git）。
///
/// 富展示：命令行（等宽高亮）+ 退出码标签（若有）+ stdout/stderr 输出。
/// 解析失败回退 raw 文本。
class ShellBubble extends StatelessWidget {
  const ShellBubble({super.key, required this.step});

  final ToolCallStep step;

  /// 注册表用的构建器。
  static Widget create(ToolCallStep step) => ShellBubble(step: step);

  @override
  Widget build(BuildContext context) {
    final status = toolStatusOf(step);
    return ToolBubbleChrome(
      step: step,
      child: _body(context, status),
    );
  }

  Widget _body(BuildContext context, ToolStatus status) {
    final theme = Theme.of(context);
    final argsMap = tryJsonMap(step.arguments);
    final command = jsonStr(argsMap, 'command') ??
        jsonStr(argsMap, 'cmd') ??
        (step.name == 'git' ? 'git' : null);

    final output = _output(step.result);

    if (command == null && output == null) {
      if (status == ToolStatus.pending) {
        return _runningRow(theme, '正在执行…');
      }
      return RawOutput(text: step.result ?? step.arguments);
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // ── 命令行 ──
        if (command != null)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest
                  .withValues(alpha: 0.5),
              borderRadius: BorderRadius.circular(6),
            ),
            child: Row(
              children: [
                Icon(Icons.terminal,
                    size: 13, color: theme.colorScheme.primary),
                const SizedBox(width: 6),
                Expanded(
                  child: SelectableText(
                    command,
                    maxLines: 2,
                    style: const TextStyle(
                        fontSize: 12, fontFamily: 'monospace'),
                  ),
                ),
              ],
            ),
          ),
        // ── 输出 ──
        if (output != null && output.trim().isNotEmpty) ...[
          const SizedBox(height: 8),
          RawOutput(text: output, maxHeight: 220),
        ],
        // ── 运行中 ──
        if (status == ToolStatus.pending) ...[
          const SizedBox(height: 6),
          _runningRow(theme, '正在执行…'),
        ],
      ],
    );
  }

  /// 命令输出：result 可能是纯文本或包了 JSON（取 stdout/output 等），否则原样。
  String? _output(String? raw) {
    if (raw == null || raw.trim().isEmpty) return null;
    final m = tryJsonMap(raw);
    if (m != null) {
      final parts = <String>[];
      for (final k in ['stdout', 'output', 'result', 'text']) {
        final v = m[k];
        if (v is String && v.isNotEmpty) parts.add(v);
      }
      final stderr = m['stderr'];
      if (stderr is String && stderr.isNotEmpty) {
        parts.add('[stderr]\n$stderr');
      }
      if (parts.isNotEmpty) return parts.join('\n');
    }
    return raw;
  }

  Widget _runningRow(ThemeData theme, String text) {
    return Row(
      children: [
        SizedBox(
          width: 12,
          height: 12,
          child: CircularProgressIndicator(
              strokeWidth: 1.5, color: theme.colorScheme.secondary),
        ),
        const SizedBox(width: 8),
        Text(text,
            style: TextStyle(
                fontSize: 12, color: theme.colorScheme.onSurfaceVariant)),
      ],
    );
  }
}
