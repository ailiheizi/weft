import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/providers/data_providers.dart';
import 'canvas_state.dart';
import 'models/canvas_models.dart';

/// 画布底部中央的「统一生成栏」(对标 TapNow 的生成指挥台)。
///
/// 大 prompt 输入 + 一行控件：模型下拉(从 image provider 的 models 取)、
/// 画幅比例、批量数、生成按钮。点生成即在画布铺出 N 个图像节点并并行出图。
class CentralGenerationBar extends ConsumerStatefulWidget {
  const CentralGenerationBar({super.key});

  @override
  ConsumerState<CentralGenerationBar> createState() => _CentralGenerationBarState();
}

class _CentralGenerationBarState extends ConsumerState<CentralGenerationBar> {
  final _prompt = TextEditingController();
  String _ratio = '16:9';
  String? _model;
  int _batch = 1;
  bool _busy = false;
  // 新生成节点的落点(画布坐标)，每次生成后下移避免叠在一起。
  double _spawnY = 200;

  static const _ratios = ['1:1', '16:9', '9:16'];

  @override
  void dispose() {
    _prompt.dispose();
    super.dispose();
  }

  Future<void> _generate(List<String> models) async {
    final text = _prompt.text.trim();
    if (text.isEmpty || _busy) return;
    final model = _model ?? (models.isNotEmpty ? models.first : 'gpt-image-2-vip');
    setState(() => _busy = true);
    try {
      await ref.read(canvasProvider.notifier).generateBatch(
            prompt: text,
            origin: Offset(200, _spawnY),
            params: GenParams(model: model, aspectRatio: _ratio, batchCount: _batch),
            count: _batch,
          );
      _spawnY += 360;
      _prompt.clear();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final providersAsync = ref.watch(providersProvider);
    // 取图像 provider 的模型列表(名字含 image 的 provider，否则全部模型汇总)。
    final models = providersAsync.maybeWhen(
      data: (list) {
        final img = list.where((p) => p.name.toLowerCase().contains('image'));
        final src = img.isNotEmpty ? img : list;
        return src.expand((p) => p.models).toSet().toList();
      },
      orElse: () => <String>[],
    );
    _model ??= models.isNotEmpty ? models.first : null;

    return Container(
      width: 620,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: theme.colorScheme.outline.withValues(alpha: 0.2)),
        boxShadow: const [BoxShadow(color: Colors.black38, blurRadius: 20, spreadRadius: 1)],
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          // 大 prompt 输入
          TextField(
            controller: _prompt,
            minLines: 1,
            maxLines: 3,
            textInputAction: TextInputAction.newline,
            decoration: InputDecoration(
              hintText: '描述要生成的画面，回车换行 · 点生成铺到画布',
              border: InputBorder.none,
              isDense: true,
              contentPadding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
              hintStyle: TextStyle(color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6)),
            ),
            style: theme.textTheme.bodyMedium,
          ),
          const SizedBox(height: 8),
          // 控件行
          Row(
            children: [
              // 模型下拉(带分辨率/标签由模型名体现)
              _PillDropdown(
                icon: Icons.auto_awesome,
                value: _model,
                items: models.isEmpty ? const ['gpt-image-2-vip'] : models,
                onChanged: (v) => setState(() => _model = v),
              ),
              const SizedBox(width: 8),
              // 画幅比例
              _PillDropdown(
                icon: Icons.aspect_ratio,
                value: _ratio,
                items: _ratios,
                onChanged: (v) => setState(() => _ratio = v ?? '16:9'),
              ),
              const SizedBox(width: 8),
              // 批量数 stepper
              _BatchStepper(
                value: _batch,
                onChanged: (v) => setState(() => _batch = v),
              ),
              const Spacer(),
              // 生成按钮
              FilledButton.icon(
                onPressed: _busy ? null : () => _generate(models),
                icon: _busy
                    ? const SizedBox(
                        width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))
                    : const Icon(Icons.auto_fix_high, size: 18),
                label: Text(_busy ? '生成中' : '生成'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// 胶囊样式的下拉(图标 + 当前值)。
class _PillDropdown extends StatelessWidget {
  const _PillDropdown({
    required this.icon,
    required this.value,
    required this.items,
    required this.onChanged,
  });

  final IconData icon;
  final String? value;
  final List<String> items;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(20),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 15, color: theme.colorScheme.onSurfaceVariant),
          const SizedBox(width: 6),
          DropdownButtonHideUnderline(
            child: DropdownButton<String>(
              value: items.contains(value) ? value : (items.isNotEmpty ? items.first : null),
              isDense: true,
              borderRadius: BorderRadius.circular(10),
              style: theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.onSurface),
              items: items
                  .map((it) => DropdownMenuItem(value: it, child: Text(it)))
                  .toList(),
              onChanged: onChanged,
            ),
          ),
        ],
      ),
    );
  }
}

/// 批量数加减器(1-4)。
class _BatchStepper extends StatelessWidget {
  const _BatchStepper({required this.value, required this.onChanged});

  final int value;
  final ValueChanged<int> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 4),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(20),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          InkWell(
            onTap: value > 1 ? () => onChanged(value - 1) : null,
            child: Icon(Icons.remove, size: 16,
                color: value > 1 ? theme.colorScheme.onSurface : theme.colorScheme.outline),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: Text('$value×', style: theme.textTheme.bodySmall),
          ),
          InkWell(
            onTap: value < 4 ? () => onChanged(value + 1) : null,
            child: Icon(Icons.add, size: 16,
                color: value < 4 ? theme.colorScheme.onSurface : theme.colorScheme.outline),
          ),
        ],
      ),
    );
  }
}
