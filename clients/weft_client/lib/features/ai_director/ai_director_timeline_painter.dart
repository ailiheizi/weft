import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../shared/theme/spacing.dart';
import 'ai_director_mock.dart';

/// Timeline preview for the ai-director workbench.
class AiDirectorTimeline extends StatelessWidget {
  const AiDirectorTimeline({
    super.key,
    required this.blocks,
    this.totalDuration = 120,
    this.currentSecond = 0,
    this.plan,
    this.compact = false,
  });

  final List<TimelineBlock> blocks;
  final double totalDuration;
  final double currentSecond;
  final DirectorPlan? plan;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final tracks = _buildTracks(blocks, plan);

    return Container(
      width: double.infinity,
      padding: EdgeInsets.all(compact ? 10 : Spacing.sm),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(14),
        border: Border.all(
          color: theme.colorScheme.outline.withValues(alpha: 0.2),
        ),
      ),
      child: CustomPaint(
        painter: _AiDirectorTimelinePainter(
          tracks: tracks,
          totalDuration: totalDuration,
          currentSecond: currentSecond,
          scheme: theme.colorScheme,
          textDirection: Directionality.of(context),
          compact: compact,
        ),
        child: const SizedBox.expand(),
      ),
    );
  }
}

class _TimelineTrack {
  const _TimelineTrack({
    required this.label,
    required this.blocks,
    required this.fillColor,
    this.waveform = false,
  });

  final String label;
  final List<TimelineBlock> blocks;
  final Color fillColor;
  final bool waveform;
}

class _AiDirectorTimelinePainter extends CustomPainter {
  const _AiDirectorTimelinePainter({
    required this.tracks,
    required this.totalDuration,
    required this.currentSecond,
    required this.scheme,
    required this.textDirection,
    required this.compact,
  });

  final List<_TimelineTrack> tracks;
  final double totalDuration;
  final double currentSecond;
  final ColorScheme scheme;
  final TextDirection textDirection;
  final bool compact;

  @override
  void paint(Canvas canvas, Size size) {
    final leftLabelWidth = compact ? 58.0 : 68.0;
    final topInset = compact ? 6.0 : 8.0;
    final rightInset = compact ? 8.0 : 10.0;
    final bottomInset = compact ? 10.0 : 12.0;
    final headerHeight = compact ? 26.0 : 30.0;
    final trackGap = compact ? 6.0 : 8.0;
    final bodyHeight =
        size.height - topInset - bottomInset - headerHeight - trackGap * 2;
    final trackHeight = math.max(bodyHeight / 3, compact ? 24.0 : 28.0);
    final timelineLeft = leftLabelWidth + 10;
    final usableWidth = math.max(size.width - timelineLeft - rightInset, 40.0);

    final bgPaint = Paint()
      ..color = scheme.surface.withValues(alpha: 0.52)
      ..style = PaintingStyle.fill;
    final gridPaint = Paint()
      ..color = scheme.outline.withValues(alpha: 0.14)
      ..strokeWidth = 1;
    final strongGridPaint = Paint()
      ..color = scheme.outline.withValues(alpha: 0.28)
      ..strokeWidth = 1;
    final separatorPaint = Paint()
      ..color = scheme.outline.withValues(alpha: 0.16)
      ..strokeWidth = 1;
    final playheadPaint = Paint()
      ..color = const Color(0xFFF87171)
      ..strokeWidth = 1.6;

    final timelineRect = RRect.fromRectAndRadius(
      Rect.fromLTWH(
        timelineLeft,
        topInset,
        usableWidth,
        size.height - topInset - bottomInset,
      ),
      const Radius.circular(12),
    );
    canvas.drawRRect(timelineRect, bgPaint);

    final tickStyle = TextStyle(
      color: scheme.onSurfaceVariant,
      fontSize: compact ? 9.5 : 10.5,
      fontWeight: FontWeight.w500,
    );

    for (var second = 0.0; second <= totalDuration; second += 5) {
      final ratio = second / totalDuration;
      final x = timelineLeft + usableWidth * ratio;
      final majorTick = second % 20 == 0;
      final tickTop = topInset + (majorTick ? 0 : 10);
      canvas.drawLine(
        Offset(x, tickTop),
        Offset(x, size.height - bottomInset),
        majorTick ? strongGridPaint : gridPaint,
      );

      if (majorTick) {
        final label = _formatSecond(second);
        final textPainter = TextPainter(
          text: TextSpan(text: label, style: tickStyle),
          textDirection: textDirection,
        )..layout();
        final textX = (x - textPainter.width / 2)
            .clamp(timelineLeft, size.width - rightInset - textPainter.width);
        textPainter.paint(canvas, Offset(textX, topInset + 2));
      }
    }

    final trackLabelStyle = TextStyle(
      color: scheme.onSurfaceVariant,
      fontSize: compact ? 10 : 11,
      fontWeight: FontWeight.w600,
      letterSpacing: 0.18,
    );
    final blockTextStyle = TextStyle(
      color: Colors.white,
      fontSize: compact ? 9.5 : 10.5,
      fontWeight: FontWeight.w700,
    );

    for (var trackIndex = 0; trackIndex < tracks.length; trackIndex++) {
      final track = tracks[trackIndex];
      final y = topInset + headerHeight + trackIndex * (trackHeight + trackGap);
      final trackRect = RRect.fromRectAndRadius(
        Rect.fromLTWH(timelineLeft + 4, y, usableWidth - 8, trackHeight),
        const Radius.circular(10),
      );
      final trackPaint = Paint()
        ..color = scheme.surfaceContainerHighest.withValues(alpha: 0.6);
      canvas.drawRRect(trackRect, trackPaint);

      final labelPainter = TextPainter(
        text: TextSpan(text: track.label, style: trackLabelStyle),
        textDirection: textDirection,
      )..layout(maxWidth: leftLabelWidth - 6);
      labelPainter.paint(
        canvas,
        Offset(0, y + (trackHeight - labelPainter.height) / 2),
      );

      canvas.drawLine(
        Offset(timelineLeft + 4, y - trackGap / 2),
        Offset(timelineLeft + usableWidth - 4, y - trackGap / 2),
        separatorPaint,
      );

      for (var index = 0; index < track.blocks.length; index++) {
        final block = track.blocks[index];
        final startRatio = block.startSecond / totalDuration;
        final durationRatio = block.durationSecond / totalDuration;
        final blockLeft = timelineLeft + 8 + (usableWidth - 16) * startRatio;
        final rawWidth = (usableWidth - 16) * durationRatio;
        final blockWidth = math.max<double>(rawWidth - 4, track.waveform ? 42 : 34);
        final blockRect = RRect.fromRectAndRadius(
          Rect.fromLTWH(blockLeft, y + 4, blockWidth, trackHeight - 8),
          const Radius.circular(8),
        );
        final fillPaint = Paint()..color = track.fillColor.withValues(alpha: 0.88);
        final strokePaint = Paint()
          ..style = PaintingStyle.stroke
          ..strokeWidth = 1
          ..color = Colors.white.withValues(alpha: 0.08);
        canvas.drawRRect(blockRect, fillPaint);
        canvas.drawRRect(blockRect, strokePaint);

        if (track.waveform) {
          _paintWaveform(canvas, blockRect);
        } else {
          final textPainter = TextPainter(
            text: TextSpan(
              text: block.label,
              style: blockTextStyle,
            ),
            textDirection: textDirection,
            maxLines: 1,
            ellipsis: '…',
          )..layout(maxWidth: math.max(blockWidth - 10, 10));
          textPainter.paint(
            canvas,
            Offset(
              blockLeft + 6,
              y + (trackHeight - textPainter.height) / 2,
            ),
          );
        }

        if (index < track.blocks.length - 1) {
          final gapX = blockRect.outerRect.right + 2;
          canvas.drawLine(
            Offset(gapX, y + 5),
            Offset(gapX, y + trackHeight - 5),
            Paint()
              ..color = Colors.black.withValues(alpha: 0.22)
              ..strokeWidth = 1,
          );
        }
      }
    }

    final playheadX = timelineLeft + usableWidth * (currentSecond / totalDuration);
    canvas.drawLine(
      Offset(playheadX, topInset + headerHeight - 2),
      Offset(playheadX, size.height - bottomInset),
      playheadPaint,
    );

    final trianglePath = Path()
      ..moveTo(playheadX, topInset + headerHeight - 6)
      ..lineTo(playheadX - 5, topInset + headerHeight - 14)
      ..lineTo(playheadX + 5, topInset + headerHeight - 14)
      ..close();
    canvas.drawPath(
      trianglePath,
      Paint()..color = const Color(0xFFF87171),
    );
  }

  void _paintWaveform(Canvas canvas, RRect blockRect) {
    final waveformPaint = Paint()
      ..color = Colors.white.withValues(alpha: 0.48)
      ..strokeWidth = 1.3
      ..style = PaintingStyle.stroke;
    final rect = blockRect.outerRect;
    final bars = math.max((rect.width / 6).floor(), 6);
    final centerY = rect.center.dy;

    for (var index = 0; index < bars; index++) {
      final x = rect.left + 4 + index * ((rect.width - 8) / bars);
      final seed = math.sin(index * 1.25) * 0.5 + math.cos(index * 0.7) * 0.5;
      final normalized = (0.18 + seed.abs() * 0.34) * rect.height;
      canvas.drawLine(
        Offset(x, centerY - normalized / 2),
        Offset(x, centerY + normalized / 2),
        waveformPaint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _AiDirectorTimelinePainter oldDelegate) {
    return oldDelegate.tracks != tracks ||
        oldDelegate.totalDuration != totalDuration ||
        oldDelegate.currentSecond != currentSecond ||
        oldDelegate.scheme != scheme ||
        oldDelegate.textDirection != textDirection ||
        oldDelegate.compact != compact;
  }
}

List<_TimelineTrack> _buildTracks(
  List<TimelineBlock> blocks,
  DirectorPlan? plan,
) {
  final labels = plan?.segments ?? const <String>[];
  final videoBlocks = <TimelineBlock>[];
  final bRollBlocks = <TimelineBlock>[];
  final audioBlocks = <TimelineBlock>[];

  for (var index = 0; index < blocks.length; index++) {
    final block = blocks[index];
    final label = index < labels.length ? labels[index] : block.label;
    videoBlocks.add(
      TimelineBlock(
        startSecond: block.startSecond,
        durationSecond: block.durationSecond,
        label: label,
      ),
    );
  }

  if (blocks.length >= 2) {
    final first = blocks.first;
    final last = blocks.last;
    bRollBlocks.add(
      TimelineBlock(
        startSecond: math.max(first.startSecond - 6, 0),
        durationSecond: first.durationSecond * 0.55,
        label: '00:06-00:14',
      ),
    );
    bRollBlocks.add(
      TimelineBlock(
        startSecond: last.startSecond + 4,
        durationSecond: math.max(last.durationSecond * 0.45, 6),
        label: '01:24-01:31',
      ),
    );
  }

  audioBlocks.add(
    const TimelineBlock(
      startSecond: 0,
      durationSecond: 118,
      label: '配乐主轨',
    ),
  );
  audioBlocks.add(
    const TimelineBlock(
      startSecond: 18,
      durationSecond: 34,
      label: '环境声铺底',
    ),
  );

  return [
    _TimelineTrack(
      label: 'V1',
      blocks: videoBlocks,
      fillColor: const Color(0xFF4C8DFF),
    ),
    _TimelineTrack(
      label: 'B1',
      blocks: bRollBlocks,
      fillColor: const Color(0xFFFFA43A),
    ),
    _TimelineTrack(
      label: 'A1',
      blocks: audioBlocks,
      fillColor: const Color(0xFF2FC78A),
      waveform: true,
    ),
  ];
}

String _formatSecond(double second) {
  final totalSeconds = second.round();
  final minutes = totalSeconds ~/ 60;
  final seconds = totalSeconds % 60;
  final minuteText = minutes.toString().padLeft(2, '0');
  final secondText = seconds.toString().padLeft(2, '0');
  return '$minuteText:$secondText';
}
