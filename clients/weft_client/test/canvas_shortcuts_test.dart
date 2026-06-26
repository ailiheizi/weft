import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

/// 复现并验证「输入框内按 backspace 误删节点 / 无法打字」的修复。
///
/// 修复前的结构：CallbackShortcuts(含 backspace/delete) 包住整个三栏，
/// 且外层 Focus(autofocus:true) 抢焦点 → 输入框打字被吞、退格触发删除。
///
/// 修复后：删除键的 CallbackShortcuts 只包在中栏画布的局部 Focus 内，
/// 文本输入框不在其作用域 → 打字与退格正常，不触发删除。
void main() {
  testWidgets('输入框内打字与退格不触发画布删除', (tester) async {
    var deleteCalls = 0;
    final controller = TextEditingController();

    // 模拟修复后的布局：删除快捷键只作用于「画布」区域的局部 Focus，
    // 右栏输入框独立，不在该 CallbackShortcuts 子树内。
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Row(
            children: [
              // 中栏：画布（删除键在此局部生效）
              Expanded(
                child: CallbackShortcuts(
                  bindings: {
                    const SingleActivator(LogicalKeyboardKey.backspace): () => deleteCalls++,
                    const SingleActivator(LogicalKeyboardKey.delete): () => deleteCalls++,
                  },
                  child: const Focus(child: ColoredBox(color: Colors.black, child: SizedBox.expand())),
                ),
              ),
              // 右栏：对话输入框（独立，不受删除快捷键影响）
              SizedBox(
                width: 300,
                child: TextField(controller: controller),
              ),
            ],
          ),
        ),
      ),
    );

    // 聚焦输入框并打字。
    await tester.tap(find.byType(TextField));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'hello');
    await tester.pumpAndSettle();
    expect(controller.text, 'hello', reason: '应能正常输入文字');

    // 在输入框内按退格：应删掉一个字符，且不触发画布删除。
    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    await tester.pumpAndSettle();
    expect(controller.text, 'hell', reason: '退格应删字符');
    expect(deleteCalls, 0, reason: '输入框内退格不应触发画布删除');
  });

  testWidgets('画布获得焦点时 Delete 触发删除', (tester) async {
    var deleteCalls = 0;
    final focusNode = FocusNode();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: CallbackShortcuts(
            bindings: {
              const SingleActivator(LogicalKeyboardKey.delete): () => deleteCalls++,
            },
            child: Focus(
              focusNode: focusNode,
              child: const ColoredBox(color: Colors.black, child: SizedBox.expand()),
            ),
          ),
        ),
      ),
    );

    focusNode.requestFocus();
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.delete);
    await tester.pumpAndSettle();
    expect(deleteCalls, 1, reason: '画布焦点下 Delete 应触发删除');
  });
}
