import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/client.dart';
import 'ffi_adapter.dart';
import 'http_adapter.dart';
import 'transport_adapter.dart';

export 'transport_adapter.dart';
export 'http_adapter.dart';
export 'ffi_adapter.dart';

/// 当前传输适配器。
/// 优先 FFI(桌面 + DLL 存在);否则 HTTP。
/// 初始化是异步的(FFI 需要 weft_start),用 AsyncNotifier。
class TransportNotifier extends AsyncNotifier<TransportAdapter> {
  @override
  Future<TransportAdapter> build() async {
    // 桌面平台:尝试 FFI
    if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
      try {
        final adapter = await FfiTransportAdapter.init(dataDir: './data');
        return adapter;
      } catch (_) {
        // FFI 不可用(DLL 缺失等),降级 HTTP。
      }
    }
    // Fallback: HTTP
    final dio = ref.watch(apiClientProvider);
    return HttpTransportAdapter(dio);
  }
}

final transportProvider =
    AsyncNotifierProvider<TransportNotifier, TransportAdapter>(
        TransportNotifier.new);

/// 同步版(已加载后取值;未加载时 fallback HTTP)。方便不想处理 async 的调用方。
final transportSyncProvider = Provider<TransportAdapter>((ref) {
  final async = ref.watch(transportProvider);
  return async.valueOrNull ?? HttpTransportAdapter(ref.watch(apiClientProvider));
});
