// Catalog(去中心化包清单)数据模型。手写解析,不依赖 freezed/build_runner。
library;

class CatalogAsset {
  CatalogAsset({required this.name, required this.sha256, this.optional = false});
  final String name;
  final String sha256;
  final bool optional;

  factory CatalogAsset.fromJson(Map<String, dynamic> j) => CatalogAsset(
        name: j['name'] as String? ?? '',
        sha256: (j['sha256'] as String? ?? '').trim(),
        optional: j['optional'] as bool? ?? false,
      );
}

class CatalogPackage {
  CatalogPackage({
    required this.name,
    required this.version,
    required this.kind,
    required this.description,
    required this.tags,
    required this.repo,
    required this.tag,
    required this.manifestAsset,
    required this.wasmAsset,
    required this.extraAssets,
    required this.provides,
    required this.requires,
  });

  final String name;
  final String version;
  final String kind;
  final String description;
  final List<String> tags;

  /// GitHub owner/repo,产物托管处。
  final String repo;

  /// release tag。
  final String tag;

  final CatalogAsset manifestAsset;
  final CatalogAsset wasmAsset;
  final List<CatalogAsset> extraAssets;

  final List<String> provides;

  /// 直接依赖的包名(catalog 内引用)。
  final List<String> requires;

  factory CatalogPackage.fromJson(Map<String, dynamic> j) {
    final assets = j['assets'] as Map<String, dynamic>? ?? const {};
    final extra = (assets['extra'] as List? ?? const [])
        .whereType<Map<String, dynamic>>()
        .map(CatalogAsset.fromJson)
        .toList();
    return CatalogPackage(
      name: j['name'] as String? ?? '',
      version: j['version'] as String? ?? '',
      kind: j['kind'] as String? ?? '',
      description: j['description'] as String? ?? '',
      tags: (j['tags'] as List? ?? const []).map((e) => '$e').toList(),
      repo: j['repo'] as String? ?? '',
      tag: j['tag'] as String? ?? '',
      manifestAsset: CatalogAsset.fromJson(
          assets['manifest'] as Map<String, dynamic>? ?? const {}),
      wasmAsset: CatalogAsset.fromJson(
          assets['wasm'] as Map<String, dynamic>? ?? const {}),
      extraAssets: extra,
      provides: (j['provides'] as List? ?? const []).map((e) => '$e').toList(),
      requires: (j['requires'] as List? ?? const []).map((e) => '$e').toList(),
    );
  }

  /// 拼某个 asset 的 GitHub release 下载地址(browser_download_url 规则:
  /// 302 → CDN blob,免 token、不计 api.github.com 限流额度)。
  String downloadUrl(String assetName) =>
      'https://github.com/$repo/releases/download/$tag/$assetName';
}

class Catalog {
  Catalog({required this.catalogVersion, required this.packages});
  final int catalogVersion;
  final List<CatalogPackage> packages;

  CatalogPackage? byName(String name) {
    for (final p in packages) {
      if (p.name == name) return p;
    }
    return null;
  }

  factory Catalog.fromJson(Map<String, dynamic> j) => Catalog(
        catalogVersion: j['catalogVersion'] as int? ?? 1,
        packages: (j['packages'] as List? ?? const [])
            .whereType<Map<String, dynamic>>()
            .map(CatalogPackage.fromJson)
            .toList(),
      );
}
