import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/models/catalog.dart';
import '../../core/providers/catalog_service.dart';
import '../../core/providers/core_repository.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/package_install_provider.dart';
import '../../core/providers/package_importer.dart';
import '../../shared/widgets/glass_card.dart';

/// 在线安装(去中心化 GitHub 清单 registry):拉 catalog → 可搜列表 →
/// 选包算依赖闭包 → 下载+校验+装+reload。core 零改动,全走 CDN 免限流。
class OnlineInstallDialog extends ConsumerStatefulWidget {
  const OnlineInstallDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showGeneralDialog(
      context: context,
      barrierDismissible: true,
      barrierLabel: 'Online install',
      barrierColor: const Color(0x99000000),
      transitionDuration: const Duration(milliseconds: 160),
      pageBuilder: (a, b, c) => const OnlineInstallDialog(),
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
  ConsumerState<OnlineInstallDialog> createState() =>
      _OnlineInstallDialogState();
}

enum _Stage { loading, browse, confirm, installing, done }

class _OnlineInstallDialogState extends ConsumerState<OnlineInstallDialog> {
  _Stage _stage = _Stage.loading;
  String? _error;
  Catalog? _catalog;
  Set<String> _installedNames = const {};
  String _query = '';

  // confirm 阶段
  CatalogPackage? _target;
  List<CatalogPackage> _closure = const [];
  List<String> _missingDeps = const [];

  // installing/done
  final List<String> _progress = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() { _stage = _Stage.loading; _error = null; });
    try {
      final cat = await ref.read(catalogServiceProvider).fetchCatalog();
      Set<String> installed = const {};
      try {
        final pkgs = await ref.read(coreRepositoryProvider).getPackages();
        installed = pkgs.map((p) => p.name).toSet();
      } catch (_) {}
      if (!mounted) return;
      setState(() {
        _catalog = cat;
        _installedNames = installed;
        _stage = _Stage.browse;
      });
    } catch (e) {
      if (mounted) setState(() { _error = '拉取清单失败:$e'; _stage = _Stage.browse; });
    }
  }

  void _selectPackage(CatalogPackage pkg) {
    final missing = <String>[];
    final closure = ref
        .read(catalogServiceProvider)
        .resolveClosure(_catalog!, pkg.name, missing: missing);
    setState(() {
      _target = pkg;
      _closure = closure;
      _missingDeps = missing;
      _stage = _Stage.confirm;
    });
  }

  Future<void> _install() async {
    if (!ref.read(packageInstallProvider).isLocated) {
      setState(() => _error = '请先在「导入本地包」里定位一次 installed 目录');
      return;
    }
    final dir = ref.read(packageInstallProvider).installedDir!;
    setState(() { _stage = _Stage.installing; _progress.clear(); _error = null; });

    final svc = ref.read(catalogServiceProvider);
    final importer = ref.read(packageImporterProvider);

    // 只装未安装的(增量)。依赖序:closure 已是拓扑序(依赖在前)。
    final toInstall =
        _closure.where((p) => !_installedNames.contains(p.name)).toList();

    for (final pkg in toInstall) {
      setState(() => _progress.add('下载 ${pkg.name}…'));
      try {
        final folder = await svc.downloadPackage(pkg);
        final cands = importer.inspect(folder);
        if (cands.isEmpty || !cands.first.valid) {
          setState(() => _progress.add('✗ ${pkg.name} 包无效'));
          continue;
        }
        final res = await importer.install(cands.first, dir);
        setState(() => _progress.add(
            res.ok ? '✓ ${pkg.name} 已安装' : '✗ ${pkg.name}: ${res.error}'));
      } catch (e) {
        setState(() => _progress.add('✗ ${pkg.name}: $e'));
      }
    }
    ref.invalidate(packagesProvider);
    if (mounted) setState(() => _stage = _Stage.done);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Align(
      alignment: const Alignment(0, -0.35),
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560, maxHeight: 600),
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
                    Row(children: [
                      Icon(Icons.cloud_download_outlined,
                          size: 18, color: theme.colorScheme.primary),
                      const SizedBox(width: 8),
                      Text('在线安装', style: theme.textTheme.titleMedium),
                      const Spacer(),
                      if (_stage == _Stage.browse)
                        IconButton(
                          icon: const Icon(Icons.refresh, size: 16),
                          tooltip: '刷新清单',
                          onPressed: () => _load(),
                        ),
                      IconButton(
                        icon: const Icon(Icons.close, size: 18),
                        onPressed: () => Navigator.of(context).pop(),
                      ),
                    ]),
                    const SizedBox(height: 12),
                    Flexible(child: SingleChildScrollView(child: _body(theme))),
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

  Widget _body(ThemeData theme) {
    switch (_stage) {
      case _Stage.loading:
        return const Padding(
          padding: EdgeInsets.all(28),
          child: Center(child: CircularProgressIndicator()),
        );
      case _Stage.browse:
        return _browse(theme);
      case _Stage.confirm:
        return _confirm(theme);
      case _Stage.installing:
      case _Stage.done:
        return _progressView(theme);
    }
  }

  Widget _browse(ThemeData theme) {
    final pkgs = (_catalog?.packages ?? const <CatalogPackage>[])
        .where((p) {
          final q = _query.trim().toLowerCase();
          if (q.isEmpty) return true;
          return ('${p.name} ${p.description} ${p.tags.join(' ')}')
              .toLowerCase()
              .contains(q);
        })
        .toList();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        TextField(
          autofocus: true,
          decoration: const InputDecoration(
            isDense: true,
            prefixIcon: Icon(Icons.search, size: 18),
            hintText: '搜索包…',
          ),
          onChanged: (v) => setState(() => _query = v),
        ),
        const SizedBox(height: 12),
        if (pkgs.isEmpty)
          Padding(
            padding: const EdgeInsets.all(20),
            child: Text('没有匹配的包', style: theme.textTheme.bodySmall),
          )
        else
          ...pkgs.map((p) => _pkgRow(theme, p)),
      ],
    );
  }

  Widget _pkgRow(ThemeData theme, CatalogPackage p) {
    final installed = _installedNames.contains(p.name);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(children: [
                  Text(p.name, style: theme.textTheme.bodyMedium),
                  const SizedBox(width: 6),
                  Text('v${p.version} · ${p.kind}',
                      style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant)),
                ]),
                if (p.description.isNotEmpty)
                  Text(p.description,
                      style: theme.textTheme.bodySmall,
                      maxLines: 1, overflow: TextOverflow.ellipsis),
              ],
            ),
          ),
          const SizedBox(width: 8),
          installed
              ? Text('已安装',
                  style: theme.textTheme.labelSmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant))
              : FilledButton(
                  onPressed: () => _selectPackage(p),
                  child: const Text('安装'),
                ),
        ],
      ),
    );
  }

  Widget _confirm(ThemeData theme) {
    final toInstall =
        _closure.where((p) => !_installedNames.contains(p.name)).toList();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('安装 ${_target?.name}',
            style: theme.textTheme.titleSmall),
        const SizedBox(height: 4),
        Text('将安装 ${toInstall.length} 个包(含依赖),${_closure.length - toInstall.length} 个已满足',
            style: theme.textTheme.bodySmall),
        const SizedBox(height: 12),
        ..._closure.map((p) {
          final has = _installedNames.contains(p.name);
          return Padding(
            padding: const EdgeInsets.symmetric(vertical: 3),
            child: Row(children: [
              Icon(has ? Icons.check_circle_outline : Icons.add_circle_outline,
                  size: 15,
                  color: has
                      ? theme.colorScheme.onSurfaceVariant
                      : theme.colorScheme.primary),
              const SizedBox(width: 8),
              Expanded(child: Text(p.name, style: theme.textTheme.bodyMedium)),
              Text(has ? '已满足' : '将安装',
                  style: theme.textTheme.labelSmall?.copyWith(
                      color: has
                          ? theme.colorScheme.onSurfaceVariant
                          : theme.colorScheme.primary)),
            ]),
          );
        }),
        if (_missingDeps.isNotEmpty) ...[
          const SizedBox(height: 10),
          Text('⚠ 清单缺失依赖:${_missingDeps.join(", ")}(安装后可能无法解析,core 会提示缺哪个能力)',
              style: theme.textTheme.bodySmall
                  ?.copyWith(color: theme.colorScheme.error)),
        ],
        const SizedBox(height: 8),
        Text('来源:GitHub Release,sha256 校验。',
            style: theme.textTheme.bodySmall),
        const SizedBox(height: 12),
        Row(children: [
          TextButton(
            onPressed: () => setState(() => _stage = _Stage.browse),
            child: const Text('返回'),
          ),
          const Spacer(),
          FilledButton(
            onPressed: toInstall.isEmpty ? null : _install,
            child: Text(toInstall.isEmpty ? '已全部安装' : '安装全部'),
          ),
        ]),
      ],
    );
  }

  Widget _progressView(ThemeData theme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        ..._progress.map((line) => Padding(
              padding: const EdgeInsets.symmetric(vertical: 2),
              child: Text(line, style: theme.textTheme.bodyMedium),
            )),
        if (_stage == _Stage.installing) ...[
          const SizedBox(height: 12),
          const Center(child: CircularProgressIndicator()),
        ],
        if (_stage == _Stage.done) ...[
          const SizedBox(height: 14),
          Align(
            alignment: Alignment.centerRight,
            child: FilledButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('完成'),
            ),
          ),
        ],
      ],
    );
  }
}
