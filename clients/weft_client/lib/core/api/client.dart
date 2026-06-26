import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import '../models/error.dart';
import '../providers/preferences_provider.dart';
import 'ffi_dio_adapter.dart';

part 'client.g.dart';

const _defaultBaseUrl = 'http://127.0.0.1:17830';
const _runtimeTokenFileName = 'runtime-token';

/// Authoritative path to the loopback `runtime-token`. When the client launches
/// the core itself (sidecar mode) it sets this to the exact file the core wrote
/// under its `--data-dir`, so requests read the right token instead of guessing
/// among conventional locations (the root cause of 401s). Null = guess as before.
String? runtimeTokenPathOverride;

/// 公开的 token 读取：复用 interceptor 的候选路径搜索。
/// 供 webview surface 等需要把 token 传给 web 页的场景使用。
Future<String> readLoopbackToken() async {
  final interceptor = _LoopbackAuthInterceptor();
  return (await interceptor._readLoopbackToken()) ?? '';
}

class _LoopbackAuthInterceptor extends Interceptor {
  @override
  Future<void> onRequest(
    RequestOptions options,
    RequestInterceptorHandler handler,
  ) async {
    final token = await _readLoopbackToken();
    if (token != null && token.isNotEmpty) {
      options.headers['Authorization'] = 'Bearer $token';
    }
    handler.next(options);
  }

  Future<String?> _readLoopbackToken() async {
    for (final candidate in await _runtimeTokenCandidates()) {
      try {
        if (!await candidate.exists()) continue;
        final token = (await candidate.readAsString()).trim();
        if (token.isNotEmpty) return token;
      } catch (_) {
        // 开发期允许 token 文件尚未生成或路径不可访问，静默降级为无鉴权请求。
      }
    }
    return null;
  }

  Future<List<File>> _runtimeTokenCandidates() async {
    final candidates = <String>{};

    // Authoritative location set by the sidecar manager — try it first.
    final override = runtimeTokenPathOverride;
    if (override != null && override.isNotEmpty) {
      candidates.add(override);
    }

    for (final dir in _repoDataDirs()) {
      candidates.add(_joinPath(dir, _runtimeTokenFileName));
    }

    final weftBinary = _findWeftBinary();
    if (weftBinary != null) {
      candidates.add(_joinPath(
        File(weftBinary).parent.path,
        'data',
        _runtimeTokenFileName,
      ));
    }

    candidates.add(_joinPath(_systemDataDir(), _runtimeTokenFileName));

    try {
      final supportDir = await getApplicationSupportDirectory();
      candidates.add(_joinPath(supportDir.path, 'data', _runtimeTokenFileName));
      candidates.add(_joinPath(supportDir.path, _runtimeTokenFileName));
    } catch (_) {
      // path_provider 在当前平台不可用时继续走其他约定路径。
    }

    return candidates.map(File.new).toList(growable: false);
  }

  List<String> _repoDataDirs() {
    final dirs = <String>[];
    // Walk up from cwd.
    var dir = Directory.current;
    for (var i = 0; i < 8; i++) {
      dirs.add(_joinPath(dir.path, 'data'));
      final parent = dir.parent;
      if (parent.path == dir.path) break;
      dir = parent;
    }
    // Also walk up from the exe location (on Windows, cwd may not match
    // Start-Process -WorkingDirectory due to platform quirks).
    var exeDir = File(Platform.resolvedExecutable).parent;
    for (var i = 0; i < 8; i++) {
      final candidate = _joinPath(exeDir.path, 'data');
      if (!dirs.contains(candidate)) dirs.add(candidate);
      final parent = exeDir.parent;
      if (parent.path == exeDir.path) break;
      exeDir = parent;
    }
    return dirs;
  }

  String? _findWeftBinary() {
    final ext = Platform.isWindows ? '.exe' : '';
    final exeDir = File(Platform.resolvedExecutable).parent.path;
    final directCandidate = _joinPath(exeDir, 'weft$ext');
    if (File(directCandidate).existsSync()) return directCandidate;

    var dir = File(Platform.resolvedExecutable).parent;
    for (var i = 0; i < 8; i++) {
      final candidate = '${dir.path}${Platform.pathSeparator}core'
          '${Platform.pathSeparator}target'
          '${Platform.pathSeparator}debug'
          '${Platform.pathSeparator}weft$ext';
      if (File(candidate).existsSync()) return candidate;
      final parent = dir.parent;
      if (parent.path == dir.path) break;
      dir = parent;
    }

    return null;
  }

  String _systemDataDir() {
    if (Platform.isWindows) {
      final programData =
          Platform.environment['ProgramData'] ?? 'C:\\ProgramData';
      return _joinPath(programData, 'WEFT', 'data');
    }
    if (Platform.isMacOS) {
      return '/Library/Application Support/WEFT/data';
    }
    return '/var/lib/weft';
  }

  String _joinPath(
    String first, [
    String? second,
    String? third,
    String? fourth,
    String? fifth,
  ]) {
    final parts = [first, second, third, fourth, fifth]
        .whereType<String>()
        .where((part) => part.isNotEmpty)
        .toList(growable: false);
    return parts.join(Platform.pathSeparator);
  }
}

@riverpod
Dio apiClient(Ref ref) {
  // Watch the configured core address so changing it in Settings rebuilds the
  // client (and everything depending on it) against the new base URL.
  final base = ref.watch(preferencesProvider.select((p) => p.coreBaseUrl));
  final dio = Dio(BaseOptions(
    baseUrl: base.isEmpty ? _defaultBaseUrl : base,
    connectTimeout: const Duration(seconds: 5),
    receiveTimeout: const Duration(seconds: 30),
    headers: {'Content-Type': 'application/json'},
  ));

  dio.interceptors.add(LogInterceptor(
    requestBody: false,
    responseBody: false,
    error: true,
  ));

  dio.interceptors.add(_LoopbackAuthInterceptor());
  dio.interceptors.add(AppErrorInterceptor());

  // FFI 就绪时:所有 Dio 请求透明走 FFI dispatch(不经网络),上层零改动。
  if (FfiDioAdapter.enabled) {
    dio.httpClientAdapter = FfiDioAdapter();
  }

  return dio;
}

/// Converts [DioException] into typed [AppException] subclasses. Shared
/// by every Dio in the app (core client + store client).
class AppErrorInterceptor extends Interceptor {
  @override
  void onError(DioException err, ErrorInterceptorHandler handler) {
    final appEx = _convert(err);
    handler.next(err.copyWith(error: appEx));
  }

  AppException _convert(DioException err) {
    switch (err.type) {
      case DioExceptionType.connectionTimeout:
      case DioExceptionType.sendTimeout:
      case DioExceptionType.receiveTimeout:
        return const CoreOfflineException('Connection timed out');
      case DioExceptionType.connectionError:
        return const CoreOfflineException();
      case DioExceptionType.badResponse:
        final statusCode = err.response?.statusCode ?? 0;
        final body = err.response?.data;
        final message = _extractMessage(body) ??
            err.response?.statusMessage ??
            'HTTP $statusCode';
        if (statusCode == 401) {
          final detail = _extractMessage(body);
          return AuthException(detail == null || detail.isEmpty
              ? 'weft-core rejected the request (401): loopback token mismatch. '
                  'Restart the app so the client re-reads the current token.'
              : 'weft-core rejected the request (401): $detail');
        }
        return ApiException(statusCode: statusCode, message: message);
      default:
        // unknown / cancel — wrap as offline so callers get a typed exception
        return CoreOfflineException(err.message ?? 'Unknown error');
    }
  }

  String? _extractMessage(dynamic body) {
    if (body is Map<String, dynamic>) {
      return (body['message'] ?? body['error'])?.toString();
    }
    return null;
  }
}

@riverpod
String baseUrl(Ref ref) => _defaultBaseUrl;
