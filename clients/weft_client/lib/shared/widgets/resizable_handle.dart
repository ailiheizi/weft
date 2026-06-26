import 'package:flutter/material.dart';

/// 可拖拽的竖直分隔条。拖动改变相邻面板宽度。
/// 用法:把它放在 Row 里,onDelta 回调里更新宽度 state(自行 clamp)。
class ResizableHandle extends StatefulWidget {
  const ResizableHandle({super.key, required this.onDelta});

  /// 水平拖动增量(像素,向右为正)。
  final ValueChanged<double> onDelta;

  @override
  State<ResizableHandle> createState() => _ResizableHandleState();
}

class _ResizableHandleState extends State<ResizableHandle> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return MouseRegion(
      cursor: SystemMouseCursors.resizeLeftRight,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onHorizontalDragUpdate: (d) => widget.onDelta(d.delta.dx),
        child: SizedBox(
          width: 6,
          child: Center(
            // 默认完全透明,hover 时才微弱显示(MD3 无边界感)。
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 150),
              width: _hover ? 2 : 0,
              decoration: BoxDecoration(
                color: _hover
                    ? theme.colorScheme.primary.withValues(alpha: 0.3)
                    : Colors.transparent,
                borderRadius: BorderRadius.circular(1),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
