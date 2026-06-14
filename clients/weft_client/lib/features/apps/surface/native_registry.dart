import 'package:flutter/material.dart';

import '../../../core/models/app.dart';
import '../../ai_director/ai_director_workbench_screen.dart';

/// Builder for a native (Flutter-authored) package surface.
typedef NativeSurfaceBuilder = Widget Function(
  BuildContext context,
  ResolvedApp app,
);

/// Registry mapping a `native:<id>` mode to a Flutter page builder.
///
/// A product package that wants a bespoke, high-interaction UI declares
/// `mode = "native:<id>"`; the client renders the page registered here under
/// `<id>`. Keep ids stable and namespaced by product.
///
/// Example (added when the ai-director surface lands):
///   'ai-director': (context, app) => AiDirectorWorkbench(app: app),
final Map<String, NativeSurfaceBuilder> nativeSurfaceRegistry = {
  'ai-director': (context, app) => AiDirectorWorkbenchScreen(app: app),
};
