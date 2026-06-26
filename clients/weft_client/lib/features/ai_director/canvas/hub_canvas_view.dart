import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../shared/theme/spacing.dart';
import 'canvas_state.dart';
import 'central_generation_bar.dart';
import 'director_chat_panel.dart';
import 'embedded_webview_canvas.dart';
import 'infinite_canvas.dart';
import 'models/canvas_models.dart';
import 'models/workflow_blueprint.dart';
import 'node_param_panel.dart';
import 'shot_library_panel.dart';

/// Hub 形态三栏工作台：左 Shot 资产库 / 中 无限画布 / 右 Agent 对话。
class HubCanvasView extends ConsumerStatefulWidget {
  const HubCanvasView({super.key});

  @override
  ConsumerState<HubCanvasView> createState() => _HubCanvasViewState();
}

class _HubCanvasViewState extends ConsumerState<HubCanvasView> {
  /// 中栏默认使用嵌入式 webview（React Flow），旧自研画布弃用。
  bool _useWebview = true;

  @override
  void initState() {
    super.initState();
    // 加载已保存的工程；无存档则填充演示节点。
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      final notifier = ref.read(canvasProvider.notifier);
      final hasProject = await notifier.loadProject();
      if (!hasProject && ref.read(canvasProvider).nodes.isEmpty) {
        notifier.seedDemo();
      }
    });
  }

  /// 把选中节点信息拼成对话上下文提示。
  String _contextHint() {
    final node = ref.read(canvasProvider).selectedNode;
    if (node == null) return '';
    final kind = switch (node.kind) {
      CanvasNodeKind.image => '图像',
      CanvasNodeKind.video => '视频',
      CanvasNodeKind.music => '音乐',
      CanvasNodeKind.text => '文本',
    };
    return '当前选中$kind节点「${node.title.isEmpty ? node.id : node.title}」';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final notifier = ref.read(canvasProvider.notifier);
    final divider = VerticalDivider(
      width: 1,
      thickness: 1,
      color: theme.colorScheme.outline.withValues(alpha: 0.15),
    );

    return CallbackShortcuts(
      bindings: {
        // 仅组合键放全局（不干扰文本输入）。删除键放中栏画布的局部焦点内。
        const SingleActivator(LogicalKeyboardKey.keyZ, control: true): () => notifier.undo(),
        const SingleActivator(LogicalKeyboardKey.keyY, control: true): () => notifier.redo(),
        const SingleActivator(LogicalKeyboardKey.keyZ, control: true, shift: true): () => notifier.redo(),
      },
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // 左栏
          SizedBox(
            width: 240,
            child: _panel(theme, const ShotLibraryPanel()),
          ),
          divider,
          // 中栏（核心）
          Expanded(child: _buildCenter(theme)),
          divider,
          // 右栏
          SizedBox(
            width: 340,
            child: _panel(theme, DirectorChatPanel(contextHintBuilder: _contextHint)),
          ),
        ],
      ),
    );
  }

  Widget _buildCenter(ThemeData theme) {
    final canvas = ref.watch(canvasProvider);
    final selected = canvas.selectedNode;
    final multiCount = canvas.selectedIds.length;
    final notifier = ref.read(canvasProvider.notifier);

    // 注意：中栏不能包 CallbackShortcuts/Focus 去抢键盘——那会拦截右栏对话输入框，
    // 导致整个 ai-director 输入框打不了字。删除节点改用工具栏按钮 + 右键菜单（不碰键盘焦点）。
    return Stack(
        children: [
        Positioned.fill(
          child: _useWebview
              ? const EmbeddedWebviewCanvas()
              : const InfiniteCanvas(),
        ),
        // webview 模式下 web 页自带所有交互 UI，Flutter 浮层不显示。
        if (!_useWebview) ...[
        // 工具栏：新建生成节点
        Positioned(
          left: Spacing.md,
          top: Spacing.md,
          child: Material(
            color: theme.colorScheme.surface,
            borderRadius: BorderRadius.circular(10),
            elevation: 2,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: Spacing.xs, vertical: Spacing.xs),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                    tooltip: '新建生成节点',
                    icon: const Icon(Icons.add_photo_alternate_outlined, size: 20),
                    onPressed: () {
                      final id = notifier.proposeImageNode(
                        prompt: '',
                        position: const Offset(300, 300),
                        title: '新建图像',
                      );
                      notifier.select(id);
                    },
                  ),
                  IconButton(
                    tooltip: '导入本地图片',
                    icon: const Icon(Icons.upload_file_outlined, size: 20),
                    onPressed: () => _importImages(notifier),
                  ),
                  IconButton(
                    tooltip: '把本镜图片合成视频',
                    icon: const Icon(Icons.movie_creation_outlined, size: 20),
                    onPressed: () => _composeVideo(notifier),
                  ),
                  IconButton(
                    tooltip: '删除选中节点',
                    icon: const Icon(Icons.delete_outline, size: 20),
                    onPressed: selected == null ? null : () => notifier.deleteSelected(),
                  ),
                  IconButton(
                    tooltip: '复制选中节点',
                    icon: const Icon(Icons.copy_outlined, size: 20),
                    onPressed: (selected == null && multiCount < 2)
                        ? null
                        : () => notifier.duplicateSelected(),
                  ),
                  Container(
                    width: 1,
                    height: 20,
                    margin: const EdgeInsets.symmetric(horizontal: Spacing.xs),
                    color: theme.colorScheme.outline.withValues(alpha: 0.2),
                  ),
                  IconButton(
                    tooltip: '铺一张演示工作流 DAG',
                    icon: const Icon(Icons.account_tree_outlined, size: 20),
                    onPressed: () => notifier.applyBlueprint(WorkflowBlueprint.demo()),
                  ),
                  IconButton(
                    tooltip: '运行工作流（并行出图→汇聚成片）',
                    icon: const Icon(Icons.play_circle_outline, size: 20),
                    onPressed: () => _runWorkflow(notifier),
                  ),
                  Container(
                    width: 1,
                    height: 20,
                    margin: const EdgeInsets.symmetric(horizontal: Spacing.xs),
                    color: theme.colorScheme.outline.withValues(alpha: 0.2),
                  ),
                  IconButton(
                    tooltip: '新建工程（清空画布）',
                    icon: const Icon(Icons.note_add_outlined, size: 20),
                    onPressed: () => _confirmNewProject(notifier),
                  ),
                  IconButton(
                    tooltip: '撤销 (Ctrl+Z)',
                    icon: const Icon(Icons.undo, size: 20),
                    onPressed: notifier.canUndo ? notifier.undo : null,
                  ),
                  IconButton(
                    tooltip: '重做 (Ctrl+Y)',
                    icon: const Icon(Icons.redo, size: 20),
                    onPressed: notifier.canRedo ? notifier.redo : null,
                  ),
                  Container(
                    width: 1,
                    height: 20,
                    margin: const EdgeInsets.symmetric(horizontal: Spacing.xs),
                    color: theme.colorScheme.outline.withValues(alpha: 0.2),
                  ),
                  Builder(
                    builder: (ctx) => IconButton(
                      tooltip: '操作指南',
                      icon: const Icon(Icons.help_outline, size: 20),
                      onPressed: () => _showHelp(ctx),
                    ),
                  ),
                  Builder(
                    builder: (ctx) => IconButton(
                      tooltip: _useWebview ? '切回自研画布' : '切换 Web UI（嵌入式）',
                      icon: Icon(
                        _useWebview ? Icons.grid_view : Icons.science_outlined,
                        size: 20,
                        color: _useWebview ? theme.colorScheme.primary : null,
                      ),
                      onPressed: () => setState(() => _useWebview = !_useWebview),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
        // 选中节点参数面板（右上角浮层）。多选时不显示单节点面板。
        if (selected != null && multiCount < 2)
          Positioned(
            right: Spacing.md,
            top: Spacing.md,
            child: NodeParamPanel(node: selected),
          ),
        // 多选批量操作栏（顶部居中浮出）
        if (multiCount >= 2)
          Positioned(
            top: Spacing.md,
            left: 0,
            right: 0,
            child: Center(child: _batchBar(theme, multiCount, notifier)),
          ),
        // 中央统一生成栏（底部居中浮出）——对标 TapNow 生成指挥台。
        const Positioned(
          left: 0,
          right: 0,
          bottom: 20,
          child: Center(child: CentralGenerationBar()),
        ),
        ], // end if (!_useWebview)
      ],
        );
  }

  /// 多选时浮出的批量操作栏：选中数 + 对齐/分布/复制/删除。
  Widget _batchBar(ThemeData theme, int count, CanvasNotifier notifier) {
    Widget btn(IconData icon, String tip, VoidCallback onTap, {Color? color}) => IconButton(
          tooltip: tip,
          visualDensity: VisualDensity.compact,
          iconSize: 18,
          constraints: const BoxConstraints(minWidth: 34, minHeight: 34),
          icon: Icon(icon, color: color),
          onPressed: onTap,
        );
    Widget sep() => Container(
          width: 1,
          height: 20,
          margin: const EdgeInsets.symmetric(horizontal: Spacing.xs),
          color: theme.colorScheme.outline.withValues(alpha: 0.2),
        );
    return Material(
      color: theme.colorScheme.surface,
      borderRadius: BorderRadius.circular(10),
      elevation: 3,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: Spacing.sm, vertical: 2),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: Spacing.xs),
              child: Text('已选 $count', style: theme.textTheme.bodySmall),
            ),
            sep(),
            btn(Icons.align_horizontal_left, '左对齐', () => notifier.alignSelected('left')),
            btn(Icons.align_horizontal_center, '水平居中', () => notifier.alignSelected('centerH')),
            btn(Icons.align_horizontal_right, '右对齐', () => notifier.alignSelected('right')),
            btn(Icons.align_vertical_top, '顶对齐', () => notifier.alignSelected('top')),
            btn(Icons.align_vertical_center, '垂直居中', () => notifier.alignSelected('centerV')),
            btn(Icons.align_vertical_bottom, '底对齐', () => notifier.alignSelected('bottom')),
            sep(),
            btn(Icons.horizontal_distribute, '水平等距', () => notifier.distributeSelected('h')),
            btn(Icons.vertical_distribute, '垂直等距', () => notifier.distributeSelected('v')),
            sep(),
            btn(Icons.copy_outlined, '批量复制', () => notifier.duplicateSelected()),
            btn(Icons.delete_outline, '批量删除', () => notifier.deleteSelected(),
                color: theme.colorScheme.error),
          ],
        ),
      ),
    );
  }

  Widget _panel(ThemeData theme, Widget child) {
    return ColoredBox(
      color: theme.colorScheme.surface,
      child: child,
    );
  }

  /// 新建工程前确认（清空当前画布）。
  Future<void> _confirmNewProject(CanvasNotifier notifier) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('新建工程'),
        content: const Text('将清空当前画布上的所有节点和连线，确定吗？'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('新建')),
        ],
      ),
    );
    if (ok == true) notifier.newProject();
  }

  /// 操作指南弹窗：画布手势 + 节点操作速查。
  void _showHelp(BuildContext context) {
    final theme = Theme.of(context);
    Widget row(IconData icon, String op, String desc) => Padding(
          padding: const EdgeInsets.symmetric(vertical: 5),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(icon, size: 18, color: theme.colorScheme.primary),
              const SizedBox(width: 10),
              SizedBox(
                width: 120,
                child: Text(op, style: theme.textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600)),
              ),
              Expanded(child: Text(desc, style: theme.textTheme.bodyMedium)),
            ],
          ),
        );
    showDialog<void>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Row(
          children: [
            Icon(Icons.help_outline, size: 20),
            SizedBox(width: 8),
            Text('画布操作指南'),
          ],
        ),
        content: SizedBox(
          width: 460,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('画布', style: theme.textTheme.titleSmall),
              const SizedBox(height: 4),
              row(Icons.crop_free, '拖拽空白', '框选多个节点'),
              row(Icons.pan_tool_outlined, '中键拖动 / 空格+拖动', '平移画布'),
              row(Icons.mouse, '滚轮', '以光标为中心缩放'),
              row(Icons.touch_app, 'Ctrl/⇧ + 点击', '加选 / 减选节点'),
              const Divider(height: 20),
              Text('节点', style: theme.textTheme.titleSmall),
              const SizedBox(height: 4),
              row(Icons.ads_click, '点击节点', '选中，弹出右侧参数面板与节点工具条'),
              row(Icons.open_with, '拖动节点', '移动（多选时整组移动，松手吸附网格）'),
              row(Icons.circle, '拖右侧圆点', '连线到目标节点；拖到空白处新建下游节点'),
              row(Icons.open_in_full, '拖右下角', '缩放节点'),
              row(Icons.menu, '右键节点', '重命名 / 删除等菜单'),
              const Divider(height: 20),
              Text('工作流', style: theme.textTheme.titleSmall),
              const SizedBox(height: 4),
              row(Icons.account_tree_outlined, '铺演示 DAG', '一键铺一条多图汇聚成片的示例工作流'),
              row(Icons.play_circle_outline, '运行工作流', '按连线拓扑：并行出图 → 汇聚合成视频'),
              row(Icons.forum_outlined, '右栏 🌳', '让 AI 导演把创意拆成工作流蓝图，铺到画布'),
            ],
          ),
        ),
        actions: [
          FilledButton(onPressed: () => Navigator.pop(ctx), child: const Text('知道了')),
        ],
      ),
    );
  }

  /// 导入本地图片为画布节点（可多选）。
  Future<void> _importImages(CanvasNotifier notifier) async {
    final result = await FilePicker.pickFiles(
      allowMultiple: true,
      type: FileType.image,
    );
    if (result == null) return;
    var i = 0;
    String? firstId;
    for (final file in result.files) {
      final path = file.path;
      if (path == null) continue;
      final id = notifier.importImage(
        path: path,
        position: Offset(200 + (i % 4) * 220, 200 + (i ~/ 4) * 220),
      );
      firstId ??= id;
      i++;
    }
    if (firstId != null) notifier.select(firstId);
  }

  /// 把选中节点所属 Shot 的所有 ready 图节点合成视频。
  /// 一键运行整张工作流 DAG，完成后提示结果。
  Future<void> _runWorkflow(CanvasNotifier notifier) async {
    final messenger = ScaffoldMessenger.maybeOf(context);
    messenger?.showSnackBar(
      const SnackBar(content: Text('开始执行工作流：并行出图 → 汇聚成片…')),
    );
    final ok = await notifier.runWorkflow();
    if (!mounted) return;
    messenger?.hideCurrentSnackBar();
    messenger?.showSnackBar(
      SnackBar(content: Text(ok ? '工作流执行完成 ✅' : '工作流执行有节点失败，请检查红色节点')),
    );
  }

  void _composeVideo(CanvasNotifier notifier) {
    final state = ref.read(canvasProvider);
    final selected = state.selectedNode;

    // 确定目标 Shot：优先选中节点的 shot，否则第一个 shot。
    String? shotId = selected?.shotId;
    shotId ??= state.shots.isNotEmpty ? state.shots.first.id : null;

    // 收集该 Shot 下 ready 的图节点；无 Shot 时取所有 ready 图。
    final readyImages = state.nodes.values
        .where((n) =>
            n.kind == CanvasNodeKind.image &&
            n.status == NodeStatus.ready &&
            n.assetPath != null &&
            (shotId == null || n.shotId == shotId))
        .toList();

    if (readyImages.isEmpty) {
      ScaffoldMessenger.maybeOf(context)?.showSnackBar(
        const SnackBar(content: Text('需要至少 1 张已生成的图片才能合成视频')),
      );
      return;
    }

    // 视频节点落在源图右侧。
    final maxX = readyImages.map((n) => n.position.dx).reduce((a, b) => a > b ? a : b);
    final avgY = readyImages.map((n) => n.position.dy).reduce((a, b) => a + b) / readyImages.length;

    final id = notifier.proposeVideoNode(
      sourceImageNodeIds: readyImages.map((n) => n.id).toList(),
      position: Offset(maxX + 320, avgY),
      shotId: shotId,
    );
    if (id != null) {
      notifier.select(id);
      notifier.confirmAndGenerateVideo(id);
    }
  }
}
