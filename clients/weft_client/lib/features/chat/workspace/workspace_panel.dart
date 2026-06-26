import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_highlight/flutter_highlight.dart';
import 'package:flutter_highlight/themes/github.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_widget_from_html_core/flutter_widget_from_html_core.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:webview_windows/webview_windows.dart';

import '../../../core/providers/chat_provider.dart';
import '../../../shared/theme/spacing.dart';
import '../../../shared/widgets/empty_state.dart';
import '../../orchestration/orchestration_provider.dart';
import '../../orchestration/orchestration_view.dart';
import 'artifact.dart';

class WorkspacePanel extends ConsumerWidget {
  const WorkspacePanel({super.key, required this.sessionId});

  final String sessionId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final workspace = ref.watch(workspaceProvider);
    final session = ref.watch(chatProvider(sessionId));
    final artifacts = artifactsFromMessages(session.messages);

    // 选中优先用 workspace.selected；为空时回退到最新产物。
    final selected = workspace.selected ??
        (artifacts.isNotEmpty ? artifacts.last : null);

    return Container(
      margin: const EdgeInsets.fromLTRB(4, 8, 8, 8),
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(16),
      ),
      child: Column(
        children: [
          _WorkspaceHeader(sessionId: sessionId, selected: selected),
          Expanded(
            child: artifacts.isEmpty
                ? const EmptyState(
                    icon: Icons.space_dashboard_outlined,
                    title: '工作区',
                    subtitle: 'agent 写文件 / 运行命令 / 生成网页时在此自动预览',
                  )
                : selected == null
                    ? const SizedBox.shrink()
                    : switch (selected.kind) {
                        ArtifactKind.file => _FilePreview(artifact: selected),
                        ArtifactKind.terminal =>
                          _TerminalPreview(artifact: selected),
                        ArtifactKind.web => _WebPreview(artifact: selected),
                        ArtifactKind.orchestration =>
                          _OrchestrationPreview(artifact: selected),
                        ArtifactKind.step => const SizedBox.shrink(),
                      },
          ),
        ],
      ),
    );
  }
}

class _WorkspaceHeader extends ConsumerWidget {
  const _WorkspaceHeader({required this.sessionId, required this.selected});

  final String sessionId;
  final Artifact? selected;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final workspace = ref.watch(workspaceProvider);
    final session = ref.watch(chatProvider(sessionId));
    final artifacts = artifactsFromMessages(session.messages);

    return Padding(
      padding: const EdgeInsets.fromLTRB(
        Spacing.md,
        Spacing.sm,
        Spacing.sm,
        Spacing.sm,
      ),
      child: Row(
        children: [
          if (selected != null) ...[
            Icon(_kindIcon(selected!.kind),
                size: 16, color: theme.colorScheme.primary),
            const SizedBox(width: Spacing.sm),
          ],
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  selected?.title ?? '工作区',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodyMedium
                      ?.copyWith(fontWeight: FontWeight.w600),
                ),
                if (selected?.path != null)
                  Text(
                    selected!.path!,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant),
                  ),
              ],
            ),
          ),
          // 自动跟随指示 / 恢复跟随
          if (!workspace.following && artifacts.isNotEmpty)
            IconButton(
              tooltip: '跟随最新产物',
              visualDensity: VisualDensity.compact,
              onPressed: () =>
                  ref.read(workspaceProvider.notifier).resumeFollow(artifacts),
              icon: Icon(Icons.vertical_align_bottom,
                  size: 18, color: theme.colorScheme.onSurfaceVariant),
            ),
          // 产物历史切换（>1 时显示）
          if (artifacts.length > 1)
            PopupMenuButton<Artifact>(
              tooltip: '历史产物 (${artifacts.length})',
              icon: Icon(Icons.history,
                  size: 18, color: theme.colorScheme.onSurfaceVariant),
              onSelected: (a) =>
                  ref.read(workspaceProvider.notifier).show(a),
              itemBuilder: (_) => artifacts.reversed
                  .map((a) => PopupMenuItem<Artifact>(
                        value: a,
                        child: Row(
                          children: [
                            Icon(_kindIcon(a.kind), size: 14),
                            const SizedBox(width: 8),
                            Flexible(
                              child: Text(a.title,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis),
                            ),
                          ],
                        ),
                      ))
                  .toList(),
            ),
          IconButton(
            tooltip: '关闭工作区',
            visualDensity: VisualDensity.compact,
            onPressed: () => ref.read(workspaceProvider.notifier).close(),
            icon: Icon(Icons.close, color: theme.colorScheme.onSurfaceVariant),
          ),
        ],
      ),
    );
  }
}

IconData _kindIcon(ArtifactKind kind) {
  switch (kind) {
    case ArtifactKind.file:
      return Icons.description_outlined;
    case ArtifactKind.terminal:
      return Icons.terminal_outlined;
    case ArtifactKind.web:
      return Icons.public_outlined;
    case ArtifactKind.orchestration:
      return Icons.groups_outlined;
    case ArtifactKind.step:
      return Icons.auto_awesome_motion_outlined;
  }
}

/// 工作区内嵌的团队编排进度。watch 按 boardId 隔离的 provider,
/// 自动轮询,复用 OrchestrationView(阶段流水线 + 活动流)。
class _OrchestrationPreview extends ConsumerWidget {
  const _OrchestrationPreview({required this.artifact});

  final Artifact artifact;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final boardId = artifact.boardId;
    if (boardId == null || boardId.isEmpty) {
      return const EmptyState(
        icon: Icons.groups_outlined,
        title: '团队编排',
        subtitle: '编排尚未启动',
      );
    }
    final state = ref.watch(boardWatchProvider(boardId));
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 12, 12, 4),
          child: Row(
            children: [
              const Icon(Icons.groups, size: 16),
              const SizedBox(width: 8),
              Expanded(
                child: Text(artifact.title,
                    style: theme.textTheme.titleSmall,
                    overflow: TextOverflow.ellipsis),
              ),
            ],
          ),
        ),
        Expanded(
          child: OrchestrationView(
            tasks: state.tasks,
            handoffs: state.handoffs,
            activity: state.activity,
            compact: true,
            error: state.error,
            running: state.running,
          ),
        ),
      ],
    );
  }
}

class _FilePreview extends StatelessWidget {
  const _FilePreview({required this.artifact});

  final Artifact artifact;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final filePath = artifact.path;
    // 优先读磁盘完整文件：stream/events 里的 content 被截断到 1000 字符（仅预览用），
    // 直接渲染会导致 HTML/代码不完整。磁盘文件才是完整的。
    final content = _fullContent(artifact);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _PreviewHeader(
          icon: Icons.insert_drive_file_outlined,
          title: artifact.title,
          subtitle: filePath ?? '未提供路径',
        ),
        const Divider(height: 1),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.all(Spacing.md),
            child: artifact.isImage
                ? _ImagePreviewBody(path: filePath)
                : content.isEmpty
                    ? const EmptyState(
                        icon: Icons.notes_outlined,
                        title: '暂无文件内容',
                        subtitle: '当前产物没有返回可预览的文本内容',
                      )
                    : artifact.isMarkdown
                        ? SingleChildScrollView(
                            child: MarkdownBody(
                              data: content,
                              selectable: true,
                              styleSheet:
                                  MarkdownStyleSheet.fromTheme(theme).copyWith(
                                p: theme.textTheme.bodyMedium?.copyWith(
                                  color: theme.colorScheme.onSurface,
                                ),
                                code: theme.textTheme.bodySmall?.copyWith(
                                  fontFamily: 'monospace',
                                  backgroundColor: theme
                                      .colorScheme.surfaceContainerHighest,
                                ),
                              ),
                            ),
                          )
                        : SingleChildScrollView(
                            child: ClipRRect(
                              borderRadius: BorderRadius.circular(16),
                              child: HighlightView(
                                content,
                                language: artifact.language ?? 'plaintext',
                                theme: githubTheme,
                                padding: const EdgeInsets.all(Spacing.md),
                                textStyle: theme.textTheme.bodySmall?.copyWith(
                                  height: 1.5,
                                  fontFamily: 'monospace',
                                ),
                              ),
                            ),
                          ),
          ),
        ),
      ],
    );
  }
}

class _ImagePreviewBody extends StatelessWidget {
  const _ImagePreviewBody({required this.path});

  final String? path;

  @override
  Widget build(BuildContext context) {
    if (path == null || path!.trim().isEmpty) {
      return const EmptyState(
        icon: Icons.broken_image_outlined,
        title: '图片路径缺失',
        subtitle: '当前产物没有可读取的本地图片路径',
      );
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        return SingleChildScrollView(
          child: ConstrainedBox(
            constraints: BoxConstraints(minHeight: constraints.maxHeight),
            child: Center(
              child: Image.file(
                File(path!),
                fit: BoxFit.contain,
                errorBuilder: (context, error, stackTrace) {
                  return const EmptyState(
                    icon: Icons.hide_image_outlined,
                    title: '图片加载失败',
                    subtitle: '文件不存在或当前环境无法访问该图片',
                  );
                },
              ),
            ),
          ),
        );
      },
    );
  }
}

class _TerminalPreview extends StatelessWidget {
  const _TerminalPreview({required this.artifact});

  final Artifact artifact;

  static final RegExp _ansiPattern = RegExp(r'\x1B\[[0-9;]*[A-Za-z]');

  /// 保守退出徽标：有输出视为已完成，无输出（仍在跑/无回显）显示 "—"。
  /// host_fn 结果未单独回传 exit code，故不臆造 "exit 0"。
  static String _exitBadgeLabel(String? output) {
    return (output ?? '').trim().isNotEmpty ? 'done' : '—';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final output = (artifact.output ?? '').replaceAll(_ansiPattern, '');
    final command = artifact.command?.trim().isNotEmpty == true
        ? artifact.command!.trim()
        : artifact.title;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _PreviewHeader(
          icon: Icons.terminal,
          title: artifact.title,
          subtitle: command,
          trailing: Wrap(
            spacing: Spacing.sm,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              if ((artifact.toolName ?? '').isNotEmpty)
                _MetaBadge(label: artifact.toolName!),
              _MetaBadge(label: _exitBadgeLabel(artifact.output)),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: Container(
            color: theme.colorScheme.surfaceContainerLowest,
            padding: const EdgeInsets.all(Spacing.md),
            child: output.trim().isEmpty
                ? const EmptyState(
                    icon: Icons.terminal_outlined,
                    title: '暂无终端输出',
                    subtitle: '命令已记录，但当前没有可展示的输出内容',
                  )
                : SelectionArea(
                    child: SingleChildScrollView(
                      child: Text(
                        output,
                        style: theme.textTheme.bodySmall?.copyWith(
                          fontFamily: 'monospace',
                          height: 1.5,
                          color: theme.colorScheme.onSurface,
                        ),
                      ),
                    ),
                  ),
          ),
        ),
      ],
    );
  }
}

/// 网页/HTML 产物预览。
///
/// Windows: 用 webview_windows（WebView2/Edge 内核）真实渲染 HTML（完整 CSS/JS/grid/渐变），
/// 所见即所得。webview 初始化失败或非 HTML 内容 → 回退到文本摘要 + "在浏览器打开"。
class _WebPreview extends StatefulWidget {
  const _WebPreview({required this.artifact});

  final Artifact artifact;

  @override
  State<_WebPreview> createState() => _WebPreviewState();
}

class _WebPreviewState extends State<_WebPreview> {
  final _controller = WebviewController();
  bool _webviewReady = false;
  String? _webviewError;
  // 记录已加载的内容签名，避免重复 loadStringContent。
  String? _loadedSig;

  /// 内嵌 webview 仅在 Windows 可用（webview_windows = WebView2/Edge 内核）。
  /// macOS/Linux 尚未接入原生 webview，统一走"外部浏览器打开"兜底。
  bool get _webviewSupported => Platform.isWindows;

  @override
  void initState() {
    super.initState();
    _initWebview();
  }

  Future<void> _initWebview() async {
    if (!_webviewSupported) return; // 仅 Windows(WebView2）；其他平台走外部浏览器兜底。
    if (!_isHtml) return; // 非 HTML 不初始化 webview，省资源。
    try {
      await _controller.initialize();
      if (!mounted) return;
      setState(() => _webviewReady = true);
      await _loadCurrent();
    } catch (e) {
      if (!mounted) return;
      setState(() => _webviewError = e.toString());
    }
  }

  /// 把当前产物的完整 HTML 加载进 webview。内容变化时才重载。
  Future<void> _loadCurrent() async {
    if (!_webviewReady) return;
    final html = _fullContent(widget.artifact);
    if (html.isEmpty) return;
    final sig = '${widget.artifact.path}:${html.length}';
    if (sig == _loadedSig) return;
    _loadedSig = sig;
    try {
      // 用 file:// base 让相对资源/字体能解析；直接灌完整 HTML 字符串。
      await _controller.loadStringContent(html);
    } catch (e) {
      if (mounted) setState(() => _webviewError = e.toString());
    }
  }

  @override
  void didUpdateWidget(covariant _WebPreview oldWidget) {
    super.didUpdateWidget(oldWidget);
    // 切换到另一个 HTML 产物 → 重新加载。
    if (_webviewSupported && _isHtml) _loadCurrent();
  }

  @override
  void dispose() {
    if (_webviewSupported) _controller.dispose();
    super.dispose();
  }

  /// 可在浏览器打开的目标：本地 HTML 文件 → file:// URI；否则远程 url。
  String? get _target {
    final path = widget.artifact.path?.trim();
    if (path != null && path.isNotEmpty) return Uri.file(path).toString();
    final url = widget.artifact.url?.trim();
    if (url != null && url.isNotEmpty) return url;
    return null;
  }

  /// 解析出一个可打开的本地绝对路径：
  /// 1) artifact.path 能定位到存在的文件 → 用它的绝对路径；
  /// 2) 否则（常见：AI 用相对路径写文件，app 与 core 工作目录不同）→
  ///    把完整 HTML 内容落到临时文件，返回临时文件路径。
  /// 这样"在浏览器打开"不依赖猜测 core 的工作目录，始终可用。
  Future<String?> _resolveOrMaterialize() async {
    final path = widget.artifact.path?.trim();
    if (path != null && path.isNotEmpty) {
      final f = File(path);
      if (f.existsSync()) return f.absolute.path;
    }
    // 路径不可达 → 用内容兜底写临时文件。
    final html = _fullContent(widget.artifact);
    if (html.isEmpty) return null;
    try {
      final dir = Directory.systemTemp.createTempSync('weft_preview_');
      final name = (path != null && path.isNotEmpty)
          ? path.split(RegExp(r'[\\/]')).last
          : 'preview.html';
      final tmp = File('${dir.path}${Platform.pathSeparator}$name');
      tmp.writeAsStringSync(html);
      return tmp.absolute.path;
    } catch (_) {
      return null;
    }
  }

  /// 用系统默认程序打开本地文件（Windows 上 launchUrl 对 file:// 常失败，
  /// 故走 explorer/open/xdg-open）。相对路径无法定位时用内容落临时文件兜底。
  Future<void> _openExternal() async {
    final abs = await _resolveOrMaterialize();
    if (abs != null) {
      try {
        if (Platform.isWindows) {
          await Process.start('explorer', [abs]);
          return;
        } else if (Platform.isMacOS) {
          await Process.start('open', [abs]);
          return;
        } else {
          await Process.start('xdg-open', [abs]);
          return;
        }
      } catch (_) {
        // 落到 launchUrl 兜底。
      }
      final uri = Uri.file(abs);
      await launchUrl(uri, mode: LaunchMode.externalApplication);
      return;
    }
    // 远程 url 产物。
    final url = widget.artifact.url?.trim();
    if (url == null || url.isEmpty) return;
    final uri = Uri.tryParse(url);
    if (uri == null) return;
    await launchUrl(uri, mode: LaunchMode.externalApplication);
  }

  /// content 是否为 HTML（.html 文件，或完整内容含明显 HTML 标签）。
  /// 用 _fullContent（磁盘完整文件），不用截断的 artifact.content。
  bool get _isHtml {
    final p = widget.artifact.path?.toLowerCase();
    if (p != null && (p.endsWith('.html') || p.endsWith('.htm'))) return true;
    final c = _fullContent(widget.artifact);
    return c.contains('<html') ||
        c.contains('<!DOCTYPE') ||
        c.contains('<body') ||
        c.contains('<div');
  }

  @override
  Widget build(BuildContext context) {
    final target = _target;
    final summary = _fullContent(widget.artifact);
    final a = widget.artifact;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _PreviewHeader(
          icon: Icons.public,
          title: a.title,
          subtitle: (a.url != null && a.url!.trim().isNotEmpty)
              ? a.url!.trim()
              : (a.path != null &&
                      a.path!.trim().isNotEmpty &&
                      a.path!.trim() != a.title)
                  ? a.path!.trim()
                  : null,
          trailing: target == null
              ? null
              : FilledButton.tonalIcon(
                  onPressed: _openExternal,
                  icon: const Icon(Icons.open_in_new, size: 16),
                  label: const Text('在浏览器打开'),
                ),
        ),
        const Divider(height: 1),
        Expanded(child: _buildBody(summary, target)),
      ],
    );
  }

  Widget _buildBody(String summary, String? target) {
    // HTML 且 webview 支持(Windows)且就绪 → 真实渲染。
    if (_webviewSupported && _isHtml && _webviewError == null) {
      if (_webviewReady) {
        return Webview(_controller);
      }
      // 初始化中。
      return const Center(child: CircularProgressIndicator());
    }

    // webview 失败 → 提示 + 文本兜底（仍可外部打开）。
    if (_webviewSupported && _isHtml && _webviewError != null) {
      return _FallbackHtmlText(summary: summary, error: _webviewError);
    }

    // 非 Windows 的 HTML → 简易渲染 + 引导外部浏览器（无内嵌 webview）。
    if (!_webviewSupported && _isHtml && summary.isNotEmpty) {
      return _FallbackHtmlText(
        summary: summary,
        error: null,
      );
    }

    // 非 HTML（web_search 文本结果等）→ 纯文本。
    if (summary.isEmpty) {
      return EmptyState(
        icon: Icons.travel_explore_outlined,
        title: target != null ? '网页产物' : '搜索结果待补充',
        subtitle: target != null
            ? '点右上角"在浏览器打开"查看完整渲染效果'
            : '当前产物没有返回可预览的网页文本结果',
      );
    }
    return Container(
      margin: const EdgeInsets.all(Spacing.md),
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: Theme.of(context)
              .colorScheme
              .outlineVariant
              .withValues(alpha: 0.6),
        ),
      ),
      clipBehavior: Clip.antiAlias,
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(Spacing.md),
        child: SelectionArea(
          child: Text(
            summary,
            style: const TextStyle(color: Colors.black87, height: 1.6),
          ),
        ),
      ),
    );
  }
}

/// 简化 HTML 渲染 + 提示条。
/// error != null：webview 初始化失败的降级；error == null：非 Windows 平台无内嵌 webview。
class _FallbackHtmlText extends StatelessWidget {
  const _FallbackHtmlText({required this.summary, this.error});

  final String summary;
  final String? error;

  @override
  Widget build(BuildContext context) {
    final isError = error != null;
    final scheme = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(Spacing.sm),
          color: isError
              ? scheme.errorContainer
              : scheme.secondaryContainer,
          child: Text(
            isError
                ? '内嵌预览不可用，已降级为简易渲染（点"在浏览器打开"看完整效果）'
                : '当前平台暂用简易渲染，点右上角"在浏览器打开"查看完整效果',
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: isError
                      ? scheme.onErrorContainer
                      : scheme.onSecondaryContainer,
                ),
          ),
        ),
        Expanded(
          child: Container(
            margin: const EdgeInsets.all(Spacing.md),
            decoration: BoxDecoration(
              color: Colors.white,
              borderRadius: BorderRadius.circular(12),
            ),
            clipBehavior: Clip.antiAlias,
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(Spacing.md),
              child: HtmlWidget(
                summary,
                textStyle: const TextStyle(
                  color: Colors.black87,
                  fontFamilyFallback: [
                    'Microsoft YaHei',
                    'PingFang SC',
                    'Noto Sans CJK SC',
                    'SimSun',
                  ],
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _PreviewHeader extends StatelessWidget {
  const _PreviewHeader({
    required this.icon,
    required this.title,
    this.subtitle,
    this.trailing,
  });

  final IconData icon;
  final String title;
  final String? subtitle;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.all(Spacing.md),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHigh,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Icon(icon, size: 18, color: theme.colorScheme.primary),
          ),
          const SizedBox(width: Spacing.sm),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                if (subtitle != null && subtitle!.isNotEmpty) ...[
                  const SizedBox(height: 4),
                  SelectableText(
                    subtitle!,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                      fontFamily: 'monospace',
                      height: 1.4,
                    ),
                  ),
                ],
              ],
            ),
          ),
          if (trailing != null) ...[
            const SizedBox(width: Spacing.md),
            trailing!,
          ],
        ],
      ),
    );
  }
}

class _MetaBadge extends StatelessWidget {
  const _MetaBadge({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        label,
        style: theme.textTheme.labelMedium?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }
}


/// 取产物的完整内容：优先读磁盘文件（完整），回退到 artifact.content
/// （来自 stream/events 的预览，被截断到 1000 字符，渲染会不完整）。
/// 桌面端小文件同步读可接受。
String _fullContent(Artifact artifact) {
  final path = artifact.path?.trim();
  if (path != null && path.isNotEmpty) {
    try {
      final f = File(path);
      if (f.existsSync()) {
        final disk = f.readAsStringSync();
        if (disk.trim().isNotEmpty) return disk;
      }
    } catch (_) {
      // 读盘失败（路径不可达/权限）→ 回退 event content。
    }
  }
  return artifact.content?.trim() ?? '';
}
