import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';

import '../api/client.dart';
import '../api/weft_claw_api.dart';
import '../models/chat_session_meta.dart';

const _uuid = Uuid();

// Provider for WeftClawApi
final weftClawApiProvider = Provider<WeftClawApi>((ref) {
  final dio = ref.watch(apiClientProvider);
  return WeftClawApi(dio);
});

class SessionsNotifier extends StateNotifier<List<ChatSessionMeta>> {
  SessionsNotifier(this._api) : super([]) {
    _load();
  }

  final WeftClawApi _api;

  Future<void> _load() async {
    try {
      final sessions = await _api.listSessions();
      final metas = sessions
          .where((s) => s.id.isNotEmpty)
          .map((s) => ChatSessionMeta(
                id: s.id,
                title: s.title.isEmpty ? 'Chat' : s.title,
                createdAt: s.createdAt,
                updatedAt: s.updatedAt,
                messageCount: 0,
              ))
          .toList();
      // 按 updatedAt 倒序
      metas.sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
      state = metas;
    } catch (_) {
      // weft-core 未启动时静默失败，保持空列表
    }
  }

  Future<void> refresh() => _load();

  /// 创建新 session（本地生成 UUID，weft-claw 在首次 send_message 时自动创建）
  Future<ChatSessionMeta> createSession({
    String? provider,
    String? model,
  }) async {
    final now = DateTime.now();
    final meta = ChatSessionMeta(
      id: _uuid.v4(),
      title: 'New Chat',
      createdAt: now,
      updatedAt: now,
      messageCount: 0,
      provider: provider,
      model: model,
    );
    state = [meta, ...state];
    return meta;
  }

  Future<void> deleteSession(String id) async {
    state = state.where((m) => m.id != id).toList();
    try {
      await _api.deleteSession(id);
    } catch (_) {}
  }

  Future<void> updateSessionTitle(String id, String title) async {
    state = state.map((m) {
      if (m.id != id) return m;
      return m.copyWith(title: title, updatedAt: DateTime.now());
    }).toList();
  }

  /// 消息发送后调用：更新 updatedAt、messageCount，并在首条消息时自动截取标题
  Future<void> onMessageAdded(
    String id, {
    required int messageCount,
    String? firstUserContent,
  }) async {
    state = state.map((m) {
      if (m.id != id) return m;
      final newTitle = (m.title == 'New Chat' &&
              firstUserContent != null &&
              firstUserContent.isNotEmpty)
          ? firstUserContent.length > 40
              ? '${firstUserContent.substring(0, 40)}…'
              : firstUserContent
          : m.title;
      return m.copyWith(
        title: newTitle,
        updatedAt: DateTime.now(),
        messageCount: messageCount,
      );
    }).toList();
    // 重新排序
    final sorted = [...state]
      ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    state = sorted;
  }
}

final sessionsProvider =
    StateNotifierProvider<SessionsNotifier, List<ChatSessionMeta>>(
  (ref) {
    final api = ref.watch(weftClawApiProvider);
    return SessionsNotifier(api);
  },
);
