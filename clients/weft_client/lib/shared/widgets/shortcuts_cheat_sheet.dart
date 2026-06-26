import 'package:flutter/material.dart';

/// 键盘快捷键速查面板。按 `?`(非输入态)或命令面板"快捷键"项唤起。
/// 集中列出当前生效的快捷键,解决可发现性问题。
class ShortcutsCheatSheet extends StatelessWidget {
  const ShortcutsCheatSheet({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      barrierColor: const Color(0x99000000),
      builder: (_) => const ShortcutsCheatSheet(),
    );
  }

  static const _groups = <(String, List<(String, String)>)>[
    ('全局', [
      ('Ctrl/⌘ + K', '命令面板'),
    ]),
    ('聊天', [
      ('Enter', '发送消息'),
      ('Shift + Enter', '换行'),
      ('↑ (输入框为空)', '调出上一条消息编辑'),
      ('/', '斜杠指令菜单(如 /team)'),
      ('Tab', '接受指令补全'),
      ('Ctrl + N', '新建会话'),
    ]),
    ('消息 / 会话', [
      ('hover 消息', '复制 / 重新生成 / 编辑重发'),
      ('hover 会话 · 右键', '重命名 / 删除'),
    ]),
  ];

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Dialog(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 460, maxHeight: 560),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 18, 20, 8),
              child: Row(
                children: [
                  Icon(Icons.keyboard_outlined,
                      size: 20, color: theme.colorScheme.primary),
                  const SizedBox(width: 8),
                  Text('键盘快捷键',
                      style: theme.textTheme.titleMedium
                          ?.copyWith(fontWeight: FontWeight.w600)),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close, size: 18),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Flexible(
              child: ListView(
                padding: const EdgeInsets.fromLTRB(20, 12, 20, 18),
                children: [
                  for (final group in _groups) ...[
                    Padding(
                      padding: const EdgeInsets.only(top: 8, bottom: 6),
                      child: Text(group.$1,
                          style: theme.textTheme.labelMedium?.copyWith(
                              color: theme.colorScheme.primary,
                              fontWeight: FontWeight.w600)),
                    ),
                    for (final row in group.$2)
                      Padding(
                        padding: const EdgeInsets.symmetric(vertical: 4),
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _KeyCap(row.$1),
                            const SizedBox(width: 12),
                            Expanded(
                              child: Text(row.$2,
                                  style: theme.textTheme.bodyMedium?.copyWith(
                                      color:
                                          theme.colorScheme.onSurfaceVariant)),
                            ),
                          ],
                        ),
                      ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// 键帽样式标签。
class _KeyCap extends StatelessWidget {
  const _KeyCap(this.label);
  final String label;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      constraints: const BoxConstraints(minWidth: 150),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: theme.colorScheme.outlineVariant),
      ),
      child: Text(label,
          style: theme.textTheme.bodySmall?.copyWith(
              fontFamily: 'monospace',
              color: theme.colorScheme.onSurface)),
    );
  }
}
