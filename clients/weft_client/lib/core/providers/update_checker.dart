import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../constants.dart';

/// State holding the result of an update check.
class UpdateInfo {
  const UpdateInfo({
    required this.available,
    this.latestVersion,
    this.downloadUrl,
    this.releaseNotes,
  });

  final bool available;
  final String? latestVersion;
  final String? downloadUrl;
  final String? releaseNotes;

  static const upToDate = UpdateInfo(available: false);
}

/// Provider that checks GitHub releases for a newer version.
final updateCheckerProvider =
    FutureProvider.autoDispose<UpdateInfo>((ref) async {
  final dio = Dio(BaseOptions(
    connectTimeout: const Duration(seconds: 10),
    receiveTimeout: const Duration(seconds: 10),
  ));

  try {
    final response = await dio.get<Map<String, dynamic>>(
      'https://api.github.com/repos/$kGitHubRepo/releases/latest',
      options: Options(headers: {'Accept': 'application/vnd.github.v3+json'}),
    );

    final data = response.data;
    if (data == null) return UpdateInfo.upToDate;

    final tagName = data['tag_name'] as String? ?? '';
    final latestVersion = tagName.startsWith('v') ? tagName.substring(1) : tagName;

    if (_isNewer(latestVersion, kAppVersion)) {
      return UpdateInfo(
        available: true,
        latestVersion: latestVersion,
        downloadUrl: data['html_url'] as String?,
        releaseNotes: data['body'] as String?,
      );
    }

    return UpdateInfo.upToDate;
  } on DioException {
    // Network error — silently treat as no update.
    return UpdateInfo.upToDate;
  } finally {
    dio.close();
  }
});

/// Compare two semver-like version strings (e.g. "0.2.5.1" vs "0.2.4").
/// Returns true if [remote] is strictly newer than [local].
bool _isNewer(String remote, String local) {
  final r = remote.split('.').map((s) => int.tryParse(s) ?? 0).toList();
  final l = local.split('.').map((s) => int.tryParse(s) ?? 0).toList();
  final len = r.length > l.length ? r.length : l.length;
  for (var i = 0; i < len; i++) {
    final rv = i < r.length ? r[i] : 0;
    final lv = i < l.length ? l[i] : 0;
    if (rv > lv) return true;
    if (rv < lv) return false;
  }
  return false;
}
