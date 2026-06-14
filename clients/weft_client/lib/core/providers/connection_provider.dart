import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'core_repository.dart';

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

enum CoreConnectionStatus { connected, connecting, offline }

class CoreConnectionState {
  const CoreConnectionState({required this.status, this.lastChecked});

  final CoreConnectionStatus status;
  final DateTime? lastChecked;

  bool get isConnected => status == CoreConnectionStatus.connected;
  bool get isOffline => status == CoreConnectionStatus.offline;

  CoreConnectionState copyWith({
    CoreConnectionStatus? status,
    DateTime? lastChecked,
  }) =>
      CoreConnectionState(
        status: status ?? this.status,
        lastChecked: lastChecked ?? this.lastChecked,
      );
}

// ---------------------------------------------------------------------------
// Notifier
// ---------------------------------------------------------------------------

class ConnectionNotifier extends StateNotifier<CoreConnectionState> {
  ConnectionNotifier(this._repository)
      : super(const CoreConnectionState(status: CoreConnectionStatus.connecting)) {
    _startPolling();
  }

  final CoreRepository _repository;
  Timer? _timer;

  static const _pollInterval = Duration(seconds: 10);

  void _startPolling() {
    // Immediate first check
    _check();
    _timer = Timer.periodic(_pollInterval, (_) => _check());
  }

  Future<void> _check() async {
    // Don't reset to connecting on background polls — only on explicit retry
    final ok = await _repository.checkHealth();
    if (!mounted) return;
    state = state.copyWith(
      status: ok
          ? CoreConnectionStatus.connected
          : CoreConnectionStatus.offline,
      lastChecked: DateTime.now(),
    );
  }

  /// Manually trigger a reconnection attempt (resets to connecting first).
  Future<void> retry() async {
    if (!mounted) return;
    state = state.copyWith(status: CoreConnectionStatus.connecting);
    await _check();
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

final connectionProvider =
    StateNotifierProvider.autoDispose<ConnectionNotifier, CoreConnectionState>(
  (ref) => ConnectionNotifier(ref.watch(coreRepositoryProvider)),
);
