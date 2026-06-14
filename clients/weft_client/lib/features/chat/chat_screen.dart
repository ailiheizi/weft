import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import '../../core/models/chat.dart';
import '../../core/models/chat_session_meta.dart';
import '../../core/providers/chat_provider.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/sessions_provider.dart';
import '../../shared/widgets/glass_card.dart';
import '../../shared/widgets/empty_state.dart';
import 'workspace/artifact.dart';
import 'workspace/workspace_panel.dart';

class ChatScreen extends ConsumerStatefulWidget {
  const ChatScreen({super.key});

  @override
  ConsumerState<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends ConsumerState<ChatScreen>
    with SingleTickerProviderStateMixin {
  final _textController = TextEditingController();
  final _scrollController = ScrollController();
  late final AnimationController _cursorController;

  @override
  void initState() {
    super.initState();
    _cursorController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 600),
    )..repeat(reverse: true);

    // 启动时若无活跃 session，自动创建一个
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      final activeId = ref.read(activeSessionIdProvider);
      if (activeId == null) {
        final meta =
            await ref.read(sessionsProvider.notifier).createSession();
        ref.read(activeSessionIdProvider.notifier).state = meta.id;
      }
    });
  }

  @override
  void dispose() {
    _textController.dispose();
    _scrollController.dispose();
    _cursorController.dispose();
    super.dispose();
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        );
      }
    });
  }

  void _sendMessage(String sessionId) {
    final text = _textController.text.trim();
    if (text.isEmpty) return;
    _textController.clear();
    ref.read(chatProvider(sessionId).notifier).sendMessage(text);
    _scrollToBottom();
  }

  @override
  Widget build(BuildContext context) {
    final activeSessionId = ref.watch(activeSessionIdProvider);
    final workspaceOpen = ref.watch(workspaceProvider).open;

    // 自动跟随最新产物 + 会话切换时同步工作区（清掉其它会话的残留产物）。
    // 放在常驻的 ChatScreen 里监听（不能放 WorkspacePanel，因为面板未展开时
    // 不渲染，会造成"不开→不监听→永不开"的死锁）。
    if (activeSessionId != null) {
      final session = ref.watch(chatProvider(activeSessionId));
      final artifacts = artifactsFromMessages(session.messages);
      WidgetsBinding.instance.addPostFrameCallback((_) {
        ref.read(workspaceProvider.notifier).syncToSession(artifacts);
      });
    }

    return Row(
      children: [
        // ── 左侧 session 列表 ──────────────────────────────────────────────
        _SessionSidebar(
          activeSessionId: activeSessionId,
          onSelectSession: (id) {
            ref.read(activeSessionIdProvider.notifier).state = id;
          },
          onNewChat: () async {
            final meta =
                await ref.read(sessionsProvider.notifier).createSession();
            ref.read(activeSessionIdProvider.notifier).state = meta.id;
          },
        ),
        const VerticalDivider(width: 1),
        // ── 中间聊天区域 ───────────────────────────────────────────────────
        Expanded(
          child: activeSessionId == null
              ? const Center(child: CircularProgressIndicator())
              : _ChatArea(
                  sessionId: activeSessionId,
                  scrollController: _scrollController,
                  cursorController: _cursorController,
                  textController: _textController,
                  onSend: () => _sendMessage(activeSessionId),
                  onStop: () => ref
                      .read(chatProvider(activeSessionId).notifier)
                      .stopStreaming(),
                  scrollToBottom: _scrollToBottom,
                ),
        ),
        // ── 右侧工作区面板（类 Manus，按需展开）──────────────────────────────
        if (workspaceOpen && activeSessionId != null) ...[
          const VerticalDivider(width: 1),
          Expanded(
            flex: 1,
            child: WorkspacePanel(sessionId: activeSessionId),
          ),
        ],
      ],
    );
  }
}

// ─── Session 侧边栏 ────────────────────────────────────────────────────────────

class _SessionSidebar extends ConsumerWidget {
  const _SessionSidebar({
    required this.activeSessionId,
    required this.onSelectSession,
    required this.onNewChat,
  });

  final String? activeSessionId;
  final ValueChanged<String> onSelectSession;
  final VoidCallback onNewChat;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final sessions = ref.watch(sessionsProvider);

    return SizedBox(
      width: 220,
      child: Column(
        children: [
          // New Chat 按钮
          Padding(
            padding: const EdgeInsets.all(8),
            child: SizedBox(
              width: double.infinity,
              child: FilledButton.tonal(
                onPressed: onNewChat,
                style: FilledButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 10),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12),
                  ),
                ),
                child: const Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.add, size: 16),
                    SizedBox(width: 6),
                    Text('New Chat', style: TextStyle(fontSize: 13)),
                  ],
                ),
              ),
            ),
          ),
          const Divider(height: 1),
          // Session 列表
          Expanded(
            child: sessions.isEmpty
                ? Center(
                    child: Text(
                      'No sessions',
                      style: TextStyle(
                        fontSize: 12,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  )
                : ListView.builder(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    itemCount: sessions.length,
                    itemBuilder: (context, index) {
                      final meta = sessions[index];
                      final isActive = meta.id == activeSessionId;
                      return _SessionTile(
                        meta: meta,
                        isActive: isActive,
                        onTap: () => onSelectSession(meta.id),
                        onDelete: () => ref
                            .read(sessionsProvider.notifier)
                            .deleteSession(meta.id),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}

class _SessionTile extends StatefulWidget {
  const _SessionTile({
    required this.meta,
    required this.isActive,
    required this.onTap,
    required this.onDelete,
  });

  final ChatSessionMeta meta;
  final bool isActive;
  final VoidCallback onTap;
  final VoidCallback onDelete;

  @override
  State<_SessionTile> createState() => _SessionTileState();
}

class _SessionTileState extends State<_SessionTile> {
  bool _hovered = false;

  String _formatTime(DateTime dt) {
    final now = DateTime.now();
    final diff = now.difference(dt);
    if (diff.inDays == 0) return DateFormat('HH:mm').format(dt);
    if (diff.inDays < 7) return DateFormat('E').format(dt);
    return DateFormat('MM/dd').format(dt);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return MouseRegion(
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: Container(
          margin: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
          decoration: BoxDecoration(
            color: widget.isActive
                ? theme.colorScheme.primaryContainer.withValues(alpha: 0.56)
                : _hovered
                    ? theme.colorScheme.surfaceContainerHigh
                        .withValues(alpha: 0.48)
                    : Colors.transparent,
            borderRadius: BorderRadius.circular(12),
          ),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      widget.meta.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 13,
                        fontWeight: widget.isActive
                            ? FontWeight.w600
                            : FontWeight.normal,
                        color: widget.isActive
                            ? theme.colorScheme.onPrimaryContainer
                            : theme.colorScheme.onSurface,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      _formatTime(widget.meta.updatedAt),
                      style: TextStyle(
                        fontSize: 11,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              if (_hovered || widget.isActive)
                GestureDetector(
                  onTap: widget.onDelete,
                  child: Icon(
                    Icons.delete_outline,
                    size: 15,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

// ─── 聊天主区域 ────────────────────────────────────────────────────────────────

class _ChatArea extends ConsumerWidget {
  const _ChatArea({
    required this.sessionId,
    required this.scrollController,
    required this.cursorController,
    required this.textController,
    required this.onSend,
    required this.onStop,
    required this.scrollToBottom,
  });

  final String sessionId;
  final ScrollController scrollController;
  final AnimationController cursorController;
  final TextEditingController textController;
  final VoidCallback onSend;
  final VoidCallback onStop;
  final VoidCallback scrollToBottom;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(chatProvider(sessionId));
    final noProviders =
        ref.watch(providersProvider).asData?.value.isEmpty ?? false;

    if (session.isStreaming) {
      scrollToBottom();
    }

    return Column(
      children: [
        _TopBar(session: session, sessionId: sessionId),
        const Divider(height: 1),
        Expanded(
          child: noProviders && session.messages.isEmpty
              ? const _NoProviderState()
              : session.messages.isEmpty
                  ? _EmptyState(session: session)
                  : _MessageList(
                      messages: session.messages,
                      isStreaming: session.isStreaming,
                      scrollController: scrollController,
                      cursorController: cursorController,
                    ),
        ),
        const Divider(height: 1),
        _InputBar(
          controller: textController,
          isStreaming: session.isStreaming,
          onSend: onSend,
          onStop: onStop,
          disabledHint:
              noProviders ? 'Add an AI provider in Settings to start chatting' : null,
        ),
      ],
    );
  }
}

// ─── 顶部 Provider/Model 选择栏 ───────────────────────────────────────────────

class _TopBar extends ConsumerWidget {
  const _TopBar({required this.session, required this.sessionId});
  final ChatSession session;
  final String sessionId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final providersAsync = ref.watch(providersProvider);

    return Container(
      height: 48,
      padding: const EdgeInsets.symmetric(horizontal: 16),
      child: Row(
        children: [
          Icon(Icons.chat_outlined,
              size: 18, color: theme.colorScheme.primary),
          const SizedBox(width: 8),
          Text('Chat',
              style: theme.textTheme.titleSmall
                  ?.copyWith(color: theme.colorScheme.primary)),
          const SizedBox(width: 12),
          _WorkspaceToggle(session: session),
          const Spacer(),
          Flexible(
            child: providersAsync.when(
            data: (providers) => Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Flexible(
                  child: _ProviderDropdown(
                    providers: providers.map((p) => p.name).toList(),
                    selected: session.selectedProvider,
                    onChanged: (v) =>
                        ref.read(chatProvider(sessionId).notifier).setProvider(v),
                  ),
                ),
                if (session.selectedProvider != null) ...[
                  const SizedBox(width: 8),
                  Flexible(
                    child: _ModelDropdown(
                      models: providers
                          .where((p) => p.name == session.selectedProvider)
                          .expand((p) => p.models)
                          .toList(),
                      selected: session.selectedModel,
                      onChanged: (v) =>
                          ref.read(chatProvider(sessionId).notifier).setModel(v),
                    ),
                  ),
                ],
              ],
            ),
            loading: () => const SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
            error: (_, _) => Text('Failed to load providers',
                style: TextStyle(
                    color: theme.colorScheme.error, fontSize: 12)),
          ),
          ),
          const SizedBox(width: 8),
          if (session.messages.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.delete_outline, size: 18),
              tooltip: 'Clear messages',
              onPressed: () =>
                  ref.read(chatProvider(sessionId).notifier).clearMessages(),
            ),
        ],
      ),
    );
  }
}

class _WorkspaceToggle extends ConsumerWidget {
  const _WorkspaceToggle({required this.session});
  final ChatSession session;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final open = ref.watch(workspaceProvider).open;
    final count = artifactsFromMessages(session.messages).length;

    return Tooltip(
      message: open ? '隐藏工作区' : '显示工作区',
      child: InkWell(
        borderRadius: BorderRadius.circular(10),
        onTap: () => ref.read(workspaceProvider.notifier).toggle(),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.view_sidebar_outlined,
                size: 18,
                color: open
                    ? theme.colorScheme.primary
                    : theme.colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 6),
              Text('工作区',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: open
                        ? theme.colorScheme.primary
                        : theme.colorScheme.onSurfaceVariant,
                  )),
              if (count > 0) ...[
                const SizedBox(width: 6),
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.primaryContainer
                        .withValues(alpha: 0.58),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text('$count',
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onPrimaryContainer,
                      )),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _ProviderDropdown extends StatelessWidget {
  const _ProviderDropdown({
    required this.providers,
    required this.selected,
    required this.onChanged,
  });

  final List<String> providers;
  final String? selected;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return DropdownButtonHideUnderline(
      child: DropdownButton<String>(
        value: selected,
        hint: Text('Provider',
            style: TextStyle(
                fontSize: 12, color: theme.colorScheme.onSurfaceVariant)),
        style: TextStyle(fontSize: 12, color: theme.colorScheme.onSurface),
        isDense: true,
        isExpanded: true,
        items: [
          const DropdownMenuItem<String>(
            value: null,
            child: Text('— none —', style: TextStyle(fontSize: 12)),
          ),
          ...providers.map((p) => DropdownMenuItem(
                value: p,
                child: Text(p, style: const TextStyle(fontSize: 12)),
              )),
        ],
        onChanged: onChanged,
      ),
    );
  }
}

class _ModelDropdown extends StatelessWidget {
  const _ModelDropdown({
    required this.models,
    required this.selected,
    required this.onChanged,
  });

  final List<String> models;
  final String? selected;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    if (models.isEmpty) return const SizedBox.shrink();
    return DropdownButtonHideUnderline(
      child: DropdownButton<String>(
        value: models.contains(selected) ? selected : null,
        hint: Text('Model',
            style: TextStyle(
                fontSize: 12, color: theme.colorScheme.onSurfaceVariant)),
        style: TextStyle(fontSize: 12, color: theme.colorScheme.onSurface),
        isDense: true,
        isExpanded: true,
        items: models
            .map((m) => DropdownMenuItem(
                  value: m,
                  child: Text(m, style: const TextStyle(fontSize: 12)),
                ))
            .toList(),
        onChanged: onChanged,
      ),
    );
  }
}

// ─── 空状态 ───────────────────────────────────────────────────────────────────

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.session});
  final ChatSession session;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final hasProvider = session.selectedProvider != null;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.chat_bubble_outline,
              size: 48, color: theme.colorScheme.outlineVariant),
          const SizedBox(height: 16),
          Text(
            hasProvider
                ? 'Send a message to start'
                : 'Select a Provider to start',
            style: theme.textTheme.bodyMedium
                ?.copyWith(color: theme.colorScheme.onSurfaceVariant),
          ),
        ],
      ),
    );
  }
}

// ─── 消息列表 ─────────────────────────────────────────────────────────────────

class _MessageList extends StatelessWidget {
  const _MessageList({
    required this.messages,
    required this.isStreaming,
    required this.scrollController,
    required this.cursorController,
  });

  final List<ChatMessage> messages;
  final bool isStreaming;
  final ScrollController scrollController;
  final AnimationController cursorController;

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      controller: scrollController,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      itemCount: messages.length,
      itemBuilder: (context, index) {
        final msg = messages[index];
        final isLast = index == messages.length - 1;
        final showCursor = isStreaming && isLast && msg.role == 'assistant';
        return _MessageBubble(
          message: msg,
          showCursor: showCursor,
          cursorController: cursorController,
        );
      },
    );
  }
}

// ─── 单条消息气泡 ─────────────────────────────────────────────────────────────

class _MessageBubble extends StatelessWidget {
  const _MessageBubble({
    required this.message,
    required this.showCursor,
    required this.cursorController,
  });

  final ChatMessage message;
  final bool showCursor;
  final AnimationController cursorController;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isUser = message.role == 'user';

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        mainAxisAlignment:
            isUser ? MainAxisAlignment.end : MainAxisAlignment.start,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (!isUser) ...[
            CircleAvatar(
              radius: 14,
              backgroundColor: theme.colorScheme.primaryContainer,
              child: Icon(Icons.smart_toy_outlined,
                  size: 16, color: theme.colorScheme.onPrimaryContainer),
            ),
            const SizedBox(width: 8),
          ],
          Flexible(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 执行步骤（仅 assistant 且有 steps）
                if (!isUser && message.steps.isNotEmpty)
                  _ExecutionStepsPanel(
                    steps: message.steps,
                    forceExpanded: showCursor,
                  ),
                // 消息气泡
                Container(
                  constraints: BoxConstraints(
                    maxWidth: MediaQuery.of(context).size.width * 0.72,
                  ),
                  padding:
                      const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                  decoration: BoxDecoration(
                    color: isUser
                        ? theme.colorScheme.primary.withValues(alpha: 0.88)
                        : theme.colorScheme.surfaceContainerHigh
                            .withValues(alpha: 0.48),
                    border: isUser
                        ? null
                        : Border.all(color: GlassTokens.borderIdle),
                    borderRadius: BorderRadius.only(
                      topLeft: const Radius.circular(18),
                      topRight: const Radius.circular(18),
                      bottomLeft: Radius.circular(isUser ? 18 : 8),
                      bottomRight: Radius.circular(isUser ? 8 : 18),
                    ),
                  ),
                  child: isUser
                      ? Text(
                          message.content,
                          style: TextStyle(
                            color: theme.colorScheme.onPrimary,
                            fontSize: 14,
                          ),
                        )
                      : _AssistantContent(
                          content: message.content,
                          showCursor: showCursor,
                          cursorController: cursorController,
                        ),
                ),
                // ask_user:AI 提问的可点选项(取消息里最后一个 askUser step)
                if (!isUser)
                  ...message.steps.whereType<AskUserStep>().take(1).map(
                        (s) => _AskUserOptions(step: s),
                      ),
              ],
            ),
          ),
          if (isUser) ...[
            const SizedBox(width: 8),
            CircleAvatar(
              radius: 14,
              backgroundColor: theme.colorScheme.secondaryContainer,
              child: Icon(Icons.person_outline,
                  size: 16, color: theme.colorScheme.onSecondaryContainer),
            ),
          ],
        ],
      ),
    );
  }
}

/// 渲染 AI ask_user 提问的可点选项;点击后作为新消息发送,继续对话。
class _AskUserOptions extends ConsumerWidget {
  const _AskUserOptions({required this.step});
  final AskUserStep step;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (step.options.isEmpty) return const SizedBox.shrink();
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.only(top: 8),
      constraints: const BoxConstraints(maxWidth: 520),
      child: Wrap(
        spacing: 8,
        runSpacing: 8,
        children: [
          for (final opt in step.options)
            ActionChip(
              label: Text(opt),
              avatar: Icon(Icons.touch_app_outlined,
                  size: 16, color: theme.colorScheme.primary),
              onPressed: () {
                final sid = ref.read(activeSessionIdProvider);
                if (sid != null) {
                  ref.read(chatProvider(sid).notifier).sendMessage(opt);
                }
              },
            ),
        ],
      ),
    );
  }
}

// ─── 执行步骤面板 ─────────────────────────────────────────────────────────────

class _ExecutionStepsPanel extends StatefulWidget {
  const _ExecutionStepsPanel({required this.steps, this.forceExpanded = false});
  final List<ExecutionStep> steps;
  final bool forceExpanded;

  @override
  State<_ExecutionStepsPanel> createState() => _ExecutionStepsPanelState();
}

class _ExecutionStepsPanelState extends State<_ExecutionStepsPanel> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final expanded = widget.forceExpanded || _expanded;
    final thinkingSteps =
        widget.steps.whereType<ThinkingStep>().toList();
    final toolSteps = widget.steps.whereType<ToolCallStep>().toList();

    final summary = [
      if (thinkingSteps.isNotEmpty) 'Thinking',
      if (toolSteps.isNotEmpty) '${toolSteps.length} tool call${toolSteps.length > 1 ? 's' : ''}',
    ].join(' · ');

    return Container(
      margin: const EdgeInsets.only(bottom: 6),
      constraints: const BoxConstraints(maxWidth: 520),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerLowest.withValues(alpha: 0.42),
        border: Border.all(color: GlassTokens.borderIdle),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // 折叠头
          InkWell(
            onTap: () => setState(() => _expanded = !_expanded),
            borderRadius: BorderRadius.circular(12),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
              child: Row(
                children: [
                  Icon(
                    Icons.psychology_outlined,
                    size: 14,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(
                      summary,
                      style: TextStyle(
                        fontSize: 12,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  Icon(
                    expanded ? Icons.expand_less : Icons.expand_more,
                    size: 16,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ],
              ),
            ),
          ),
          // 展开内容
          if (expanded) ...[
            Divider(height: 1, color: theme.colorScheme.outlineVariant),
            Padding(
              padding: const EdgeInsets.all(10),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  for (final step in widget.steps) _StepItem(step: step),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _StepItem extends ConsumerWidget {
  const _StepItem({required this.step});
  final ExecutionStep step;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    return step.map(
      thinking: (s) => Padding(
        padding: const EdgeInsets.only(bottom: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.lightbulb_outline,
                    size: 12, color: theme.colorScheme.tertiary),
                const SizedBox(width: 4),
                Text(
                  'Thinking',
                  style: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                    color: theme.colorScheme.tertiary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              s.content,
              style: TextStyle(
                fontSize: 12,
                color: theme.colorScheme.onSurfaceVariant,
                fontStyle: FontStyle.italic,
              ),
            ),
          ],
        ),
      ),
      toolCall: (s) {
        final artifact = Artifact.fromToolCall(s);
        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(Icons.build_outlined,
                      size: 12, color: theme.colorScheme.secondary),
                  const SizedBox(width: 4),
                  Text(
                    s.name.isEmpty ? 'Tool call' : s.name,
                    style: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.w600,
                      color: theme.colorScheme.secondary,
                    ),
                  ),
                  if (artifact != null) ...[
                    const SizedBox(width: 6),
                    InkWell(
                      borderRadius: BorderRadius.circular(8),
                      onTap: () =>
                          ref.read(workspaceProvider.notifier).show(artifact),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 4, vertical: 1),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(_artifactIcon(artifact.kind),
                                size: 11, color: theme.colorScheme.primary),
                            const SizedBox(width: 3),
                            Text('查看',
                                style: TextStyle(
                                    fontSize: 10,
                                    color: theme.colorScheme.primary)),
                          ],
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            if (s.arguments.isNotEmpty) ...[
              const SizedBox(height: 4),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: GlassTokens.innerTileFill,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Text(
                  s.arguments,
                  style: TextStyle(
                    fontSize: 11,
                    fontFamily: 'monospace',
                    color: theme.colorScheme.onSurface,
                  ),
                ),
              ),
            ],
            if (s.result == null) ...[
              const SizedBox(height: 4),
              Row(
                children: [
                  SizedBox(
                    width: 11,
                    height: 11,
                    child: CircularProgressIndicator(
                      strokeWidth: 1.5,
                      color: theme.colorScheme.secondary,
                    ),
                  ),
                  const SizedBox(width: 4),
                  Text(
                    'Running…',
                    style: TextStyle(
                      fontSize: 11,
                      color: theme.colorScheme.secondary,
                    ),
                  ),
                ],
              ),
            ],
            if (s.result != null) ...[
              const SizedBox(height: 4),
              Row(
                children: [
                  Icon(Icons.check_circle_outline,
                      size: 11, color: theme.colorScheme.primary),
                  const SizedBox(width: 4),
                  Expanded(
                    child: Text(
                      s.result!,
                      style: TextStyle(
                        fontSize: 11,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ],
        ),
        );
      },
      askUser: (s) => Padding(
        padding: const EdgeInsets.only(bottom: 8),
        child: Row(
          children: [
            Icon(Icons.help_outline,
                size: 12, color: theme.colorScheme.primary),
            const SizedBox(width: 4),
            Expanded(
              child: Text(
                s.question,
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.w600,
                  color: theme.colorScheme.primary,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

IconData _artifactIcon(ArtifactKind kind) {
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

class _AssistantContent extends StatelessWidget {
  const _AssistantContent({
    required this.content,
    required this.showCursor,
    required this.cursorController,
  });

  final String content;
  final bool showCursor;
  final AnimationController cursorController;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (content.isEmpty && showCursor) {
      return AnimatedBuilder(
        animation: cursorController,
        builder: (_, _) => Text(
          cursorController.value > 0.5 ? '▋' : '',
          style:
              TextStyle(color: theme.colorScheme.onSurface, fontSize: 14),
        ),
      );
    }

    if (showCursor) {
      return AnimatedBuilder(
        animation: cursorController,
        builder: (_, _) {
          final cursor = cursorController.value > 0.5 ? '▋' : '';
          return MarkdownBody(
            data: content + cursor,
            styleSheet: MarkdownStyleSheet.fromTheme(theme).copyWith(
              p: theme.textTheme.bodyMedium
                  ?.copyWith(color: theme.colorScheme.onSurface),
              code: theme.textTheme.bodySmall?.copyWith(
                fontFamily: 'monospace',
                backgroundColor: GlassTokens.innerTileFill,
              ),
            ),
            selectable: true,
          );
        },
      );
    }

    return MarkdownBody(
      data: content.isEmpty ? '…' : content,
      styleSheet: MarkdownStyleSheet.fromTheme(theme).copyWith(
        p: theme.textTheme.bodyMedium
            ?.copyWith(color: theme.colorScheme.onSurface),
        code: theme.textTheme.bodySmall?.copyWith(
          fontFamily: 'monospace',
          backgroundColor: GlassTokens.innerTileFill,
        ),
      ),
      selectable: true,
    );
  }
}

// ─── 无 Provider 引导态 ───────────────────────────────────────────────────────

class _NoProviderState extends StatelessWidget {
  const _NoProviderState();

  @override
  Widget build(BuildContext context) {
    return EmptyState(
      icon: Icons.bolt_outlined,
      title: 'No AI provider configured',
      subtitle:
          'Add a provider and API key before you can chat. It only takes a minute.',
      action: FilledButton.icon(
        onPressed: () => context.go('/providers'),
        icon: const Icon(Icons.add, size: 16),
        label: const Text('Add provider'),
      ),
    );
  }
}

// ─── 底部输入栏 ───────────────────────────────────────────────────────────────

class _InputBar extends StatelessWidget {
  const _InputBar({
    required this.controller,
    required this.isStreaming,
    required this.onSend,
    required this.onStop,
    this.disabledHint,
  });

  final TextEditingController controller;
  final bool isStreaming;
  final VoidCallback onSend;
  final VoidCallback onStop;

  /// When set, the composer is disabled and this hint explains why
  /// (e.g. no provider configured yet).
  final String? disabledHint;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final disabled = disabledHint != null;
    final inputDisabled = isStreaming || disabled;

    return Padding(
      padding: const EdgeInsets.all(12),
      child: Row(
        children: [
          Expanded(
            child: TextField(
              controller: controller,
              enabled: !inputDisabled,
              maxLines: null,
              keyboardType: TextInputType.multiline,
              textInputAction: TextInputAction.newline,
              decoration: InputDecoration(
                hintText: disabled
                    ? disabledHint
                    : isStreaming
                        ? 'Generating…'
                        : 'Type a message…  (/team 强制组建团队)',
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(12),
                  borderSide: BorderSide(color: theme.colorScheme.outline),
                ),
                contentPadding: const EdgeInsets.symmetric(
                    horizontal: 14, vertical: 10),
                isDense: true,
              ),
              onSubmitted: inputDisabled ? null : (_) => onSend(),
            ),
          ),
          const SizedBox(width: 8),
          if (isStreaming)
            IconButton.filled(
              onPressed: onStop,
              icon: const Icon(Icons.stop_rounded),
              tooltip: 'Stop',
              style: IconButton.styleFrom(
                backgroundColor: theme.colorScheme.error,
                foregroundColor: theme.colorScheme.onError,
              ),
            )
          else
            IconButton.filled(
              onPressed: disabled ? null : onSend,
              icon: const Icon(Icons.send_rounded),
              tooltip: 'Send',
            ),
        ],
      ),
    );
  }
}
