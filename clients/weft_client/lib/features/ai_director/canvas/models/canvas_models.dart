import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';

/// 画布节点类型。
enum CanvasNodeKind { image, video, music, text }

/// 节点生成状态机。
/// idle: 已有素材，静态展示
/// proposed: Agent 提议生成，等待用户确认
/// generating: 确认后执行中
/// ready: 生成完成，有素材
/// failed: 生成失败
enum NodeStatus { idle, proposed, generating, ready, failed }

/// 生成参数（就地生成面板用）。
@immutable
class GenParams {
  const GenParams({
    this.model = 'gpt-image-2-vip',
    this.aspectRatio = '16:9',
    this.resolution = '720p',
    this.durationSec = 5,
  });

  final String model;
  final String aspectRatio;
  final String resolution;
  final int durationSec;

  GenParams copyWith({
    String? model,
    String? aspectRatio,
    String? resolution,
    int? durationSec,
  }) {
    return GenParams(
      model: model ?? this.model,
      aspectRatio: aspectRatio ?? this.aspectRatio,
      resolution: resolution ?? this.resolution,
      durationSec: durationSec ?? this.durationSec,
    );
  }
}

/// 画布节点 — 一张图 / 一段视频 / 一段音乐 / 一段文本，定位在画布坐标系上。
@immutable
class CanvasNode {
  const CanvasNode({
    required this.id,
    required this.kind,
    required this.position,
    this.size = const Size(180, 180),
    this.shotId,
    this.title = '',
    this.assetPath,
    this.thumbnailPath,
    this.prompt,
    this.params = const GenParams(),
    this.status = NodeStatus.idle,
    this.errorMessage,
  });

  final String id;
  final CanvasNodeKind kind;

  /// 画布坐标（未经视口变换的逻辑坐标）。
  final Offset position;
  final Size size;

  /// 归属的 Shot（分镜）id，可为空（散落节点）。
  final String? shotId;
  final String title;

  /// 落地素材路径（图片/视频文件绝对路径）。
  final String? assetPath;
  final String? thumbnailPath;

  /// 生成用提示词。
  final String? prompt;
  final GenParams params;
  final NodeStatus status;
  final String? errorMessage;

  Rect get rect => position & size;

  /// 用于连线的右侧出点（画布坐标）。
  Offset get outPort => Offset(position.dx + size.width, position.dy + size.height / 2);

  /// 用于连线的左侧入点（画布坐标）。
  Offset get inPort => Offset(position.dx, position.dy + size.height / 2);

  CanvasNode copyWith({
    CanvasNodeKind? kind,
    Offset? position,
    Size? size,
    Object? shotId = _sentinel,
    String? title,
    Object? assetPath = _sentinel,
    Object? thumbnailPath = _sentinel,
    Object? prompt = _sentinel,
    GenParams? params,
    NodeStatus? status,
    Object? errorMessage = _sentinel,
  }) {
    return CanvasNode(
      id: id,
      kind: kind ?? this.kind,
      position: position ?? this.position,
      size: size ?? this.size,
      shotId: shotId == _sentinel ? this.shotId : shotId as String?,
      title: title ?? this.title,
      assetPath: assetPath == _sentinel ? this.assetPath : assetPath as String?,
      thumbnailPath:
          thumbnailPath == _sentinel ? this.thumbnailPath : thumbnailPath as String?,
      prompt: prompt == _sentinel ? this.prompt : prompt as String?,
      params: params ?? this.params,
      status: status ?? this.status,
      errorMessage:
          errorMessage == _sentinel ? this.errorMessage : errorMessage as String?,
    );
  }
}

/// 节点间连线 — 表达派生关系 / 分镜流。
@immutable
class CanvasEdge {
  const CanvasEdge({
    required this.id,
    required this.fromNodeId,
    required this.toNodeId,
  });

  final String id;
  final String fromNodeId;
  final String toNodeId;
}

/// Shot（分镜）— 把若干节点归为一组。
@immutable
class Shot {
  const Shot({
    required this.id,
    required this.title,
    required this.order,
    this.nodeIds = const [],
  });

  final String id;
  final String title;
  final int order;
  final List<String> nodeIds;

  Shot copyWith({String? title, int? order, List<String>? nodeIds}) {
    return Shot(
      id: id,
      title: title ?? this.title,
      order: order ?? this.order,
      nodeIds: nodeIds ?? this.nodeIds,
    );
  }
}

/// copyWith 哨兵 — 区分「不传」与「显式置 null」。
const Object _sentinel = Object();
