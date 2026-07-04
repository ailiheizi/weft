import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/api/mcp_api.dart'
    show McpServer, McpToolsInfo, McpDiagnostic;
import '../../../core/api/scene_api.dart' show Scene;
import '../../../core/providers/scenes_provider.dart';
import '../../../core/providers/skills_provider.dart';
import '../../../core/providers/mcp_provider.dart';

/// 聊天工作区内的「场景」管理面板。列出/激活/删除 Scene。
class ScenesPanel extends ConsumerWidget {
  const ScenesPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final state = ref.watch(scenesProvider);
    final notifier = ref.read(scenesProvider.notifier);

    return _PanelScaffold(
      title: '场景偏好',
      subtitle: '一套场景 = 启用插件 / provider 替换 / 模型路由 / 工作区',
      loading: state.loading,
      error: state.error,
      onRefresh: () => notifier.load(),
      empty: state.scenes.isEmpty,
      emptyHint: '暂无场景。场景定义一套偏好,激活后切换团队模型路由等。',
      children: [
        for (final scene in state.scenes)
          _Row(
            leading: Icon(
              scene.name == state.activeScene
                  ? Icons.check_circle
                  : Icons.circle_outlined,
              size: 18,
              color: scene.name == state.activeScene
                  ? theme.colorScheme.primary
                  : theme.colorScheme.onSurfaceVariant,
            ),
            title: scene.name,
            active: scene.name == state.activeScene,
            subtitle: _sceneSummary(scene),
            trailing: Row(mainAxisSize: MainAxisSize.min, children: [
              if (scene.name != state.activeScene)
                TextButton(
                  onPressed: state.loading ? null : () => notifier.bind(scene.name),
                  child: const Text('激活'),
                ),
              IconButton(
                icon: const Icon(Icons.delete_outline, size: 18),
                tooltip: scene.name == state.activeScene ? '不能删除激活场景' : '删除',
                onPressed: (state.loading || scene.name == state.activeScene)
                    ? null
                    : () => notifier.delete(scene.name),
              ),
            ]),
          ),
      ],
    );
  }

  static String _sceneSummary(Scene scene) {
    final parts = <String>[];
    if (scene.profile.isNotEmpty) parts.add('profile=${scene.profile}');
    if (scene.roleRouting.isNotEmpty) {
      parts.add('${scene.roleRouting.length} 角色路由');
    }
    if (scene.enabledFeatures.isNotEmpty) {
      parts.add('${scene.enabledFeatures.length} 启用');
    }
    if (scene.workspace.isNotEmpty) parts.add('工作区:${scene.workspace}');
    return parts.isEmpty
        ? (scene.description.isEmpty ? '(无额外偏好)' : scene.description)
        : parts.join(' · ');
  }
}

/// 聊天工作区内的「技能」管理面板 — 真实技能列表(ext.skills),支持启用/禁用。
class SkillsCatalogPanel extends ConsumerWidget {
  const SkillsCatalogPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final state = ref.watch(skillsCatalogProvider);
    final notifier = ref.read(skillsCatalogProvider.notifier);

    return _PanelScaffold(
      title: '技能',
      subtitle: 'agent 可用的工具技能 — 开关控制 AI 调用范围',
      loading: state.loading,
      error: state.error,
      onRefresh: () => notifier.load(),
      empty: state.skills.isEmpty,
      emptyHint: '暂无技能。技能由已加载的包提供,检查 core 是否正常。',
      children: [
        for (final skill in state.skills)
          _Row(
            leading: Icon(
              Icons.auto_awesome_outlined,
              size: 18,
              color: skill.enabled
                  ? theme.colorScheme.primary
                  : theme.colorScheme.onSurfaceVariant,
            ),
            title: skill.name,
            subtitle: skill.description,
            trailing: SizedBox(
              height: 28,
              child: Switch(
                value: skill.enabled,
                onChanged: state.loading
                    ? null
                    : (v) => notifier.toggle(skill.name, v),
                materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
              ),
            ),
          ),
      ],
    );
  }
}

/// 聊天工作区内的「演化技能」管理面板。
class SkillsPanel extends ConsumerWidget {
  const SkillsPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final state = ref.watch(skillsProvider);
    final notifier = ref.read(skillsProvider.notifier);

    return _PanelScaffold(
      title: '演化技能',
      subtitle: 'agent 从成功任务轨迹自动结晶的可复用流程',
      loading: state.loading,
      error: state.error,
      onRefresh: () => notifier.load(),
      empty: state.skills.isEmpty,
      emptyHint: '暂无技能。agent 积累成功经验后会在这里出现,可审批/查看。',
      children: [
        for (final skill in state.skills)
          _Row(
            leading: Icon(_skillIcon(skill.status),
                size: 18, color: _skillColor(skill.status, theme)),
            title: skill.title.isEmpty ? skill.id : skill.title,
            badges: [
              skill.status,
              if (skill.riskLevel.isNotEmpty && skill.riskLevel != 'low')
                'risk:${skill.riskLevel}',
            ],
            subtitle:
                '质量 ${(skill.qualityScore * 100).toStringAsFixed(0)}% · 用 ${skill.successfulUses}✓/${skill.failedUses}✗'
                '${skill.triggers.isNotEmpty ? ' · ${skill.triggers.length} 触发词' : ''}',
            trailing: (skill.reviewRequired || skill.status == 'pending_review')
                ? Row(mainAxisSize: MainAxisSize.min, children: [
                    TextButton(
                      onPressed: state.loading
                          ? null
                          : () => notifier.review(skill.id, true),
                      child: const Text('通过'),
                    ),
                    TextButton(
                      onPressed: state.loading
                          ? null
                          : () => notifier.review(skill.id, false),
                      style: TextButton.styleFrom(
                          foregroundColor: theme.colorScheme.error),
                      child: const Text('拒绝'),
                    ),
                  ])
                : null,
          ),
      ],
    );
  }

  static IconData _skillIcon(String s) => switch (s) {
        'active' => Icons.bolt,
        'pending_review' => Icons.rate_review_outlined,
        'archived' => Icons.archive_outlined,
        _ => Icons.auto_awesome_outlined,
      };
  static Color? _skillColor(String s, ThemeData t) => switch (s) {
        'active' => t.colorScheme.primary,
        'pending_review' => t.colorScheme.tertiary,
        'archived' => t.colorScheme.onSurfaceVariant,
        _ => null,
      };
}

/// 聊天工作区内的「MCP server」管理面板。
class McpPanel extends ConsumerWidget {
  const McpPanel({super.key});

  /// 添加入口:粘贴 JSON 导入(兼容全生态) 或 输入远程 URL。
  Future<void> _showAddDialog(BuildContext context, WidgetRef ref) async {
    final choice = await showDialog<_AddChoice>(
      context: context,
      builder: (ctx) => Dialog(
        insetPadding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 480, maxHeight: 520),
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(children: [
                  const Expanded(
                      child: Text('添加 MCP 工具',
                          style: TextStyle(
                              fontSize: 18, fontWeight: FontWeight.w600))),
                  IconButton(
                    icon: const Icon(Icons.close, size: 18),
                    onPressed: () => Navigator.pop(ctx),
                  ),
                ]),
                const SizedBox(height: 4),
                Text(
                    '从 Smithery / Claude Desktop / GitHub 复制配置,或输入远程 URL',
                    style: Theme.of(ctx).textTheme.bodySmall?.copyWith(
                        color: Theme.of(ctx).colorScheme.onSurfaceVariant)),
                const SizedBox(height: 16),
                // 常用工具快捷安装(无需 API key,npx 直装)
                Align(
                  alignment: Alignment.centerLeft,
                  child: Text('常用工具',
                      style: Theme.of(ctx).textTheme.labelMedium?.copyWith(
                          color: Theme.of(ctx).colorScheme.onSurfaceVariant)),
                ),
                const SizedBox(height: 8),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    for (final p in _presetServers)
                      ActionChip(
                        avatar: Icon(p.icon, size: 16),
                        label: Text(p.label),
                        onPressed: () => Navigator.pop(ctx, _AddChoice.preset(p)),
                      ),
                  ],
                ),
                const Divider(height: 24),
                const SizedBox(height: 4),
                // 粘贴 JSON
                _AddOptionTile(
                  icon: Icons.content_paste,
                  title: '粘贴配置 JSON',
                  subtitle: '兼容 Claude Desktop / Cursor / Smithery 格式',
                  onTap: () => Navigator.pop(ctx, _AddChoice.paste()),
                ),
                const SizedBox(height: 10),
                // 远程 URL
                _AddOptionTile(
                  icon: Icons.link,
                  title: '远程 MCP (URL)',
                  subtitle: '无需本地环境,直接连接 HTTP/SSE 服务器',
                  onTap: () => Navigator.pop(ctx, _AddChoice.url()),
                ),
                const SizedBox(height: 10),
                // 手动
                _AddOptionTile(
                  icon: Icons.edit_outlined,
                  title: '手动添加',
                  subtitle: '自填 command / args(高级)',
                  onTap: () => Navigator.pop(ctx, _AddChoice.manual()),
                ),
              ],
            ),
          ),
        ),
      ),
    );

    if (choice == null || !context.mounted) return;

    if (choice.isManual) {
      await _showManualDialog(context, ref);
    } else if (choice.isPaste) {
      await _showPasteDialog(context, ref);
    } else if (choice.isUrl) {
      await _showUrlDialog(context, ref);
    } else if (choice.preset != null) {
      await _installPreset(context, ref, choice.preset!);
    }
  }

  /// 一键安装预置常用工具。
  Future<void> _installPreset(
      BuildContext context, WidgetRef ref, _PresetServer p) async {
    await ref.read(mcpProvider.notifier).add(McpServer(
          name: p.name,
          command: p.command,
          args: p.args,
        ));
    if (context.mounted) {
      final err = ref.read(mcpProvider).error;
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text(err == null ? '已添加 ${p.label}' : '添加失败: $err'),
        duration: const Duration(seconds: 2),
      ));
    }
  }

  // ── 粘贴导入(智能多格式解析) ──────────────────────────────────────────────

  /// 智能解析多种 MCP 安装格式:
  /// 1. JSON (Claude Desktop / VS Code / Smithery)
  /// 2. `claude mcp add <name> [--transport sse] [--] <command|url> <args...>`
  /// 3. `code --add-mcp '{json}'`
  /// 4. 裸命令 `npx -y <pkg> ...` / `uvx <pkg>`
  static List<_ParsedServer> _parseMcpConfig(String raw) {
    final text = raw.trim();
    if (text.isEmpty) return [];
    // 先试 JSON。
    final jsonResult = _parseMcpJson(text);
    if (jsonResult.isNotEmpty) return jsonResult;
    // 再试 CLI 命令。
    return _parseMcpCli(text);
  }

  /// 解析 mcpServers JSON 配置(兼容 Claude Desktop + VS Code 格式)。
  static List<_ParsedServer> _parseMcpJson(String raw) {
    final results = <_ParsedServer>[];
    // 从文本中提取第一个 JSON 对象(容忍前后有说明文字)。
    var text = raw.trim();
    final braceStart = text.indexOf('{');
    final braceEnd = text.lastIndexOf('}');
    if (braceStart >= 0 && braceEnd > braceStart) {
      text = text.substring(braceStart, braceEnd + 1);
    }
    try {
      final obj = json.decode(text);
      if (obj is! Map<String, dynamic>) return results;
      // 兼容 {mcpServers: {...}} 或 {servers: {...}} 或直接 {"name": {...}}
      final servers = (obj['mcpServers'] as Map<String, dynamic>?) ??
          (obj['servers'] as Map<String, dynamic>?) ??
          obj;
      for (final entry in servers.entries) {
        if (entry.value is! Map<String, dynamic>) continue;
        final v = entry.value as Map<String, dynamic>;
        final command = v['command'] as String? ?? '';
        final args = (v['args'] as List? ?? [])
            .whereType<String>()
            .toList();
        final env = (v['env'] as Map<String, dynamic>? ?? {})
            .map((k, val) => MapEntry(k, val?.toString() ?? ''));
        final url = v['url'] as String? ?? '';
        final transport = url.isNotEmpty
            ? 'sse'
            : (v['type'] as String? ?? 'stdio');
        results.add(_ParsedServer(
          name: entry.key,
          command: command,
          args: args,
          env: env,
          url: url,
          transport: transport,
        ));
      }
    } catch (_) {}
    return results;
  }

  /// 解析 CLI 安装命令(claude mcp add / code --add-mcp / 裸 npx)。
  static List<_ParsedServer> _parseMcpCli(String raw) {
    // 去掉行首 `$ ` 之类前缀,合并多行续行符。
    var line = raw
        .replaceAll('\\\n', ' ')
        .replaceAll('\r', ' ')
        .replaceAll('\n', ' ')
        .trim();
    if (line.startsWith('\$')) line = line.substring(1).trim();

    final tokens = _tokenize(line);
    if (tokens.isEmpty) return [];

    // 形式 A: code --add-mcp '{json}'
    final addMcpIdx = tokens.indexOf('--add-mcp');
    if (addMcpIdx >= 0 && addMcpIdx + 1 < tokens.length) {
      final j = _parseMcpJson(tokens[addMcpIdx + 1]);
      if (j.isNotEmpty) return j;
    }

    // 形式 B: claude mcp add [flags] <name> [--] <command|url> [args...]
    final mcpIdx = tokens.indexOf('mcp');
    if (mcpIdx >= 0 &&
        mcpIdx + 1 < tokens.length &&
        tokens[mcpIdx + 1] == 'add') {
      return [_parseClaudeAdd(tokens.sublist(mcpIdx + 2))];
    }

    // 形式 C: 裸命令,如 `npx -y @scope/pkg arg` 或 `uvx pkg`
    const runners = {'npx', 'uvx', 'uv', 'node', 'python', 'python3', 'bun',
        'deno', 'docker'};
    if (runners.contains(tokens.first)) {
      // 拒绝"安装器"命令 —— 它们跑完即退出,不是可常驻的 MCP server。
      // 如 `npx @michaellatman/mcp-get install <pkg>` / smithery install。
      if (_isInstallerCommand(tokens)) return [];
      final name = _guessNameFromArgs(tokens);
      return [
        _ParsedServer(
          name: name,
          command: tokens.first,
          args: tokens.sublist(1),
          env: {},
          url: '',
          transport: 'stdio',
        )
      ];
    }

    return [];
  }

  /// 解析 `claude mcp add` 之后的 token(不含 `claude mcp add`)。
  static _ParsedServer _parseClaudeAdd(List<String> toks) {
    var transport = 'stdio';
    var url = '';
    final env = <String, String>{};
    String? name;
    final rest = <String>[];

    for (var i = 0; i < toks.length; i++) {
      final t = toks[i];
      if (t == '--transport' || t == '-t') {
        if (i + 1 < toks.length) transport = toks[++i];
      } else if (t == '--env' || t == '-e') {
        if (i + 1 < toks.length) {
          final kv = toks[++i];
          final eq = kv.indexOf('=');
          if (eq > 0) env[kv.substring(0, eq)] = kv.substring(eq + 1);
        }
      } else if (t == '--') {
        rest.addAll(toks.sublist(i + 1));
        break;
      } else if (t.startsWith('-')) {
        // 跳过其它未知 flag(及其可能的值由下轮判断)。
        continue;
      } else if (name == null) {
        name = t;
      } else {
        rest.add(t);
      }
    }

    // rest 第一个是 command 或 url。
    if (rest.isNotEmpty &&
        (rest.first.startsWith('http://') ||
            rest.first.startsWith('https://'))) {
      url = rest.first;
      if (transport == 'stdio') transport = 'sse';
      return _ParsedServer(
        name: name ?? Uri.tryParse(url)?.host ?? 'mcp',
        command: '',
        args: const [],
        env: env,
        url: url,
        transport: transport,
      );
    }
    return _ParsedServer(
      name: name ?? 'mcp',
      command: rest.isNotEmpty ? rest.first : '',
      args: rest.length > 1 ? rest.sublist(1) : const [],
      env: env,
      url: url,
      transport: transport,
    );
  }

  /// 从 npx/uvx 参数里猜一个 server 名(取包名最后一段)。
  /// 检测是否为"安装器"命令(跑完即退出,不能当常驻 MCP server)。
  static bool _isInstallerCommand(List<String> tokens) {
    final joined = tokens.join(' ').toLowerCase();
    // 已知的安装器包 / 子命令。
    const installerPkgs = [
      'mcp-get',
      '@michaellatman/mcp-get',
      '@smithery/cli',
      'smithery',
    ];
    for (final p in installerPkgs) {
      if (joined.contains(p)) return true;
    }
    return false;
  }

  static String _guessNameFromArgs(List<String> tokens) {
    for (final t in tokens.skip(1)) {
      if (t.startsWith('-')) continue;
      // @scope/server-foo → foo;pkg-bar → pkg-bar
      var pkg = t.split('/').last;
      pkg = pkg.replaceFirst(RegExp(r'^(mcp-)?server-'), '');
      pkg = pkg.replaceFirst(RegExp(r'-mcp$'), '');
      if (pkg.isNotEmpty) return pkg;
    }
    return 'mcp';
  }

  /// 简单 shell tokenizer:按空格切分,尊重单/双引号。
  static List<String> _tokenize(String s) {
    final tokens = <String>[];
    final buf = StringBuffer();
    String? quote;
    for (var i = 0; i < s.length; i++) {
      final c = s[i];
      if (quote != null) {
        if (c == quote) {
          quote = null;
        } else {
          buf.write(c);
        }
      } else if (c == '"' || c == "'") {
        quote = c;
      } else if (c == ' ' || c == '\t') {
        if (buf.isNotEmpty) {
          tokens.add(buf.toString());
          buf.clear();
        }
      } else {
        buf.write(c);
      }
    }
    if (buf.isNotEmpty) tokens.add(buf.toString());
    return tokens;
  }

  /// 命令安全评估:白名单内的视为安全,否则警告。
  static _CmdSafety _assessCommand(String command) {
    const safe = {'npx', 'node', 'python', 'python3', 'uvx', 'uv', 'docker',
        'deno', 'bun', 'go', 'cargo'};
    final bin = command.split('/').last.split('\\').last.toLowerCase();
    if (safe.contains(bin)) return _CmdSafety.normal;
    const suspicious = {'cmd', 'cmd.exe', 'powershell', 'powershell.exe',
        'pwsh', 'pwsh.exe', 'bash', 'sh', 'curl', 'wget', 'rm', 'del'};
    if (suspicious.contains(bin)) return _CmdSafety.warning;
    return _CmdSafety.unknown;
  }

  Future<void> _showPasteDialog(BuildContext context, WidgetRef ref) async {
    final ctrl = TextEditingController();
    final parsed = ValueNotifier<List<_ParsedServer>>([]);

    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => Dialog(
        insetPadding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560, maxHeight: 560),
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text('粘贴 MCP 配置', style: Theme.of(ctx).textTheme.titleMedium),
                const SizedBox(height: 4),
                Text('支持 JSON 配置、claude mcp add 命令、npx/uvx 命令 — 直接粘贴',
                    style: Theme.of(ctx).textTheme.bodySmall?.copyWith(
                        color: Theme.of(ctx).colorScheme.onSurfaceVariant)),
                const SizedBox(height: 12),
                Expanded(
                  child: TextField(
                    controller: ctrl,
                    maxLines: null,
                    expands: true,
                    textAlignVertical: TextAlignVertical.top,
                    decoration: const InputDecoration(
                      hintText: 'claude mcp add my-server -- npx -y @scope/server\n或\n{"mcpServers": {...}}\n或\nnpx -y @modelcontextprotocol/server-filesystem .',
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.all(12),
                    ),
                    style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
                    onChanged: (v) => parsed.value = _parseMcpConfig(v),
                  ),
                ),
                const SizedBox(height: 12),
                // 解析预览
                ValueListenableBuilder<List<_ParsedServer>>(
                  valueListenable: parsed,
                  builder: (_, servers, child2) {
                    if (servers.isEmpty) {
                      return Text('粘贴后自动解析...',
                          style: Theme.of(ctx).textTheme.bodySmall);
                    }
                    return Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text('解析到 ${servers.length} 个服务器:',
                            style: Theme.of(ctx).textTheme.labelMedium),
                        const SizedBox(height: 6),
                        for (final s in servers.take(5))
                          _ParsedServerPreview(server: s),
                      ],
                    );
                  },
                ),
                const SizedBox(height: 12),
                Row(mainAxisAlignment: MainAxisAlignment.end, children: [
                  TextButton(
                      onPressed: () => Navigator.pop(ctx, false),
                      child: const Text('取消')),
                  const SizedBox(width: 8),
                  ValueListenableBuilder<List<_ParsedServer>>(
                    valueListenable: parsed,
                    builder: (_, servers, child2) => FilledButton(
                      onPressed:
                          servers.isEmpty ? null : () => Navigator.pop(ctx, true),
                      child: Text('安装 (${servers.length})'),
                    ),
                  ),
                ]),
              ],
            ),
          ),
        ),
      ),
    );

    if (confirmed != true || !context.mounted) return;
    final servers = parsed.value;
    var installed = 0;
    for (final s in servers) {
      await ref.read(mcpProvider.notifier).add(McpServer(
            name: s.name,
            command: s.command.isNotEmpty ? s.command : s.url,
            args: s.args,
            transport: s.transport,
          ));
      installed++;
    }
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text('已添加 $installed 个 MCP 工具'),
        duration: const Duration(seconds: 2),
      ));
    }
  }

  // ── 远程 URL 输入 ──────────────────────────────────────────────────────────

  Future<void> _showUrlDialog(BuildContext context, WidgetRef ref) async {
    final nameCtrl = TextEditingController();
    final urlCtrl = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('远程 MCP Server'),
        content: SingleChildScrollView(
          child: Column(mainAxisSize: MainAxisSize.min, children: [
            TextField(
                controller: nameCtrl,
                decoration: const InputDecoration(
                    labelText: '名称', hintText: 'e.g. exa-search')),
            TextField(
                controller: urlCtrl,
                decoration: const InputDecoration(
                    labelText: 'URL',
                    hintText: 'https://mcp.example.com/sse')),
          ]),
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('取消')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('添加')),
        ],
      ),
    );
    if (ok == true && urlCtrl.text.trim().isNotEmpty) {
      await ref.read(mcpProvider.notifier).add(McpServer(
            name: nameCtrl.text.trim().isEmpty
                ? Uri.tryParse(urlCtrl.text.trim())?.host ?? 'remote'
                : nameCtrl.text.trim(),
            command: urlCtrl.text.trim(),
            args: const [],
            transport: 'sse',
          ));
      if (context.mounted) {
        final err = ref.read(mcpProvider).error;
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(
          content:
              Text(err == null ? '已添加远程 MCP' : '添加失败: $err'),
          duration: const Duration(seconds: 2),
        ));
      }
    }
  }

  // ── 手动添加(高级) ─────────────────────────────────────────────────────────

  Future<void> _showManualDialog(BuildContext context, WidgetRef ref) async {
    final nameCtrl = TextEditingController();
    final cmdCtrl = TextEditingController();
    final argsCtrl = TextEditingController();

    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('手动添加 MCP Server'),
        content: SingleChildScrollView(
          child: Column(mainAxisSize: MainAxisSize.min, children: [
            TextField(
                controller: nameCtrl,
                decoration: const InputDecoration(
                    labelText: '名称', hintText: 'e.g. filesystem')),
            TextField(
                controller: cmdCtrl,
                decoration: const InputDecoration(
                    labelText: '命令', hintText: 'e.g. npx')),
            TextField(
                controller: argsCtrl,
                decoration: const InputDecoration(
                    labelText: '参数(空格分隔)',
                    hintText:
                        '-y @modelcontextprotocol/server-filesystem /path')),
          ]),
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('取消')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('添加')),
        ],
      ),
    );
    if (ok == true && nameCtrl.text.trim().isNotEmpty) {
      final args = argsCtrl.text
          .trim()
          .split(RegExp(r'\s+'))
          .where((s) => s.isNotEmpty)
          .toList();
      await ref.read(mcpProvider.notifier).add(McpServer(
            name: nameCtrl.text.trim(),
            command: cmdCtrl.text.trim(),
            args: args,
          ));
      if (context.mounted) {
        final err = ref.read(mcpProvider).error;
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(
          content: Text(err == null
              ? '已添加 ${nameCtrl.text.trim()}'
              : '添加失败: $err'),
          duration: const Duration(seconds: 2),
        ));
      }
    }
  }

  /// server 卡片副标题:命令行 + 连接状态(优先展示诊断)。
  static String _serverSubtitle(McpServer s, McpToolsInfo info) {
    final cmdLine = s.url != null && s.url!.isNotEmpty
        ? s.url!
        : '${s.command}${s.args.isNotEmpty ? ' ${s.args.join(' ')}' : ''}';
    // 查该 server 的诊断。
    final diag = info.diagnostics
        .where((d) => d.server == s.name)
        .cast<McpDiagnostic?>()
        .firstWhere((_) => true, orElse: () => null);
    String statusText;
    if (diag != null) {
      if (diag.liveError != null && diag.liveError!.isNotEmpty) {
        statusText = diag.cachedTools > 0
            ? '⚠ 连接失败,使用缓存(${diag.cachedTools} 工具)'
            : '⚠ 连接失败';
      } else if (diag.source == 'live') {
        statusText = '✓ 已连接';
      } else {
        statusText = s.status;
      }
    } else {
      statusText = s.status;
    }
    return '$cmdLine · $statusText';
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final state = ref.watch(mcpProvider);
    final notifier = ref.read(mcpProvider.notifier);

    return _PanelScaffold(
      title: 'MCP Servers',
      subtitle: '添加后 agent 可通过 MCP 协议调用外部工具',
      loading: state.loading,
      error: state.error,
      onRefresh: () => notifier.load(),
      onAdd: () => _showAddDialog(context, ref),
      empty: state.servers.isEmpty,
      emptyHint: '暂无 MCP server。点右上 + 添加。',
      children: [
        for (final s in state.servers)
          _Row(
            leading: Icon(
              (s.status == 'running' || s.status == 'active')
                  ? Icons.cloud_done
                  : Icons.cloud_outlined,
              size: 18,
              color: (s.status == 'running' || s.status == 'active')
                  ? theme.colorScheme.primary
                  : null,
            ),
            title: s.name,
            badges: [
              s.transport,
              if (state.toolsInfo.countByServer[s.name] != null)
                '${state.toolsInfo.countByServer[s.name]} 工具',
            ],
            subtitle: _serverSubtitle(s, state.toolsInfo),
            trailing: Row(mainAxisSize: MainAxisSize.min, children: [
              TextButton(
                onPressed: state.loading
                    ? null
                    : () => (s.status == 'running' || s.status == 'active')
                        ? notifier.stop(s.name)
                        : notifier.start(s.name),
                child: Text((s.status == 'running' || s.status == 'active')
                    ? '停止'
                    : '启动'),
              ),
              IconButton(
                icon: const Icon(Icons.delete_outline, size: 18),
                tooltip: '删除',
                onPressed: state.loading ? null : () => notifier.remove(s.name),
              ),
            ]),
          ),
      ],
    );
  }
}

/// 添加流程的选择结果。
class _AddChoice {
  const _AddChoice._(this.isManual, this.isPaste, this.isUrl, [this.preset]);
  factory _AddChoice.manual() => const _AddChoice._(true, false, false);
  factory _AddChoice.paste() => const _AddChoice._(false, true, false);
  factory _AddChoice.url() => const _AddChoice._(false, false, true);
  factory _AddChoice.preset(_PresetServer p) =>
      _AddChoice._(false, false, false, p);
  final bool isManual;
  final bool isPaste;
  final bool isUrl;
  final _PresetServer? preset;
}

/// 预置常用 MCP 工具(无需 API key,npx 直装)。
class _PresetServer {
  const _PresetServer({
    required this.name,
    required this.label,
    required this.icon,
    required this.command,
    required this.args,
  });
  final String name;
  final String label;
  final IconData icon;
  final String command;
  final List<String> args;
}

const _presetServers = <_PresetServer>[
  _PresetServer(
    name: 'filesystem',
    label: '文件系统',
    icon: Icons.folder_outlined,
    command: 'npx',
    args: ['-y', '@modelcontextprotocol/server-filesystem', '.'],
  ),
  _PresetServer(
    name: 'fetch',
    label: '网页抓取',
    icon: Icons.language,
    command: 'uvx',
    args: ['mcp-server-fetch'],
  ),
  _PresetServer(
    name: 'memory',
    label: '持久记忆',
    icon: Icons.memory,
    command: 'npx',
    args: ['-y', '@modelcontextprotocol/server-memory'],
  ),
  _PresetServer(
    name: 'sequential-thinking',
    label: '深度推理',
    icon: Icons.psychology_outlined,
    command: 'npx',
    args: ['-y', '@modelcontextprotocol/server-sequential-thinking'],
  ),
  _PresetServer(
    name: 'time',
    label: '时间日期',
    icon: Icons.schedule,
    command: 'uvx',
    args: ['mcp-server-time'],
  ),
];

/// 命令安全级别。
enum _CmdSafety { normal, unknown, warning }

/// 从粘贴 JSON 解析出的一个服务器条目。
class _ParsedServer {
  const _ParsedServer({
    required this.name,
    required this.command,
    required this.args,
    required this.env,
    required this.url,
    required this.transport,
  });
  final String name;
  final String command;
  final List<String> args;
  final Map<String, String> env;
  final String url;
  final String transport;
}

/// 添加入口的一个选项磁贴。
class _AddOptionTile extends StatelessWidget {
  const _AddOptionTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });
  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(10),
      child: Container(
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerLow,
          borderRadius: BorderRadius.circular(10),
          border: Border.all(
              color: theme.colorScheme.outlineVariant.withValues(alpha: 0.5)),
        ),
        child: Row(
          children: [
            Icon(icon, size: 22, color: theme.colorScheme.primary),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: theme.textTheme.titleSmall),
                  Text(subtitle,
                      style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant)),
                ],
              ),
            ),
            Icon(Icons.chevron_right,
                size: 18, color: theme.colorScheme.onSurfaceVariant),
          ],
        ),
      ),
    );
  }
}

/// 粘贴解析后的单个服务器预览(带命令安全提示)。
class _ParsedServerPreview extends StatelessWidget {
  const _ParsedServerPreview({required this.server});
  final _ParsedServer server;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final safety = McpPanel._assessCommand(server.command);
    final isRemote = server.url.isNotEmpty;
    final (Color color, IconData icon, String note) = switch (safety) {
      _CmdSafety.warning => (
          theme.colorScheme.error,
          Icons.warning_amber_rounded,
          '可疑命令,请确认来源可信'
        ),
      _CmdSafety.unknown => (
          theme.colorScheme.tertiary,
          Icons.help_outline,
          '非常见命令'
        ),
      _CmdSafety.normal => (
          theme.colorScheme.primary,
          Icons.check_circle_outline,
          ''
        ),
    };
    final cmdLine = isRemote
        ? server.url
        : '${server.command} ${server.args.join(' ')}'.trim();
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(isRemote ? Icons.link : icon, size: 14, color: color),
          const SizedBox(width: 6),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(server.name,
                    style: theme.textTheme.bodySmall
                        ?.copyWith(fontWeight: FontWeight.w600)),
                Text(cmdLine,
                    style: theme.textTheme.bodySmall?.copyWith(
                        fontFamily: 'monospace',
                        fontSize: 11,
                        color: theme.colorScheme.onSurfaceVariant),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis),
                if (note.isNotEmpty)
                  Text(note,
                      style: theme.textTheme.bodySmall
                          ?.copyWith(color: color, fontSize: 11)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

// ── 共用脚手架 ────────────────────────────────────────────────────────────────

class _PanelScaffold extends StatelessWidget {
  const _PanelScaffold({
    required this.title,
    required this.subtitle,
    required this.loading,
    required this.error,
    required this.onRefresh,
    required this.empty,
    required this.emptyHint,
    required this.children,
    this.onAdd,
  });

  final String title;
  final String subtitle;
  final bool loading;
  final String? error;
  final VoidCallback onRefresh;
  final VoidCallback? onAdd;
  final bool empty;
  final String emptyHint;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 8, 8),
          child: Row(children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: theme.textTheme.titleSmall),
                  const SizedBox(height: 2),
                  Text(subtitle,
                      style: theme.textTheme.bodySmall
                          ?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
                ],
              ),
            ),
            if (onAdd != null)
              IconButton(
                icon: const Icon(Icons.add, size: 18),
                tooltip: '添加',
                onPressed: loading ? null : onAdd,
              ),
            loading
                ? const Padding(
                    padding: EdgeInsets.all(8),
                    child: SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2)),
                  )
                : IconButton(
                    icon: const Icon(Icons.refresh, size: 18),
                    tooltip: '刷新',
                    onPressed: onRefresh,
                  ),
          ]),
        ),
        if (error != null && error!.trim().isNotEmpty)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            color: theme.colorScheme.errorContainer,
            child: Text(error!,
                style: theme.textTheme.bodySmall
                    ?.copyWith(color: theme.colorScheme.onErrorContainer)),
          ),
        const Divider(height: 1),
        Expanded(
          child: (empty && !loading)
              ? Center(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Text(emptyHint,
                        textAlign: TextAlign.center,
                        style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant)),
                  ),
                )
              : ListView.separated(
                  itemCount: children.length,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (_, i) => children[i],
                ),
        ),
      ],
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({
    required this.leading,
    required this.title,
    required this.subtitle,
    this.trailing,
    this.badges = const [],
    this.active = false,
  });

  final Widget leading;
  final String title;
  final String subtitle;
  final Widget? trailing;
  final List<String> badges;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListTile(
      dense: true,
      leading: leading,
      title: Row(children: [
        Flexible(
          child: Text(title,
              style: TextStyle(
                  fontSize: 14,
                  fontWeight: active ? FontWeight.w600 : FontWeight.normal),
              overflow: TextOverflow.ellipsis),
        ),
        for (final b in badges) ...[
          const SizedBox(width: 6),
          _Badge(text: b, theme: theme),
        ],
      ]),
      subtitle: Text(subtitle,
          style: const TextStyle(fontSize: 12),
          maxLines: 1,
          overflow: TextOverflow.ellipsis),
      trailing: trailing,
    );
  }
}

class _Badge extends StatelessWidget {
  const _Badge({required this.text, required this.theme});
  final String text;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(text,
          style: theme.textTheme.labelSmall
              ?.copyWith(color: theme.colorScheme.onSurfaceVariant)),
    );
  }
}
