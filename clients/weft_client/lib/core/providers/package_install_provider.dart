import 'dart:io';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'core_repository.dart';

/// 本地包安装状态 + installed 目录定位/持久化。
///
/// 背景：客户端无法从 core API 拿到 installed 目录的绝对路径（任何
/// /api/packages* 端点都不返回 repo_root）。因此用户首次导入时需手动指定
/// installed 目录一次，之后持久化到 shared_preferences 复用。
///
/// 指定后做交叉校验：把该目录的子文件夹名与 core 已加载的包名比对，至少一个
/// 对得上才认为目录正确，避免用户指错导致后续静默失败。
class PackageInstallState {
  const PackageInstallState({this.installedDir});

  /// 已确认的 installed 目录绝对路径；null = 尚未定位。
  final String? installedDir;

  bool get isLocated => installedDir != null && installedDir!.isNotEmpty;

  PackageInstallState copyWith({String? installedDir}) =>
      PackageInstallState(installedDir: installedDir ?? this.installedDir);
}

class PackageInstallNotifier extends StateNotifier<PackageInstallState> {
  PackageInstallNotifier(this._ref) : super(const PackageInstallState()) {
    _load();
  }

  final Ref _ref;
  static const _kInstalledDir = 'pkg_installed_dir';

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    final dir = prefs.getString(_kInstalledDir);
    if (dir != null && dir.isNotEmpty && Directory(dir).existsSync()) {
      state = state.copyWith(installedDir: dir);
    }
  }

  /// 校验候选目录是否是真正的 installed 目录：子文件夹名与 core 已装包名
  /// 至少有一个交集。返回校验通过的目录（可能是 candidate 本身，或其下的
  /// packages/installed、plugins/installed），失败返回 null。
  Future<String?> validateDir(String candidate) async {
    final installedNames = await _installedPackageNames();
    if (installedNames.isEmpty) {
      // core 一个包都没报告——无法交叉校验，保守地只接受目录存在。
      return Directory(candidate).existsSync() ? candidate : null;
    }

    // 候选目录本身 + 两个常见子路径都试一遍。
    final candidates = <String>[
      candidate,
      _join(candidate, ['packages', 'installed']),
      _join(candidate, ['plugins', 'installed']),
    ];
    for (final dir in candidates) {
      if (_dirMatches(dir, installedNames)) return dir;
    }
    return null;
  }

  /// 持久化已校验通过的目录。
  Future<void> setInstalledDir(String dir) async {
    state = state.copyWith(installedDir: dir);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_kInstalledDir, dir);
  }

  /// 该目录下的子文件夹名与已装包名是否有交集。
  bool _dirMatches(String dir, Set<String> installedNames) {
    final d = Directory(dir);
    if (!d.existsSync()) return false;
    for (final entry in d.listSync()) {
      if (entry is! Directory) continue;
      final folder = entry.path.split(Platform.pathSeparator).last;
      if (installedNames.contains(folder)) return true;
    }
    return false;
  }

  Future<Set<String>> _installedPackageNames() async {
    try {
      final pkgs = await _ref.read(coreRepositoryProvider).getPackages();
      return pkgs.map((p) => p.name).toSet();
    } catch (_) {
      return const {};
    }
  }

  String _join(String base, List<String> parts) =>
      [base, ...parts].join(Platform.pathSeparator);
}

final packageInstallProvider =
    StateNotifierProvider<PackageInstallNotifier, PackageInstallState>(
  (ref) => PackageInstallNotifier(ref),
);
