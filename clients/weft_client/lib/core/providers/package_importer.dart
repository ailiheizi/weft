import 'dart:io';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'core_repository.dart';

/// 单个待导入包的预检结果。
class ImportCandidate {
  ImportCandidate({
    required this.folderPath,
    required this.manifestName,
    required this.valid,
    this.error,
  });

  /// 源文件夹绝对路径。
  final String folderPath;

  /// 从 package.toml [package] / [identity] 解析出的真实包名（reload 要用）。
  final String manifestName;

  /// 是否含合法 package.toml + package.wasm。
  final bool valid;
  final String? error;

  String get folderName => folderPath.split(Platform.pathSeparator).last;
}

/// 单个包的导入结果。
class ImportResult {
  ImportResult({required this.name, required this.ok, this.error});
  final String name;
  final bool ok;
  final String? error;
}

/// 本地包导入服务：预检 → 事务化拷贝 → reload。纯逻辑，无 UI。
class PackageImporter {
  PackageImporter(this._ref);
  final Ref _ref;

  /// 预检 bundle 目录：枚举子文件夹（或 bundle 本身就是单个包），
  /// 校验每个含 package.toml + package.wasm，解析 manifest name。
  List<ImportCandidate> inspect(String bundlePath) {
    final root = Directory(bundlePath);
    if (!root.existsSync()) return const [];

    // bundle 本身就是一个包（含 package.toml）→ 单包导入。
    if (File(_join(bundlePath, 'package.toml')).existsSync()) {
      return [_inspectOne(bundlePath)];
    }

    // 否则把每个子文件夹当作一个包。
    final out = <ImportCandidate>[];
    for (final entry in root.listSync()) {
      if (entry is Directory) out.add(_inspectOne(entry.path));
    }
    return out;
  }

  ImportCandidate _inspectOne(String folder) {
    final toml = File(_join(folder, 'package.toml'));
    final wasm = File(_join(folder, 'package.wasm'));
    final name = folder.split(Platform.pathSeparator).last;

    if (!toml.existsSync()) {
      return ImportCandidate(
          folderPath: folder,
          manifestName: name,
          valid: false,
          error: '缺少 package.toml');
    }
    if (!wasm.existsSync()) {
      return ImportCandidate(
          folderPath: folder,
          manifestName: name,
          valid: false,
          error: '缺少 package.wasm');
    }
    final manifestName = _parseName(toml.readAsStringSync()) ?? name;
    return ImportCandidate(
        folderPath: folder, manifestName: manifestName, valid: true);
  }

  /// 从 package.toml 文本里抽 [identity]/[package] 下的 name = "..."。
  String? _parseName(String toml) {
    final re = RegExp(r'''(?:^|\n)\s*name\s*=\s*["']([^"']+)["']''');
    final m = re.firstMatch(toml);
    return m?.group(1)?.trim();
  }

  /// 把一个候选包事务化拷贝进 installedDir，目录名用 manifestName。
  /// 先拷到 `name.tmp` 再 rename，规避 core live read_dir 看到半成品。
  Future<ImportResult> install(
      ImportCandidate cand, String installedDir) async {
    if (!cand.valid) {
      return ImportResult(
          name: cand.manifestName, ok: false, error: cand.error);
    }
    final dest = _join(installedDir, cand.manifestName);
    final tmp = '$dest.tmp';
    try {
      final tmpDir = Directory(tmp);
      if (tmpDir.existsSync()) tmpDir.deleteSync(recursive: true);
      _copyDir(Directory(cand.folderPath), tmpDir);

      // 目标已存在（升级）→ 先挪走旧的。
      final destDir = Directory(dest);
      if (destDir.existsSync()) destDir.deleteSync(recursive: true);
      tmpDir.renameSync(dest);

      // 让 core 重载该包（按 manifest name；core 实时扫盘，无需重启）。
      await _ref.read(coreRepositoryProvider).reloadPackage(cand.manifestName);
      return ImportResult(name: cand.manifestName, ok: true);
    } catch (e) {
      return ImportResult(
          name: cand.manifestName, ok: false, error: e.toString());
    }
  }

  void _copyDir(Directory src, Directory dst) {
    dst.createSync(recursive: true);
    for (final entry in src.listSync()) {
      final name = entry.path.split(Platform.pathSeparator).last;
      final target = _join(dst.path, name);
      if (entry is Directory) {
        _copyDir(entry, Directory(target));
      } else if (entry is File) {
        entry.copySync(target);
      }
    }
  }

  String _join(String base, String part) =>
      [base, part].join(Platform.pathSeparator);
}

final packageImporterProvider =
    Provider<PackageImporter>((ref) => PackageImporter(ref));
