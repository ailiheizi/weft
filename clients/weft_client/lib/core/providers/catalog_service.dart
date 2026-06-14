import 'dart:convert';
import 'dart:io';
import 'package:crypto/crypto.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../models/catalog.dart';

/// 默认 catalog 地址(raw.githubusercontent CDN,不计 GitHub API 限流额度)。
const _defaultCatalogUrl =
    'https://raw.githubusercontent.com/weft-dev/weft-catalog/main/catalog.json';

/// 在线包安装服务:拉 GitHub 清单 → 本地算依赖闭包 → 下载+校验 → 组装临时包。
/// 全程走 CDN(raw + release blob),公开 repo 免 token、不计限流。core 零改动。
class CatalogService {
  CatalogService();
  final Dio _dio = Dio();

  static const _kCatalogUrl = 'catalog_url';
  static const _kEtag = 'catalog_etag';
  static const _kBody = 'catalog_body';

  Future<String> catalogUrl() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(_kCatalogUrl) ?? _defaultCatalogUrl;
  }

  Future<void> setCatalogUrl(String url) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_kCatalogUrl, url);
  }

  /// 拉清单。带 ETag 协商缓存:304 时用本地缓存,不耗流量。
  Future<Catalog> fetchCatalog({bool force = false}) async {
    final prefs = await SharedPreferences.getInstance();
    final url = await catalogUrl();
    final etag = prefs.getString(_kEtag);
    final cachedBody = prefs.getString(_kBody);

    try {
      final res = await _dio.get<String>(
        url,
        options: Options(
          responseType: ResponseType.plain,
          headers: (etag != null && !force) ? {'If-None-Match': etag} : null,
          validateStatus: (s) => s != null && (s == 200 || s == 304),
        ),
      );
      if (res.statusCode == 304 && cachedBody != null) {
        return Catalog.fromJson(jsonDecode(cachedBody) as Map<String, dynamic>);
      }
      final body = res.data ?? '{}';
      final newEtag = res.headers.value('etag');
      if (newEtag != null) await prefs.setString(_kEtag, newEtag);
      await prefs.setString(_kBody, body);
      return Catalog.fromJson(jsonDecode(body) as Map<String, dynamic>);
    } catch (e) {
      // 网络失败时回退到缓存。
      if (cachedBody != null) {
        return Catalog.fromJson(jsonDecode(cachedBody) as Map<String, dynamic>);
      }
      rethrow;
    }
  }

  /// 对目标包做递归依赖闭包,返回拓扑序(依赖在前、目标在后)。
  /// 缺失依赖(catalog 里找不到)记入 [missing]。
  List<CatalogPackage> resolveClosure(
    Catalog catalog,
    String rootName, {
    required List<String> missing,
  }) {
    final ordered = <CatalogPackage>[];
    final seen = <String>{};
    final visiting = <String>{};

    void visit(String name) {
      if (seen.contains(name)) return;
      if (visiting.contains(name)) return; // 防环
      final pkg = catalog.byName(name);
      if (pkg == null) {
        if (!missing.contains(name)) missing.add(name);
        return;
      }
      visiting.add(name);
      for (final dep in pkg.requires) {
        visit(dep);
      }
      visiting.remove(name);
      seen.add(name);
      ordered.add(pkg); // 依赖先入,保证拓扑序
    }

    visit(rootName);
    return ordered;
  }

  /// 下载一个包的产物到临时目录,sha256 校验,返回可交给 importer 的目录路径。
  /// 校验失败抛异常(供应链 fail-safe)。
  Future<String> downloadPackage(CatalogPackage pkg) async {
    final tmpRoot = await getTemporaryDirectory();
    final dir = Directory(
        '${tmpRoot.path}${Platform.pathSeparator}weft-dl${Platform.pathSeparator}${pkg.name}');
    if (dir.existsSync()) dir.deleteSync(recursive: true);
    dir.createSync(recursive: true);

    await _downloadAsset(pkg, pkg.manifestAsset, 'package.toml', dir.path);
    await _downloadAsset(pkg, pkg.wasmAsset, 'package.wasm', dir.path);
    return dir.path;
  }

  Future<void> _downloadAsset(
      CatalogPackage pkg, CatalogAsset asset, String saveAs, String dir) async {
    if (asset.name.isEmpty) {
      throw Exception('${pkg.name}: 缺少 $saveAs 资产定义');
    }
    final url = pkg.downloadUrl(asset.name);
    final res = await _dio.get<List<int>>(
      url,
      options: Options(
          responseType: ResponseType.bytes,
          followRedirects: true,
          validateStatus: (s) => s != null && s == 200),
    );
    final bytes = res.data ?? const [];
    // sha256 校验(catalog 提供了 sha256 才校验;示例 catalog 留空表示跳过)。
    if (asset.sha256.isNotEmpty) {
      final actual = sha256.convert(bytes).toString();
      if (!actual.eqIgnoreCase(asset.sha256)) {
        throw Exception(
            '${pkg.name}/${asset.name} sha256 不匹配:期望 ${asset.sha256}, 实际 $actual');
      }
    }
    File('$dir${Platform.pathSeparator}$saveAs').writeAsBytesSync(bytes);
  }
}

extension on String {
  bool eqIgnoreCase(String other) => toLowerCase() == other.toLowerCase();
}

final catalogServiceProvider =
    Provider<CatalogService>((ref) => CatalogService());
