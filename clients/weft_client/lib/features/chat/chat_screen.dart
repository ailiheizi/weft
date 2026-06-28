import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/models/chat.dart';
import '../../core/providers/chat_provider.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/preferences_provider.dart';
import '../../core/providers/sessions_provider.dart';
import '../../core/providers/scenes_provider.dart';
import '../../core/providers/selector_provider.dart';
import '../../shared/widgets/glass_card.dart';
import '../../shared/widgets/empty_state.dart';
import '../../shared/widgets/resizable_handle.dart';
import 'tool_display/generic_tool_bubble.dart';
import 'tool_display/tool_bubble_chrome.dart';
import 'tool_display/tool_registry.dart';
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

  /// 用户是否在底部附近(决定流式时是否自动跟随)。
  bool _atBottom = true;

  /// 右侧工作区宽度(可拖拽调节)。
  double _workspaceWidth = 420;

  @override
  void initState() {
    super.initState();
    _cursorController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 600),
    )..repeat(reverse: true);
    _scrollController.addListener(_onScroll);
    _textController.addListener(_onTextChangedForSelector);

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

  void _onTextChangedForSelector() {
    final sessionId = ref.read(activeSessionIdProvider);
    if (sessionId == null) return;
    ref.read(inputTextProvider(sessionId).notifier).state =
        _textController.text;
    ref
        .read(toolSelectorProvider(sessionId).notifier)
        .onTextChanged(_textController.text);
  }

  void _onScroll() {
    if (!_scrollController.hasClients) return;
    final pos = _scrollController.position;
    // 距底部 80px 内视为"在底部"。
    final atBottom = pos.maxScrollExtent - pos.pixels < 80;
    if (atBottom != _atBottom) {
      setState(() => _atBottom = atBottom);
    }
  }

  @override
  void dispose() {
    _scrollController.removeListener(_onScroll);
    _textController.removeListener(_onTextChangedForSelector);
    _textController.dispose();
    _scrollController.dispose();
    _cursorController.dispose();
    super.dispose();
  }

  /// 自动跟随:仅当用户在底部时滚动(流式增量时调用,不打断向上翻阅)。
  void _autoFollow() {
    if (!_atBottom) return;
    _scrollToBottom();
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
    // Read selected tools before clearing.
    final selectedTools = ref
        .read(toolSelectorProvider(sessionId))
        .selectedToolIds;
    ref.read(chatProvider(sessionId).notifier).sendMessage(
      text,
      selectedTools: selectedTools.isNotEmpty ? selectedTools : null,
    );
    ref.read(toolSelectorProvider(sessionId).notifier).clear();
    // 发送后恢复自动跟随并滚到底部。
    _atBottom = true;
    _scrollToBottom();
  }

  Future<void> _newChat() async {
    final meta = await ref.read(sessionsProvider.notifier).createSession();
    ref.read(activeSessionIdProvider.notifier).state = meta.id;
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

    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.keyN, control: true): _newChat,
      },
      child: Row(
      children: [
        // ── 中间聊天区域(会话列表已上移到全局侧栏 AppShell)─────────────────
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
                  scrollToBottom: _autoFollow,
                  atBottom: _atBottom,
                  onJumpToBottom: () {
                    setState(() => _atBottom = true);
                    _scrollToBottom();
                  },
                ),
        ),
        // ── 右侧工作区面板（类 Manus，按需展开,可拖拽调宽）─────────────────────
        if (workspaceOpen && activeSessionId != null) ...[
          ResizableHandle(
            // 向左拖(dx<0)变宽,故取负。clamp 到 [320, 760]。
            onDelta: (dx) => setState(() {
              _workspaceWidth = (_workspaceWidth - dx).clamp(320.0, 760.0);
            }),
          ),
          SizedBox(
            width: _workspaceWidth,
            child: WorkspacePanel(sessionId: activeSessionId),
          ),
        ],
      ],
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
    required this.atBottom,
    required this.onJumpToBottom,
  });

  final String sessionId;
  final ScrollController scrollController;
  final AnimationController cursorController;
  final TextEditingController textController;
  final VoidCallback onSend;
  final VoidCallback onStop;
  final VoidCallback scrollToBottom;
  final bool atBottom;
  final VoidCallback onJumpToBottom;

  /// 最近一条用户消息文本（供输入框 ↑ 键调出重新编辑）。
  String? _lastUserText(List<ChatMessage> messages) {
    for (final m in messages.reversed) {
      if (m.role == 'user' && m.content.trim().isNotEmpty) {
        return m.content.trim();
      }
    }
    return null;
  }

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
        Expanded(
          child: noProviders && session.messages.isEmpty
              ? const _NoProviderState()
              : session.messages.isEmpty
                  ? _EmptyState(session: session, textController: textController)
                  : Stack(
                      children: [
                        _MessageList(
                          sessionId: sessionId,
                          messages: session.messages,
                          isStreaming: session.isStreaming,
                          scrollController: scrollController,
                          cursorController: cursorController,
                        ),
                        // 回到底部悬浮按钮:用户向上翻阅(不在底部)时出现。
                        if (!atBottom)
                          Positioned(
                            right: 16,
                            bottom: 12,
                            child: _JumpToBottomButton(
                              streaming: session.isStreaming,
                              onTap: onJumpToBottom,
                            ),
                          ),
                      ],
                    ),
        ),
        _ToolSelectorChips(sessionId: sessionId),
        _InputBar(
          controller: textController,
          isStreaming: session.isStreaming,
          onSend: onSend,
          onStop: onStop,
          lastUserMessage: _lastUserText(session.messages),
          fileNames: [
            for (final a in artifactsFromMessages(session.messages))
              if (a.kind == ArtifactKind.file) a.title,
          ],
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
          const SizedBox(width: 8),
          const _SceneChip(),
          const SizedBox(width: 8),
          const _WorkspaceDirChip(),
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

/// 顶栏场景 chip:显示当前激活的 Scene。无激活场景时显示"默认场景"。
/// 纯展示(切换在右侧工作区「场景」标签页操作)。
class _SceneChip extends ConsumerWidget {
  const _SceneChip();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final active = ref.watch(scenesProvider).activeScene;
    final label = active.isEmpty ? '默认场景' : active;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
            color: theme.colorScheme.outlineVariant.withValues(alpha: 0.5)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.tune, size: 13, color: theme.colorScheme.onSurfaceVariant),
          const SizedBox(width: 5),
          Text(label,
              style: theme.textTheme.bodySmall
                  ?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
        ],
      ),
    );
  }
}

/// 顶栏工作目录 chip：显示当前 AI 文件沙盒路径，点击可快速修改。
class _WorkspaceDirChip extends ConsumerWidget {
  const _WorkspaceDirChip();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final dir = ref.watch(preferencesProvider).workspaceDir;
    final label = dir.isEmpty ? 'workspaces/' : _shortenPath(dir);
    return Tooltip(
      message: dir.isEmpty ? '默认工作目录 (data/workspaces/)' : dir,
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: () => _editDir(context, ref),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          decoration: BoxDecoration(
            color:
                theme.colorScheme.surfaceContainerHigh.withValues(alpha: 0.5),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
                color: theme.colorScheme.outlineVariant.withValues(alpha: 0.5)),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.folder_outlined,
                  size: 13, color: theme.colorScheme.onSurfaceVariant),
              const SizedBox(width: 5),
              ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 120),
                child: Text(label,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodySmall
                        ?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _editDir(BuildContext context, WidgetRef ref) async {
    final prefs = ref.read(preferencesProvider);
    final controller = TextEditingController(text: prefs.workspaceDir);
    final result = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('工作目录'),
        content: Row(
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                decoration: const InputDecoration(
                  hintText: '留空使用默认',
                  isDense: true,
                ),
              ),
            ),
            const SizedBox(width: 8),
            IconButton(
              icon: const Icon(Icons.folder_open),
              onPressed: () async {
                final picked =
                    await FilePicker.getDirectoryPath(dialogTitle: '选择工作目录');
                if (picked != null) controller.text = picked;
              },
            ),
          ],
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
          TextButton(
              onPressed: () => Navigator.pop(ctx, ''),
              child: const Text('恢复默认')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, controller.text.trim()),
              child: const Text('保存')),
        ],
      ),
    );
    if (result != null) {
      ref.read(preferencesProvider.notifier).setWorkspaceDir(result);
    }
  }

  static String _shortenPath(String path) {
    final parts = path.replaceAll('/', '\\').split('\\');
    if (parts.length <= 2) return path;
    return '...\\${parts[parts.length - 2]}\\${parts.last}';
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
  const _EmptyState({required this.session, this.textController});
  final ChatSession session;
  final TextEditingController? textController;

  static const _quickActions = [
    (icon: Icons.travel_explore, label: '探索代码库', prompt: '帮我探索并梳理这个代码库的结构与核心模块'),
    (icon: Icons.bug_report_outlined, label: '修复 Bug', prompt: '我遇到一个 bug,帮我定位并修复:'),
    (icon: Icons.auto_awesome, label: '实现功能', prompt: '帮我实现一个新功能:'),
  ];

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
          if (hasProvider && textController != null) ...[
            const SizedBox(height: 24),
            Wrap(
              spacing: 10,
              runSpacing: 10,
              alignment: WrapAlignment.center,
              children: [
                for (final a in _quickActions)
                  _QuickActionCard(
                    icon: a.icon,
                    label: a.label,
                    onTap: () {
                      textController!.text = a.prompt;
                      textController!.selection = TextSelection.fromPosition(
                        TextPosition(offset: textController!.text.length),
                      );
                    },
                  ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

class _QuickActionCard extends StatelessWidget {
  const _QuickActionCard({
    required this.icon,
    required this.label,
    required this.onTap,
  });
  final IconData icon;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: theme.colorScheme.surfaceContainerHigh.withValues(alpha: 0.5),
      borderRadius: BorderRadius.circular(12),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          constraints: const BoxConstraints(minWidth: 120),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 22, color: theme.colorScheme.primary),
              const SizedBox(height: 8),
              Text(label,
                  style: theme.textTheme.bodySmall
                      ?.copyWith(fontWeight: FontWeight.w500)),
            ],
          ),
        ),
      ),
    );
  }
}

// ─── 消息列表 ─────────────────────────────────────────────────────────────────

/// 回到底部悬浮按钮(流式时附"新消息"提示)。
class _JumpToBottomButton extends StatelessWidget {
  const _JumpToBottomButton({required this.streaming, required this.onTap});

  final bool streaming;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      elevation: 3,
      borderRadius: BorderRadius.circular(20),
      color: theme.colorScheme.secondaryContainer,
      child: InkWell(
        borderRadius: BorderRadius.circular(20),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.arrow_downward_rounded,
                  size: 16, color: theme.colorScheme.onSecondaryContainer),
              const SizedBox(width: 6),
              Text(
                streaming ? '新消息' : '回到底部',
                style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSecondaryContainer,
                    fontWeight: FontWeight.w500),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _MessageList extends StatelessWidget {
  const _MessageList({
    required this.sessionId,
    required this.messages,
    required this.isStreaming,
    required this.scrollController,
    required this.cursorController,
  });

  final String sessionId;
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
          sessionId: sessionId,
          message: msg,
          showCursor: showCursor,
          streaming: isStreaming,
          cursorController: cursorController,
        );
      },
    );
  }
}

// ─── 单条消息气泡 ─────────────────────────────────────────────────────────────

class _MessageBubble extends ConsumerStatefulWidget {
  const _MessageBubble({
    required this.sessionId,
    required this.message,
    required this.showCursor,
    required this.streaming,
    required this.cursorController,
  });

  final String sessionId;
  final ChatMessage message;
  final bool showCursor;
  final bool streaming;
  final AnimationController cursorController;

  @override
  ConsumerState<_MessageBubble> createState() => _MessageBubbleState();
}

class _MessageBubbleState extends ConsumerState<_MessageBubble> {
  bool _hovered = false;

  ChatMessage get message => widget.message;

  Future<void> _copy() async {
    await Clipboard.setData(ClipboardData(text: message.content));
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('已复制'), duration: Duration(seconds: 1)),
      );
    }
  }

  void _regenerate() {
    ref.read(chatProvider(widget.sessionId).notifier).regenerate(message.id);
  }

  Future<void> _edit() async {
    final controller = TextEditingController(text: message.content);
    final newText = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('编辑并重发'),
        content: TextField(
          controller: controller,
          autofocus: true,
          maxLines: null,
          decoration: const InputDecoration(hintText: '修改后重新发送'),
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, controller.text.trim()),
              child: const Text('重发')),
        ],
      ),
    );
    if (newText != null && newText.isNotEmpty) {
      ref
          .read(chatProvider(widget.sessionId).notifier)
          .editAndResend(message.id, newText);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isUser = message.role == 'user';
    final showCursor = widget.showCursor;
    final cursorController = widget.cursorController;
    // 操作条:hover 时显示;流式进行中的最后一条不显示(避免误操作)。
    final showActions = _hovered && !showCursor && message.content.trim().isNotEmpty;

    return MouseRegion(
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: Padding(
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
                        : Border.all(color: GlassTokens.borderIdleOf(context)),
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
                // 消息级操作条(hover 显示):复制 / 重新生成(助手) / 编辑重发(用户)
                if (showActions)
                  Padding(
                    padding: const EdgeInsets.only(top: 4),
                    child: Row(
                      mainAxisAlignment:
                          isUser ? MainAxisAlignment.end : MainAxisAlignment.start,
                      children: [
                        _MsgActionButton(
                            icon: Icons.copy_rounded,
                            tooltip: '复制',
                            onTap: _copy),
                        if (!isUser && !widget.streaming)
                          _MsgActionButton(
                              icon: Icons.refresh_rounded,
                              tooltip: '重新生成',
                              onTap: _regenerate),
                        if (isUser && !widget.streaming)
                          _MsgActionButton(
                              icon: Icons.edit_outlined,
                              tooltip: '编辑重发',
                              onTap: _edit),
                        if (isUser && !widget.streaming)
                          _MsgActionButton(
                              icon: Icons.undo_rounded,
                              tooltip: '撤回',
                              onTap: () => ref
                                  .read(chatProvider(widget.sessionId).notifier)
                                  .undoLastRound()),
                      ],
                    ),
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
      ),
    );
  }
}

/// 消息级操作小按钮。
class _MsgActionButton extends StatelessWidget {
  const _MsgActionButton(
      {required this.icon, required this.tooltip, required this.onTap});

  final IconData icon;
  final String tooltip;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Tooltip(
      message: tooltip,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(6),
        child: Padding(
          padding: const EdgeInsets.all(4),
          child: Icon(icon,
              size: 15, color: theme.colorScheme.onSurfaceVariant),
        ),
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
        border: Border.all(color: GlassTokens.borderIdleOf(context)),
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
                children: _buildGroupedSteps(widget.steps),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

/// 把连续的同名 tool_call 合并为分组(≥2 个才合并),其余原样渲染。
List<Widget> _buildGroupedSteps(List<ExecutionStep> steps) {
  final widgets = <Widget>[];
  var i = 0;
  while (i < steps.length) {
    final step = steps[i];
    if (step is ToolCallStep) {
      // 收集连续同名 tool call。
      var j = i + 1;
      while (j < steps.length &&
          steps[j] is ToolCallStep &&
          (steps[j] as ToolCallStep).name == step.name) {
        j++;
      }
      final run = steps.sublist(i, j).cast<ToolCallStep>();
      if (run.length >= 2) {
        widgets.add(_ToolGroupItem(name: step.name, calls: run));
        i = j;
        continue;
      }
    }
    widgets.add(_StepItem(step: step));
    i++;
  }
  return widgets;
}

/// 连续同名工具调用的折叠分组(默认折叠显示"name ×N",展开看每条)。
class _ToolGroupItem extends StatefulWidget {
  const _ToolGroupItem({required this.name, required this.calls});
  final String name;
  final List<ToolCallStep> calls;

  @override
  State<_ToolGroupItem> createState() => _ToolGroupItemState();
}

class _ToolGroupItemState extends State<_ToolGroupItem> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    // 分组聚合状态:有 error→error;有 pending→pending;否则 success。
    final statuses = widget.calls.map(toolStatusOf).toList();
    final agg = statuses.contains(ToolStatus.error)
        ? ToolStatus.error
        : statuses.contains(ToolStatus.pending)
            ? ToolStatus.pending
            : ToolStatus.success;
    final c = toolStatusColors(agg, theme);
    final label = widget.name.isEmpty ? 'Tool call' : widget.name;

    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: c.bg,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          InkWell(
            onTap: () => setState(() => _expanded = !_expanded),
            borderRadius: BorderRadius.circular(8),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
              child: Row(
                children: [
                  if (agg == ToolStatus.pending)
                    SizedBox(
                      width: 12,
                      height: 12,
                      child: CircularProgressIndicator(
                          strokeWidth: 1.5, color: c.fg),
                    )
                  else
                    Icon(
                      agg == ToolStatus.error
                          ? Icons.error_outline
                          : Icons.check_circle_outline,
                      size: 13,
                      color: c.fg,
                    ),
                  const SizedBox(width: 6),
                  Text('$label ×${widget.calls.length}',
                      style: TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w600,
                          color: c.fg)),
                  const Spacer(),
                  Icon(_expanded ? Icons.expand_less : Icons.expand_more,
                      size: 15, color: c.fg),
                ],
              ),
            ),
          ),
          if (_expanded)
            Padding(
              padding: const EdgeInsets.fromLTRB(8, 0, 8, 6),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  for (final call in widget.calls)
                    _StepItem(step: call),
                ],
              ),
            ),
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
        // 专属气泡优先：命中注册表则用领域化富展示；未命中走通用气泡兜底。
        final bubbleBuilder = bubbleBuilderFor(s.name);
        if (bubbleBuilder != null) {
          return bubbleBuilder(s);
        }
        // 通用兜底：统一外壳 + KvTable/RawOutput（含所有 MCP 工具）。
        return GenericToolBubble(step: s);
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
                backgroundColor: GlassTokens.innerTileFillOf(context),
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
          backgroundColor: GlassTokens.innerTileFillOf(context),
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

/// 可用的斜杠指令(输入 / 时弹出)。usage 给参数提示。
const _slashCommands = [
  (
    cmd: '/team',
    desc: '强制组建多 agent 团队完成复杂任务',
    usage: '/team <任务描述>',
  ),
];

typedef _SlashCmd = ({String cmd, String desc, String usage});

// ─── 工具自动选择芯片条 ────────────────────────────────────────────────────────

class _ToolSelectorChips extends ConsumerWidget {
  const _ToolSelectorChips({required this.sessionId});

  final String sessionId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectorState = ref.watch(toolSelectorProvider(sessionId));
    final session = ref.watch(chatProvider(sessionId));
    final theme = Theme.of(context);
    final isInjected = selectorState.active.isNotEmpty && session.isStreaming;
    final modelReady = selectorState.modelReady;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          // ── 控制行：始终可见 ──
          Row(
            children: [
              // 主按钮：手动触发 / 显示状态
              _SelectorButton(sessionId: sessionId),
              const SizedBox(width: 8),
              if (selectorState.loading)
                SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: theme.colorScheme.primary,
                  ),
                ),
              if (!selectorState.loading && selectorState.active.isNotEmpty)
                Text(
                  isInjected ? '✓ 已注入 ${selectorState.active.length} 个工具' : '${selectorState.active.length} 个工具已选',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: isInjected
                        ? theme.colorScheme.primary
                        : theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              // ── 模型未就绪提示 ──
              if (!modelReady) ...[
                const SizedBox(width: 6),
                Tooltip(
                  message: '语义选择引擎需要下载模型',
                  child: Icon(
                    Icons.info_outline,
                    size: 14,
                    color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
                  ),
                ),
              ],
            ],
          ),
          // ── 芯片展示区：有结果时展示 ──
          if (selectorState.matches.isNotEmpty) ...[
            const SizedBox(height: 4),
            Opacity(
              opacity: modelReady ? 1.0 : 0.4,
              child: Wrap(
                spacing: 6,
                runSpacing: 4,
                children: [
                  for (final match in selectorState.active)
                    Tooltip(
                      message: modelReady ? '' : '语义选择引擎需要下载模型',
                      child: InputChip(
                        label: Text(
                          '${match.name} ${(match.score * 100).toStringAsFixed(0)}%',
                          style: theme.textTheme.labelSmall,
                        ),
                        avatar: Icon(_toolIcon(match.id), size: 14),
                        deleteIcon: const Icon(Icons.close, size: 14),
                        onDeleted: () => ref
                            .read(toolSelectorProvider(sessionId).notifier)
                            .deselect(match.id),
                        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                        visualDensity: VisualDensity.compact,
                      ),
                    ),
                  // Deselected: faded, tappable to re-add
                  for (final match in selectorState.matches
                      .where((m) => selectorState.deselected.contains(m.id)))
                    ActionChip(
                      label: Text(
                        match.name,
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
                        ),
                      ),
                      onPressed: () => ref
                          .read(toolSelectorProvider(sessionId).notifier)
                          .reselect(match.id),
                      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      visualDensity: VisualDensity.compact,
                    ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }

  IconData _toolIcon(String toolId) {
    return switch (toolId) {
      'web_search' => Icons.search,
      'file_read' || 'file_write' => Icons.description,
      'shell_exec' || 'run_command' => Icons.terminal,
      'image_gen' => Icons.image,
      'calculator' => Icons.calculate,
      'translate' => Icons.translate,
      'code_run' || 'python_exec' => Icons.code,
      'todoist' || 'todo' => Icons.checklist,
      _ => Icons.build_circle_outlined,
    };
  }
}

/// 工具选择主按钮：
/// - 点击：手动触发 selector
/// - 旁边小三角展开菜单：切换自动/关闭
class _SelectorButton extends ConsumerWidget {
  const _SelectorButton({required this.sessionId});

  final String sessionId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(toolSelectorProvider(sessionId));
    final theme = Theme.of(context);
    final modelReady = state.modelReady;

    final isActive = state.autoSelect;
    final Color color = !modelReady
        ? theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4)
        : isActive
            ? theme.colorScheme.primary
            : theme.colorScheme.onSurfaceVariant;

    final button = Container(
      height: 28,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(14),
        border: Border.all(
          color: color.withValues(alpha: 0.3),
          width: 1,
        ),
        color: isActive && modelReady
            ? theme.colorScheme.primaryContainer.withValues(alpha: 0.3)
            : Colors.transparent,
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          // 主按钮区域
          InkWell(
            borderRadius: const BorderRadius.horizontal(left: Radius.circular(14)),
            onTap: modelReady
                ? () {
                    final text = ref.read(inputTextProvider(sessionId));
                    if (text.trim().isNotEmpty) {
                      ref.read(toolSelectorProvider(sessionId).notifier)
                          .manualSelect(text);
                    }
                  }
                : null,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.auto_awesome, size: 14, color: color),
                  const SizedBox(width: 4),
                  Text(
                    '工具',
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: color,
                      fontWeight: isActive && modelReady ? FontWeight.w600 : FontWeight.normal,
                    ),
                  ),
                ],
              ),
            ),
          ),
          // 分隔线 + 下拉箭头
          Container(width: 1, height: 16, color: color.withValues(alpha: 0.2)),
          PopupMenuButton<String>(
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
            position: PopupMenuPosition.over,
            icon: Icon(Icons.expand_less, size: 14, color: color),
            tooltip: '',
            itemBuilder: (context) => [
              PopupMenuItem(
                value: 'toggle_auto',
                height: 36,
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      isActive ? Icons.toggle_on : Icons.toggle_off,
                      size: 20,
                      color: isActive ? theme.colorScheme.primary : null,
                    ),
                    const SizedBox(width: 8),
                    Text(isActive ? '自动选择：开' : '自动选择：关',
                        style: theme.textTheme.bodySmall),
                  ],
                ),
              ),
              if (state.matches.isNotEmpty)
                PopupMenuItem(
                  value: 'clear',
                  height: 36,
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      const Icon(Icons.clear_all, size: 20),
                      const SizedBox(width: 8),
                      Text('清除选择', style: theme.textTheme.bodySmall),
                    ],
                  ),
                ),
            ],
            onSelected: (value) {
              switch (value) {
                case 'toggle_auto':
                  ref.read(toolSelectorProvider(sessionId).notifier)
                      .toggleAutoSelect();
                case 'clear':
                  ref.read(toolSelectorProvider(sessionId).notifier).clear();
              }
            },
          ),
        ],
      ),
    );

    if (!modelReady) {
      return Tooltip(
        message: '语义选择引擎需要下载模型',
        child: Opacity(opacity: 0.6, child: button),
      );
    }
    return button;
  }
}

class _InputBar extends StatefulWidget {
  const _InputBar({
    required this.controller,
    required this.isStreaming,
    required this.onSend,
    required this.onStop,
    this.lastUserMessage,
    this.fileNames = const [],
    this.disabledHint,
  });

  final TextEditingController controller;
  final bool isStreaming;
  final VoidCallback onSend;
  final VoidCallback onStop;

  /// 最近一条用户消息(输入框为空时按 ↑ 调出重新编辑)。
  final String? lastUserMessage;

  /// 本会话产出的文件名(供 @ 引用菜单)。
  final List<String> fileNames;

  /// When set, the composer is disabled and this hint explains why
  /// (e.g. no provider configured yet).
  final String? disabledHint;

  @override
  State<_InputBar> createState() => _InputBarState();
}

class _InputBarState extends State<_InputBar> {
  final FocusNode _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_onTextChanged);
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onTextChanged);
    _focusNode.dispose();
    super.dispose();
  }

  void _onTextChanged() {
    // 重建以驱动斜杠菜单的显示/过滤。
    if (mounted) setState(() {});
  }

  /// 当前是否应显示斜杠菜单 + 匹配的指令(前缀优先,其次子串模糊匹配)。
  List<_SlashCmd> get _matchingSlash {
    final text = widget.controller.text;
    if (!text.startsWith('/')) return const [];
    // 仅当还在输入指令本身(没有空格)时显示;输入了参数就收起。
    if (text.contains(' ') || text.contains('\n')) return const [];
    final q = text.toLowerCase();
    final prefix = _slashCommands.where((c) => c.cmd.startsWith(q)).toList();
    if (prefix.isNotEmpty) return prefix;
    // 模糊:指令名或描述含查询(去掉前导 /)。
    final qq = q.replaceFirst('/', '');
    return _slashCommands
        .where((c) =>
            c.cmd.contains(qq) || c.desc.toLowerCase().contains(qq))
        .toList();
  }

  /// 灰色 ghost-text:正在输入指令前缀时,补全到第一个匹配指令的剩余部分。
  String get _ghostSuffix {
    final text = widget.controller.text;
    final m = _matchingSlash;
    if (m.isEmpty) return '';
    final top = m.first.cmd;
    if (top.startsWith(text) && top.length > text.length) {
      return top.substring(text.length);
    }
    return '';
  }

  void _applySlash(String cmd) {
    widget.controller.text = '$cmd ';
    widget.controller.selection = TextSelection.collapsed(
      offset: widget.controller.text.length,
    );
    _focusNode.requestFocus();
  }

  /// 光标前正在输入的 @词(无空格);非 @ 上下文返回 null。
  String? get _atQuery {
    final sel = widget.controller.selection;
    final text = widget.controller.text;
    final caret = sel.baseOffset < 0 ? text.length : sel.baseOffset;
    final before = text.substring(0, caret);
    final at = before.lastIndexOf('@');
    if (at < 0) return null;
    final token = before.substring(at + 1);
    if (token.contains(' ') || token.contains('\n')) return null;
    // @ 前若是字母数字(如 email),不当作引用。
    if (at > 0) {
      final prev = before[at - 1];
      if (RegExp(r'[A-Za-z0-9]').hasMatch(prev)) return null;
    }
    return token;
  }

  /// 匹配的文件名(@ 引用菜单)。
  List<String> get _matchingFiles {
    final q = _atQuery;
    if (q == null || widget.fileNames.isEmpty) return const [];
    final ql = q.toLowerCase();
    final seen = <String>{};
    return widget.fileNames
        .where((n) => seen.add(n) && (ql.isEmpty || n.toLowerCase().contains(ql)))
        .take(6)
        .toList();
  }

  void _applyFile(String name) {
    final text = widget.controller.text;
    final sel = widget.controller.selection;
    final caret = sel.baseOffset < 0 ? text.length : sel.baseOffset;
    final before = text.substring(0, caret);
    final at = before.lastIndexOf('@');
    if (at < 0) return;
    final newText = '${text.substring(0, at)}@$name ${text.substring(caret)}';
    widget.controller.text = newText;
    final newCaret = at + name.length + 2;
    widget.controller.selection = TextSelection.collapsed(offset: newCaret);
    _focusNode.requestFocus();
  }

  KeyEventResult _handleKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent) return KeyEventResult.ignored;
    final isEnter = event.logicalKey == LogicalKeyboardKey.enter ||
        event.logicalKey == LogicalKeyboardKey.numpadEnter;
    final shift = HardwareKeyboard.instance.isShiftPressed;
    final slash = _matchingSlash;

    // 斜杠菜单可见时,Enter 选中唯一/第一个指令。
    if (slash.isNotEmpty && isEnter && !shift) {
      _applySlash(slash.first.cmd);
      return KeyEventResult.handled;
    }
    // @ 文件菜单可见时,Enter 选中第一个文件。
    final files = _matchingFiles;
    if (files.isNotEmpty && isEnter && !shift) {
      _applyFile(files.first);
      return KeyEventResult.handled;
    }
    // Tab 接受 ghost-text 补全(补全到完整指令名)。
    if (event.logicalKey == LogicalKeyboardKey.tab && _ghostSuffix.isNotEmpty) {
      _applySlash(slash.first.cmd);
      return KeyEventResult.handled;
    }
    // Enter 发送(Shift+Enter 换行,交给 TextField 默认处理)。
    if (isEnter && !shift) {
      widget.onSend();
      return KeyEventResult.handled;
    }
    // 输入框为空时,↑ 调出上一条用户消息重新编辑。
    if (event.logicalKey == LogicalKeyboardKey.arrowUp &&
        widget.controller.text.isEmpty &&
        (widget.lastUserMessage?.isNotEmpty ?? false)) {
      widget.controller.text = widget.lastUserMessage!;
      widget.controller.selection = TextSelection.collapsed(
        offset: widget.controller.text.length,
      );
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final disabled = widget.disabledHint != null;
    final inputDisabled = widget.isStreaming || disabled;
    final slash = inputDisabled ? const <_SlashCmd>[] : _matchingSlash;
    // 指令模式:输入以 / 开头(给 composer 边框换强调色,提示回车会执行指令)。
    final commandMode = !inputDisabled && widget.controller.text.startsWith('/');
    final accent = theme.colorScheme.primary;
    final ghost = inputDisabled ? '' : _ghostSuffix;

    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 14),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (slash.isNotEmpty) _SlashMenu(items: slash, onPick: _applySlash),
          if (slash.isEmpty && !inputDisabled && _matchingFiles.isNotEmpty)
            _FileMentionMenu(files: _matchingFiles, onPick: _applyFile),
          // 浮起的输入框:圆角 + surfaceContainerHigh + 轻阴影,脱离底边。
          DecoratedBox(
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHigh,
              borderRadius: BorderRadius.circular(16),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withValues(alpha: 0.18),
                  blurRadius: 12,
                  offset: const Offset(0, 4),
                ),
              ],
            ),
            child: Padding(
              padding: const EdgeInsets.all(6),
              child: Row(
            children: [
              Expanded(
                child: Focus(
                  focusNode: _focusNode,
                  onKeyEvent: inputDisabled ? null : _handleKey,
                  child: Stack(
                    children: [
                      // 灰色 ghost-text 补全(对齐输入文字,垫在 TextField 下层)。
                      if (ghost.isNotEmpty)
                        Positioned.fill(
                          child: Padding(
                            padding: const EdgeInsets.symmetric(
                                horizontal: 14, vertical: 10),
                            child: Text.rich(
                              TextSpan(children: [
                                TextSpan(
                                    text: widget.controller.text,
                                    style: const TextStyle(
                                        color: Colors.transparent)),
                                TextSpan(
                                    text: ghost,
                                    style: TextStyle(
                                        color: theme
                                            .colorScheme.onSurfaceVariant
                                            .withValues(alpha: 0.5))),
                              ]),
                            ),
                          ),
                        ),
                      AnimatedContainer(
                        duration: const Duration(milliseconds: 150),
                        decoration: BoxDecoration(
                          borderRadius: BorderRadius.circular(12),
                          border: Border(
                            left: BorderSide(
                              color: commandMode ? accent : Colors.transparent,
                              width: 3,
                            ),
                          ),
                        ),
                        child: TextField(
                          controller: widget.controller,
                          enabled: !inputDisabled,
                          maxLines: null,
                          keyboardType: TextInputType.multiline,
                          textInputAction: TextInputAction.newline,
                          decoration: InputDecoration(
                            hintText: disabled
                                ? widget.disabledHint
                                : widget.isStreaming
                                    ? 'Generating…'
                                    : '输入消息…  Enter 发送 · / 指令 · @ 引用文件',
                            border: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(12),
                              borderSide:
                                  BorderSide(color: theme.colorScheme.outline),
                            ),
                            enabledBorder: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(12),
                              borderSide: BorderSide(
                                  color: commandMode
                                      ? accent
                                      : theme.colorScheme.outline),
                            ),
                            contentPadding: const EdgeInsets.symmetric(
                                horizontal: 14, vertical: 10),
                            isDense: true,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 8),
              if (widget.isStreaming)
                IconButton.filled(
                  onPressed: widget.onStop,
                  icon: const Icon(Icons.stop_rounded),
                  tooltip: 'Stop',
                  style: IconButton.styleFrom(
                    backgroundColor: theme.colorScheme.error,
                    foregroundColor: theme.colorScheme.onError,
                  ),
                )
              else
                IconButton.filled(
                  onPressed: disabled ? null : widget.onSend,
                  icon: const Icon(Icons.send_rounded),
                  tooltip: 'Send',
                ),
            ],
          ),
            ),
          ),
        ],
      ),
    );
  }
}

/// 斜杠指令弹出菜单(输入框上方)。
/// @ 文件引用弹出菜单(输入框上方)。
class _FileMentionMenu extends StatelessWidget {
  const _FileMentionMenu({required this.files, required this.onPick});

  final List<String> files;
  final void Function(String name) onPick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: theme.colorScheme.outlineVariant),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (final f in files)
            InkWell(
              onTap: () => onPick(f),
              borderRadius: BorderRadius.circular(12),
              child: Padding(
                padding:
                    const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                child: Row(
                  children: [
                    Icon(Icons.insert_drive_file_outlined,
                        size: 16, color: theme.colorScheme.primary),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Text(f,
                          style: theme.textTheme.bodyMedium
                              ?.copyWith(fontFamily: 'monospace'),
                          overflow: TextOverflow.ellipsis),
                    ),
                    Text('引用',
                        style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant)),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _SlashMenu extends StatelessWidget {
  const _SlashMenu({required this.items, required this.onPick});

  final List<_SlashCmd> items;
  final void Function(String cmd) onPick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: theme.colorScheme.outlineVariant),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (final it in items)
            InkWell(
              onTap: () => onPick(it.cmd),
              borderRadius: BorderRadius.circular(12),
              child: Padding(
                padding:
                    const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                child: Row(
                  children: [
                    Icon(Icons.groups_outlined,
                        size: 18, color: theme.colorScheme.primary),
                    const SizedBox(width: 10),
                    Text(it.cmd,
                        style: theme.textTheme.bodyMedium?.copyWith(
                            fontWeight: FontWeight.w600,
                            fontFamily: 'monospace')),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(it.desc,
                          style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant),
                          overflow: TextOverflow.ellipsis),
                    ),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }
}
