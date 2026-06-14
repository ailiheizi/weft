import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/models/chat.dart';

/// 工作区产物类型。
enum ArtifactKind { file, terminal, web, step, orchestration }

/// 从 agent 的 ToolCall 归一化出的"产物"，供右侧工作区面板预览。
///
/// 不使用 freezed（避免引入 build_runner 代码生成依赖）；纯 Dart class。
class Artifact {
  const Artifact({
    required this.id,
    required this.kind,
    required this.title,
    this.path,
    this.content,
    this.language,
    this.command,
    this.output,
    this.url,
    this.boardId,
    this.toolName,
  });

  /// 对应 ToolCallStep.id（用于选中/去重）。
  final String id;
  final ArtifactKind kind;

  /// 面板标题 / chip 文案，如 "foo.rs" / "git status"。
  final String title;

  // file
  final String? path;
  final String? content;
  final String? language;

  // terminal
  final String? command;
  final String? output;

  // web
  final String? url;

  // orchestration: 团队编排看板 id
  final String? boardId;

  final String? toolName;

  /// 从一个 ToolCallStep 归一化出 Artifact；无法识别为产物时返回 null。
  static Artifact? fromToolCall(ToolCallStep step) {
    final name = step.name.trim();
    final args = _tryJson(step.arguments);
    final result = _tryJson(step.result);

    switch (name) {
      case 'fs_write':
      case 'fs_read':
        final path = _str(args, 'path');
        if (path == null) return null;
        // fs_write 的 content 在 args；fs_read 的内容在 result。
        final content = _str(args, 'content') ??
            _resultText(step.result, result) ??
            '';
        // HTML 文件路由到 web 预览（看渲染效果），而非源码高亮。
        final lower = path.toLowerCase();
        if (lower.endsWith('.html') || lower.endsWith('.htm')) {
          return Artifact(
            id: step.id,
            kind: ArtifactKind.web,
            title: _basename(path),
            path: path,
            content: content,
            toolName: name,
          );
        }
        return Artifact(
          id: step.id,
          kind: ArtifactKind.file,
          title: _basename(path),
          path: path,
          content: content,
          language: _languageFromPath(path),
          toolName: name,
        );
      case 'shell_exec':
      case 'git':
        final command = _str(args, 'command') ??
            _str(args, 'cmd') ??
            (name == 'git' ? 'git' : '');
        return Artifact(
          id: step.id,
          kind: ArtifactKind.terminal,
          title: command.isEmpty ? name : command,
          command: command,
          output: _resultText(step.result, result) ?? '',
          toolName: name,
        );
      case 'web_fetch':
      case 'web_search':
        // 搜索/抓取结果不是"AI 输出的文件"，不占用工作区预览；
        // 其结果在聊天区文本展示即可。返回 null 让它不进工作区。
        return null;
      case 'delegate_to_team':
        // AI 把任务委托给 agent 团队 → 在工作区内嵌编排进度。
        // board_id 在 tool result 里(可能嵌在 response.data 或顶层)。
        final boardId = _deepStr(result, 'board_id') ?? _str(args, 'board_id');
        if (boardId == null) return null;
        final title = _deepStr(result, 'title') ??
            _str(args, 'title') ??
            _str(args, 'goal') ??
            '团队编排';
        return Artifact(
          id: step.id,
          kind: ArtifactKind.orchestration,
          title: title,
          boardId: boardId,
          toolName: name,
        );
      default:
        return null;
    }
  }

  // ── helpers ──────────────────────────────────────────────────────────────

  static Map<String, dynamic>? _tryJson(String? raw) {
    if (raw == null || raw.trim().isEmpty) return null;
    try {
      final v = jsonDecode(raw);
      return v is Map<String, dynamic> ? v : null;
    } catch (_) {
      return null;
    }
  }

  static String? _str(Map<String, dynamic>? m, String key) {
    final v = m?[key];
    if (v == null) return null;
    final s = v is String ? v : v.toString();
    return s.isEmpty ? null : s;
  }

  /// 在嵌套 JSON 里递归找第一个匹配 key 的字符串值。
  /// tool result 的 board_id 可能在顶层或嵌在 response.data 里，故深度搜索。
  static String? _deepStr(dynamic node, String key, [int depth = 0]) {
    if (depth > 5 || node == null) return null;
    if (node is Map) {
      final direct = node[key];
      if (direct is String && direct.isNotEmpty) return direct;
      for (final v in node.values) {
        final found = _deepStr(v, key, depth + 1);
        if (found != null) return found;
      }
    } else if (node is List) {
      for (final v in node) {
        final found = _deepStr(v, key, depth + 1);
        if (found != null) return found;
      }
    }
    return null;
  }

  /// result 可能是 JSON（取常见字段）或纯文本，回退到原始字符串。
  static String? _resultText(String? raw, Map<String, dynamic>? parsed) {
    if (parsed != null) {
      for (final k in ['stdout', 'output', 'content', 'result', 'text']) {
        final v = parsed[k];
        if (v is String && v.isNotEmpty) return v;
      }
    }
    final s = raw?.trim();
    return (s == null || s.isEmpty) ? null : s;
  }

  static String _basename(String path) {
    final norm = path.replaceAll('\\', '/');
    final idx = norm.lastIndexOf('/');
    return idx >= 0 ? norm.substring(idx + 1) : norm;
  }

  static String? _languageFromPath(String path) {
    final lower = path.toLowerCase();
    final dot = lower.lastIndexOf('.');
    if (dot < 0) return null;
    switch (lower.substring(dot + 1)) {
      case 'rs':
        return 'rust';
      case 'dart':
        return 'dart';
      case 'js':
      case 'mjs':
        return 'javascript';
      case 'ts':
        return 'typescript';
      case 'py':
        return 'python';
      case 'go':
        return 'go';
      case 'json':
        return 'json';
      case 'toml':
        return 'ini';
      case 'yaml':
      case 'yml':
        return 'yaml';
      case 'sh':
        return 'bash';
      case 'html':
        return 'xml';
      case 'css':
        return 'css';
      case 'md':
      case 'markdown':
        return 'markdown';
      default:
        return null;
    }
  }

  /// markdown 文件用 markdown 渲染器，其余用代码高亮。
  bool get isMarkdown =>
      kind == ArtifactKind.file &&
      (path?.toLowerCase().endsWith('.md') == true ||
          path?.toLowerCase().endsWith('.markdown') == true);

  bool get isImage {
    final p = path?.toLowerCase();
    if (p == null) return false;
    return p.endsWith('.png') ||
        p.endsWith('.jpg') ||
        p.endsWith('.jpeg') ||
        p.endsWith('.gif') ||
        p.endsWith('.webp') ||
        p.endsWith('.bmp');
  }
}

/// 工作区面板状态：展开/折叠、当前选中产物、当前 Tab。
class WorkspaceState {
  const WorkspaceState({
    this.open = false,
    this.selected,
    this.following = true,
  });

  final bool open;
  final Artifact? selected;

  /// 是否自动跟随最新产物。用户手动选了历史产物时关闭，新会话或点"跟随最新"时恢复。
  final bool following;

  WorkspaceState copyWith({
    bool? open,
    Artifact? selected,
    bool clearSelected = false,
    bool? following,
  }) {
    return WorkspaceState(
      open: open ?? this.open,
      selected: clearSelected ? null : (selected ?? this.selected),
      following: following ?? this.following,
    );
  }
}

class WorkspaceNotifier extends StateNotifier<WorkspaceState> {
  WorkspaceNotifier() : super(const WorkspaceState());

  /// 用户手动点某历史产物 → 展开、选中、停止自动跟随。
  void show(Artifact artifact) {
    state = state.copyWith(open: true, selected: artifact, following: false);
  }

  /// 自动跟随：产物列表更新时调用。following 时自动选中最新产物并展开面板。
  /// 这是 agent 写文件/生成网页后"工作区自动加载"的核心。
  void followLatest(List<Artifact> artifacts) {
    if (artifacts.isEmpty) return;
    if (!state.following) return;
    final latest = artifacts.last;
    if (state.selected?.id == latest.id && state.open) return;
    state = state.copyWith(open: true, selected: latest);
  }

  /// 会话切换/产物列表变化时同步工作区，避免显示其它会话的残留产物：
  /// - 列表为空 → 清空 selected（关闭残留预览）。
  /// - selected 已不在当前列表 → 回退跟随最新。
  /// - following 时 → 跟随最新。
  void syncToSession(List<Artifact> artifacts) {
    if (artifacts.isEmpty) {
      if (state.selected != null) {
        state = state.copyWith(clearSelected: true, following: true);
      }
      return;
    }
    final ids = artifacts.map((a) => a.id).toSet();
    final selectedGone =
        state.selected != null && !ids.contains(state.selected!.id);
    if (selectedGone) {
      // 当前选中的产物不属于这个会话 → 跟随最新。
      state = state.copyWith(selected: artifacts.last, following: true);
      return;
    }
    followLatest(artifacts);
  }

  /// 恢复自动跟随最新。
  void resumeFollow(List<Artifact> artifacts) {
    state = state.copyWith(
      following: true,
      open: true,
      selected: artifacts.isNotEmpty ? artifacts.last : null,
      clearSelected: artifacts.isEmpty,
    );
  }

  /// 顶栏开关：关闭时停止自动跟随(尊重用户)；打开时恢复跟随(主动看=想跟最新)。
  void toggle() {
    final willOpen = !state.open;
    state = state.copyWith(open: willOpen, following: willOpen);
  }

  /// 用户手动关闭工作区：同时停止自动跟随，避免 agent 下次写文件又强制弹开。
  /// 想恢复自动预览可手动点产物(show)或调 resumeFollow。
  void close() => state = state.copyWith(open: false, following: false);
}

final workspaceProvider =
    StateNotifierProvider<WorkspaceNotifier, WorkspaceState>(
  (ref) => WorkspaceNotifier(),
);

/// 从一组消息里抽出所有产物（按出现顺序，去重 by id）。
List<Artifact> artifactsFromMessages(List<ChatMessage> messages) {
  final out = <Artifact>[];
  final seen = <String>{};
  for (final m in messages) {
    for (final step in m.steps) {
      if (step is ToolCallStep) {
        final a = Artifact.fromToolCall(step);
        if (a != null && seen.add(a.id)) out.add(a);
      }
    }
  }
  return out;
}
