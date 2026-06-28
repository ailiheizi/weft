import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../shared/widgets/glass_card.dart';

/// 键值表原子组件：把 `Map<String, dynamic>` 渲染成对齐的 key/value 两列。
///
/// 设计（对标专属气泡的"结构化→富展示"风格）：
/// - key 列等宽对齐（`IntrinsicColumnWidth`），右侧 value 列自适应填充。
/// - value 区分两类：
///     · 基元（String/num/bool/null）→ 单行可选文本，超长省略号。
///     · 嵌套对象 / 列表 → 缩进 JSON 串，放进限高(`maxNestedHeight`)可滚动框。
/// - 条目过多时只显示前 [maxRows] 条，附"仅显示前 N / 共 M 项"提示。
///
/// 通用气泡用它展示入参；result 能 JSON 解析为 Map 时也可复用。
class KvTable extends StatelessWidget {
  const KvTable({
    super.key,
    required this.data,
    this.maxRows = 20,
    this.maxNestedHeight = 120,
  });

  /// 待展示的键值数据。
  final Map<String, dynamic> data;

  /// 最多展示的条目数；超出折叠为提示行。
  final int maxRows;

  /// 嵌套值（对象/列表）滚动框的最大高度。
  final double maxNestedHeight;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final entries = data.entries.toList();
    final total = entries.length;
    final shown = total > maxRows ? maxRows : total;
    final visible = entries.take(shown).toList();

    if (total == 0) {
      return Text(
        '（无参数）',
        style: TextStyle(
          fontSize: 11,
          color: theme.colorScheme.onSurfaceVariant,
        ),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Table(
          columnWidths: const {
            0: IntrinsicColumnWidth(),
            1: FlexColumnWidth(),
          },
          defaultVerticalAlignment: TableCellVerticalAlignment.top,
          children: [
            for (final e in visible)
              TableRow(
                children: [
                  // ── key 列 ──
                  Padding(
                    padding: const EdgeInsets.only(right: 10, bottom: 4),
                    child: Text(
                      e.key,
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        fontFamily: 'monospace',
                        color: theme.colorScheme.primary,
                      ),
                    ),
                  ),
                  // ── value 列 ──
                  Padding(
                    padding: const EdgeInsets.only(bottom: 4),
                    child: _value(context, e.value),
                  ),
                ],
              ),
          ],
        ),
        // ── 截断提示 ──
        if (total > shown)
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Text(
              '仅显示前 $shown / 共 $total 项',
              style: TextStyle(
                fontSize: 10,
                fontStyle: FontStyle.italic,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
      ],
    );
  }

  /// 渲染单个 value：基元单行文本；嵌套对象/列表转缩进 JSON 放进限高滚动框。
  Widget _value(BuildContext context, dynamic value) {
    final theme = Theme.of(context);
    final isNested = value is Map || value is List;

    if (!isNested) {
      final text = value == null ? 'null' : value.toString();
      return SelectableText(
        text.isEmpty ? '""' : text,
        maxLines: 4,
        style: TextStyle(
          fontSize: 11,
          fontFamily: 'monospace',
          color: theme.colorScheme.onSurface,
        ),
      );
    }

    // 嵌套：缩进 JSON 串 + 限高滚动。
    String pretty;
    try {
      pretty = const JsonEncoder.withIndent('  ').convert(value);
    } catch (_) {
      pretty = value.toString();
    }
    return Container(
      width: double.infinity,
      constraints: BoxConstraints(maxHeight: maxNestedHeight),
      padding: const EdgeInsets.all(6),
      decoration: BoxDecoration(
        color: GlassTokens.innerTileFillOf(context),
        borderRadius: BorderRadius.circular(GlassTokens.radiusInner),
      ),
      child: SingleChildScrollView(
        child: SelectableText(
          pretty,
          style: TextStyle(
            fontSize: 11,
            fontFamily: 'monospace',
            color: theme.colorScheme.onSurface,
          ),
        ),
      ),
    );
  }
}
