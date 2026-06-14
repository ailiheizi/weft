import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import '../models/app.dart';
import '../models/package.dart';
import '../models/provider.dart';
import '../models/service.dart';
import 'core_repository.dart';

part 'data_providers.g.dart';

@riverpod
Future<bool> coreHealth(Ref ref) async {
  return ref.watch(coreRepositoryProvider).checkHealth();
}

@riverpod
Future<List<ResolvedApp>> apps(Ref ref) async {
  return ref.watch(coreRepositoryProvider).getApps();
}

@riverpod
Future<ResolvedApp> appDetail(Ref ref, String appName) async {
  return ref.watch(coreRepositoryProvider).getApp(appName);
}

@riverpod
Future<List<ProviderConfig>> providers(Ref ref) async {
  return ref.watch(coreRepositoryProvider).getProviders();
}

@riverpod
Future<List<PackageInfo>> packages(Ref ref) async {
  return ref.watch(coreRepositoryProvider).getPackages();
}

@riverpod
Future<List<ServiceInfo>> services(Ref ref) async {
  return ref.watch(coreRepositoryProvider).getServices();
}
