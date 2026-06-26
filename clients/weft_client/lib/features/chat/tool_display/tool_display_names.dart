/// 工具名 → 友好显示名（动态人性化 + 极小可选覆盖）。
///
/// 设计：**不**维护"所有工具"的大静态表（工具集会变，MCP 工具名 `mcp:server:tool`
/// 是动态的，静态表必然漏、必然过时）。改为：
///   1. 极小覆盖表 `_overrides`：只收"自动人性化效果差"的少数名字（缩写/全小写专名）。
///   2. `_humanize`：对任意工具名兜底——下划线/中划线转空格、词首大写。
/// 新工具 / 新 MCP 工具天然有个像样的名字，零维护。
library;

/// 极小可选覆盖表：仅当 `_humanize` 结果不理想时才加一条。
/// 不是"枚举所有工具"——绝大多数工具靠 `_humanize` 自动处理。
const Map<String, String> _overrides = {
  'fs_read': '读取文件',
  'fs_write': '写入文件',
  'fs_list': '文件列表',
  'shell_exec': '执行命令',
  'git': 'Git 操作',
  'web_search': '网络搜索',
  'web_fetch': '网页抓取',
  'sequentialthinking': '顺序推理',
  'delegate_to_team': '团队编排',
};

/// 取工具的友好显示名。
///
/// 顺序：精确覆盖 → MCP `mcp:server:tool`（tool 段覆盖 → `server · 人性化(tool)`）
/// → 人性化原始名。任意未知工具都能得到一个合理名字。
String toolDisplayName(String name) {
  final trimmed = name.trim();
  if (trimmed.isEmpty) return 'Tool call';

  final exact = _overrides[trimmed];
  if (exact != null) return exact;

  // MCP 工具：mcp:server:tool。
  if (trimmed.startsWith('mcp:')) {
    final parts = trimmed.split(':');
    if (parts.length >= 3) {
      final tool = parts.sublist(2).join(':');
      final byTool = _overrides[tool];
      if (byTool != null) return byTool;
      // server · 人性化(tool)，去掉 mcp: 前缀。
      return '${parts[1]} · ${_humanize(tool)}';
    }
  }

  return _humanize(trimmed);
}

/// 把 snake_case / kebab-case / camelCase 转成"词首大写 + 空格分隔"。
/// 例：`run_python` → `Run Python`，`getUserInfo` → `Get User Info`。
String _humanize(String raw) {
  if (raw.isEmpty) return raw;
  // camelCase → 在小写后接大写处插下划线。
  final spaced = raw.replaceAllMapped(
    RegExp(r'([a-z0-9])([A-Z])'),
    (m) => '${m[1]}_${m[2]}',
  );
  final words = spaced
      .split(RegExp(r'[_\-\s]+'))
      .where((w) => w.isNotEmpty)
      .map((w) => w[0].toUpperCase() + w.substring(1))
      .toList();
  return words.isEmpty ? raw : words.join(' ');
}
