import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:file_picker/file_picker.dart';

import '../../core/providers/core_repository.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/package_install_provider.dart';
import '../../core/providers/package_importer.dart';
import '../../shared/widgets/glass_card.dart';

/// 本地导入包对话框：首次引导定位 installed 目录 → 选 bundle → 预检 →
/// 一键安装 → 逐包 reload → 刷新列表。不改 core,纯客户端编排。
class ImportPackageDialog extends ConsumerStatefulWidget {
  const ImportPackageDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showGeneralDialog(
      context: context,
      barrierDismissible: true,
      barrierLabel: 'Import package',
      barrierColor: const Color(0x99000000),
      transitionDuration: const Duration(milliseconds: 160),
      pageBuilder: (a, b, c) => const ImportPackageDialog(),
      transitionBuilder: (ctx, anim, sec, child) {
        final curved =
            CurvedAnimation(parent: anim, curve: Curves.easeOutCubic);
        return FadeTransition(
          opacity: curved,
          child: SlideTransition(
            position: Tween(begin: const Offset(0, -0.03), end: Offset.zero)
                .animate(curved),
            child: child,
          ),
        );
      },
    );
  }

  @override
  ConsumerState<ImportPackageDialog> createState() =>
      _ImportPackageDialogState();
}

class _ImportPackageDialogState extends ConsumerState<ImportPackageDialog> {
  String? _error;
  bool _busy = false;
  List<ImportCandidate> _candidates = const [];
  Set<String> _installedNames = const {};
  List<ImportResult>? _results;

  @override
  void initState() {
    super.initState();
    _loadInstalledNames();
  }

  Future<void> _loadInstalledNames() async {
    try {
      final pkgs = await ref.read(coreRepositoryProvider).getPackages();
      if (mounted) setState(() => _installedNames = pkgs.map((p) => p.name).toSet());
    } catch (_) {}
  }

  // ── 首次:定位 installed 目录 ──────────────────────────────────────────
  Future<void> _openCoreDir() async {
    // 没有绝对路径时无法直接打开,提示用户手动定位。
    final dir = ref.read(packageInstallProvider).installedDir;
    if (dir != null) {
      try {
        // 确保目录存在,避免打开一个尚未创建的路径。
        Directory(dir).createSync(recursive: true);
        if (Platform.isWindows) {
          await Process.start('explorer', [dir]);
        } else if (Platform.isMacOS) {
          await Process.start('open', [dir]);
        } else {
          await Process.start('xdg-open', [dir]);
        }
      } catch (_) {}
    }
  }

  Future<void> _locateDir() async {
    setState(() { _busy = true; _error = null; });
    try {
      final picked = await FilePicker.getDirectoryPath(
          dialogTitle: '选择 packages/installed 目录');
      if (picked == null) return;
      final validated =
          await ref.read(packageInstallProvider.notifier).validateDir(picked);
      if (validated == null) {
        setState(() => _error =
            '这里没看到已装的包,可能指错了目录。请选 core 的 packages/installed(或它的上级 repo 根目录)。');
        return;
      }
      await ref.read(packageInstallProvider.notifier).setInstalledDir(validated);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  // ── 选 bundle + 预检 ──────────────────────────────────────────────────
  Future<void> _pickBundle() async {
    setState(() { _busy = true; _error = null; _results = null; });
    try {
      final picked = await FilePicker.getDirectoryPath(
          dialogTitle: '选择包文件夹(含 package.toml + package.wasm)');
      if (picked == null) return;
      final cands = ref.read(packageImporterProvider).inspect(picked);
      if (cands.isEmpty) {
        setState(() => _error = '该文件夹里没找到包(需含 package.toml + package.wasm)');
        return;
      }
      setState(() => _candidates = cands);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  // ── 确认安装 ──────────────────────────────────────────────────────────
  Future<void> _install() async {
    final dir = ref.read(packageInstallProvider).installedDir;
    if (dir == null) return;
    setState(() { _busy = true; _error = null; });
    final importer = ref.read(packageImporterProvider);
    final results = <ImportResult>[];
    for (final c in _candidates.where((c) => c.valid)) {
      results.add(await importer.install(c, dir));
    }
    ref.invalidate(packagesProvider);
    if (mounted) setState(() { _busy = false; _results = results; });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final located = ref.watch(packageInstallProvider).isLocated;

    return Align(
      alignment: const Alignment(0, -0.4),
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 520, maxHeight: 560),
          child: GlassCard(
            radius: 14,
            child: Material(
              type: MaterialType.transparency,
              child: Padding(
                padding: const EdgeInsets.all(20),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Icon(Icons.download_outlined,
                            size: 18, color: theme.colorScheme.primary),
                        const SizedBox(width: 8),
                        Text('导入本地包',
                            style: theme.textTheme.titleMedium),
                        const Spacer(),
                        IconButton(
                          icon: const Icon(Icons.close, size: 18),
                          onPressed: () => Navigator.of(context).pop(),
                        ),
                      ],
                    ),
                    const SizedBox(height: 12),
                    Flexible(
                      child: SingleChildScrollView(
                        child: located
                            ? _buildImportStep(theme)
                            : _buildLocateStep(theme),
                      ),
                    ),
                    if (_error != null) ...[
                      const SizedBox(height: 10),
                      Text(_error!,
                          style: theme.textTheme.bodySmall
                              ?.copyWith(color: theme.colorScheme.error)),
                    ],
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  // 首次:引导定位目录。
  Widget _buildLocateStep(ThemeData theme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('首次导入需先定位 core 的包目录',
            style: theme.textTheme.bodyMedium),
        const SizedBox(height: 8),
        Text(
          '通常在 weft-core 的 packages/installed/ 下。选中后客户端会用已装包名'
          '交叉校验,确认没指错。',
          style: theme.textTheme.bodySmall,
        ),
        const SizedBox(height: 16),
        FilledButton.icon(
          onPressed: _busy ? null : _locateDir,
          icon: const Icon(Icons.folder_open, size: 16),
          label: const Text('选择 installed 目录'),
        ),
      ],
    );
  }

  // 已定位:选包 + 预检 + 安装。
  Widget _buildImportStep(ThemeData theme) {
    if (_results != null) return _buildResults(theme);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            OutlinedButton.icon(
              onPressed: _busy ? null : _pickBundle,
              icon: const Icon(Icons.folder_open, size: 16),
              label: const Text('选择包文件夹'),
            ),
            const Spacer(),
            TextButton.icon(
              onPressed: _openCoreDir,
              icon: const Icon(Icons.folder_outlined, size: 14),
              label: const Text('打开包目录'),
            ),
          ],
        ),
        if (_candidates.isNotEmpty) ...[
          const SizedBox(height: 14),
          Text('将安装 ${_candidates.where((c) => c.valid).length} 个包',
              style: theme.textTheme.labelLarge),
          const SizedBox(height: 8),
          ..._candidates.map((c) => _candidateRow(theme, c)),
          const SizedBox(height: 8),
          Text('本地导入,未经 Store 签名校验。',
              style: theme.textTheme.bodySmall),
          const SizedBox(height: 12),
          Align(
            alignment: Alignment.centerRight,
            child: FilledButton(
              onPressed: _busy || !_candidates.any((c) => c.valid)
                  ? null
                  : _install,
              child: _busy
                  ? const SizedBox(
                      width: 16, height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2))
                  : const Text('全部安装'),
            ),
          ),
        ],
      ],
    );
  }

  Widget _candidateRow(ThemeData theme, ImportCandidate c) {
    final already = _installedNames.contains(c.manifestName);
    final (icon, color, tag) = !c.valid
        ? (Icons.error_outline, theme.colorScheme.error, c.error ?? '无效')
        : already
            ? (Icons.check_circle_outline, theme.colorScheme.onSurfaceVariant,
                '已安装·将覆盖')
            : (Icons.add_circle_outline, theme.colorScheme.primary, '将安装');
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          Icon(icon, size: 15, color: color),
          const SizedBox(width: 8),
          Expanded(
            child: Text(c.manifestName,
                style: theme.textTheme.bodyMedium,
                overflow: TextOverflow.ellipsis),
          ),
          Text(tag,
              style: theme.textTheme.labelSmall?.copyWith(color: color)),
        ],
      ),
    );
  }

  Widget _buildResults(ThemeData theme) {
    final ok = _results!.where((r) => r.ok).length;
    final fail = _results!.length - ok;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('完成:$ok 成功${fail > 0 ? ' · $fail 失败' : ''}',
            style: theme.textTheme.titleSmall),
        const SizedBox(height: 10),
        ..._results!.map((r) => Padding(
              padding: const EdgeInsets.symmetric(vertical: 3),
              child: Row(
                children: [
                  Icon(r.ok ? Icons.check_circle : Icons.error,
                      size: 15,
                      color: r.ok
                          ? const Color(0xFF4CB782)
                          : theme.colorScheme.error),
                  const SizedBox(width: 8),
                  Expanded(child: Text(r.name, style: theme.textTheme.bodyMedium)),
                  if (!r.ok && r.error != null)
                    Expanded(
                      child: Text(r.error!,
                          style: theme.textTheme.labelSmall
                              ?.copyWith(color: theme.colorScheme.error),
                          overflow: TextOverflow.ellipsis),
                    ),
                ],
              ),
            )),
        const SizedBox(height: 14),
        Align(
          alignment: Alignment.centerRight,
          child: FilledButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('完成'),
          ),
        ),
      ],
    );
  }
}
