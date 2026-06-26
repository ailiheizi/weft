// Driver 入口：启用 Flutter Driver 扩展后再启动正常 app。
// 用于 MCP / flutter_driver 真实模拟点击、输入等用户交互测试。
// 跑法：flutter run -d windows -t lib/driver_main.dart
import 'package:flutter_driver/driver_extension.dart';

import 'main.dart' as app;

void main() {
  // 必须在 runApp 之前启用，driver 才能连接并操作 UI。
  enableFlutterDriverExtension();
  app.main();
}
