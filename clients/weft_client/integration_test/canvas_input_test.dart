import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:weft_client/features/ai_director/canvas/hub_canvas_view.dart';
import 'package:weft_client/features/ai_director/canvas/canvas_state.dart';

/// 真实 engine 集成测试：渲染完整 HubCanvasView 三栏画布，
/// 验证「右栏对话输入框打字/退格正常、且不误删画布节点」的修复。
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpHub(WidgetTester tester) async {
    await tester.pumpWidget(
      const ProviderScope(
        child: MaterialApp(
          home: Scaffold(body: HubCanvasView()),
        ),
      ),
    );
    // 等 seedDemo / 首帧稳定。
    await tester.pumpAndSettle(const Duration(seconds: 1));
  }

  testWidgets('右栏输入框能正常打字并退格，不触发画布节点删除', (tester) async {
    await pumpHub(tester);

    final container = ProviderScope.containerOf(
      tester.element(find.byType(HubCanvasView)),
    );
    final nodeCountBefore = container.read(canvasProvider).nodes.length;
    expect(nodeCountBefore, greaterThan(0), reason: 'seedDemo 应有演示节点');

    // 找到右栏对话输入框（hintText 含「创意」）。
    final input = find.widgetWithText(TextField, '输入创意，或用 / 唤起技能…');
    expect(input, findsOneWidget, reason: '应能找到导演对话输入框');

    await tester.tap(input);
    await tester.pumpAndSettle();
    await tester.enterText(input, '你好导演');
    await tester.pumpAndSettle();
    expect(
      find.text('你好导演'),
      findsWidgets,
      reason: '输入框应能正常显示输入的文字',
    );

    // 在输入框内连按退格：应删字符，节点数不变。
    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    await tester.pumpAndSettle();

    final nodeCountAfter = container.read(canvasProvider).nodes.length;
    expect(
      nodeCountAfter,
      nodeCountBefore,
      reason: '输入框内退格不应删除画布节点（修复前会误删）',
    );
  });
}
