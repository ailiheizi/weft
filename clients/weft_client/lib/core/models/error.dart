/// Unified exception hierarchy for weft-core API errors.
sealed class AppException implements Exception {
  const AppException();
  String get message;

  @override
  String toString() => 'AppException: $message';
}

/// weft-core is unreachable (connection refused / timeout).
class CoreOfflineException extends AppException {
  const CoreOfflineException([String message = 'weft-core is not running'])
      : _message = message;

  final String _message;

  @override
  String get message => _message;
}

/// The server returned a 4xx or 5xx response.
class ApiException extends AppException {
  const ApiException({required this.statusCode, required String message})
      : _message = message;

  final int statusCode;
  final String _message;

  @override
  String get message => _message;

  @override
  String toString() => 'ApiException($statusCode): $message';
}
