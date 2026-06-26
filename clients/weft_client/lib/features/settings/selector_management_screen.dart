import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Selector panel — view/sync/manage the tool candidate library.
/// Embeddable inside a tab/drawer (no Scaffold/AppBar of its own).
class SelectorPanel extends ConsumerStatefulWidget {
  const SelectorPanel({super.key});

  @override
  ConsumerState<SelectorPanel> createState() => _SelectorPanelState();
}

class _SelectorPanelState extends ConsumerState<SelectorPanel> {
  static const _selectorUrl = 'http://127.0.0.1:17860';

  bool _loading = false;
  String? _error;
  List<_ToolEntry> _tools = [];
  List<String> _libraries = [];
  String _selectedLibrary = 'tools';
  bool _serverOnline = false;

  @override
  void initState() {
    super.initState();
    _checkHealth();
  }

  Future<void> _checkHealth() async {
    setState(() => _loading = true);
    try {
      final resp = await Dio().get<Map<String, dynamic>>(
        _selectorUrl,
        options: Options(receiveTimeout: const Duration(seconds: 3)),
      );
      final libs = (resp.data?['libraries'] as List?)
              ?.whereType<String>()
              .toList() ??
          [];
      setState(() {
        _serverOnline = resp.data?['ready'] == true;
        _libraries = libs;
        _error = null;
      });
      if (_serverOnline) await _loadTools();
    } catch (e) {
      setState(() {
        _serverOnline = false;
        _error = 'Selector 服务未运行: $e';
      });
    } finally {
      setState(() => _loading = false);
    }
  }

  Future<void> _loadTools() async {
    setState(() => _loading = true);
    try {
      final resp = await Dio().post<Map<String, dynamic>>(
        _selectorUrl,
        data: {
          'method': 'list_tools',
          'params': {'library': _selectedLibrary},
        },
        options: Options(receiveTimeout: const Duration(seconds: 5)),
      );
      final tools = (resp.data?['result']?['tools'] as List? ?? [])
          .whereType<Map<String, dynamic>>()
          .map(_ToolEntry.fromJson)
          .toList();
      setState(() {
        _tools = tools;
        _error = null;
      });
    } catch (e) {
      setState(() => _error = '加载工具列表失败: $e');
    } finally {
      setState(() => _loading = false);
    }
  }

  Future<void> _syncMcp() async {
    setState(() => _loading = true);
    try {
      // Sync doesn't need auth token — selector reads it from file.
      final resp = await Dio().post<Map<String, dynamic>>(
        _selectorUrl,
        data: {
          'method': 'sync_mcp',
          'params': {
            'core_port': 17830,
            'token': '', // selector reads token from file if needed
            'library': _selectedLibrary,
          },
        },
        options: Options(receiveTimeout: const Duration(seconds: 15)),
      );
      final result = resp.data?['result'] as Map<String, dynamic>?;
      final added = result?['mcp_tools_added'] ?? 0;
      final total = result?['total'] ?? 0;
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('同步完成: +$added MCP工具, 共$total')),
        );
        await _loadTools();
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('同步失败: $e')),
        );
      }
    } finally {
      setState(() => _loading = false);
    }
  }

  Future<void> _rebuild() async {
    setState(() => _loading = true);
    try {
      final resp = await Dio().post<Map<String, dynamic>>(
        _selectorUrl,
        data: {
          'method': 'rebuild',
          'params': {'library': _selectedLibrary},
        },
        options: Options(receiveTimeout: const Duration(seconds: 30)),
      );
      final result = resp.data?['result'] as Map<String, dynamic>?;
      final count = result?['count'] ?? 0;
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Embeddings 重建完成: $count 条')),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('重建失败: $e')),
        );
      }
    } finally {
      setState(() => _loading = false);
    }
  }

  Future<void> _removeTool(String id) async {
    try {
      await Dio().post<Map<String, dynamic>>(
        _selectorUrl,
        data: {
          'method': 'unregister',
          'params': {'id': id, 'library': _selectedLibrary},
        },
      );
      await _loadTools();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('删除失败: $e')),
        );
      }
    }
  }

  Future<void> _addTool() async {
    final result = await showDialog<_ToolEntry>(
      context: context,
      builder: (ctx) => const _AddToolDialog(),
    );
    if (result == null) return;
    try {
      await Dio().post<Map<String, dynamic>>(
        _selectorUrl,
        data: {
          'method': 'register',
          'params': {
            'id': result.id,
            'name': result.name,
            'description': result.description,
            'source': result.source,
            'library': _selectedLibrary,
          },
        },
      );
      await _loadTools();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('添加失败: $e')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      children: [
        // Status bar
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          color: _serverOnline
              ? Colors.green.withValues(alpha: 0.1)
              : Colors.red.withValues(alpha: 0.1),
          child: Row(
            children: [
              Icon(
                _serverOnline ? Icons.check_circle : Icons.error,
                size: 16,
                color: _serverOnline ? Colors.green : Colors.red,
              ),
              const SizedBox(width: 8),
              Text(
                _serverOnline ? '服务运行中 (port 17860)' : '服务离线',
                style: theme.textTheme.bodySmall,
              ),
              const Spacer(),
              if (_libraries.isNotEmpty)
                DropdownButton<String>(
                  value: _selectedLibrary,
                  underline: const SizedBox.shrink(),
                  isDense: true,
                  items: _libraries
                      .map((l) => DropdownMenuItem(value: l, child: Text(l)))
                      .toList(),
                  onChanged: (v) {
                    if (v != null) {
                      setState(() => _selectedLibrary = v);
                      _loadTools();
                    }
                  },
                ),
              IconButton(
                icon: const Icon(Icons.refresh, size: 18),
                tooltip: '刷新',
                onPressed: _loading ? null : _checkHealth,
              ),
            ],
          ),
        ),
          // Action buttons
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Wrap(
              spacing: 8,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                FilledButton.icon(
                  icon: const Icon(Icons.sync, size: 16),
                  label: const Text('从 MCP 同步'),
                  onPressed: _loading || !_serverOnline ? null : _syncMcp,
                ),
                OutlinedButton.icon(
                  icon: const Icon(Icons.build, size: 16),
                  label: const Text('重建'),
                  onPressed: _loading || !_serverOnline ? null : _rebuild,
                ),
                OutlinedButton.icon(
                  icon: const Icon(Icons.add, size: 16),
                  label: const Text('手动添加'),
                  onPressed: _loading || !_serverOnline ? null : _addTool,
                ),
              ],
            ),
          ),
          if (_error != null)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Text(_error!,
                  style: const TextStyle(color: Colors.red, fontSize: 12)),
            ),
          // 工具计数行
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 6),
            child: Align(
              alignment: Alignment.centerLeft,
              child: Text(
                '${_tools.length} 个工具',
                style: theme.textTheme.bodySmall?.copyWith(color: theme.hintColor),
              ),
            ),
          ),
          const Divider(height: 1),
          // Tool list
          Expanded(
            child: _loading && _tools.isEmpty
                ? const Center(child: CircularProgressIndicator())
                : ListView.separated(
                    itemCount: _tools.length,
                    separatorBuilder: (_, _) => const Divider(height: 1),
                    itemBuilder: (context, index) {
                      final tool = _tools[index];
                      return _ToolTile(
                        tool: tool,
                        onDelete: () => _removeTool(tool.id),
                      );
                    },
                  ),
          ),
        ],
      );
  }
}

// ── Data ──────────────────────────────────────────────────────────────────────

class _ToolEntry {
  const _ToolEntry({
    required this.id,
    required this.name,
    required this.description,
    required this.source,
    this.mcpServer,
  });

  final String id;
  final String name;
  final String description;
  final String source;
  final String? mcpServer;

  factory _ToolEntry.fromJson(Map<String, dynamic> json) => _ToolEntry(
        id: json['id'] as String? ?? '',
        name: json['name'] as String? ?? '',
        description: json['description'] as String? ?? '',
        source: json['source'] as String? ?? 'unknown',
        mcpServer: json['mcp_server'] as String?,
      );
}

// ── Widgets ───────────────────────────────────────────────────────────────────

class _ToolTile extends StatelessWidget {
  const _ToolTile({required this.tool, required this.onDelete});

  final _ToolEntry tool;
  final VoidCallback onDelete;

  Color _sourceColor(String source) => switch (source) {
        'virtual' => Colors.blue,
        'skill' => Colors.teal,
        'mcp' => Colors.purple,
        'manual' => Colors.orange,
        'always-on' => Colors.green,
        _ => Colors.grey,
      };

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListTile(
      dense: true,
      leading: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          color: _sourceColor(tool.source).withValues(alpha: 0.15),
          borderRadius: BorderRadius.circular(4),
        ),
        child: Text(
          tool.source,
          style: TextStyle(
            fontSize: 10,
            color: _sourceColor(tool.source),
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
      title: Text(tool.name, style: const TextStyle(fontSize: 13)),
      subtitle: Text(
        tool.id + (tool.mcpServer != null ? ' (${tool.mcpServer})' : ''),
        style: TextStyle(fontSize: 11, color: theme.hintColor),
      ),
      trailing: IconButton(
        icon: const Icon(Icons.delete_outline, size: 18),
        onPressed: onDelete,
        tooltip: '移除',
      ),
    );
  }
}

class _AddToolDialog extends StatefulWidget {
  const _AddToolDialog();

  @override
  State<_AddToolDialog> createState() => _AddToolDialogState();
}

class _AddToolDialogState extends State<_AddToolDialog> {
  final _idCtrl = TextEditingController();
  final _nameCtrl = TextEditingController();
  final _descCtrl = TextEditingController();
  String _source = 'manual';

  @override
  void dispose() {
    _idCtrl.dispose();
    _nameCtrl.dispose();
    _descCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('添加工具'),
      content: SizedBox(
        width: 400,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: _idCtrl,
              decoration: const InputDecoration(
                labelText: 'ID',
                hintText: 'mcp:server:tool_name',
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _nameCtrl,
              decoration: const InputDecoration(
                labelText: '名称',
                hintText: '工具显示名',
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _descCtrl,
              decoration: const InputDecoration(
                labelText: '描述（用于语义匹配）',
                hintText: '关键词 中英文 功能描述',
              ),
              maxLines: 3,
            ),
            const SizedBox(height: 8),
            DropdownButtonFormField<String>(
              initialValue: _source,
              decoration: const InputDecoration(labelText: '来源'),
              items: const [
                DropdownMenuItem(value: 'manual', child: Text('manual')),
                DropdownMenuItem(value: 'mcp', child: Text('mcp')),
                DropdownMenuItem(value: 'skill', child: Text('skill')),
                DropdownMenuItem(value: 'virtual', child: Text('virtual')),
              ],
              onChanged: (v) => setState(() => _source = v ?? 'manual'),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: () {
            if (_idCtrl.text.trim().isEmpty) return;
            Navigator.pop(
              context,
              _ToolEntry(
                id: _idCtrl.text.trim(),
                name: _nameCtrl.text.trim().isEmpty
                    ? _idCtrl.text.trim()
                    : _nameCtrl.text.trim(),
                description: _descCtrl.text.trim(),
                source: _source,
              ),
            );
          },
          child: const Text('添加'),
        ),
      ],
    );
  }
}
