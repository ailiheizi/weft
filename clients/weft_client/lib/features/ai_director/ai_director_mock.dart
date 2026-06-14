// Mock data for the ai-director native workbench.

class DirectorMediaAsset {
  const DirectorMediaAsset({
    required this.fileName,
    required this.durationLabel,
  });

  final String fileName;
  final String durationLabel;
}

class DirectorPlan {
  const DirectorPlan({
    required this.title,
    required this.segments,
    required this.orderLabel,
    required this.moodTags,
    required this.reason,
  });

  final String title;
  final List<String> segments;
  final String orderLabel;
  final List<String> moodTags;
  final String reason;
}

class TimelineBlock {
  const TimelineBlock({
    required this.startSecond,
    required this.durationSecond,
    required this.label,
  });

  final double startSecond;
  final double durationSecond;
  final String label;
}

const mockDirectorAssets = <DirectorMediaAsset>[
  DirectorMediaAsset(fileName: '品牌主片_A-cam.mp4', durationLabel: '02:16'),
  DirectorMediaAsset(fileName: '街景补拍_B-roll.mov', durationLabel: '00:48'),
];

const mockDirectorPlans = <DirectorPlan>[
  DirectorPlan(
    title: '方案 A · 情绪先行',
    segments: ['00:12-00:27', '00:41-00:58', '01:20-01:36'],
    orderLabel: '开场钩子 → 人物特写 → 收束留白',
    moodTags: ['快切', '冷色调', '克制'],
    reason:
        '先用街景切入制造悬念，再把人物特写压到第二拍出现，能更快建立气质；结尾留 2 秒空镜，方便强化“会继续发生”的余味。',
  ),
  DirectorPlan(
    title: '方案 B · 信息优先',
    segments: ['00:03-00:18', '00:32-00:45', '01:02-01:19'],
    orderLabel: '产品亮相 → 动作演示 → 品牌口号',
    moodTags: ['中速', '清晰', '功能感'],
    reason:
        '把产品镜头提前，能在前 5 秒明确主题，减少观众理解成本；中段保留动作连续性，让信息密度和可看性更平衡。',
  ),
  DirectorPlan(
    title: '方案 C · 反差记忆点',
    segments: ['00:21-00:35', '00:59-01:11', '01:28-01:44'],
    orderLabel: '静场铺垫 → 节奏提速 → 反差收尾',
    moodTags: ['留白', '反差', '记忆点'],
    reason:
        '前段先压低节奏，能衬托后段提速的冲击力；最后用反差镜头收尾，更符合“先克制、后爆发”的个人偏好。',
  ),
];

const mockTimelineBlocks = <TimelineBlock>[
  TimelineBlock(startSecond: 12, durationSecond: 15, label: '片段 1'),
  TimelineBlock(startSecond: 41, durationSecond: 17, label: '片段 2'),
  TimelineBlock(startSecond: 80, durationSecond: 16, label: '片段 3'),
];

const mockStyleProfile = <String>['偏好快切', '冷色调', '留白结尾'];
