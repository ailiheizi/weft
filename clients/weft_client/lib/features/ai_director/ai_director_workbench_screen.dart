import 'dart:async';
import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:uuid/uuid.dart';

import '../../core/models/app.dart';
import '../../core/providers/core_repository.dart';
import '../../shared/theme/spacing.dart';
import '../../shared/widgets/empty_state.dart';
import '../../shared/widgets/hover_card.dart';
import 'ai_director_mock.dart';
import 'ai_director_timeline_painter.dart';
import 'canvas/hub_canvas_view.dart';

const _uuid = Uuid();

/// Native workbench surface for the ai-director product.
class AiDirectorWorkbenchScreen extends ConsumerStatefulWidget {
  const AiDirectorWorkbenchScreen({super.key, required this.app});

  final ResolvedApp app;

  @override
  ConsumerState<AiDirectorWorkbenchScreen> createState() =>
      _AiDirectorWorkbenchScreenState();
}

class _AiDirectorWorkbenchScreenState
    extends ConsumerState<AiDirectorWorkbenchScreen> {
  static const _headerHeight = 72.0;
  static const _timelineHeight = 220.0;
  static const _mockDurationSecond = 136.0;

  late final Player _player = Player();
  late final VideoController _videoController = VideoController(_player);
  late final TextEditingController _messageController = TextEditingController();

  int _selectedPlanIndex = 0;
  double _previewSecond = 12;
  double _videoDurationSecond = 0;
  bool _isPlaying = false;

  /// Hub 画布视图（默认）↔ 经典视图（旧 mock 时间线）。
  bool _hubView = true;
  String? _importedVideoPath;
  bool _generatingPlan = false;
  String? _aiPlanText;
  String? _planError;
  String? _directorSessionId;
  String? _askUserQuestion;
  List<String> _askUserOptions = const [];

  StreamSubscription<Duration>? _durationSubscription;
  StreamSubscription<Duration>? _positionSubscription;
  StreamSubscription<bool>? _playingSubscription;

  bool get _hasVideo => _importedVideoPath != null;

  double get _effectiveTotalSecond => _hasVideo && _videoDurationSecond > 0
      ? _videoDurationSecond
      : _mockDurationSecond;

  String get _effectiveTotalLabel => _hasVideo && _videoDurationSecond > 0
      ? _formatTimelineTime(_videoDurationSecond)
      : mockDirectorAssets.isNotEmpty
            ? mockDirectorAssets.first.durationLabel
            : '02:16';

  DirectorMediaAsset? get _importedAsset {
    final path = _importedVideoPath;
    if (path == null) {
      return null;
    }
    return DirectorMediaAsset(
      fileName: path.split(Platform.pathSeparator).last,
      durationLabel: _videoDurationSecond > 0
          ? _formatTimelineTime(_videoDurationSecond)
          : '载入中',
    );
  }

  @override
  void initState() {
    super.initState();
    _durationSubscription = _player.stream.duration.listen((duration) {
      if (!mounted) {
        return;
      }
      setState(() {
        _videoDurationSecond = duration.inMilliseconds / 1000;
      });
    });
    _positionSubscription = _player.stream.position.listen((position) {
      if (!mounted || !_hasVideo) {
        return;
      }
      setState(() {
        final second = position.inMilliseconds / 1000;
        _previewSecond = second.clamp(0, _effectiveTotalSecond).toDouble();
      });
    });
    _playingSubscription = _player.stream.playing.listen((playing) {
      if (!mounted) {
        return;
      }
      setState(() {
        _isPlaying = playing;
      });
    });
  }

  @override
  void dispose() {
    _durationSubscription?.cancel();
    _positionSubscription?.cancel();
    _playingSubscription?.cancel();
    _messageController.dispose();
    _player.dispose();
    super.dispose();
  }

  Future<void> _importVideo() async {
    // TODO: 接 runApp(app.name, capability, 'import_media', data) 打开素材导入流程。
    final result = await FilePicker.pickFiles(type: FileType.video);
    final path = result?.files.single.path;
    if (path == null) {
      return;
    }

    setState(() {
      _importedVideoPath = path;
      _previewSecond = 0;
      _videoDurationSecond = 0;
      _isPlaying = false;
    });

    await _player.open(Media(path));
  }

  Future<void> _togglePlayPause() async {
    if (_hasVideo) {
      await _player.playOrPause();
      return;
    }
    setState(() {
      _previewSecond = (_previewSecond + 1)
          .clamp(0, _effectiveTotalSecond)
          .toDouble();
    });
  }

  Future<void> _seekPreview(double value) async {
    final target = value.clamp(0, _effectiveTotalSecond).toDouble();
    if (_hasVideo) {
      await _player.seek(Duration(milliseconds: (target * 1000).round()));
      return;
    }
    setState(() {
      _previewSecond = target;
    });
  }

  Future<void> _seekBy(double deltaSecond) async {
    await _seekPreview(_previewSecond + deltaSecond);
  }

  Future<void> _sendDirectorMessage([String? presetContent]) async {
    if (_generatingPlan) {
      return;
    }

    final content = (presetContent ?? _messageController.text).trim();
    if (content.isEmpty) {
      setState(() {
        _planError = '请先输入你的创意或选择导演提问选项。';
      });
      return;
    }

    final sessionId = _directorSessionId ?? _uuid.v4();

    setState(() {
      _directorSessionId = sessionId;
      _generatingPlan = true;
      _planError = null;
      _askUserQuestion = null;
      _askUserOptions = const [];
    });

    try {
      final result = await ref.read(coreRepositoryProvider).runApp(
            'ai-director',
            'director.turn',
            'send_message',
            {
              'content': content,
              'session_id': sessionId,
            },
          );
      final reply = _extractDirectorReply(result);
      final askUserQuestion = _extractAskUserQuestion(result);
      final askUserOptions = _extractAskUserOptions(result);
      if (!mounted) {
        return;
      }
      setState(() {
        _aiPlanText = reply;
        _askUserQuestion = askUserQuestion;
        _askUserOptions = askUserOptions;
        if (presetContent == null) {
          _messageController.clear();
        }
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      final message = error is StateError
          ? error.message.toString()
          : '主导演暂时没有响应，请稍后重试。';
      setState(() {
        _planError = message;
      });
    }

    if (!mounted) {
      return;
    }
    setState(() {
      _generatingPlan = false;
    });
  }

  String _extractDirectorReply(Map<String, dynamic> result) {
    final responseData = _asMap(_asMap(_asMap(result['result'])?['response'])?['data']);
    final reply = _readText(responseData?['reply']);
    if (reply != null && reply.isNotEmpty) {
      return reply;
    }

    for (final event in _extractEvents(result)) {
      if (event['type']?.toString() != 'assistant_message') {
        continue;
      }
      final payload = _asMap(event['payload']);
      final content = _readText(payload?['content']);
      if (content != null && content.isNotEmpty) {
        return content;
      }
    }

    throw StateError('主导演未返回可展示的回复内容。');
  }

  String? _extractAskUserQuestion(Map<String, dynamic> result) {
    for (final event in _extractEvents(result)) {
      if (event['type']?.toString() != 'ask_user') {
        continue;
      }
      final payload = _asMap(event['payload']);
      return _readText(payload?['question']);
    }
    return null;
  }

  List<String> _extractAskUserOptions(Map<String, dynamic> result) {
    for (final event in _extractEvents(result)) {
      if (event['type']?.toString() != 'ask_user') {
        continue;
      }
      final payload = _asMap(event['payload']);
      final options = _asList(payload?['options'])
          .map((item) => item.toString().trim())
          .where((item) => item.isNotEmpty)
          .toList();
      if (options.isNotEmpty) {
        return options;
      }
    }
    return const [];
  }

  List<Map<String, dynamic>> _extractEvents(Map<String, dynamic> result) {
    final events = _asList(_asMap(_asMap(result['result'])?['response'])?['events']);
    return events
        .map(_asMap)
        .whereType<Map<String, dynamic>>()
        .toList(growable: false);
  }

  Map<String, dynamic>? _asMap(dynamic value) {
    if (value is Map<String, dynamic>) {
      return value;
    }
    if (value is Map) {
      return Map<String, dynamic>.from(value);
    }
    return null;
  }

  List<dynamic> _asList(dynamic value) {
    return value is List ? value : const [];
  }

  String? _readText(dynamic value) {
    if (value is String) {
      final trimmed = value.trim();
      return trimmed.isEmpty ? null : trimmed;
    }
    if (value is List) {
      final buffer = <String>[];
      for (final item in value) {
        if (item is String) {
          final trimmed = item.trim();
          if (trimmed.isNotEmpty) {
            buffer.add(trimmed);
          }
          continue;
        }
        final itemMap = _asMap(item);
        final nestedText = _readText(itemMap?['text'] ?? itemMap?['content']);
        if (nestedText != null && nestedText.isNotEmpty) {
          buffer.add(nestedText);
        }
      }
      if (buffer.isNotEmpty) {
        return buffer.join('\n');
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    // TODO: 接 runApp(widget.app.name, capability, action, data) 拉取素材、方案、风格档案与时间线数据。
    final assets = [
      ...?_importedAsset == null ? null : [_importedAsset!],
      ...mockDirectorAssets,
    ];
    final plans = mockDirectorPlans;
    final styleProfile = mockStyleProfile;
    final selectedPlan = plans.isNotEmpty ? plans[_selectedPlanIndex] : null;
    final planActionLabel = _aiPlanText == null ? '发消息' : '继续追问';

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: SafeArea(
        bottom: false,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final isWide = constraints.maxWidth >= 1000;
            final compactHeight = constraints.maxHeight < 760;
            final leftRailWidth = isWide ? 292.0 : 252.0;

            return Column(
              children: [
                _WorkbenchHeader(
                  app: widget.app,
                  styleProfile: styleProfile,
                  height: _headerHeight,
                ),
                Expanded(
                  child: _hubView
                      ? const HubCanvasView()
                      : Padding(
                    padding: const EdgeInsets.fromLTRB(
                      Spacing.md,
                      Spacing.sm,
                      Spacing.md,
                      Spacing.sm,
                    ),
                    child: Column(
                      children: [
                        Expanded(
                          child: Row(
                            crossAxisAlignment: CrossAxisAlignment.stretch,
                            children: [
                              SizedBox(
                                width: leftRailWidth,
                                child: _LeftSidebar(
                                  assets: assets,
                                  importedAsset: _importedAsset,
                                  plans: plans,
                                  selectedPlanIndex: _selectedPlanIndex,
                                  compactHeight: compactHeight,
                                  generatingPlan: _generatingPlan,
                                  aiPlanText: _aiPlanText,
                                  planError: _planError,
                                  askUserQuestion: _askUserQuestion,
                                  askUserOptions: _askUserOptions,
                                  sessionId: _directorSessionId,
                                  messageController: _messageController,
                                  planActionLabel: planActionLabel,
                                  onImportPressed: _importVideo,
                                  onGeneratePlanPressed: _sendDirectorMessage,
                                  onAskUserOptionPressed: _sendDirectorMessage,
                                  onPlanSelected: (index) {
                                    setState(() {
                                      _selectedPlanIndex = index;
                                    });
                                  },
                                  styleProfile: styleProfile,
                                ),
                              ),
                              const SizedBox(width: Spacing.sm),
                              Expanded(
                                child: _PreviewStage(
                                  currentSecond: _previewSecond,
                                  totalLabel: _effectiveTotalLabel,
                                  totalSecond: _effectiveTotalSecond,
                                  selectedPlan: selectedPlan,
                                  compactHeight: compactHeight,
                                  videoController: _hasVideo ? _videoController : null,
                                  hasVideo: _hasVideo,
                                  isPlaying: _isPlaying,
                                  onPlayPause: _togglePlayPause,
                                  onSeekBackward: () => _seekBy(-5),
                                  onSeekForward: () => _seekBy(5),
                                  onScrub: _seekPreview,
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(height: Spacing.sm),
                        SizedBox(
                          height: _timelineHeight,
                          child: _TimelineSection(
                            plan: selectedPlan,
                            currentSecond: _previewSecond,
                            totalDuration: _effectiveTotalSecond,
                            compactHeight: compactHeight,
                            onAccept: () {
                              // TODO: 接 runApp(widget.app.name, capability, 'accept_plan', data) 采纳方案。
                            },
                            onRecut: () {
                              // TODO: 接 runApp(widget.app.name, capability, 'reject_plan', data) 请求重剪。
                            },
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _WorkbenchHeader extends StatelessWidget {
  const _WorkbenchHeader({
    required this.app,
    required this.styleProfile,
    required this.height,
  });

  final ResolvedApp app;
  final List<String> styleProfile;
  final double height;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      height: height,
      padding: const EdgeInsets.symmetric(horizontal: Spacing.md),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface.withValues(alpha: 0.96),
        border: Border(
          bottom: BorderSide(
            color: theme.colorScheme.outline.withValues(alpha: 0.18),
          ),
        ),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.14),
            blurRadius: 14,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  app.displayName.isNotEmpty ? app.displayName : 'AI 导演剪辑台',
                  style: theme.textTheme.titleLarge?.copyWith(
                    fontWeight: FontWeight.w700,
                    letterSpacing: 0.15,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  app.description.isNotEmpty
                      ? app.description
                      : '顶部预览、左侧决策、底部时间线的专业剪辑工作流。',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          Wrap(
            spacing: Spacing.sm,
            runSpacing: Spacing.sm,
            alignment: WrapAlignment.end,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              ...styleProfile.take(3).map(
                    (item) => _InfoChip(
                      icon: Icons.auto_awesome_outlined,
                      label: item,
                      compact: true,
                    ),
                  ),
              FilledButton.icon(
                onPressed: () {
                  // TODO: 接 runApp(app.name, capability, 'export_video', data) 触发导出。
                },
                icon: const Icon(Icons.ios_share_outlined, size: 16),
                label: const Text('导出成片'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _LeftSidebar extends StatelessWidget {
  const _LeftSidebar({
    required this.assets,
    required this.importedAsset,
    required this.plans,
    required this.selectedPlanIndex,
    required this.compactHeight,
    required this.generatingPlan,
    required this.aiPlanText,
    required this.planError,
    required this.askUserQuestion,
    required this.askUserOptions,
    required this.sessionId,
    required this.messageController,
    required this.planActionLabel,
    required this.onImportPressed,
    required this.onGeneratePlanPressed,
    required this.onAskUserOptionPressed,
    required this.onPlanSelected,
    required this.styleProfile,
  });

  final List<DirectorMediaAsset> assets;
  final DirectorMediaAsset? importedAsset;
  final List<DirectorPlan> plans;
  final int selectedPlanIndex;
  final bool compactHeight;
  final bool generatingPlan;
  final String? aiPlanText;
  final String? planError;
  final String? askUserQuestion;
  final List<String> askUserOptions;
  final String? sessionId;
  final TextEditingController messageController;
  final String planActionLabel;
  final Future<void> Function() onImportPressed;
  final Future<void> Function() onGeneratePlanPressed;
  final Future<void> Function(String) onAskUserOptionPressed;
  final ValueChanged<int> onPlanSelected;
  final List<String> styleProfile;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          flex: compactHeight ? 9 : 8,
          child: _WorkbenchPanel(
            title: '素材库',
            subtitle: '导入镜头与补拍素材',
            compact: compactHeight,
            action: IconButton(
              tooltip: '导入素材',
              onPressed: onImportPressed,
              icon: const Icon(Icons.add_photo_alternate_outlined, size: 18),
            ),
            child: assets.isEmpty
                ? const EmptyState(
                    icon: Icons.video_library_outlined,
                    title: '还没有导入素材',
                    subtitle: '导入视频后，AI 导演会在这里组织镜头与方案。',
                  )
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      if (importedAsset != null) ...[
                        Container(
                          padding: const EdgeInsets.all(10),
                          decoration: BoxDecoration(
                            color: theme.colorScheme.primaryContainer
                                .withValues(alpha: 0.5),
                            borderRadius: BorderRadius.circular(10),
                            border: Border.all(
                              color: theme.colorScheme.primary
                                  .withValues(alpha: 0.18),
                            ),
                          ),
                          child: Row(
                            children: [
                              Icon(
                                Icons.video_file_outlined,
                                size: 16,
                                color: theme.colorScheme.primary,
                              ),
                              const SizedBox(width: Spacing.xs),
                              Expanded(
                                child: Text(
                                  '当前：${importedAsset!.fileName}',
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: theme.textTheme.labelMedium?.copyWith(
                                    fontWeight: FontWeight.w700,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(height: Spacing.sm),
                      ],
                      Expanded(
                        child: ListView.separated(
                          primary: false,
                          itemCount: assets.length,
                          itemBuilder: (context, index) {
                            final asset = assets[index];
                            return _AssetTile(asset: asset);
                          },
                          separatorBuilder: (_, _) =>
                              const SizedBox(height: 6),
                        ),
                      ),
                    ],
                  ),
          ),
        ),
        const SizedBox(height: Spacing.sm),
        Expanded(
          flex: compactHeight ? 11 : 12,
          child: _WorkbenchPanel(
            title: '主导演对话',
            subtitle: '输入创意，让导演追问并展开方案',
            compact: compactHeight,
            action: FilledButton.icon(
              onPressed: generatingPlan ? null : onGeneratePlanPressed,
              icon: generatingPlan
                  ? const SizedBox(
                      width: 14,
                      height: 14,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('✨'),
              label: Text(planActionLabel),
            ),
            child: _DirectorConversationPanel(
              compactHeight: compactHeight,
              generatingPlan: generatingPlan,
              aiPlanText: aiPlanText,
              planError: planError,
              askUserQuestion: askUserQuestion,
              askUserOptions: askUserOptions,
              sessionId: sessionId,
              messageController: messageController,
              planActionLabel: planActionLabel,
              onGeneratePlanPressed: onGeneratePlanPressed,
              onAskUserOptionPressed: onAskUserOptionPressed,
            ),
          ),
        ),
      ],
    );
  }
}

class _DirectorConversationPanel extends StatelessWidget {
  const _DirectorConversationPanel({
    required this.compactHeight,
    required this.generatingPlan,
    required this.aiPlanText,
    required this.planError,
    required this.askUserQuestion,
    required this.askUserOptions,
    required this.sessionId,
    required this.messageController,
    required this.planActionLabel,
    required this.onGeneratePlanPressed,
    required this.onAskUserOptionPressed,
  });

  final bool compactHeight;
  final bool generatingPlan;
  final String? aiPlanText;
  final String? planError;
  final String? askUserQuestion;
  final List<String> askUserOptions;
  final String? sessionId;
  final TextEditingController messageController;
  final String planActionLabel;
  final Future<void> Function() onGeneratePlanPressed;
  final Future<void> Function(String) onAskUserOptionPressed;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        TextField(
          controller: messageController,
          minLines: compactHeight ? 2 : 3,
          maxLines: compactHeight ? 3 : 4,
          textInputAction: TextInputAction.send,
          onSubmitted: generatingPlan ? null : (_) => onGeneratePlanPressed(),
          decoration: InputDecoration(
            labelText: '创意描述',
            hintText: '例如：做 30 秒精品手冲咖啡品牌短视频',
            alignLabelWithHint: true,
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(14),
            ),
            suffixIcon: IconButton(
              tooltip: planActionLabel,
              onPressed: generatingPlan ? null : onGeneratePlanPressed,
              icon: generatingPlan
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.send_rounded),
            ),
          ),
        ),
        const SizedBox(height: Spacing.sm),
        _WorkbenchMiniCard(
          title: '会话状态',
          compact: true,
          child: Text(
            sessionId == null ? '首次发送后创建主导演会话。' : '当前会话：$sessionId',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
              height: 1.4,
            ),
          ),
        ),
        const SizedBox(height: Spacing.sm),
        Expanded(
          child: LayoutBuilder(
            builder: (context, panelConstraints) {
              return Scrollbar(
                child: SingleChildScrollView(
                  padding: const EdgeInsets.only(right: 6),
                  child: ConstrainedBox(
                    constraints: BoxConstraints(
                      minHeight: panelConstraints.maxHeight,
                    ),
                    child: generatingPlan
                        ? const _AiPlanLoadingView()
                        : planError != null
                            ? _AiPlanErrorView(message: planError!)
                            : aiPlanText != null
                                ? Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.start,
                                    children: [
                                      _AiPlanMarkdownView(
                                        planText: aiPlanText!,
                                        compactHeight: compactHeight,
                                      ),
                                      if (askUserQuestion != null ||
                                          askUserOptions.isNotEmpty) ...[
                                        const SizedBox(height: Spacing.md),
                                        _AskUserOptionsCard(
                                          question: askUserQuestion,
                                          options: askUserOptions,
                                          onOptionPressed:
                                              onAskUserOptionPressed,
                                        ),
                                      ],
                                    ],
                                  )
                                : const EmptyState(
                                    icon: Icons.auto_awesome_outlined,
                                    title: '开始和主导演对话',
                                    subtitle: '输入创意后，导演会理解需求、主动追问并逐步展开方案。',
                                  ),
                  ),
                ),
              );
            },
          ),
        ),
      ],
    );
  }
}

class _AskUserOptionsCard extends StatelessWidget {
  const _AskUserOptionsCard({
    required this.question,
    required this.options,
    required this.onOptionPressed,
  });

  final String? question;
  final List<String> options;
  final Future<void> Function(String) onOptionPressed;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return _WorkbenchMiniCard(
      title: '导演追问',
      compact: true,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (question != null && question!.trim().isNotEmpty) ...[
            Text(
              question!,
              style: theme.textTheme.bodyMedium?.copyWith(height: 1.45),
            ),
            const SizedBox(height: Spacing.sm),
          ],
          if (options.isEmpty)
            Text(
              '主导演正在等待你补充更多细节。',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            )
          else
            Wrap(
              spacing: Spacing.sm,
              runSpacing: Spacing.sm,
              children: [
                for (final option in options)
                  ActionChip(
                    label: Text(option),
                    avatar: Icon(
                      Icons.touch_app_outlined,
                      size: 16,
                      color: theme.colorScheme.primary,
                    ),
                    onPressed: () => onOptionPressed(option),
                  ),
              ],
            ),
        ],
      ),
    );
  }
}

class _AiPlanLoadingView extends StatelessWidget {
  const _AiPlanLoadingView();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: Spacing.lg),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const CircularProgressIndicator(),
            const SizedBox(height: Spacing.sm),
            Text(
              'AI 正在构思导演方案…',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: Spacing.xs),
            Text(
              '首次生成可能需要 30-60 秒，请稍候。',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
            ),
          ],
        ),
      ),
    );
  }
}

class _AiPlanErrorView extends StatelessWidget {
  const _AiPlanErrorView({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: Spacing.lg),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.error_outline,
              color: Theme.of(context).colorScheme.error,
            ),
            const SizedBox(height: Spacing.sm),
            Text(
              message,
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ],
        ),
      ),
    );
  }
}

class _AiPlanMarkdownView extends StatelessWidget {
  const _AiPlanMarkdownView({
    required this.planText,
    required this.compactHeight,
  });

  final String planText;
  final bool compactHeight;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return MarkdownBody(
      data: planText,
      selectable: true,
      styleSheet: MarkdownStyleSheet.fromTheme(theme).copyWith(
        p: theme.textTheme.bodyMedium?.copyWith(
          color: theme.colorScheme.onSurface,
          height: 1.55,
          fontSize: compactHeight ? 12.5 : null,
        ),
        h1: theme.textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w700),
        h2: theme.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
        h3: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
        listBullet: theme.textTheme.bodyMedium?.copyWith(
          color: theme.colorScheme.onSurface,
        ),
        blockquote: theme.textTheme.bodyMedium?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
          height: 1.5,
        ),
        blockquoteDecoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHigh,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(
            color: theme.colorScheme.outline.withValues(alpha: 0.16),
          ),
        ),
      ),
    );
  }
}

class _PreviewStage extends StatelessWidget {
  const _PreviewStage({
    required this.currentSecond,
    required this.totalLabel,
    required this.totalSecond,
    required this.selectedPlan,
    required this.compactHeight,
    required this.videoController,
    required this.hasVideo,
    required this.isPlaying,
    required this.onPlayPause,
    required this.onSeekBackward,
    required this.onSeekForward,
    required this.onScrub,
  });

  final double currentSecond;
  final String totalLabel;
  final double totalSecond;
  final DirectorPlan? selectedPlan;
  final bool compactHeight;
  final VideoController? videoController;
  final bool hasVideo;
  final bool isPlaying;
  final Future<void> Function() onPlayPause;
  final Future<void> Function() onSeekBackward;
  final Future<void> Function() onSeekForward;
  final ValueChanged<double> onScrub;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return _WorkbenchPanel(
      title: '节目监视器',
      subtitle: selectedPlan?.title ?? '等待方案生成',
      compact: compactHeight,
      action: Container(
        padding: const EdgeInsets.symmetric(
          horizontal: Spacing.sm,
          vertical: 4,
        ),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHigh,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(
            color: theme.colorScheme.outline.withValues(alpha: 0.22),
          ),
        ),
        child: Text(
          '16:9 预览',
          style: theme.textTheme.labelSmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: const Color(0xFF06080D),
                borderRadius: BorderRadius.circular(16),
                border: Border.all(
                  color: theme.colorScheme.outline.withValues(alpha: 0.24),
                ),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.32),
                    blurRadius: 24,
                    offset: const Offset(0, 14),
                  ),
                ],
                gradient: const LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [Color(0xFF0D1017), Color(0xFF030405)],
                ),
              ),
              child: LayoutBuilder(
                builder: (context, previewConstraints) {
                  final frameWidth = previewConstraints.maxWidth;

                  return Center(
                    child: FittedBox(
                      fit: BoxFit.contain,
                      child: SizedBox(
                        width: frameWidth,
                        height: frameWidth / (16 / 9),
                        child: Container(
                          decoration: BoxDecoration(
                            borderRadius: BorderRadius.circular(14),
                            border: Border.all(
                              color: Colors.white.withValues(alpha: 0.08),
                            ),
                            gradient: const LinearGradient(
                              begin: Alignment.topCenter,
                              end: Alignment.bottomCenter,
                              colors: [Color(0xFF141922), Color(0xFF08090D)],
                            ),
                          ),
                          child: ClipRRect(
                            borderRadius: BorderRadius.circular(14),
                            child: Stack(
                              children: [
                                Positioned.fill(
                                  child: hasVideo
                                      ? Video(
                                          controller: videoController!,
                                          fit: BoxFit.contain,
                                        )
                                      : CustomPaint(
                                          painter: _PreviewOverlayPainter(
                                            color: theme.colorScheme.primary,
                                          ),
                                        ),
                                ),
                                if (!hasVideo)
                                  Center(
                                    child: Column(
                                      mainAxisSize: MainAxisSize.min,
                                      children: [
                                        Icon(
                                          Icons.play_circle_outline,
                                          size: compactHeight ? 62 : 76,
                                          color: Colors.white.withValues(alpha: 0.92),
                                        ),
                                        const SizedBox(height: Spacing.sm),
                                        Text(
                                          selectedPlan?.title ?? '等待 AI 剪辑方案',
                                          style: theme.textTheme.titleMedium?.copyWith(
                                            color: Colors.white,
                                            fontWeight: FontWeight.w600,
                                            fontSize: compactHeight ? 15 : 16,
                                          ),
                                        ),
                                        const SizedBox(height: Spacing.xs),
                                        Text(
                                          '首帧缩略图占位\nTODO: media_kit 播放本地文件',
                                          textAlign: TextAlign.center,
                                          style: theme.textTheme.bodySmall?.copyWith(
                                            color: Colors.white70,
                                            height: 1.45,
                                          ),
                                        ),
                                      ],
                                    ),
                                  ),
                                Positioned(
                                  top: 12,
                                  left: 12,
                                  child: _PreviewBadge(
                                    label: _formatPreviewTimecode(currentSecond),
                                    icon: Icons.fiber_manual_record,
                                  ),
                                ),
                                const Positioned(
                                  top: 12,
                                  right: 12,
                                  child: _PreviewBadge(label: '1080p'),
                                ),
                              ],
                            ),
                          ),
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),
          ),
          const SizedBox(height: Spacing.sm),
          Container(
            padding: const EdgeInsets.symmetric(
              horizontal: Spacing.md,
              vertical: Spacing.sm,
            ),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHigh,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(
                color: theme.colorScheme.outline.withValues(alpha: 0.18),
              ),
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  children: [
                    _TransportButton(
                      icon: Icons.skip_previous_rounded,
                      onTap: onSeekBackward,
                    ),
                    const SizedBox(width: Spacing.sm),
                    _TransportButton(
                      icon: isPlaying
                          ? Icons.pause_rounded
                          : Icons.play_arrow_rounded,
                      filled: true,
                      onTap: onPlayPause,
                    ),
                    const SizedBox(width: Spacing.sm),
                    _TransportButton(
                      icon: Icons.skip_next_rounded,
                      onTap: onSeekForward,
                    ),
                    const SizedBox(width: Spacing.md),
                    Text(
                      '${_formatTimelineTime(currentSecond)} / $totalLabel',
                      style: theme.textTheme.bodySmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const Spacer(),
                    Icon(
                      Icons.volume_up_outlined,
                      size: 16,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: Spacing.sm),
                    SizedBox(
                      width: compactHeight ? 52 : 72,
                      child: const LinearProgressIndicator(value: 0.64),
                    ),
                    const SizedBox(width: Spacing.sm),
                    Icon(
                      Icons.fullscreen,
                      size: 18,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ],
                ),
                if (selectedPlan != null) ...[
                  const SizedBox(height: Spacing.xs),
                  Align(
                    alignment: Alignment.centerLeft,
                    child: Text(
                      selectedPlan!.segments.join('   '),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                        letterSpacing: 0.15,
                      ),
                    ),
                  ),
                ],
                const SizedBox(height: Spacing.xs),
                SliderTheme(
                  data: SliderTheme.of(context).copyWith(
                    trackHeight: 3,
                    thumbShape: const RoundSliderThumbShape(
                      enabledThumbRadius: 6,
                    ),
                    overlayShape: const RoundSliderOverlayShape(
                      overlayRadius: 12,
                    ),
                  ),
                  child: Slider(
                    value: currentSecond.clamp(0, totalSecond <= 0 ? 1 : totalSecond),
                    min: 0,
                    max: totalSecond <= 0 ? 1 : totalSecond,
                    onChanged: onScrub,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _TimelineSection extends StatelessWidget {
  const _TimelineSection({
    required this.plan,
    required this.currentSecond,
    required this.totalDuration,
    required this.compactHeight,
    required this.onAccept,
    required this.onRecut,
  });

  final DirectorPlan? plan;
  final double currentSecond;
  final double totalDuration;
  final bool compactHeight;
  final VoidCallback onAccept;
  final VoidCallback onRecut;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return _WorkbenchPanel(
      title: '多轨时间线',
      subtitle: '视频轨 / B-roll 轨 / 音频轨',
      compact: compactHeight,
      child: LayoutBuilder(
        builder: (context, sectionConstraints) {
          final timelineCanvasHeight = compactHeight ? 156.0 : 184.0;

          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: SingleChildScrollView(
                  child: ConstrainedBox(
                    constraints: BoxConstraints(
                      minHeight: timelineCanvasHeight,
                    ),
                    child: plan == null
                        ? SizedBox(
                            height: timelineCanvasHeight,
                            child: const EmptyState(
                              icon: Icons.timeline_outlined,
                              title: '暂无时间线',
                              subtitle: '选择导演方案后，这里会生成多轨道编排预览。',
                            ),
                          )
                        : SizedBox(
                            height: timelineCanvasHeight,
                            child: AiDirectorTimeline(
                              blocks: mockTimelineBlocks,
                              currentSecond: currentSecond,
                              totalDuration: totalDuration,
                              compact: sectionConstraints.maxHeight < 220,
                              plan: plan,
                            ),
                          ),
                  ),
                ),
              ),
              const SizedBox(height: Spacing.sm),
              Wrap(
                spacing: Spacing.sm,
                runSpacing: Spacing.sm,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  ConstrainedBox(
                    constraints: BoxConstraints(
                      maxWidth: sectionConstraints.maxWidth * 0.52,
                    ),
                    child: Text(
                      plan == null
                          ? '等待 AI 输出方案'
                          : '当前采用 ${plan!.title}，可继续采纳或请求重剪。',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  FilledButton.icon(
                    onPressed: plan == null ? null : onAccept,
                    icon: const Icon(Icons.done_all_outlined, size: 16),
                    label: const Text('采纳方案'),
                  ),
                  OutlinedButton.icon(
                    onPressed: plan == null ? null : onRecut,
                    icon: const Icon(Icons.refresh_outlined, size: 16),
                    label: const Text('重剪'),
                  ),
                ],
              ),
            ],
          );
        },
      ),
    );
  }
}

class _WorkbenchPanel extends StatelessWidget {
  const _WorkbenchPanel({
    required this.title,
    required this.subtitle,
    required this.child,
    this.action,
    this.compact = false,
  });

  final String title;
  final String subtitle;
  final Widget child;
  final Widget? action;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(18),
        border: Border.all(
          color: theme.colorScheme.outline.withValues(alpha: 0.18),
        ),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.12),
            blurRadius: 14,
            offset: const Offset(0, 8),
          ),
        ],
      ),
      child: Padding(
        padding: EdgeInsets.all(compact ? 12 : Spacing.md),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        title,
                        style: theme.textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w700,
                          fontSize: compact ? 15 : null,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        subtitle,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                          fontSize: compact ? 11.5 : null,
                        ),
                      ),
                    ],
                  ),
                ),
                if (action != null) ...[
                  const SizedBox(width: Spacing.sm),
                  action!,
                ],
              ],
            ),
            SizedBox(height: compact ? Spacing.sm : Spacing.md),
            Expanded(child: child),
          ],
        ),
      ),
    );
  }
}

class _WorkbenchMiniCard extends StatelessWidget {
  const _WorkbenchMiniCard({
    required this.title,
    required this.child,
    this.compact = false,
  });

  final String title;
  final Widget child;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      width: double.infinity,
      padding: EdgeInsets.all(compact ? 10 : Spacing.sm),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: theme.colorScheme.outline.withValues(alpha: 0.16),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            style: theme.textTheme.labelMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: Spacing.xs),
          child,
        ],
      ),
    );
  }
}

class _AssetTile extends StatelessWidget {
  const _AssetTile({required this.asset});

  final DirectorMediaAsset asset;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return HoverCard(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(10),
        child: Row(
          children: [
            Container(
              width: 80,
              height: 46,
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(8),
                gradient: const LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [Color(0xFF39404A), Color(0xFF181C23)],
                ),
              ),
              child: Stack(
                children: [
                  Positioned(
                    left: 6,
                    bottom: 6,
                    child: Icon(
                      Icons.play_arrow_rounded,
                      size: 16,
                      color: Colors.white.withValues(alpha: 0.92),
                    ),
                  ),
                  Positioned(
                    right: 6,
                    bottom: 6,
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 5,
                        vertical: 2,
                      ),
                      decoration: BoxDecoration(
                        color: Colors.black.withValues(alpha: 0.58),
                        borderRadius: BorderRadius.circular(999),
                      ),
                      child: Text(
                        asset.durationLabel,
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: Colors.white,
                          fontSize: 10,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: Spacing.sm),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    asset.fileName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                      fontSize: 13,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    '代理素材已就绪  ·  ${asset.durationLabel}',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                      fontSize: 11.5,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 6),
            Icon(
              Icons.more_horiz_rounded,
              size: 18,
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ],
        ),
      ),
    );
  }
}

class _TransportButton extends StatelessWidget {
  const _TransportButton({
    required this.icon,
    this.filled = false,
    this.onTap,
  });

  final IconData icon;
  final bool filled;
  final Future<void> Function()? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(10),
        onTap: onTap == null ? null : () => onTap!.call(),
        child: Container(
          width: 34,
          height: 34,
          decoration: BoxDecoration(
            color: filled
                ? theme.colorScheme.primary
                : theme.colorScheme.surface.withValues(alpha: 0.4),
            borderRadius: BorderRadius.circular(10),
            border: Border.all(
              color: filled
                  ? theme.colorScheme.primary
                  : theme.colorScheme.outline.withValues(alpha: 0.18),
            ),
          ),
          child: Icon(
            icon,
            size: 18,
            color: filled
                ? theme.colorScheme.onPrimary
                : theme.colorScheme.onSurface,
          ),
        ),
      ),
    );
  }
}

class _InfoChip extends StatelessWidget {
  const _InfoChip({
    required this.icon,
    required this.label,
    this.compact = false,
  });

  final IconData icon;
  final String label;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      padding: EdgeInsets.symmetric(
        horizontal: compact ? Spacing.sm : 9,
        vertical: compact ? 5 : 7,
      ),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(
          color: theme.colorScheme.outline.withValues(alpha: 0.18),
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            icon,
            size: compact ? 12 : 14,
            color: theme.colorScheme.primary,
          ),
          const SizedBox(width: 6),
          Text(
            label,
            style: theme.textTheme.labelSmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}

class _PreviewOverlayPainter extends CustomPainter {
  const _PreviewOverlayPainter({required this.color});

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final framePaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1
      ..color = Colors.white.withValues(alpha: 0.1);
    final accentPaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1
      ..color = color.withValues(alpha: 0.16);

    canvas.drawRect(
      Rect.fromLTWH(18, 18, size.width - 36, size.height - 36),
      framePaint,
    );
    canvas.drawRect(
      Rect.fromLTWH(
        size.width * 0.12,
        size.height * 0.12,
        size.width * 0.76,
        size.height * 0.76,
      ),
      accentPaint,
    );
    canvas.drawLine(
      Offset(size.width * 0.18, size.height * 0.22),
      Offset(size.width * 0.82, size.height * 0.22),
      accentPaint,
    );
    canvas.drawLine(
      Offset(size.width * 0.2, size.height * 0.76),
      Offset(size.width * 0.8, size.height * 0.76),
      accentPaint,
    );
  }

  @override
  bool shouldRepaint(covariant _PreviewOverlayPainter oldDelegate) {
    return oldDelegate.color != color;
  }
}

class _PreviewBadge extends StatelessWidget {
  const _PreviewBadge({this.icon, required this.label});

  final IconData? icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.46),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 10, color: const Color(0xFFEF4444)),
            const SizedBox(width: 4),
          ],
          Text(
            label,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: Colors.white,
              fontSize: 10.5,
              fontWeight: FontWeight.w700,
            ),
          ),
        ],
      ),
    );
  }
}

String _formatTimelineTime(double second) {
  final totalSeconds = second.round().clamp(0, 3599);
  final minutes = totalSeconds ~/ 60;
  final seconds = totalSeconds % 60;
  return '${minutes.toString().padLeft(2, '0')}:${seconds.toString().padLeft(2, '0')}';
}

String _formatPreviewTimecode(double second) {
  final totalSeconds = second.round().clamp(0, 3599);
  final minutes = totalSeconds ~/ 60;
  final seconds = totalSeconds % 60;
  return '00:${minutes.toString().padLeft(2, '0')}:${seconds.toString().padLeft(2, '0')}';
}
