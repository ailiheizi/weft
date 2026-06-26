import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../shared/theme/spacing.dart';
import 'director_chat_state.dart';

/// 右栏 — 导演 Agent 对话面板。消息流 + 输入框 + ask_user 选项。
class DirectorChatPanel extends ConsumerStatefulWidget {
  const DirectorChatPanel({super.key, this.contextHintBuilder});

  /// 可选：发送时把画布上下文（如选中节点标题）拼进去。
  final String Function()? contextHintBuilder;

  @override
  ConsumerState<DirectorChatPanel> createState() => _DirectorChatPanelState();
}

class _DirectorChatPanelState extends ConsumerState<DirectorChatPanel> {
  final _controller = TextEditingController();
  final _scroll = ScrollController();

  @override
  void dispose() {
    _controller.dispose();
    _scroll.dispose();
    super.dispose();
  }

  void _send([String? preset]) {
    final text = preset ?? _controller.text;
    if (text.trim().isEmpty) return;
    final hint = widget.contextHintBuilder?.call();
    ref.read(directorChatProvider.notifier).send(text, contextHint: hint);
    if (preset == null) _controller.clear();
    _scrollToBottomSoon();
  }

  void _scrollToBottomSoon() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scroll.hasClients) {
        _scroll.animateTo(
          _scroll.position.maxScrollExtent,
          duration: const Duration(milliseconds: 250),
          curve: Curves.easeOut,
        );
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final state = ref.watch(directorChatProvider);
    ref.listen(directorChatProvider, (_, _) => _scrollToBottomSoon());

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _header(theme),
        Expanded(
          child: state.messages.isEmpty
              ? _emptyState(theme)
              : ListView.builder(
                  controller: _scroll,
                  padding: const EdgeInsets.all(Spacing.md),
                  itemCount: state.messages.length,
                  itemBuilder: (_, i) => _bubble(theme, state.messages[i]),
                ),
        ),
        if (state.sending) _typingIndicator(theme),
        if (state.error != null) _errorBar(theme, state.error!),
        _composer(theme, state.sending),
      ],
    );
  }

  Widget _header(ThemeData theme) => Padding(
        padding: const EdgeInsets.fromLTRB(Spacing.md, Spacing.md, Spacing.md, Spacing.sm),
        child: Row(
          children: [
            Icon(Icons.auto_awesome, size: 18, color: theme.colorScheme.primary),
            const SizedBox(width: Spacing.xs),
            Text('AI 导演', style: theme.textTheme.titleSmall),
          ],
        ),
      );

  Widget _emptyState(ThemeData theme) => Center(
        child: Padding(
          padding: const EdgeInsets.all(Spacing.lg),
          child: Text(
            '告诉导演你的创意，\n它会和团队一起把它变成作品。',
            textAlign: TextAlign.center,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      );

  Widget _bubble(ThemeData theme, DirectorMessage m) {
    final isUser = m.isUser;
    final bg = isUser ? theme.colorScheme.primaryContainer : theme.colorScheme.surfaceContainerHigh;
    final fg = isUser ? theme.colorScheme.onPrimaryContainer : theme.colorScheme.onSurface;
    return Align(
      alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.only(bottom: Spacing.sm),
        padding: const EdgeInsets.symmetric(horizontal: Spacing.md, vertical: Spacing.sm),
        constraints: const BoxConstraints(maxWidth: 320),
        decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(12)),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(m.content, style: theme.textTheme.bodyMedium?.copyWith(color: fg)),
            if (m.askUserOptions.isNotEmpty) ...[
              const SizedBox(height: Spacing.sm),
              Wrap(
                spacing: Spacing.xs,
                runSpacing: Spacing.xs,
                children: m.askUserOptions
                    .map((opt) => ActionChip(
                          label: Text(opt, style: theme.textTheme.bodySmall),
                          onPressed: () => _send(opt),
                        ))
                    .toList(),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _typingIndicator(ThemeData theme) => Padding(
        padding: const EdgeInsets.symmetric(horizontal: Spacing.md, vertical: Spacing.xs),
        child: Row(
          children: [
            const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 2)),
            const SizedBox(width: Spacing.sm),
            Text('导演思考中…', style: theme.textTheme.bodySmall),
          ],
        ),
      );

  Widget _errorBar(ThemeData theme, String msg) => Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(horizontal: Spacing.md, vertical: Spacing.sm),
        color: theme.colorScheme.errorContainer,
        child: Text(msg, style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onErrorContainer)),
      );

  Widget _composer(ThemeData theme, bool sending) => Padding(
        padding: const EdgeInsets.all(Spacing.md),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            Expanded(
              child: TextField(
                controller: _controller,
                minLines: 1,
                maxLines: 4,
                textInputAction: TextInputAction.send,
                onSubmitted: (_) => _send(),
                decoration: InputDecoration(
                  hintText: '输入创意，或用 / 唤起技能…',
                  isDense: true,
                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(10)),
                ),
              ),
            ),
            const SizedBox(width: Spacing.sm),
            IconButton.filled(
              onPressed: sending ? null : () => _send(),
              icon: const Icon(Icons.send, size: 18),
            ),
          ],
        ),
      );
}
