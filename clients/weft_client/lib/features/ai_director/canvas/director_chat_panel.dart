import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../shared/theme/spacing.dart';
import 'canvas_state.dart';
import 'director_chat_state.dart';
import 'models/canvas_models.dart';
import 'models/workflow_blueprint.dart';

/// 右栏 — 导演 Agent 对话面板。消息流 + 输入框 + ask_user 选项。
class DirectorChatPanel extends ConsumerStatefulWidget {
  const DirectorChatPanel({super.key, this.contextHintBuilder});

  /// 可选：发送时把画布上下文（如选中节点标题）拼进去。
  final String Function()? contextHintBuilder;

  @override
  ConsumerState<DirectorChatPanel> createState() => _DirectorChatPanelState();
}

/// 一个可唤起的技能（对应导演 Agent 能力）。
class _SkillItem {
  const _SkillItem(this.name, this.label, this.icon, this.template);
  final String name;
  final String label;
  final IconData icon;
  final String template; // 选中后插入输入框的提示模板
}

class _DirectorChatPanelState extends ConsumerState<DirectorChatPanel> {
  final _controller = TextEditingController();
  final _scroll = ScrollController();

  /// 是否显示「/」技能菜单。
  bool _showSkills = false;

  static const _skills = <_SkillItem>[
    _SkillItem('generate_image', '生成图像', Icons.image_outlined,
        '生成一张图：'),
    _SkillItem('render_video', '合成视频', Icons.movie_outlined,
        '把这些镜头合成一段视频：'),
    _SkillItem('director_plan', '剪辑方案', Icons.dashboard_outlined,
        '给我一个剪辑方案：'),
    _SkillItem('delegate_to_team', '委派团队', Icons.groups_outlined,
        '把这个任务委派给创作团队：'),
    _SkillItem('ask_user', '反问澄清', Icons.help_outline,
        '我不确定方向，帮我理清：'),
  ];

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onInputChanged);
  }

  void _onInputChanged() {
    // 输入以「/」开头且尚未补全时，显示技能菜单。
    final show = _controller.text == '/' ||
        (_controller.text.startsWith('/') && !_controller.text.contains(' '));
    if (show != _showSkills) {
      setState(() => _showSkills = show);
    }
  }

  void _applySkill(_SkillItem skill) {
    _controller.text = skill.template;
    _controller.selection = TextSelection.fromPosition(
      TextPosition(offset: _controller.text.length),
    );
    setState(() => _showSkills = false);
  }

  @override
  void dispose() {
    _controller.removeListener(_onInputChanged);
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
        if (_showSkills) _skillsMenu(theme),
        _composer(theme, state.sending),
      ],
    );
  }

  /// 「/」技能菜单。
  Widget _skillsMenu(ThemeData theme) {
    return Container(
      margin: const EdgeInsets.symmetric(horizontal: Spacing.md),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: theme.colorScheme.outline.withValues(alpha: 0.2)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (final skill in _skills)
            InkWell(
              onTap: () => _applySkill(skill),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: Spacing.md, vertical: Spacing.sm),
                child: Row(
                  children: [
                    Icon(skill.icon, size: 16, color: theme.colorScheme.primary),
                    const SizedBox(width: Spacing.sm),
                    Text('/${skill.name}', style: theme.textTheme.bodySmall?.copyWith(
                      fontFeatures: const [],
                      color: theme.colorScheme.onSurface,
                    )),
                    const SizedBox(width: Spacing.sm),
                    Text(skill.label, style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    )),
                  ],
                ),
              ),
            ),
        ],
      ),
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
            if (m.blueprint != null) ...[
              const SizedBox(height: Spacing.sm),
              _blueprintCard(theme, m.blueprint!),
            ],
          ],
        ),
      ),
    );
  }

  /// 导演产出的工作流蓝图卡片：展示镜头数 + 一键铺到画布并生成。
  Widget _blueprintCard(ThemeData theme, WorkflowBlueprint bp) {
    final imageCount = bp.nodes.where((n) => n.kind == CanvasNodeKind.image).length;
    return Container(
      padding: const EdgeInsets.all(Spacing.sm),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: theme.colorScheme.primary.withValues(alpha: 0.4)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.account_tree_outlined, size: 16, color: theme.colorScheme.primary),
              const SizedBox(width: Spacing.xs),
              Expanded(
                child: Text(
                  bp.title.isEmpty ? '工作流方案' : bp.title,
                  style: theme.textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
                ),
              ),
            ],
          ),
          const SizedBox(height: 2),
          Text('$imageCount 个镜头 → 合成短片',
              style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
          const SizedBox(height: Spacing.sm),
          Row(
            children: [
              FilledButton.icon(
                onPressed: () => _applyBlueprint(bp, run: true),
                icon: const Icon(Icons.play_arrow, size: 16),
                label: const Text('铺到画布并生成'),
              ),
              const SizedBox(width: Spacing.xs),
              OutlinedButton(
                onPressed: () => _applyBlueprint(bp, run: false),
                child: const Text('只铺画布'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  void _applyBlueprint(WorkflowBlueprint bp, {required bool run}) {
    final notifier = ref.read(canvasProvider.notifier);
    notifier.applyBlueprint(bp);
    if (run) {
      notifier.runWorkflow();
      ScaffoldMessenger.maybeOf(context)?.showSnackBar(
        const SnackBar(content: Text('已铺开 DAG，开始并行出图 → 汇聚成片…')),
      );
    }
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
            IconButton(
              tooltip: '把创意拆成多镜头工作流 DAG',
              onPressed: sending ? null : () => _planWorkflow(),
              icon: const Icon(Icons.account_tree_outlined, size: 18),
            ),
            IconButton.filled(
              onPressed: sending ? null : () => _send(),
              icon: const Icon(Icons.send, size: 18),
            ),
          ],
        ),
      );

  void _planWorkflow() {
    final text = _controller.text;
    if (text.trim().isEmpty) return;
    ref.read(directorChatProvider.notifier).planWorkflow(text);
    _controller.clear();
    _scrollToBottomSoon();
  }
}
