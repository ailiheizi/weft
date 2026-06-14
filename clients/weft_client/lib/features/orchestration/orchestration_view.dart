import 'package:flutter/material.dart';

import '../../core/models/orchestration.dart';
import 'orchestration_dag.dart';

/// 编排进度视图(阶段流水线/DAG + 活动流),被独立"团队"页和聊天工作区内嵌复用。
/// 纯展示,不含状态;调用方传入 tasks/handoffs/activity。
class OrchestrationView extends StatefulWidget {
  const OrchestrationView({
    super.key,
    required this.tasks,
    required this.handoffs,
    required this.activity,
    this.compact = false,
  });

  final List<TeamTask> tasks;
  final List<Handoff> handoffs;
  final List<ActivityEntry> activity;

  /// compact=true 用于聊天工作区窄面板(竖向堆叠)。
  final bool compact;

  @override
  State<OrchestrationView> createState() => _OrchestrationViewState();
}

class _OrchestrationViewState extends State<OrchestrationView> {
  bool _dagMode = false;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // 视图切换:流水线 / DAG。
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 8, 12, 0),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              SegmentedButton<bool>(
                style: const ButtonStyle(
                  visualDensity: VisualDensity.compact,
                ),
                segments: const [
                  ButtonSegment(value: false, label: Text('流水线'), icon: Icon(Icons.linear_scale, size: 14)),
                  ButtonSegment(value: true, label: Text('DAG'), icon: Icon(Icons.account_tree_outlined, size: 14)),
                ],
                selected: {_dagMode},
                onSelectionChanged: (s) => setState(() => _dagMode = s.first),
              ),
            ],
          ),
        ),
        SizedBox(
          height: _dagMode ? 260 : null,
          child: _dagMode
              ? OrchestrationDag(tasks: widget.tasks)
              : Padding(
                  padding: const EdgeInsets.fromLTRB(12, 12, 12, 8),
                  child: PhasePipeline(tasks: widget.tasks),
                ),
        ),
        const Divider(height: 1),
        Expanded(
          child: ActivityFeed(
              activity: widget.activity, handoffs: widget.handoffs),
        ),
      ],
    );
  }
}

/// 阶段流水线:6 阶段,按任务最靠后的 phase 高亮进度。
class PhasePipeline extends StatelessWidget {
  const PhasePipeline({super.key, required this.tasks});
  final List<TeamTask> tasks;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
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
    final color =
        active ? theme.colorScheme.primary : theme.colorScheme.outline;
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

/// 活动流:展示 handoff 进度 + agent 回复(delegate_reply 事件)。
class ActivityFeed extends StatelessWidget {
  const ActivityFeed(
      {super.key, required this.activity, required this.handoffs});
  final List<ActivityEntry> activity;
  final List<Handoff> handoffs;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 10, 12, 8),
          child: Row(
            children: [
              const Icon(Icons.timeline, size: 16),
              const SizedBox(width: 6),
              Text('活动流', style: theme.textTheme.titleSmall),
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
                  padding: const EdgeInsets.all(10),
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
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: isReply
            ? theme.colorScheme.surfaceContainerHighest
            : theme.colorScheme.surfaceContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(isReply ? Icons.smart_toy_outlined : Icons.bolt,
                  size: 13, color: theme.colorScheme.primary),
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
            const SizedBox(height: 5),
            SelectableText(entry.summary, style: theme.textTheme.bodySmall),
          ],
        ],
      ),
    );
  }
}
