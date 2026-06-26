import 'package:dio/dio.dart';

/// MCP(Model Context Protocol)server 管理 API。对应 mcp-client 能力 `ext.mcp`:
/// list_servers / add_server / remove_server / start_server / stop_server。
/// 走通用能力端点 `/api/capabilities/ext.mcp/call`。
class McpApi {
  McpApi(this._dio);

  final Dio _dio;
  static const _cap = 'ext.mcp';

  Future<Map<String, dynamic>> _call(
    String action,
    Map<String, dynamic> data,
  ) async {
    final resp = await _dio.post<Map<String, dynamic>>(
      '/api/capabilities/$_cap/call',
      data: {'action': action, 'data': data},
    );
    final response = resp.data?['response'] as Map<String, dynamic>?;
    final inner = response?['data'];
    return inner is Map<String, dynamic> ? inner : <String, dynamic>{};
  }

  Future<List<McpServer>> listServers(String agent) async {
    final data = await _call('list_servers', {'agent': agent});
    return (data['servers'] as List? ?? [])
        .whereType<Map<String, dynamic>>()
        .map(McpServer.fromJson)
        .toList();
  }

  Future<void> addServer(String agent, McpServer server) async {
    await _call('add_server', {'agent': agent, ...server.toJson()});
  }

  Future<void> removeServer(String agent, String name) async {
    await _call('remove_server', {'agent': agent, 'name': name});
  }

  Future<void> startServer(String agent, String name) async {
    await _call('start_server', {'agent': agent, 'name': name});
  }

  Future<void> stopServer(String agent, String name) async {
    await _call('stop_server', {'agent': agent, 'name': name});
  }
}

class McpServer {
  const McpServer({
    required this.name,
    required this.command,
    this.args = const [],
    this.transport = 'stdio',
    this.url,
    this.status = 'unknown',
  });

  final String name;
  final String command;
  final List<String> args;
  final String transport;
  final String? url;
  final String status;

  factory McpServer.fromJson(Map<String, dynamic> j) {
    String st = 'unknown';
    final rawStatus = j['status'];
    if (rawStatus is Map) {
      st = rawStatus['status'] as String? ?? 'unknown';
    } else if (rawStatus is String) {
      st = rawStatus;
    }
    return McpServer(
      name: j['name'] as String? ?? '',
      command: j['command'] as String? ?? '',
      args: (j['args'] as List? ?? []).whereType<String>().toList(),
      transport: j['transport'] as String? ?? 'stdio',
      url: j['url'] as String?,
      status: st,
    );
  }

  Map<String, dynamic> toJson() => {
        'name': name,
        'command': command,
        'args': args,
        'transport': transport,
        if (url != null && url!.isNotEmpty) 'url': url,
      };
}
