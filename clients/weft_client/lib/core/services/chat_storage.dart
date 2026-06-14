import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

import '../models/chat.dart';
import '../models/chat_session_meta.dart';

/// 本地持久化服务
/// - 每个 session 存一个 JSONL 文件：`appDocDir/.weft_client/sessions/{session_id}.jsonl`
/// - session 元数据列表存在：`appDocDir/.weft_client/sessions.json`
class ChatStorage {
  static const _dirName = '.weft_client';
  static const _sessionsFile = 'sessions.json';

  // ── 目录/文件路径 ──────────────────────────────────────────────────────────

  Future<Directory> _baseDir() async {
    final docs = await getApplicationDocumentsDirectory();
    final dir = Directory('${docs.path}/$_dirName');
    if (!await dir.exists()) await dir.create(recursive: true);
    return dir;
  }

  Future<Directory> _sessionsDir() async {
    final base = await _baseDir();
    final dir = Directory('${base.path}/sessions');
    if (!await dir.exists()) await dir.create(recursive: true);
    return dir;
  }

  Future<File> _sessionFile(String sessionId) async {
    final dir = await _sessionsDir();
    return File('${dir.path}/$sessionId.jsonl');
  }

  Future<File> _metaFile() async {
    final base = await _baseDir();
    return File('${base.path}/$_sessionsFile');
  }

  // ── 消息 CRUD ──────────────────────────────────────────────────────────────

  /// 追加一条消息到 JSONL 文件
  Future<void> saveMessage(String sessionId, ChatMessage msg) async {
    final file = await _sessionFile(sessionId);
    final line = jsonEncode({
      'id': msg.id,
      'role': msg.role,
      'content': msg.content,
    });
    await file.writeAsString('$line\n', mode: FileMode.append, flush: true);
  }

  /// 读取 session 的所有消息
  Future<List<ChatMessage>> loadSession(String sessionId) async {
    final file = await _sessionFile(sessionId);
    if (!await file.exists()) return [];

    final lines = await file.readAsLines();
    final messages = <ChatMessage>[];
    for (final line in lines) {
      final trimmed = line.trim();
      if (trimmed.isEmpty) continue;
      try {
        final json = jsonDecode(trimmed) as Map<String, dynamic>;
        messages.add(ChatMessage(
          id: json['id'] as String,
          role: json['role'] as String,
          content: json['content'] as String,
        ));
      } catch (_) {
        // 跳过损坏行
      }
    }
    return messages;
  }

  /// 覆盖写入整个 session 的消息列表（用于编辑/删除消息场景）
  Future<void> saveAllMessages(
      String sessionId, List<ChatMessage> messages) async {
    final file = await _sessionFile(sessionId);
    final buffer = StringBuffer();
    for (final msg in messages) {
      buffer.writeln(jsonEncode({
        'id': msg.id,
        'role': msg.role,
        'content': msg.content,
      }));
    }
    await file.writeAsString(buffer.toString(), flush: true);
  }

  /// 删除 session 文件
  Future<void> deleteSession(String sessionId) async {
    final file = await _sessionFile(sessionId);
    if (await file.exists()) await file.delete();
  }

  /// 返回所有 session id（按文件修改时间倒序）
  Future<List<String>> listSessions() async {
    final dir = await _sessionsDir();
    final entities = await dir
        .list()
        .where((e) => e is File && e.path.endsWith('.jsonl'))
        .cast<File>()
        .toList();

    // 按修改时间倒序
    final withStat = await Future.wait(
      entities.map((f) async => MapEntry(f, await f.lastModified())),
    );
    withStat.sort((a, b) => b.value.compareTo(a.value));

    return withStat.map((e) {
      final name = e.key.uri.pathSegments.last;
      return name.substring(0, name.length - '.jsonl'.length);
    }).toList();
  }

  // ── Session 元数据 ─────────────────────────────────────────────────────────

  Future<List<ChatSessionMeta>> loadSessionMetas() async {
    final file = await _metaFile();
    if (!await file.exists()) return [];
    try {
      final content = await file.readAsString();
      final list = jsonDecode(content) as List<dynamic>;
      return list
          .map((e) => ChatSessionMeta.fromJson(e as Map<String, dynamic>))
          .toList();
    } catch (_) {
      return [];
    }
  }

  Future<void> saveSessionMetas(List<ChatSessionMeta> metas) async {
    final file = await _metaFile();
    final json = jsonEncode(metas.map((m) => m.toJson()).toList());
    await file.writeAsString(json, flush: true);
  }
}
