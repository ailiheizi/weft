import 'package:flutter/material.dart';

import '../../../../core/models/chat.dart';
import '../tool_bubble_chrome.dart';
import '_bubble_utils.dart';

/// 文件读写类工具气泡（fs_read / fs_write / file_read / file_write）。
///
/// 富展示：文件路径（basename 高亮 + 完整路径副标题）+ 读/写标签 + 内容预览。
/// fs_write 的内容在 arguments；fs_read 的内容在 result。解析失败回退 raw。
class FileBubble extends StatelessWidget {
  const FileBubble({super.key, required this.step});

  final ToolCallStep step;

  /// 注册表用的构建器。
  static Widget create(ToolCallStep step) => FileBubble(step: step);

  bool get _isWrite => step.name.contains('write');

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
    final path = jsonStr(argsMap, 'path') ??
        jsonStr(argsMap, 'file_path') ??
        jsonStr(argsMap, 'filename');

    // fs_write 内容在 args；fs_read 内容在 result。
    final content = _isWrite
        ? (jsonStr(argsMap, 'content') ?? jsonStr(argsMap, 'text'))
        : _readContent(step.result);

    if (path == null && content == null) {
      if (status == ToolStatus.pending) {
        return _runningRow(theme, _isWrite ? '正在写入…' : '正在读取…');
      }
      return RawOutput(text: step.result ?? step.arguments);
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // ── 文件头：图标 + basename + 读/写标签 ──
        Row(
          children: [
            Icon(Icons.insert_drive_file_outlined,
                size: 14, color: theme.colorScheme.primary),
            const SizedBox(width: 6),
            Flexible(
              child: Text(
                path != null ? _basename(path) : '(未知文件)',
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                    fontFamily: 'monospace'),
              ),
            ),
            const SizedBox(width: 8),
            BubbleTag(
              text: _isWrite ? '写入' : '读取',
              icon: _isWrite ? Icons.edit_outlined : Icons.visibility_outlined,
            ),
          ],
        ),
        // ── 完整路径副标题 ──
        if (path != null && _basename(path) != path)
          Padding(
            padding: const EdgeInsets.only(top: 2, left: 20),
            child: Text(
              path,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                  fontSize: 10, color: theme.colorScheme.onSurfaceVariant),
            ),
          ),
        // ── 内容预览 ──
        if (content != null && content.trim().isNotEmpty) ...[
          const SizedBox(height: 8),
          RawOutput(text: content, maxHeight: 200),
        ],
        // ── 运行中 ──
        if (status == ToolStatus.pending) ...[
          const SizedBox(height: 6),
          _runningRow(theme, _isWrite ? '正在写入…' : '正在读取…'),
        ],
      ],
    );
  }

  /// fs_read 的内容：result 可能是纯文本或包了 JSON（取常见字段），否则原样。
  String? _readContent(String? raw) {
    if (raw == null || raw.trim().isEmpty) return null;
    final m = tryJsonMap(raw);
    if (m != null) {
      for (final k in ['content', 'text', 'output', 'result', 'data']) {
        final v = m[k];
        if (v is String && v.isNotEmpty) return v;
      }
    }
    return raw;
  }

  static String _basename(String path) {
    final norm = path.replaceAll('\\', '/');
    final idx = norm.lastIndexOf('/');
    return idx >= 0 ? norm.substring(idx + 1) : norm;
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
