import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import '../api/client.dart';
import '../models/app.dart';
import '../models/package.dart';
import '../models/provider.dart';
import '../models/service.dart';

part 'core_repository.g.dart';

@riverpod
CoreRepository coreRepository(Ref ref) {
  return CoreRepository(ref.watch(apiClientProvider));
}

class CoreRepository {
  CoreRepository(this._dio);
  final Dio _dio;

  // --- Health ---
  Future<bool> checkHealth() async {
    try {
      final res = await _dio.get('/api/health');
      return res.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  // --- Apps ---
  Future<List<ResolvedApp>> getApps() async {
    final res = await _dio.get<Map<String, dynamic>>('/api/apps');
    final list = res.data?['apps'] as List? ?? [];
    return _parseList(list, ResolvedApp.fromJson, 'app');
  }

  Future<ResolvedApp> getApp(String name) async {
    final res = await _dio.get<Map<String, dynamic>>('/api/apps/$name');
    final data = res.data ?? const {};
    // detail 端点把 app 包在 {"app": {...}, "source_index": {...}} 里；
    // 兼容直接返回 app 对象的情况。
    final appJson = data['app'] is Map<String, dynamic>
        ? data['app'] as Map<String, dynamic>
        : data;
    return ResolvedApp.fromJson(appJson);
  }

  Future<Map<String, dynamic>> runApp(
      String name, String capability, String action, Map<String, dynamic> data) async {
    final res = await _dio.post<Map<String, dynamic>>(
      '/api/apps/$name/run',
      data: {'capability': capability, 'action': action, 'data': data},
    );
    return res.data ?? {};
  }

  Future<void> updateBinding(
      String appName, String capability, String provider) async {
    await _dio.post<Map<String, dynamic>>(
      '/api/apps/$appName/run',
      data: {
        'capability': capability,
        'action': 'update_binding',
        'data': {'provider': provider},
      },
    );
  }

  // --- Providers ---
  Future<List<ProviderConfig>> getProviders() async {
    final res = await _dio.get<Map<String, dynamic>>('/api/providers');
    final list = res.data?['providers'] as List? ?? [];
    return _parseList(list, ProviderConfig.fromJson, 'provider');
  }

  Future<ProviderConfig> createProvider(ProviderConfig config) async {
    final res = await _dio.post<Map<String, dynamic>>(
      '/api/providers',
      data: config.toJson(),
    );
    return ProviderConfig.fromJson(res.data!);
  }

  Future<ProviderConfig> updateProvider(String name, ProviderConfig config) async {
    final res = await _dio.put<Map<String, dynamic>>(
      '/api/providers/$name',
      data: config.toJson(),
    );
    return ProviderConfig.fromJson(res.data!);
  }

  Future<void> deleteProvider(String name) async {
    await _dio.delete('/api/providers/$name');
  }

  // --- Packages ---
  Future<List<PackageInfo>> getPackages() async {
    final res = await _dio.get<Map<String, dynamic>>('/api/packages/runtime');
    final list = res.data?['packages'] as List? ?? [];
    return _parseList(list, PackageInfo.fromJson, 'package');
  }

  Future<void> togglePackage(String name) async {
    await _dio.post('/api/packages/$name/toggle');
  }

  Future<void> reloadPackage(String name) async {
    await _dio.post('/api/packages/$name/reload');
  }

  Future<void> uninstallPackage(String name) async {
    await _dio.delete('/api/packages/$name');
  }

  /// Install a package FROM the remote store. Posts to the local Core,
  /// which downloads the artifact from [storeBaseUrl], verifies its
  /// SHA-512, writes it to the installed packages dir, and hot-registers
  /// it. [version] empty → latest published.
  Future<void> installPackage(
    String name, {
    required String storeBaseUrl,
    String version = '',
  }) async {
    await _dio.post('/api/packages/install', data: {
      'name': name,
      'version': version,
      'store_base_url': storeBaseUrl,
    });
  }

  Future<Map<String, dynamic>> getPackageConfig(String name) async {
    final res = await _dio.get<Map<String, dynamic>>('/api/packages/$name/config');
    return res.data ?? {};
  }

  Future<Map<String, dynamic>> getPackageConfigSchema(String name) async {
    final res =
        await _dio.get<Map<String, dynamic>>('/api/packages/$name/config/schema');
    final schema = res.data?['schema'];
    return schema is Map<String, dynamic>
        ? schema
        : Map<String, dynamic>.from(schema as Map? ?? const {});
  }

  Future<void> savePackageConfig(String name, Map<String, dynamic> config) async {
    await _dio.put('/api/packages/$name/config', data: config);
  }

  // --- Services ---
  Future<List<ServiceInfo>> getServices() async {
    final res = await _dio.get<Map<String, dynamic>>('/api/services');
    final list = res.data?['services'] as List? ?? [];
    return _parseList(list, ServiceInfo.fromJson, 'service');
  }

  Future<void> startService(String name) async {
    await _dio.post('/api/services/$name/start');
  }

  Future<void> stopService(String name) async {
    await _dio.post('/api/services/$name/stop');
  }

  Future<void> restartService(String name) async {
    await _dio.post('/api/services/$name/restart');
  }

  /// Parses a JSON list into typed models, tolerating malformed items.
  ///
  /// A single bad/partial item (e.g. a null where a String is expected during
  /// a Core startup window) is skipped with a debug log rather than throwing
  /// and crashing the whole list — which previously surfaced as a full-screen
  /// "Null is not a subtype of String" error.
  List<T> _parseList<T>(
    List<dynamic> raw,
    T Function(Map<String, dynamic>) fromJson,
    String label,
  ) {
    final out = <T>[];
    for (final e in raw) {
      try {
        out.add(fromJson(e as Map<String, dynamic>));
      } catch (err) {
        assert(() {
          // ignore: avoid_print
          print('[core_repository] skipped malformed $label: $err');
          return true;
        }());
      }
    }
    return out;
  }
}
