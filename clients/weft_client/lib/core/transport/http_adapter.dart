import 'dart:async';
import 'package:dio/dio.dart';

import 'transport_adapter.dart';

/// HTTP 传输适配器:把统一信封映射为 Dio HTTP 请求。
/// Phase 0 实现——包装现有 Dio 实例,后续可替换为 WS/FFI 而不改上层。
class HttpTransportAdapter implements TransportAdapter {
  HttpTransportAdapter(this._dio);

  final Dio _dio;

  @override
  Future<ResponseEnvelope> send(RequestEnvelope request) async {
    try {
      final Response<Map<String, dynamic>> resp;
      switch (request.method) {
        case RpcMethod.call:
          resp = await _dio.post(request.path, data: request.body);
        case RpcMethod.query:
          resp = await _dio.get(request.path, queryParameters: request.body);
        case RpcMethod.subscribe:
        case RpcMethod.cancel:
          // subscribe/cancel 不走普通 HTTP(需要 WS 或 SSE)。
          // Phase 0 fallback:subscribe 用 GET 模拟单次拉取。
          resp = await _dio.get(request.path, queryParameters: request.body);
      }
      return ResponseEnvelope(
        id: request.id,
        status: resp.statusCode ?? 200,
        body: resp.data,
      );
    } on DioException catch (e) {
      final status = e.response?.statusCode ?? 500;
      final body = e.response?.data;
      return ResponseEnvelope(
        id: request.id,
        status: status,
        body: body is Map<String, dynamic> ? body : {'error': e.message},
      );
    }
  }

  @override
  Stream<ResponseEnvelope> subscribe(RequestEnvelope request) {
    // Phase 0:subscribe 降级为轮询(2s 间隔 GET)。Phase 1 WS 后替换。
    final controller = StreamController<ResponseEnvelope>();
    Timer? timer;
    timer = Timer.periodic(const Duration(seconds: 2), (_) async {
      if (controller.isClosed) {
        timer?.cancel();
        return;
      }
      final resp = await send(request);
      if (!controller.isClosed) controller.add(resp);
    });
    controller.onCancel = () => timer?.cancel();
    return controller.stream;
  }

  @override
  Future<void> close() async {
    _dio.close();
  }
}
