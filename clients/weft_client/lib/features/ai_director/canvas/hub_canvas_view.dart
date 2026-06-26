import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'canvas_state.dart';
import 'director_chat_panel.dart';
import 'infinite_canvas.dart';
import 'models/canvas_models.dart';
import 'shot_library_panel.dart';

/// Hub 形态三栏工作台：左 Shot 资产库 / 中 无限画布 / 右 Agent 对话。
class HubCanvasView extends ConsumerStatefulWidget {
  const HubCanvasView({super.key});

  @override
  ConsumerState<HubCanvasView> createState() => _HubCanvasViewState();
}

class _HubCanvasViewState extends ConsumerState<HubCanvasView> {
  @override
  void initState() {
    super.initState();
    // 首次进入填充演示节点，验证画布交互。
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final notifier = ref.read(canvasProvider.notifier);
      if (ref.read(canvasProvider).nodes.isEmpty) {
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
    final divider = VerticalDivider(
      width: 1,
      thickness: 1,
      color: theme.colorScheme.outline.withValues(alpha: 0.15),
    );

    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // 左栏
        SizedBox(
          width: 240,
          child: _panel(theme, const ShotLibraryPanel()),
        ),
        divider,
        // 中栏（核心）
        const Expanded(child: InfiniteCanvas()),
        divider,
        // 右栏
        SizedBox(
          width: 340,
          child: _panel(theme, DirectorChatPanel(contextHintBuilder: _contextHint)),
        ),
      ],
    );
  }

  Widget _panel(ThemeData theme, Widget child) {
    return ColoredBox(
      color: theme.colorScheme.surface,
      child: child,
    );
  }
}
