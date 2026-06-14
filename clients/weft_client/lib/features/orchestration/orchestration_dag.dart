import 'package:flutter/material.dart';

import '../../core/models/orchestration.dart';

/// 编排 DAG 视图:任务按阶段列布局(接收→…→完成),
/// 同阶段并行任务竖向堆叠,依赖(depends_on)与阶段流以连线表示。
/// 直观展示多 agent 并行(同列多节点)与 fan-out/join。
class OrchestrationDag extends StatelessWidget {
  const OrchestrationDag({super.key, required this.tasks});

  final List<TeamTask> tasks;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    if (tasks.isEmpty) {
      return Center(
        child: Text('暂无任务节点',
            style: TextStyle(color: theme.colorScheme.outline)),
      );
    }

    // 按阶段分组(列)。
    final byPhase = <String, List<TeamTask>>{};
    for (final t in tasks) {
      byPhase.putIfAbsent(t.phase, () => []).add(t);
    }
    // 只展示有任务的阶段列,但保持标准顺序。
    final columns = kWorkflowPhases.where(byPhase.containsKey).toList();

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      padding: const EdgeInsets.all(16),
      child: IntrinsicHeight(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (var i = 0; i < columns.length; i++) ...[
              _PhaseColumn(
                phase: columns[i],
                tasks: byPhase[columns[i]]!,
              ),
              if (i < columns.length - 1) const _FlowConnector(),
            ],
          ],
        ),
      ),
    );
  }
}

class _PhaseColumn extends StatelessWidget {
  const _PhaseColumn({required this.phase, required this.tasks});
  final String phase;
  final List<TeamTask> tasks;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
          decoration: BoxDecoration(
            color: theme.colorScheme.primaryContainer,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text(
            kPhaseLabels[phase] ?? phase,
            style: theme.textTheme.labelMedium
                ?.copyWith(color: theme.colorScheme.onPrimaryContainer),
          ),
        ),
        const SizedBox(height: 10),
        // 同阶段多个任务 = 并行节点,竖向堆叠。
        for (final t in tasks) _DagNode(task: t),
      ],
    );
  }
}

class _DagNode extends StatelessWidget {
  const _DagNode({required this.task});
  final TeamTask task;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final done = task.status == 'integrated' ||
        task.status == 'review_completed' ||
        task.phase == 'done';
    final color = done
        ? theme.colorScheme.primary
        : theme.colorScheme.outlineVariant;
    final isParent = task.dependsOn.isNotEmpty;
    return Container(
      width: 150,
      margin: const EdgeInsets.only(bottom: 10),
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: color, width: done ? 2 : 1),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                isParent
                    ? Icons.call_merge
                    : (done ? Icons.check_circle : Icons.radio_button_unchecked),
                size: 14,
                color: color,
              ),
              const SizedBox(width: 5),
              Expanded(
                child: Text(
                  task.ownerMemberId.isEmpty ? '任务' : task.ownerMemberId,
                  style: theme.textTheme.labelSmall
                      ?.copyWith(color: theme.colorScheme.primary),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            task.title,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.bodySmall,
          ),
          const SizedBox(height: 4),
          Text(
            task.status,
            style: theme.textTheme.labelSmall
                ?.copyWith(color: theme.colorScheme.outline),
          ),
        ],
      ),
    );
  }
}

/// 阶段间的流向箭头连接。
class _FlowConnector extends StatelessWidget {
  const _FlowConnector();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: 30),
      child: Icon(Icons.arrow_forward,
          size: 20, color: theme.colorScheme.outline),
    );
  }
}
