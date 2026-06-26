import 'package:dio/dio.dart';

/// Scene(命名偏好场景)API 封装。对应 weft-core 的
/// `/api/apps/{app}/scenes` CRUD + `/bind` 激活。
///
/// 一个 scene = 一套偏好:启用/禁用插件(features)、provider 替换/版本(binding_pins/
/// package_pins)、角色→模型路由(role_routing)、工作区(workspace)。激活 scene 时
/// 后端会把 role_routing 写入 KV,使团队编排按场景切换模型。
class SceneApi {
  SceneApi(this._dio);

  final Dio _dio;

  /// 列出某 app 的所有场景 + 当前激活场景名。
  Future<SceneList> list(String app) async {
    final res = await _dio.get<Map<String, dynamic>>('/api/apps/$app/scenes');
    final data = res.data ?? const {};
    final scenes = (data['scenes'] as List? ?? [])
        .whereType<Map<String, dynamic>>()
        .map(Scene.fromJson)
        .toList();
    return SceneList(
      activeScene: data['active_scene'] as String? ?? '',
      scenes: scenes,
    );
  }

  /// 激活(绑定)一个场景。后端会套用该场景的 role_routing 等偏好。
  Future<void> bind(String app, String sceneName) async {
    await _dio.post<Map<String, dynamic>>(
      '/api/apps/$app/scenes/${Uri.encodeComponent(sceneName)}/bind',
      data: const <String, dynamic>{},
    );
  }

  /// 删除一个场景。后端禁止删除当前激活场景(返回 409)。
  Future<void> delete(String app, String sceneName) async {
    await _dio.delete<Map<String, dynamic>>(
      '/api/apps/$app/scenes/${Uri.encodeComponent(sceneName)}',
    );
  }

  /// 创建/更新一个场景。
  Future<void> create(String app, Scene scene) async {
    await _dio.post<Map<String, dynamic>>(
      '/api/apps/$app/scenes',
      data: scene.toCreateJson(),
    );
  }
}

class SceneList {
  const SceneList({required this.activeScene, required this.scenes});
  final String activeScene;
  final List<Scene> scenes;
}

/// 角色→模型路由的单项。
class SceneRoleModel {
  const SceneRoleModel({this.provider, this.model});
  final String? provider;
  final String? model;

  factory SceneRoleModel.fromJson(Map<String, dynamic> j) => SceneRoleModel(
        provider: j['provider'] as String?,
        model: j['model'] as String?,
      );

  Map<String, dynamic> toJson() => {
        if (provider != null && provider!.isNotEmpty) 'provider': provider,
        if (model != null && model!.isNotEmpty) 'model': model,
      };
}

class Scene {
  const Scene({
    required this.name,
    this.description = '',
    this.profile = '',
    this.enabledFeatures = const [],
    this.disabledFeatures = const [],
    this.roleRouting = const {},
    this.workspace = '',
  });

  final String name;
  final String description;
  final String profile;
  final List<String> enabledFeatures;
  final List<String> disabledFeatures;
  final Map<String, SceneRoleModel> roleRouting;
  final String workspace;

  factory Scene.fromJson(Map<String, dynamic> j) {
    final rr = <String, SceneRoleModel>{};
    final rawRr = j['role_routing'];
    if (rawRr is Map) {
      rawRr.forEach((key, value) {
        if (value is Map<String, dynamic>) {
          rr[key as String] = SceneRoleModel.fromJson(value);
        }
      });
    }
    List<String> strList(dynamic v) =>
        (v as List? ?? []).whereType<String>().toList();
    // features 可能在顶层 enabled_features 或嵌套 features.enabled。
    final features = j['features'];
    final enabled = strList(j['enabled_features']).isNotEmpty
        ? strList(j['enabled_features'])
        : (features is Map ? strList(features['enabled']) : <String>[]);
    final disabled = strList(j['disabled_features']).isNotEmpty
        ? strList(j['disabled_features'])
        : (features is Map ? strList(features['disabled']) : <String>[]);
    return Scene(
      name: j['name'] as String? ?? '',
      description: j['description'] as String? ?? '',
      profile: j['profile'] as String? ?? '',
      enabledFeatures: enabled,
      disabledFeatures: disabled,
      roleRouting: rr,
      workspace: j['workspace'] as String? ?? '',
    );
  }

  Map<String, dynamic> toCreateJson() => {
        'name': name,
        'description': description,
        'profile': profile,
        'enabled_features': enabledFeatures,
        'disabled_features': disabledFeatures,
        'role_routing': roleRouting.map((k, v) => MapEntry(k, v.toJson())),
        if (workspace.isNotEmpty) 'workspace': workspace,
      };
}
