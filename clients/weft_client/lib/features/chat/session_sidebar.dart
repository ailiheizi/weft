import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../core/models/chat_session_meta.dart';
import '../../core/providers/sessions_provider.dart';
import 'workspace/chat_settings_panel.dart';

class SessionSidebar extends ConsumerStatefulWidget {
  const SessionSidebar({
    super.key,
    required this.activeSessionId,
    required this.onSelectSession,
    required this.onNewChat,
    this.width = 248,
  });

  final String? activeSessionId;
  final ValueChanged<String> onSelectSession;
  final VoidCallback onNewChat;
  final double width;

  @override
  ConsumerState<SessionSidebar> createState() => _SessionSidebarState();
}

class _SessionSidebarState extends ConsumerState<SessionSidebar> {
  String _query = '';
  bool _selectMode = false;
  final Set<String> _selected = {};

  /// 打开右侧统一设置抽屉(场景/技能/MCP/工具选择器)。
  void _openSettingsDrawer(BuildContext context) {
    showGeneralDialog<void>(
      context: context,
      barrierDismissible: true,
      barrierLabel: '设置',
      barrierColor: Colors.black54,
      transitionDuration: const Duration(milliseconds: 220),
      pageBuilder: (_, _, _) => const Align(
        alignment: Alignment.centerRight,
        child: ChatSettingsPanel(),
      ),
      transitionBuilder: (_, anim, _, child) {
        final curved =
            CurvedAnimation(parent: anim, curve: Curves.easeOutCubic);
        return SlideTransition(
          position: Tween<Offset>(
            begin: const Offset(1, 0),
            end: Offset.zero,
          ).animate(curved),
          child: child,
        );
      },
    );
  }

  /// 把会话按时间分组(今天/昨天/本周/更早),组内按 updatedAt 倒序。
  List<MapEntry<String, List<ChatSessionMeta>>> _groupByTime(
      List<ChatSessionMeta> sessions) {
    final now = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);
    final yesterday = today.subtract(const Duration(days: 1));
    final weekAgo = today.subtract(const Duration(days: 7));
    final groups = <String, List<ChatSessionMeta>>{
      '今天': [],
      '昨天': [],
      '本周': [],
      '更早': [],
    };
    final sorted = [...sessions]
      ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    for (final s in sorted) {
      final d = s.updatedAt;
      if (!d.isBefore(today)) {
        groups['今天']!.add(s);
      } else if (!d.isBefore(yesterday)) {
        groups['昨天']!.add(s);
      } else if (!d.isBefore(weekAgo)) {
        groups['本周']!.add(s);
      } else {
        groups['更早']!.add(s);
      }
    }
    return groups.entries.where((e) => e.value.isNotEmpty).toList();
  }

  void _toggleSelectMode() {
    setState(() {
      _selectMode = !_selectMode;
      _selected.clear();
    });
  }

  Future<void> _deleteSelected() async {
    if (_selected.isEmpty) return;
    final n = _selected.length;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('批量删除会话'),
        content: Text('确定删除选中的 $n 个会话?此操作不可撤销。'),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('取消')),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(
                backgroundColor: Theme.of(ctx).colorScheme.error),
            child: Text('删除 $n 个'),
          ),
        ],
      ),
    );
    if (ok == true) {
      await ref
          .read(sessionsProvider.notifier)
          .deleteSessions(_selected.toList());
      setState(() {
        _selected.clear();
        _selectMode = false;
      });
    }
  }

  Future<void> _clearAll() async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('清空全部会话'),
        content: const Text('确定删除所有会话?此操作不可撤销。'),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('取消')),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(
                backgroundColor: Theme.of(ctx).colorScheme.error),
            child: const Text('清空全部'),
          ),
        ],
      ),
    );
    if (ok == true) {
      await ref.read(sessionsProvider.notifier).clearAllSessions();
      setState(() {
        _selected.clear();
        _selectMode = false;
      });
    }
  }

  Future<void> _confirmDelete(ChatSessionMeta meta) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除会话'),
        content: Text('确定删除「${meta.title}」?此操作不可撤销。'),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('取消')),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(
                backgroundColor: Theme.of(ctx).colorScheme.error),
            child: const Text('删除'),
          ),
        ],
      ),
    );
    if (ok == true) {
      await ref.read(sessionsProvider.notifier).deleteSession(meta.id);
    }
  }

  Future<void> _rename(ChatSessionMeta meta) async {
    final controller = TextEditingController(text: meta.title);
    final newTitle = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('重命名会话'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(hintText: '会话标题'),
          onSubmitted: (v) => Navigator.pop(ctx, v.trim()),
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, controller.text.trim()),
              child: const Text('保存')),
        ],
      ),
    );
    if (newTitle != null && newTitle.isNotEmpty) {
      await ref.read(sessionsProvider.notifier).updateSessionTitle(meta.id, newTitle);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final sessions = ref.watch(sessionsProvider);
    final status = ref.watch(sessionsLoadStatusProvider);
    final q = _query.trim().toLowerCase();
    final filtered = q.isEmpty
        ? sessions
        : sessions
            .where((s) => s.title.toLowerCase().contains(q))
            .toList();

    return Container(
      width: widget.width,
      margin: const EdgeInsets.fromLTRB(8, 8, 4, 8),
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(16),
      ),
      child: Column(
        children: [
          // 新建会话 主按钮
          Padding(
            padding: const EdgeInsets.fromLTRB(8, 10, 8, 6),
            child: SizedBox(
              width: double.infinity,
              child: FilledButton(
                onPressed: widget.onNewChat,
                style: FilledButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 11),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12),
                  ),
                ),
                child: const Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.add, size: 16),
                    SizedBox(width: 6),
                    Text('新建会话', style: TextStyle(fontSize: 13)),
                  ],
                ),
              ),
            ),
          ),
          // 搜索框(会话数 >0 才显示)
          if (sessions.isNotEmpty)
            Padding(
              padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
              child: TextField(
                onChanged: (v) => setState(() => _query = v),
                style: const TextStyle(fontSize: 13),
                decoration: InputDecoration(
                  isDense: true,
                  prefixIcon: const Icon(Icons.search, size: 16),
                  hintText: '搜索会话…',
                  contentPadding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(10),
                  ),
                ),
              ),
            ),
          // 多选工具栏(会话数 >0 才显示)
          if (sessions.isNotEmpty)
            Padding(
              padding: const EdgeInsets.fromLTRB(8, 4, 4, 4),
              child: Row(
                children: [
                  if (_selectMode) ...[
                    Text('已选 ${_selected.length}',
                        style: const TextStyle(fontSize: 12)),
                    const Spacer(),
                    TextButton(
                      onPressed:
                          _selected.isEmpty ? null : () => _deleteSelected(),
                      child: const Text('删除', style: TextStyle(fontSize: 12)),
                    ),
                    TextButton(
                      onPressed: _toggleSelectMode,
                      child: const Text('取消', style: TextStyle(fontSize: 12)),
                    ),
                  ] else ...[
                    const Spacer(),
                    IconButton(
                      icon: const Icon(Icons.checklist, size: 16),
                      tooltip: '多选',
                      visualDensity: VisualDensity.compact,
                      onPressed: _toggleSelectMode,
                    ),
                    IconButton(
                      icon: const Icon(Icons.delete_sweep_outlined, size: 16),
                      tooltip: '清空全部',
                      visualDensity: VisualDensity.compact,
                      onPressed: _clearAll,
                    ),
                  ],
                ],
              ),
            ),
          // Session 列表(按时间分组)
          Expanded(
            child: filtered.isEmpty
                ? _SidebarEmpty(
                    status: status,
                    hasQuery: q.isNotEmpty,
                    onRetry: () => ref.read(sessionsProvider.notifier).refresh(),
                  )
                : ListView(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    children: [
                      for (final group in _groupByTime(filtered)) ...[
                        Padding(
                          padding:
                              const EdgeInsets.fromLTRB(14, 10, 14, 4),
                          child: Text(
                            group.key,
                            style: TextStyle(
                              fontSize: 11,
                              fontWeight: FontWeight.w600,
                              color: Theme.of(context)
                                  .colorScheme
                                  .onSurfaceVariant,
                            ),
                          ),
                        ),
                        for (final meta in group.value)
                          _SessionTile(
                            meta: meta,
                            isActive: meta.id == widget.activeSessionId,
                            selectMode: _selectMode,
                            selected: _selected.contains(meta.id),
                            onTap: () {
                              if (_selectMode) {
                                setState(() {
                                  if (!_selected.add(meta.id)) {
                                    _selected.remove(meta.id);
                                  }
                                });
                              } else {
                                widget.onSelectSession(meta.id);
                              }
                            },
                            onDelete: () => _confirmDelete(meta),
                            onRename: () => _rename(meta),
                          ),
                      ],
                    ],
                  ),
          ),
          // 底部设置入口(场景/技能/MCP/工具选择器)。
          const Divider(height: 1),
          InkWell(
            onTap: () => _openSettingsDrawer(context),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              child: Row(
                children: [
                  Icon(Icons.settings_outlined,
                      size: 18, color: theme.colorScheme.onSurfaceVariant),
                  const SizedBox(width: 10),
                  Text('设置',
                      style: TextStyle(
                          fontSize: 13,
                          color: theme.colorScheme.onSurfaceVariant)),
                  const Spacer(),
                  Icon(Icons.chevron_right,
                      size: 16, color: theme.colorScheme.onSurfaceVariant),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// 侧栏空态:区分 加载中 / core 未连接 / 真空 / 搜索无结果。
class _SidebarEmpty extends StatelessWidget {
  const _SidebarEmpty({
    required this.status,
    required this.hasQuery,
    required this.onRetry,
  });

  final SessionsLoadStatus status;
  final bool hasQuery;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final muted = TextStyle(fontSize: 12, color: theme.colorScheme.onSurfaceVariant);

    if (hasQuery) {
      return Center(child: Text('无匹配会话', style: muted));
    }
    switch (status) {
      case SessionsLoadStatus.loading:
        return const Center(
          child: SizedBox(
            width: 18, height: 18,
            child: CircularProgressIndicator(strokeWidth: 2),
          ),
        );
      case SessionsLoadStatus.failed:
        return Center(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.cloud_off_outlined,
                    size: 28, color: theme.colorScheme.onSurfaceVariant),
                const SizedBox(height: 8),
                Text('未连接到 weft-core', style: muted, textAlign: TextAlign.center),
                const SizedBox(height: 8),
                TextButton(onPressed: onRetry, child: const Text('重试')),
              ],
            ),
          ),
        );
      case SessionsLoadStatus.ready:
        return Center(child: Text('暂无会话,点 New Chat 开始', style: muted));
    }
  }
}

class _SessionTile extends StatefulWidget {
  const _SessionTile({
    required this.meta,
    required this.isActive,
    required this.onTap,
    required this.onDelete,
    required this.onRename,
    this.selectMode = false,
    this.selected = false,
  });

  final ChatSessionMeta meta;
  final bool isActive;
  final VoidCallback onTap;
  final VoidCallback onDelete;
  final VoidCallback onRename;
  final bool selectMode;
  final bool selected;

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
        onSecondaryTap: widget.onRename,
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
            // 激活态左侧 accent 条(对齐参考的干净选中样式)。
            border: widget.isActive
                ? Border(
                    left: BorderSide(
                      color: theme.colorScheme.primary,
                      width: 2.5,
                    ),
                  )
                : null,
          ),
          child: Row(
            children: [
              if (widget.selectMode)
                Padding(
                  padding: const EdgeInsets.only(right: 6),
                  child: Icon(
                    widget.selected
                        ? Icons.check_box
                        : Icons.check_box_outline_blank,
                    size: 18,
                    color: widget.selected
                        ? theme.colorScheme.primary
                        : theme.colorScheme.onSurfaceVariant,
                  ),
                ),
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
              if (_hovered || widget.isActive) ...[
                GestureDetector(
                  onTap: widget.onRename,
                  child: Tooltip(
                    message: '重命名',
                    child: Icon(
                      Icons.edit_outlined,
                      size: 15,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                GestureDetector(
                  onTap: widget.onDelete,
                  child: Tooltip(
                    message: '删除',
                    child: Icon(
                      Icons.delete_outline,
                      size: 15,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

