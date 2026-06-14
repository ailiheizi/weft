// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'data_providers.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

String _$coreHealthHash() => r'05f97c65acf13ffdcb5c097ed818f5cf47fb4581';

/// See also [coreHealth].
@ProviderFor(coreHealth)
final coreHealthProvider = AutoDisposeFutureProvider<bool>.internal(
  coreHealth,
  name: r'coreHealthProvider',
  debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
      ? null
      : _$coreHealthHash,
  dependencies: null,
  allTransitiveDependencies: null,
);

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef CoreHealthRef = AutoDisposeFutureProviderRef<bool>;
String _$appsHash() => r'b03d93667a7c6ddfea09983af514d1502b73f163';

/// See also [apps].
@ProviderFor(apps)
final appsProvider = AutoDisposeFutureProvider<List<ResolvedApp>>.internal(
  apps,
  name: r'appsProvider',
  debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
      ? null
      : _$appsHash,
  dependencies: null,
  allTransitiveDependencies: null,
);

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef AppsRef = AutoDisposeFutureProviderRef<List<ResolvedApp>>;
String _$appDetailHash() => r'47e7e9a27a46c31200f6479d3150a0bf3ef0c764';

/// Copied from Dart SDK
class _SystemHash {
  _SystemHash._();

  static int combine(int hash, int value) {
    // ignore: parameter_assignments
    hash = 0x1fffffff & (hash + value);
    // ignore: parameter_assignments
    hash = 0x1fffffff & (hash + ((0x0007ffff & hash) << 10));
    return hash ^ (hash >> 6);
  }

  static int finish(int hash) {
    // ignore: parameter_assignments
    hash = 0x1fffffff & (hash + ((0x03ffffff & hash) << 3));
    // ignore: parameter_assignments
    hash = hash ^ (hash >> 11);
    return 0x1fffffff & (hash + ((0x00003fff & hash) << 15));
  }
}

/// See also [appDetail].
@ProviderFor(appDetail)
const appDetailProvider = AppDetailFamily();

/// See also [appDetail].
class AppDetailFamily extends Family<AsyncValue<ResolvedApp>> {
  /// See also [appDetail].
  const AppDetailFamily();

  /// See also [appDetail].
  AppDetailProvider call(String appName) {
    return AppDetailProvider(appName);
  }

  @override
  AppDetailProvider getProviderOverride(covariant AppDetailProvider provider) {
    return call(provider.appName);
  }

  static const Iterable<ProviderOrFamily>? _dependencies = null;

  @override
  Iterable<ProviderOrFamily>? get dependencies => _dependencies;

  static const Iterable<ProviderOrFamily>? _allTransitiveDependencies = null;

  @override
  Iterable<ProviderOrFamily>? get allTransitiveDependencies =>
      _allTransitiveDependencies;

  @override
  String? get name => r'appDetailProvider';
}

/// See also [appDetail].
class AppDetailProvider extends AutoDisposeFutureProvider<ResolvedApp> {
  /// See also [appDetail].
  AppDetailProvider(String appName)
    : this._internal(
        (ref) => appDetail(ref as AppDetailRef, appName),
        from: appDetailProvider,
        name: r'appDetailProvider',
        debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
            ? null
            : _$appDetailHash,
        dependencies: AppDetailFamily._dependencies,
        allTransitiveDependencies: AppDetailFamily._allTransitiveDependencies,
        appName: appName,
      );

  AppDetailProvider._internal(
    super._createNotifier, {
    required super.name,
    required super.dependencies,
    required super.allTransitiveDependencies,
    required super.debugGetCreateSourceHash,
    required super.from,
    required this.appName,
  }) : super.internal();

  final String appName;

  @override
  Override overrideWith(
    FutureOr<ResolvedApp> Function(AppDetailRef provider) create,
  ) {
    return ProviderOverride(
      origin: this,
      override: AppDetailProvider._internal(
        (ref) => create(ref as AppDetailRef),
        from: from,
        name: null,
        dependencies: null,
        allTransitiveDependencies: null,
        debugGetCreateSourceHash: null,
        appName: appName,
      ),
    );
  }

  @override
  AutoDisposeFutureProviderElement<ResolvedApp> createElement() {
    return _AppDetailProviderElement(this);
  }

  @override
  bool operator ==(Object other) {
    return other is AppDetailProvider && other.appName == appName;
  }

  @override
  int get hashCode {
    var hash = _SystemHash.combine(0, runtimeType.hashCode);
    hash = _SystemHash.combine(hash, appName.hashCode);

    return _SystemHash.finish(hash);
  }
}

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
mixin AppDetailRef on AutoDisposeFutureProviderRef<ResolvedApp> {
  /// The parameter `appName` of this provider.
  String get appName;
}

class _AppDetailProviderElement
    extends AutoDisposeFutureProviderElement<ResolvedApp>
    with AppDetailRef {
  _AppDetailProviderElement(super.provider);

  @override
  String get appName => (origin as AppDetailProvider).appName;
}

String _$providersHash() => r'5a195fbf1c4168ae044d0a6da299722fc16d8ee5';

/// See also [providers].
@ProviderFor(providers)
final providersProvider =
    AutoDisposeFutureProvider<List<ProviderConfig>>.internal(
      providers,
      name: r'providersProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$providersHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef ProvidersRef = AutoDisposeFutureProviderRef<List<ProviderConfig>>;
String _$packagesHash() => r'44f49495cbc8038244eb5d6268231742376837e3';

/// See also [packages].
@ProviderFor(packages)
final packagesProvider = AutoDisposeFutureProvider<List<PackageInfo>>.internal(
  packages,
  name: r'packagesProvider',
  debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
      ? null
      : _$packagesHash,
  dependencies: null,
  allTransitiveDependencies: null,
);

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef PackagesRef = AutoDisposeFutureProviderRef<List<PackageInfo>>;
String _$servicesHash() => r'6caeca5557212f7f0f174c784bc772f33fd9dca4';

/// See also [services].
@ProviderFor(services)
final servicesProvider = AutoDisposeFutureProvider<List<ServiceInfo>>.internal(
  services,
  name: r'servicesProvider',
  debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
      ? null
      : _$servicesHash,
  dependencies: null,
  allTransitiveDependencies: null,
);

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef ServicesRef = AutoDisposeFutureProviderRef<List<ServiceInfo>>;
// ignore_for_file: type=lint
// ignore_for_file: subtype_of_sealed_class, invalid_use_of_internal_member, invalid_use_of_visible_for_testing_member, deprecated_member_use_from_same_package
