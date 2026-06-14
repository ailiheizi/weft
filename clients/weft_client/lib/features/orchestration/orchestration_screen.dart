import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/models/orchestration.dart';
import 'orchestration_provider.dart';

/// 多 agent 编排界面（工作区风格：左输入+阶段流水线，右活动流）。
class OrchestrationScreen extends ConsumerStatefulWidget {
  const OrchestrationScreen({super.key});

  @override
  ConsumerState<OrchestrationScreen> createState() =>
      _OrchestrationScreenState();
}

class _OrchestrationScreenState extends ConsumerState<OrchestrationScreen> {
  final _goalController = TextEditingController();

  @override
  void dispose() {
    _goalController.dispose();
    super.dispose();
  }

  void _start() {
    final goal = _goalController.text.trim();
    if (goal.isEmpty) return;
    ref.read(orchestrationProvider.notifier).startDemo(goal);
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(orchestrationProvider);
    final theme = Theme.of(context);

    return Row(
      children: [
        // ── 左：目标输入 + 阶段流水线 + 任务 ──────────────────────────────
        Expanded(
          flex: 3,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Text('多 Agent 团队编排',
                        style: theme.textTheme.titleLarge),
                    const SizedBox(height: 4),
                    Text(
                      '输入一个目标，团队（规划→执行→评审→集成）会自动协作完成。',
                      style: theme.textTheme.bodySmall
                          ?.copyWith(color: theme.colorScheme.outline),
                    ),
                    const SizedBox(height: 12),
                    Row(
                      children: [
                        Expanded(
                          child: TextField(
                            controller: _goalController,
                            decoration: const InputDecoration(
                              hintText: '例如：实现一个带校验的登录表单',
                              border: OutlineInputBorder(),
                              isDense: true,
                            ),
                            onSubmitted: (_) => _start(),
                          ),
                        ),
                        const SizedBox(width: 8),
                        FilledButton.icon(
                          onPressed: state.running ? null : _start,
                          icon: const Icon(Icons.groups, size: 18),
                          label: Text(state.running ? '运行中' : '启动团队'),
                        ),
                      ],
                    ),
                    if (state.error != null) ...[
                      const SizedBox(height: 8),
                      Text(state.error!,
                          style: TextStyle(color: theme.colorScheme.error)),
                    ],
                  ],
                ),
              ),
              const Divider(height: 1),
              // 阶段流水线
              Padding(
                padding: const EdgeInsets.symmetric(
                    horizontal: 16, vertical: 12),
                child: _PhasePipeline(tasks: state.tasks),
              ),
              const Divider(height: 1),
              // 任务列表
              Expanded(
                child: state.tasks.isEmpty
                    ? Center(
                        child: Text(
                          state.running ? '正在初始化…' : '尚无任务',
                          style: TextStyle(color: theme.colorScheme.outline),
                        ),
                      )
                    : ListView(
                        padding: const EdgeInsets.all(12),
                        children: [
                          for (final t in state.tasks)
                            _TaskCard(task: t),
                        ],
                      ),
              ),
            ],
          ),
        ),
        const VerticalDivider(width: 1),
        // ── 右：活动流（agent 执行结果）──────────────────────────────────
        Expanded(
          flex: 2,
          child: _ActivityFeed(
            activity: state.activity,
            handoffs: state.handoffs,
          ),
        ),
      ],
    );
  }
}

/// 阶段流水线：6 个阶段，按任务的 phase 高亮当前进度。
class _PhasePipeline extends StatelessWidget {
  const _PhasePipeline({required this.tasks});
  final List<TeamTask> tasks;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    // 当前进度 = 所有任务里最靠后的阶段。
    int maxIdx = 0;
    for (final t in tasks) {
      final i = kWorkflowPhases.indexOf(t.phase);
      if (i > maxIdx) maxIdx = i;
    }
    return Row(
      children: [
        for (var i = 0; i < kWorkflowPhases.length; i++) ...[
          if (i > 0)
            Expanded(
              child: Container(
                height: 2,
                color: i <= maxIdx
                    ? theme.colorScheme.primary
                    : theme.colorScheme.outlineVariant,
              ),
            ),
          _PhaseDot(
            label: kPhaseLabels[kWorkflowPhases[i]] ?? kWorkflowPhases[i],
            active: i <= maxIdx,
            current: i == maxIdx && tasks.isNotEmpty,
          ),
        ],
      ],
    );
  }
}

class _PhaseDot extends StatelessWidget {
  const _PhaseDot(
      {required this.label, required this.active, required this.current});
  final String label;
  final bool active;
  final bool current;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = active ? theme.colorScheme.primary : theme.colorScheme.outline;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: current ? 16 : 12,
          height: current ? 16 : 12,
          decoration: BoxDecoration(
            color: active ? color : Colors.transparent,
            border: Border.all(color: color, width: 2),
            shape: BoxShape.circle,
          ),
        ),
        const SizedBox(height: 4),
        Text(label,
            style: theme.textTheme.labelSmall?.copyWith(color: color)),
      ],
    );
  }
}

class _TaskCard extends StatelessWidget {
  const _TaskCard({required this.task});
  final TeamTask task;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ListTile(
        leading: const Icon(Icons.task_alt),
        title: Text(task.title),
        subtitle: Text(
          '阶段：${kPhaseLabels[task.phase] ?? task.phase} · 状态：${task.status}',
          style: theme.textTheme.bodySmall,
        ),
        trailing: Chip(
          label: Text(task.ownerMemberId.isEmpty ? '—' : task.ownerMemberId),
          visualDensity: VisualDensity.compact,
        ),
      ),
    );
  }
}

/// 活动流：展示 handoff 进度 + agent 回复（delegate_reply 事件）。
class _ActivityFeed extends StatelessWidget {
  const _ActivityFeed({required this.activity, required this.handoffs});
  final List<ActivityEntry> activity;
  final List<Handoff> handoffs;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              const Icon(Icons.timeline, size: 18),
              const SizedBox(width: 8),
              Text('活动流', style: theme.textTheme.titleMedium),
              const Spacer(),
              Text('${handoffs.length} handoff',
                  style: theme.textTheme.bodySmall
                      ?.copyWith(color: theme.colorScheme.outline)),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: activity.isEmpty
              ? Center(
                  child: Text('等待 agent 执行…',
                      style: TextStyle(color: theme.colorScheme.outline)),
                )
              : ListView.builder(
                  padding: const EdgeInsets.all(12),
                  itemCount: activity.length,
                  itemBuilder: (c, i) {
                    final e = activity[activity.length - 1 - i];
                    return _ActivityTile(entry: e);
                  },
                ),
        ),
      ],
    );
  }
}

class _ActivityTile extends StatelessWidget {
  const _ActivityTile({required this.entry});
  final ActivityEntry entry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isReply = entry.eventType == 'delegate_reply';
    return Container(
      margin: const EdgeInsets.only(bottom: 10),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: isReply
            ? theme.colorScheme.surfaceContainerHighest
            : theme.colorScheme.surfaceContainer,
        borderRadius: BorderRadius.circular(10),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(isReply ? Icons.smart_toy_outlined : Icons.bolt,
                  size: 14, color: theme.colorScheme.primary),
              const SizedBox(width: 6),
              Text(
                entry.actorMemberId.isEmpty
                    ? entry.eventType
                    : '${entry.actorMemberId} · ${entry.eventType}',
                style: theme.textTheme.labelMedium
                    ?.copyWith(color: theme.colorScheme.primary),
              ),
            ],
          ),
          if (entry.summary.isNotEmpty) ...[
            const SizedBox(height: 6),
            SelectableText(entry.summary,
                style: theme.textTheme.bodySmall),
          ],
        ],
      ),
    );
  }
}
