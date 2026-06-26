import 'package:flutter/material.dart';

import '../../../../core/models/chat.dart';
import '../tool_bubble_chrome.dart';
import '_bubble_utils.dart';

/// 网络搜索类工具气泡（web_search / tavily_search / firecrawl_search）。
///
/// 富展示：查询栏（query + 结果数）→ 可选 AI 摘要 → 带相关度的排名结果列表。
/// 解析失败回退 raw 文本。对标 SonettoHere TavilySearchBubble.vue。
class WebSearchBubble extends StatelessWidget {
  const WebSearchBubble({super.key, required this.step});

  final ToolCallStep step;

  /// 注册表用的构建器。
  static Widget create(ToolCallStep step) => WebSearchBubble(step: step);

  @override
  Widget build(BuildContext context) {
    final status = toolStatusOf(step);
    return ToolBubbleChrome(
      step: step,
      child: _body(context, status),
    );
  }

  Widget _body(BuildContext context, ToolStatus status) {
    if (status == ToolStatus.pending) {
      return _RunningHint(text: '正在搜索…');
    }

    final argsMap = tryJsonMap(step.arguments);
    final query = jsonStr(argsMap, 'query') ??
        jsonStr(argsMap, 'q') ??
        jsonStr(tryJsonMap(step.result), 'query');

    final resultMap = tryJsonMap(step.result);
    final answer = jsonStr(resultMap, 'answer');
    final items = findResultList(step.result);

    // 无法结构化解析 → raw 兜底。
    if (items.isEmpty && answer == null) {
      return RawOutput(text: step.result ?? '');
    }

    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // ── 查询栏 ──
        if (query != null)
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Row(
              children: [
                Icon(Icons.search, size: 13, color: theme.colorScheme.primary),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    query,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                        fontSize: 13, fontWeight: FontWeight.w600),
                  ),
                ),
                const SizedBox(width: 8),
                Text(
                  '${items.length} 条',
                  style: TextStyle(
                      fontSize: 11, color: theme.colorScheme.onSurfaceVariant),
                ),
              ],
            ),
          ),
        // ── AI 摘要 ──
        if (answer != null) ...[
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(10),
            margin: const EdgeInsets.only(bottom: 8),
            decoration: BoxDecoration(
              color: theme.colorScheme.primaryContainer.withValues(alpha: 0.2),
              borderRadius: BorderRadius.circular(6),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('AI 摘要',
                    style: TextStyle(
                      fontSize: 9,
                      fontWeight: FontWeight.w700,
                      letterSpacing: 0.8,
                      color: theme.colorScheme.onSurfaceVariant,
                    )),
                const SizedBox(height: 4),
                SelectableText(answer,
                    style: const TextStyle(fontSize: 12, height: 1.6)),
              ],
            ),
          ),
        ],
        // ── 结果列表 ──
        for (var i = 0; i < items.length; i++)
          _ResultItem(rank: i + 1, raw: items[i]),
      ],
    );
  }
}

class _ResultItem extends StatelessWidget {
  const _ResultItem({required this.rank, required this.raw});

  final int rank;
  final dynamic raw;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final m = raw is Map<String, dynamic> ? raw as Map<String, dynamic> : null;
    if (m == null) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 6),
        child: Text(raw.toString(),
            style: const TextStyle(fontSize: 12)),
      );
    }
    final title = jsonStr(m, 'title') ?? jsonStr(m, 'url') ?? '结果 $rank';
    final url = jsonStr(m, 'url');
    final content = jsonStr(m, 'content') ?? jsonStr(m, 'snippet');
    final score = m['score'];
    final scorePct =
        score is num ? '${(score * 100).toStringAsFixed(0)}%' : null;

    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 6),
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        border: Border.all(color: theme.colorScheme.outlineVariant),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('$rank',
                  style: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w700,
                    color: theme.colorScheme.primary,
                  )),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  title,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                      fontSize: 13, fontWeight: FontWeight.w600),
                ),
              ),
              if (scorePct != null) ...[
                const SizedBox(width: 6),
                BubbleTag(text: scorePct),
              ],
            ],
          ),
          if (url != null)
            Padding(
              padding: const EdgeInsets.only(left: 19, top: 2),
              child: Text(
                url,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                    fontSize: 11, color: theme.colorScheme.onSurfaceVariant),
              ),
            ),
          if (content != null)
            Padding(
              padding: const EdgeInsets.only(left: 19, top: 4),
              child: Text(
                content,
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                    fontSize: 12,
                    height: 1.5,
                    color: theme.colorScheme.onSurface
                        .withValues(alpha: 0.85)),
              ),
            ),
        ],
      ),
    );
  }
}

class _RunningHint extends StatelessWidget {
  const _RunningHint({required this.text});
  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        SizedBox(
          width: 12,
          height: 12,
          child: CircularProgressIndicator(
              strokeWidth: 1.5, color: theme.colorScheme.secondary),
        ),
        const SizedBox(width: 8),
        Text(text,
            style: TextStyle(
                fontSize: 12, color: theme.colorScheme.onSurfaceVariant)),
      ],
    );
  }
}
