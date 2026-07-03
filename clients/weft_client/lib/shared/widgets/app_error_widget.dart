import 'dart:async';
import 'package:flutter/material.dart';
import '../../../core/models/error.dart';

/// Unified error display widget that handles [AppException] subtypes.
///
/// - [CoreOfflineException] → "核心未就绪，正在重连…" with a live spinner and
///   automatic retry every [autoRetryInterval] (in addition to a manual button).
/// - [AuthException]        → "Authentication failed" with a lock icon
/// - [ApiException]         → "Error `statusCode`" with error icon
/// - other                  → generic warning
class AppErrorWidget extends StatefulWidget {
  const AppErrorWidget({
    super.key,
    required this.error,
    this.onRetry,
    this.autoRetryInterval = const Duration(seconds: 5),
  });

  final Object error;
  final VoidCallback? onRetry;

  /// How often to auto-retry while [error] is a [CoreOfflineException].
  final Duration autoRetryInterval;

  @override
  State<AppErrorWidget> createState() => _AppErrorWidgetState();
}

class _AppErrorWidgetState extends State<AppErrorWidget> {
  Timer? _autoRetry;

  bool get _isOffline => widget.error is CoreOfflineException;

  @override
  void initState() {
    super.initState();
    _syncAutoRetry();
  }

  @override
  void didUpdateWidget(AppErrorWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Error type may have changed (e.g. offline → 401); resync the timer.
    _syncAutoRetry();
  }

  /// Start auto-retry only for the offline case, and only if a retry handler
  /// exists. Otherwise ensure any running timer is cancelled.
  void _syncAutoRetry() {
    final shouldRun = _isOffline && widget.onRetry != null;
    if (shouldRun && _autoRetry == null) {
      _autoRetry = Timer.periodic(
        widget.autoRetryInterval,
        (_) => widget.onRetry?.call(),
      );
    } else if (!shouldRun) {
      _autoRetry?.cancel();
      _autoRetry = null;
    }
  }

  @override
  void dispose() {
    _autoRetry?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final error = widget.error;

    final (icon, title, subtitle) = switch (error) {
      CoreOfflineException _ => (
          Icons.cloud_off_outlined,
          '核心未就绪，正在重连…',
          '正在连接 weft-core，请稍候。连接恢复后会自动加载。',
        ),
      AuthException e => (
          Icons.lock_outline,
          'Authentication failed',
          e.message,
        ),
      ApiException e => (
          Icons.error_outline,
          'Error ${e.statusCode}',
          e.message,
        ),
      _ => (
          Icons.warning_amber_outlined,
          'Something went wrong',
          error.toString(),
        ),
    };

    // Offline uses a warm/neutral tint (it's transient, not a hard failure);
    // other errors keep the error color to signal something needs attention.
    final accent =
        _isOffline ? theme.colorScheme.onSurfaceVariant : theme.colorScheme.error;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(children: [
          if (_isOffline)
            SizedBox(
              width: 20,
              height: 20,
              child: CircularProgressIndicator(
                strokeWidth: 2,
                color: theme.colorScheme.primary,
              ),
            )
          else
            Icon(icon, color: accent, size: 20),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: accent,
                    fontWeight: FontWeight.w500,
                  ),
                ),
                Text(
                  subtitle,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          if (widget.onRetry != null)
            TextButton.icon(
              icon: const Icon(Icons.refresh, size: 14),
              label: Text(_isOffline ? '立即重试' : 'Retry'),
              onPressed: widget.onRetry,
            ),
        ]),
      ),
    );
  }
}
