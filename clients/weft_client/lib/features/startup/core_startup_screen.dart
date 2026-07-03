import 'package:flutter/material.dart';
import '../../shared/theme/spacing.dart';

/// 启动引导屏：core（FFI/sidecar）初始化期间显示，避免白屏/冻屏。
///
/// - 初始化中：品牌标 + 旋转指示器 + "正在启动 weft-core…"。
/// - 初始化失败：友好错误 + "重试" / "仍然进入"（core 连接会在应用内继续轮询重连）。
class CoreStartupScreen extends StatelessWidget {
  const CoreStartupScreen({
    super.key,
    required this.failed,
    this.message,
    this.statusText,
    this.onRetry,
    this.onSkip,
  });

  /// 是否处于失败态（否则视为初始化中）。
  final bool failed;

  /// 失败时的可读原因。
  final String? message;

  /// 初始化中的当前阶段文案（如"正在加载插件与服务…"）。
  final String? statusText;

  final VoidCallback? onRetry;
  final VoidCallback? onSkip;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 380),
          child: Padding(
            padding: const EdgeInsets.all(Spacing.lg),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                // 品牌标记
                Text(
                  'WEFT',
                  style: theme.textTheme.headlineMedium?.copyWith(
                    fontWeight: FontWeight.w700,
                    letterSpacing: 4,
                    color: theme.colorScheme.onSurface,
                  ),
                ),
                const SizedBox(height: Spacing.xl),
                if (!failed) ..._loading(theme) else ..._error(context, theme),
              ],
            ),
          ),
        ),
      ),
    );
  }

  List<Widget> _loading(ThemeData theme) => [
        SizedBox(
          width: 26,
          height: 26,
          child: CircularProgressIndicator(
            strokeWidth: 2.5,
            color: theme.colorScheme.primary,
          ),
        ),
        const SizedBox(height: Spacing.lg),
        Text(
          (statusText?.isNotEmpty == true) ? statusText! : '正在启动 weft-core…',
          textAlign: TextAlign.center,
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ];

  List<Widget> _error(BuildContext context, ThemeData theme) => [
        Icon(Icons.cloud_off_outlined,
            size: 28, color: theme.colorScheme.error),
        const SizedBox(height: Spacing.md),
        Text(
          '核心启动失败',
          textAlign: TextAlign.center,
          style: theme.textTheme.titleSmall?.copyWith(
            color: theme.colorScheme.error,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: Spacing.sm),
        Text(
          message?.isNotEmpty == true
              ? message!
              : '未能启动 weft-core。可重试，或先进入应用（连接会在后台自动重连）。',
          textAlign: TextAlign.center,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: Spacing.lg),
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            if (onSkip != null)
              TextButton(
                onPressed: onSkip,
                child: const Text('仍然进入'),
              ),
            if (onSkip != null && onRetry != null)
              const SizedBox(width: Spacing.sm),
            if (onRetry != null)
              FilledButton.icon(
                onPressed: onRetry,
                icon: const Icon(Icons.refresh, size: 16),
                label: const Text('重试'),
              ),
          ],
        ),
      ];
}
