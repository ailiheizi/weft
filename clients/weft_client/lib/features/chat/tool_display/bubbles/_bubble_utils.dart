import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../../shared/widgets/glass_card.dart';

/// 气泡共享：JSON 解析 helper + raw 兜底渲染。
///
/// 所有专属气泡复用这里的解析逻辑，做到"结构化数据→富展示，失败→raw 文本兜底"。

/// 尝试把字符串解析为 JSON Map；失败返回 null。
Map<String, dynamic>? tryJsonMap(String? raw) {
  if (raw == null || raw.trim().isEmpty) return null;
  try {
    final v = jsonDecode(raw);
    return v is Map<String, dynamic> ? v : null;
  } catch (_) {
    return null;
  }
}

/// 尝试把字符串解析为 JSON List；失败返回 null。
List<dynamic>? tryJsonList(String? raw) {
  if (raw == null || raw.trim().isEmpty) return null;
  try {
    final v = jsonDecode(raw);
    return v is List ? v : null;
  } catch (_) {
    return null;
  }
}

/// 从 Map 取字符串字段（非空）；缺失/空返回 null。
String? jsonStr(Map<String, dynamic>? m, String key) {
  final v = m?[key];
  if (v == null) return null;
  final s = v is String ? v : v.toString();
  return s.isEmpty ? null : s;
}

/// 在 JSON 里找"结果列表"。兼容常见包裹：顶层 List、或 `results`/`data`/`items`
/// 字段、或 `data.results`。找不到返回空列表。
List<dynamic> findResultList(String? raw) {
  final list = tryJsonList(raw);
  if (list != null) return list;
  final m = tryJsonMap(raw);
  if (m == null) return const [];
  for (final k in ['results', 'data', 'items', 'matches']) {
    final v = m[k];
    if (v is List) return v;
    if (v is Map<String, dynamic>) {
      for (final kk in ['results', 'items']) {
        if (v[kk] is List) return v[kk] as List;
      }
    }
  }
  return const [];
}

/// raw 文本兜底块（等宽、限高、可滚动）。
class RawOutput extends StatelessWidget {
  const RawOutput({super.key, required this.text, this.maxHeight = 240});

  final String text;
  final double maxHeight;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final trimmed = text.trim();
    return Container(
      width: double.infinity,
      constraints: BoxConstraints(maxHeight: maxHeight),
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: GlassTokens.innerTileFillOf(context),
        borderRadius: BorderRadius.circular(GlassTokens.radiusInner),
      ),
      child: SingleChildScrollView(
        child: SelectableText(
          trimmed.isEmpty ? '（无输出）' : trimmed,
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

/// 小标签（如"读取" / "写入" / 相关度）。
class BubbleTag extends StatelessWidget {
  const BubbleTag({super.key, required this.text, this.icon});

  final String text;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.6),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 10, color: theme.colorScheme.onSurfaceVariant),
            const SizedBox(width: 3),
          ],
          Text(
            text,
            style: TextStyle(
              fontSize: 10,
              fontWeight: FontWeight.w600,
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}
