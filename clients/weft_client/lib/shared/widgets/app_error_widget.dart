import 'package:flutter/material.dart';
import '../../../core/models/error.dart';

/// Unified error display widget that handles [AppException] subtypes.
///
/// - [CoreOfflineException] → "Core offline" with cloud-off icon
/// - [AuthException]        → "Authentication failed" with a lock icon
/// - [ApiException]         → "Error `statusCode`" with error icon
/// - other                  → generic warning
class AppErrorWidget extends StatelessWidget {
  const AppErrorWidget({super.key, required this.error, this.onRetry});

  final Object error;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    final (icon, title, subtitle) = switch (error) {
      CoreOfflineException e => (
          Icons.cloud_off_outlined,
          'Core offline',
          e.message,
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

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(children: [
          Icon(icon, color: theme.colorScheme.error, size: 20),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.error,
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
          if (onRetry != null)
            TextButton.icon(
              icon: const Icon(Icons.refresh, size: 14),
              label: const Text('Retry'),
              onPressed: onRetry,
            ),
        ]),
      ),
    );
  }
}
