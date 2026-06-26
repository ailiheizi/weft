import 'package:flutter/material.dart';

import '../../../core/models/chat.dart';
import 'bubbles/web_search_bubble.dart';
import 'bubbles/file_bubble.dart';
import 'bubbles/shell_bubble.dart';

/// 工具专属气泡构建器：拿到 ToolCallStep，返回领域化展示 widget。
typedef ToolBubbleBuilder = Widget Function(ToolCallStep step);

/// 工具名 → 专属气泡构建器注册表（对标 SonettoHere registry.ts）。
///
/// **设计原则（opt-in 增强，非穷举）**：
/// - 只为"富展示有价值"的少数工具登记专属气泡。
/// - 未登记的工具 → [bubbleBuilderFor] 返回 null → 调用方回落到现有通用渲染。
/// - 新增工具无需改动即可正常显示；想要富展示时再补一个气泡条目。
///
/// 按 weft_client **真实工具名**登记（不是 SonettoHere 的工具名）。
const Map<String, ToolBubbleBuilder> _registry = {
  // ── 网络搜索 ──
  'web_search': WebSearchBubble.create,
  'tavily_search': WebSearchBubble.create,
  // 注：firecrawl_search 暂不登记——走 IP 限流免费云端，输出稳定性未验证，
  // 让它走通用兜底渲染更稳妥；确认输出结构与 web_search 一致后再加回。

  // ── 文件读写 ──
  'fs_read': FileBubble.create,
  'fs_write': FileBubble.create,
  'file_read': FileBubble.create,
  'file_write': FileBubble.create,

  // ── 终端 / 命令 ──
  'shell_exec': ShellBubble.create,
  'run_command': ShellBubble.create,
  'git': ShellBubble.create,
};

/// 取工具的专属气泡构建器；未登记返回 null（调用方走通用兜底）。
///
/// MCP 工具（`mcp:server:tool`）按 tool 末段再查一次，让同名 MCP 工具
/// （如 `mcp:tavily:tavily_search`）也能命中已有气泡。
ToolBubbleBuilder? bubbleBuilderFor(String name) {
  final trimmed = name.trim();
  if (trimmed.isEmpty) return null;

  final exact = _registry[trimmed];
  if (exact != null) return exact;

  if (trimmed.startsWith('mcp:')) {
    final parts = trimmed.split(':');
    if (parts.length >= 3) {
      final tool = parts.sublist(2).join(':');
      return _registry[tool];
    }
  }
  return null;
}
