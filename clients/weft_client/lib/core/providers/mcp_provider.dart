import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/client.dart';
import '../api/mcp_api.dart';

final mcpApiProvider = Provider<McpApi>((ref) {
  return McpApi(ref.watch(apiClientProvider));
});

class McpState {
  const McpState({
    this.agent = 'weft-claw',
    this.servers = const [],
    this.loading = false,
    this.error,
  });

  final String agent;
  final List<McpServer> servers;
  final bool loading;
  final String? error;

  McpState copyWith({
    String? agent,
    List<McpServer>? servers,
    bool? loading,
    String? error,
    bool clearError = false,
  }) {
    return McpState(
      agent: agent ?? this.agent,
      servers: servers ?? this.servers,
      loading: loading ?? this.loading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

class McpNotifier extends StateNotifier<McpState> {
  McpNotifier(this._api) : super(const McpState()) {
    load();
  }

  final McpApi _api;

  Future<void> load() async {
    state = state.copyWith(loading: true, clearError: true);
    try {
      final servers = await _api.listServers(state.agent);
      state =
          state.copyWith(servers: servers, loading: false, clearError: true);
    } catch (e) {
      state = state.copyWith(loading: false, error: _sanitize(e));
    }
  }

  Future<void> add(McpServer server) async {
    state = state.copyWith(loading: true, clearError: true);
    try {
      await _api.addServer(state.agent, server);
      await load();
    } catch (e) {
      state = state.copyWith(loading: false, error: _sanitize(e));
    }
  }

  Future<void> remove(String name) async {
    state = state.copyWith(loading: true, clearError: true);
    try {
      await _api.removeServer(state.agent, name);
      await load();
    } catch (e) {
      state = state.copyWith(loading: false, error: _sanitize(e));
    }
  }

  Future<void> start(String name) async {
    try {
      await _api.startServer(state.agent, name);
      await load();
    } catch (e) {
      state = state.copyWith(error: _sanitize(e));
    }
  }

  Future<void> stop(String name) async {
    try {
      await _api.stopServer(state.agent, name);
      await load();
    } catch (e) {
      state = state.copyWith(error: _sanitize(e));
    }
  }

  static String _sanitize(Object e) {
    final s = e.toString();
    if (s.contains('SocketException') || s.contains('Connection')) {
      return '无法连接到 weft-core';
    }
    return s.length > 120 ? '${s.substring(0, 120)}…' : s;
  }
}

final mcpProvider = StateNotifierProvider<McpNotifier, McpState>((ref) {
  return McpNotifier(ref.watch(mcpApiProvider));
});
