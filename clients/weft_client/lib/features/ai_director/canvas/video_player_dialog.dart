import 'dart:io';

import 'package:flutter/material.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

/// 视频播放对话框 — 用 media_kit 播放画布上的视频节点产物。
class VideoPlayerDialog extends StatefulWidget {
  const VideoPlayerDialog({super.key, required this.videoPath});

  final String videoPath;

  static Future<void> show(BuildContext context, String videoPath) {
    return showDialog(
      context: context,
      builder: (_) => VideoPlayerDialog(videoPath: videoPath),
    );
  }

  @override
  State<VideoPlayerDialog> createState() => _VideoPlayerDialogState();
}

class _VideoPlayerDialogState extends State<VideoPlayerDialog> {
  late final Player _player = Player();
  late final VideoController _controller = VideoController(_player);

  @override
  void initState() {
    super.initState();
    if (File(widget.videoPath).existsSync()) {
      _player.open(Media('file:///${widget.videoPath.replaceAll('\\', '/')}'));
    }
  }

  @override
  void dispose() {
    _player.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final exists = File(widget.videoPath).existsSync();
    return Dialog(
      backgroundColor: Colors.black,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 720, maxHeight: 540),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Expanded(
              child: exists
                  ? Video(controller: _controller)
                  : const Center(
                      child: Text('视频文件不存在', style: TextStyle(color: Colors.white70)),
                    ),
            ),
            Align(
              alignment: Alignment.centerRight,
              child: TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: const Text('关闭'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
