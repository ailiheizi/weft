import 'package:flutter/material.dart';

import '../../../core/models/chat.dart';
import '../../../shared/widgets/glass_card.dart';
import 'tool_display_names.dart';

/// 工具调用状态（从 ToolCallStep 派生）。
enum ToolStatus { pending, success, error }

/// 从一个 ToolCallStep 推断状态。
///
/// result==null → pending（运行中）。
/// 否则尝试解析 result 里的结构化错误标记，命中 → error，否则 success。
/// 比纯子串匹配稳健：优先看 `"status":"error"` / `"success":false` 这类显式字段，
/// 再退回保守的关键词匹配。
ToolStatus toolStatusOf(ToolCallStep s) {
  if (s.result == null) return ToolStatus.pending;
  final r = s.result!.toLowerCase();
  // 显式结构化失败标记。
  if (r.contains('"status":"error"') ||
      r.contains('"status": "error"') ||
      r.contains('"success":false') ||
      r.contains('"success": false') ||
      r.contains('"ok":false') ||
      r.contains('"ok": false')) {
    return ToolStatus.error;
  }
  // 保守关键词兜底（仅在没有显式 success 标记时）。
  final looksOk = r.contains('"status":"ok"') ||
      r.contains('"success":true') ||
      r.contains('"ok":true');
  if (!looksOk &&
      (r.contains('"error"') ||
          r.contains('error:') ||
          r.contains('exception') ||
          r.contains('traceback'))) {
    return ToolStatus.error;
  }
  return ToolStatus.success;
}

/// 状态对应的语义前景/背景色（取 theme）。
({Color fg, Color bg}) toolStatusColors(ToolStatus st, ThemeData theme) {
  switch (st) {
    case ToolStatus.pending:
      return (
        fg: theme.colorScheme.onSurfaceVariant,
        bg: theme.colorScheme.surfaceContainerHigh.withValues(alpha: 0.4),
      );
    case ToolStatus.success:
      return (
        fg: theme.colorScheme.primary,
        bg: theme.colorScheme.primaryContainer.withValues(alpha: 0.22),
      );
    case ToolStatus.error:
      return (
        fg: theme.colorScheme.error,
        bg: theme.colorScheme.errorContainer.withValues(alpha: 0.28),
      );
  }
}

/// 状态图标。
IconData toolStatusIcon(ToolStatus st) {
  switch (st) {
    case ToolStatus.pending:
      return Icons.build_outlined;
    case ToolStatus.success:
      return Icons.check_circle_outline;
    case ToolStatus.error:
      return Icons.error_outline;
  }
}

/// 统一工具气泡外壳（对标 SonettoHere BubbleChrome.vue）。
///
/// 提供：状态点 + 友好名表头 + 可选 trailing（如"查看"链接）+ 点击展开 body。
/// running 状态默认展开；done/error 默认折叠（点表头展开）。
/// 专属气泡把领域化内容作为 [child] 传入；通用兜底也可复用此外壳。
class ToolBubbleChrome extends StatefulWidget {
  const ToolBubbleChrome({
    super.key,
    required this.step,
    required this.child,
    this.trailing,
    this.initiallyExpanded,
  });

  final ToolCallStep step;

  /// 展开后的领域化内容。
  final Widget child;

  /// 表头右侧附加件（如 artifact "查看" 按钮）。
  final Widget? trailing;

  /// 覆盖默认展开行为；null 时按状态决定（pending 展开，其余折叠）。
  final bool? initiallyExpanded;

  @override
  State<ToolBubbleChrome> createState() => _ToolBubbleChromeState();
}

class _ToolBubbleChromeState extends State<ToolBubbleChrome> {
  late bool _expanded;

  @override
  void initState() {
    super.initState();
    _expanded = widget.initiallyExpanded ??
        (toolStatusOf(widget.step) == ToolStatus.pending);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final st = toolStatusOf(widget.step);
    final c = toolStatusColors(st, theme);
    final label = toolDisplayName(widget.step.name);

    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: c.bg,
        border: Border.all(color: GlassTokens.borderIdleOf(context)),
        borderRadius: BorderRadius.circular(GlassTokens.radiusInner),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // ── 表头 ──
          InkWell(
            onTap: () => setState(() => _expanded = !_expanded),
            borderRadius: BorderRadius.circular(GlassTokens.radiusInner),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
              child: Row(
                children: [
                  if (st == ToolStatus.pending)
                    SizedBox(
                      width: 12,
                      height: 12,
                      child: CircularProgressIndicator(
                          strokeWidth: 1.5, color: c.fg),
                    )
                  else
                    Icon(toolStatusIcon(st), size: 13, color: c.fg),
                  const SizedBox(width: 6),
                  Flexible(
                    child: Text(
                      label,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        color: c.fg,
                      ),
                    ),
                  ),
                  if (widget.trailing != null) ...[
                    const SizedBox(width: 6),
                    widget.trailing!,
                  ],
                  const Spacer(),
                  Icon(_expanded ? Icons.expand_less : Icons.expand_more,
                      size: 15, color: c.fg),
                ],
              ),
            ),
          ),
          // ── body ──
          if (_expanded)
            Padding(
              padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
              child: widget.child,
            ),
        ],
      ),
    );
  }
}
