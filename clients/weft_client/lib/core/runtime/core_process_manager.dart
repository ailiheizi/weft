import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';

/// Join path segments with the platform separator (avoids a `package:path`
/// dependency that the client's pubspec doesn't declare).
String _join(String a, [String? b, String? c, String? d, String? e]) {
  final sep = Platform.pathSeparator;
  return [a, b, c, d, e].whereType<String>().join(sep);
}

/// Manages the bundled `weft-core` sidecar process.
///
/// On startup the client tries to reach an already-running core at
/// [host]:[port]; if one is found it is reused (e.g. a developer running the
/// core by hand). Otherwise the bundled `weft-core` binary is located and
/// launched with an explicit `--data-dir`, and we poll `/api/health` until it
/// is ready.
///
/// The explicit data dir is the key to avoiding loopback-token 401s: the core
/// writes its `runtime-token` there, and the client reads it back from the same
/// known location ([tokenFilePath]) instead of guessing.
///
/// Only a core that *we* started is terminated on dispose — a reused, externally
/// managed core is left untouched.
class CoreProcessManager {
  CoreProcessManager({
    this.host = '127.0.0.1',
    this.port = 3004,
    this.startupTimeout = const Duration(seconds: 30),
  });

  final String host;
  final int port;
  final Duration startupTimeout;

  Process? _process;
  bool _startedByUs = false;
  String? _dataDir;

  String get baseUrl => 'http://$host:$port';

  /// The data dir passed to the core via `--data-dir`, once resolved.
  /// Null until [ensureRunning] has run.
  String? get dataDir => _dataDir;

  /// Absolute path to the loopback `runtime-token` file, once [dataDir] is
  /// resolved. The client reads its bearer token from here.
  String? get tokenFilePath =>
      _dataDir == null ? null : _join(_dataDir!, 'runtime-token');

  /// Resolve a stable, writable data dir for the sidecar core.
  /// Uses the OS application-support directory so it is consistent across runs
  /// and independent of the working directory.
  Future<String> _resolveDataDir() async {
    if (_dataDir != null) return _dataDir!;
    final support = await getApplicationSupportDirectory();
    final dir = _join(support.path, 'weft-core-data');
    await Directory(dir).create(recursive: true);
    _dataDir = dir;
    return dir;
  }

  /// Ensure a core is reachable. Returns when `/api/health` responds OK.
  ///
  /// Throws [CoreLaunchException] if the bundled binary can't be found or the
  /// core fails to become healthy within [startupTimeout].
  Future<void> ensureRunning() async {
    // Resolve the data dir up front so [tokenFilePath] is available even when
    // we end up reusing an externally-started core.
    await _resolveDataDir();
    if (await _isHealthy()) {
      debugPrint('weft-core already running at $baseUrl — reusing it');
      return;
    }

    final binary = _locateCoreBinary();
    if (binary == null) {
      throw CoreLaunchException(
        'Could not locate the bundled weft-core binary. Searched next to the '
        'application executable and in the development target directory.',
      );
    }

    debugPrint('starting weft-core: ${binary.path} (data-dir: $_dataDir)');
    try {
      _process = await Process.start(
        binary.path,
        ['--port', '$port', '--data-dir', _dataDir!],
        workingDirectory: binary.parent.path,
        // Detach stdio; the core logs to its own facilities.
        mode: ProcessStartMode.normal,
      );
      _startedByUs = true;
    } on ProcessException catch (e) {
      throw CoreLaunchException('Failed to start weft-core: ${e.message}');
    }

    // Surface core output to the debug console during development.
    _process!.stdout.transform(const SystemEncoding().decoder).listen(
          (line) => debugPrint('[weft-core] $line'),
        );
    _process!.stderr.transform(const SystemEncoding().decoder).listen(
          (line) => debugPrint('[weft-core:err] $line'),
        );

    await _waitForHealth();
  }

  /// Poll the health endpoint until it responds or [startupTimeout] elapses.
  Future<void> _waitForHealth() async {
    final deadline = DateTime.now().add(startupTimeout);
    while (DateTime.now().isBefore(deadline)) {
      // If the process died during startup, fail fast.
      if (_startedByUs && _process != null) {
        final exitedWith = await _process!.exitCode
            .timeout(const Duration(milliseconds: 1), onTimeout: () => -999);
        if (exitedWith != -999) {
          throw CoreLaunchException(
            'weft-core exited during startup (code $exitedWith)',
          );
        }
      }
      if (await _isHealthy()) return;
      await Future<void>.delayed(const Duration(milliseconds: 400));
    }
    throw CoreLaunchException(
      'weft-core did not become healthy within ${startupTimeout.inSeconds}s',
    );
  }

  /// A lightweight GET on `/api/health` using a raw HttpClient (no dio dep here).
  Future<bool> _isHealthy() async {
    final client = HttpClient()
      ..connectionTimeout = const Duration(seconds: 1);
    try {
      final req = await client
          .getUrl(Uri.parse('$baseUrl/api/health'))
          .timeout(const Duration(seconds: 2));
      final res = await req.close().timeout(const Duration(seconds: 2));
      await res.drain<void>();
      return res.statusCode == 200;
    } catch (_) {
      return false;
    } finally {
      client.close(force: true);
    }
  }

  /// Find the `weft-core` executable.
  ///
  /// Search order:
  ///   1. Bundled next to the app executable (production install).
  ///   2. A `weft-core` subfolder next to the app executable.
  ///   3. The Cargo target dir relative to the repo (developer runs).
  File? _locateCoreBinary() {
    final exeName = Platform.isWindows ? 'weft-core.exe' : 'weft-core';

    final candidates = <String>[];

    // 1 & 2: relative to the running Flutter executable.
    final appDir = File(Platform.resolvedExecutable).parent.path;
    candidates.add(_join(appDir, exeName));
    candidates.add(_join(appDir, 'weft-core', exeName));
    candidates.add(_join(appDir, 'data', 'flutter_assets', 'core', exeName));

    // 3: development — walk up to the repo root and look in target/.
    var dir = Directory.current;
    for (var i = 0; i < 6 && dir.parent.path != dir.path; i++) {
      candidates.add(_join(dir.path, 'target', 'release', exeName));
      candidates.add(_join(dir.path, 'target', 'debug', exeName));
      dir = dir.parent;
    }

    for (final path in candidates) {
      final f = File(path);
      if (f.existsSync()) return f;
    }
    return null;
  }

  /// Stop the core if (and only if) we started it.
  Future<void> dispose() async {
    if (!_startedByUs || _process == null) return;
    debugPrint('stopping weft-core (pid ${_process!.pid})');
    _process!.kill(ProcessSignal.sigterm);
    try {
      await _process!.exitCode.timeout(const Duration(seconds: 5));
    } on TimeoutException {
      _process!.kill(ProcessSignal.sigkill);
    }
    _process = null;
    _startedByUs = false;
  }
}

class CoreLaunchException implements Exception {
  const CoreLaunchException(this.message);
  final String message;
  @override
  String toString() => 'CoreLaunchException: $message';
}
