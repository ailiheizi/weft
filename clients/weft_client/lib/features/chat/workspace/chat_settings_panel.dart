import 'package:flutter/material.dart';

import 'management_tabs.dart' show ScenesPanel, SkillsPanel, McpPanel;
import '../../settings/selector_management_screen.dart' show SelectorPanel;

/// 聊天内统一设置面板（右侧抽屉内容）。
///
/// 4 个 Tab：场景 / 技能 / MCP / 工具选择器。
/// 复用已有的 ScenesPanel / SkillsPanel / McpPanel / SelectorPanel，
/// 它们都是纯 Column（无 Scaffold），可直接嵌入 TabBarView。
class ChatSettingsPanel extends StatefulWidget {
  const ChatSettingsPanel({super.key, this.initialIndex = 0});

  /// 初始选中的 Tab（0=场景 1=技能 2=MCP 3=工具选择器）。
  final int initialIndex;

  @override
  State<ChatSettingsPanel> createState() => _ChatSettingsPanelState();
}

class _ChatSettingsPanelState extends State<ChatSettingsPanel>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;

  static const _tabs = [
    (icon: Icons.tune, label: '场景'),
    (icon: Icons.auto_awesome_outlined, label: '技能'),
    (icon: Icons.cloud_outlined, label: 'MCP'),
    (icon: Icons.auto_awesome, label: '工具选择器'),
  ];

  @override
  void initState() {
    super.initState();
    _tabController = TabController(
      length: _tabs.length,
      vsync: this,
      initialIndex: widget.initialIndex.clamp(0, _tabs.length - 1),
    );
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Drawer(
      width: 480,
      backgroundColor: theme.colorScheme.surface,
      child: SafeArea(
        child: Column(
          children: [
            // 标题栏
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 8, 0),
              child: Row(
                children: [
                  Icon(Icons.settings_outlined,
                      size: 18, color: theme.colorScheme.primary),
                  const SizedBox(width: 8),
                  Text('设置',
                      style: theme.textTheme.titleMedium
                          ?.copyWith(fontWeight: FontWeight.w600)),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close, size: 18),
                    tooltip: '关闭',
                    onPressed: () => Navigator.of(context).maybePop(),
                  ),
                ],
              ),
            ),
            // Tab 栏
            TabBar(
              controller: _tabController,
              isScrollable: true,
              tabAlignment: TabAlignment.start,
              labelStyle: const TextStyle(fontSize: 13),
              tabs: [
                for (final t in _tabs)
                  Tab(
                    height: 40,
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(t.icon, size: 16),
                        const SizedBox(width: 6),
                        Text(t.label),
                      ],
                    ),
                  ),
              ],
            ),
            const Divider(height: 1),
            // Tab 内容
            Expanded(
              child: TabBarView(
                controller: _tabController,
                children: const [
                  ScenesPanel(),
                  SkillsPanel(),
                  McpPanel(),
                  SelectorPanel(),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
