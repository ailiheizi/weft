import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

import 'transport_adapter.dart';

// C ABI 函数签名
typedef WeftStartC = Int32 Function(Pointer<Utf8> dataDir);
typedef WeftStartDart = int Function(Pointer<Utf8> dataDir);

typedef WeftCallC = Pointer<Utf8> Function(Pointer<Utf8> requestJson);
typedef WeftCallDart = Pointer<Utf8> Function(Pointer<Utf8> requestJson);

typedef WeftFreeStringC = Void Function(Pointer<Utf8> ptr);
typedef WeftFreeStringDart = void Function(Pointer<Utf8> ptr);

typedef WeftStopC = Int32 Function();
typedef WeftStopDart = int Function();

/// FFI 传输适配器:直接调用 weft_core.dll 的 C ABI,零网络开销。
/// 使用前需确保 weft_core.dll 在 exe 同目录或系统 PATH 中。
class FfiTransportAdapter implements TransportAdapter {
  FfiTransportAdapter._({required this.lib});

  final DynamicLibrary lib;
  late final WeftCallDart _call;
  late final WeftFreeStringDart _freeString;
  late final WeftStopDart _stop;

  /// 加载 DLL 并初始化 core。
  /// [dllPath]: weft_core.dll 的路径(null 则从默认位置搜索)。
  /// [dataDir]: core 的 data 目录(如 "./data")。
  static Future<FfiTransportAdapter> init({
    String? dllPath,
    String? dataDir,
  }) async {
    final lib = DynamicLibrary.open(
      dllPath ?? _findDll(),
    );

    final adapter = FfiTransportAdapter._(lib: lib);
    adapter._call = lib.lookupFunction<WeftCallC, WeftCallDart>('weft_call');
    adapter._freeString =
        lib.lookupFunction<WeftFreeStringC, WeftFreeStringDart>(
            'weft_free_string');
    adapter._stop = lib.lookupFunction<WeftStopC, WeftStopDart>('weft_stop');

    // 解析 data-dir 绝对路径(sidecar 进程继承 cwd 可能不对)。
    final resolvedDataDir = dataDir ?? _findDataDir();
    final dataDirPtr = resolvedDataDir.toNativeUtf8();

    // 启动 core sidecar
    final start =
        lib.lookupFunction<WeftStartC, WeftStartDart>('weft_start');
    final result = start(dataDirPtr);
    malloc.free(dataDirPtr);
    if (result != 0) {
      throw Exception('weft_start failed with code $result');
    }

    return adapter;
  }

  @override
  Future<ResponseEnvelope> send(RequestEnvelope request) async {
    final jsonStr = jsonEncode(request.toJson());
    final reqPtr = jsonStr.toNativeUtf8();

    final respPtr = _call(reqPtr);
    malloc.free(reqPtr);

    if (respPtr == nullptr) {
      return ResponseEnvelope(
        id: request.id,
        status: 500,
        body: {'error': 'weft_call returned null'},
      );
    }

    final respStr = respPtr.toDartString();
    _freeString(respPtr);

    try {
      final json = jsonDecode(respStr) as Map<String, dynamic>;
      return ResponseEnvelope.fromJson(json);
    } catch (e) {
      return ResponseEnvelope(
        id: request.id,
        status: 500,
        body: {'error': 'failed to parse response: $e'},
      );
    }
  }

  @override
  Stream<ResponseEnvelope> subscribe(RequestEnvelope request) {
    // FFI Phase 3 初版:subscribe 降级为轮询(同 HTTP adapter)。
    // 后续优化:用 callback 机制让 Rust 主动推送。
    final controller = StreamController<ResponseEnvelope>();
    Timer? timer;
    timer = Timer.periodic(const Duration(seconds: 2), (_) async {
      if (controller.isClosed) {
        timer?.cancel();
        return;
      }
      final resp = await send(request);
      if (!controller.isClosed) controller.add(resp);
    });
    controller.onCancel = () => timer?.cancel();
    return controller.stream;
  }

  @override
  Future<void> close() async {
    _stop();
  }

  /// 从 exe 同目录或向上搜索 weft_core.dll。
  static String _findDll() {
    final exeDir = File(Platform.resolvedExecutable).parent.path;
    final candidates = [
      '$exeDir/weft_core.dll',
      '$exeDir/../weft_core.dll',
      '$exeDir/../../weft_core.dll',
    ];
    for (final path in candidates) {
      if (File(path).existsSync()) return path;
    }
    // Fallback: let OS search PATH
    return 'weft_core.dll';
  }

  /// 从 exe 位置向上找 data/ 目录(含 runtime-token),返回绝对路径。
  static String _findDataDir() {
    var dir = File(Platform.resolvedExecutable).parent;
    for (var i = 0; i < 10; i++) {
      final candidate = Directory('${dir.path}${Platform.pathSeparator}data');
      if (candidate.existsSync()) return candidate.path;
      final parent = dir.parent;
      if (parent.path == dir.path) break;
      dir = parent;
    }
    return './data'; // fallback
  }
}
