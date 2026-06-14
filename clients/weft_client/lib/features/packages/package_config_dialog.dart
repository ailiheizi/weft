import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers/core_repository.dart';
import '../../shared/theme/spacing.dart';

class PackageConfigDialog extends ConsumerStatefulWidget {
  const PackageConfigDialog({super.key, required this.packageName});

  final String packageName;

  @override
  ConsumerState<PackageConfigDialog> createState() =>
      _PackageConfigDialogState();
}

class _PackageConfigDialogState extends ConsumerState<PackageConfigDialog> {
  static const String _secretMask = '****';

  final Map<String, TextEditingController> _textControllers = {};
  final Map<String, FocusNode> _numberFocusNodes = {};
  final Map<String, bool> _boolValues = {};
  final Map<String, String?> _selectValues = {};
  final Map<String, bool> _obscureStates = {};
  final Map<String, String?> _fieldErrors = {};
  final Set<String> _maskedSecrets = <String>{};

  List<MapEntry<String, Map<String, dynamic>>> _fields = const [];
  bool _loading = true;
  bool _saving = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    for (final controller in _textControllers.values) {
      controller.dispose();
    }
    for (final focusNode in _numberFocusNodes.values) {
      focusNode.dispose();
    }
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final repository = ref.read(coreRepositoryProvider);
      final results = await Future.wait<dynamic>([
        repository.getPackageConfigSchema(widget.packageName),
        repository.getPackageConfig(widget.packageName),
      ]);

      final schema = Map<String, dynamic>.from(results[0] as Map);
      final configResponse = Map<String, dynamic>.from(results[1] as Map);
      final rawConfig = configResponse['config'];
      final config = rawConfig is Map<String, dynamic>
          ? rawConfig
          : Map<String, dynamic>.from(rawConfig as Map? ?? configResponse);

      _initializeFields(schema, config);

      if (!mounted) {
        return;
      }
      setState(() {
        _loading = false;
      });
    } catch (e) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loading = false;
        _error = 'Failed to load package configuration: $e';
      });
    }
  }

  void _initializeFields(
    Map<String, dynamic> schema,
    Map<String, dynamic> config,
  ) {
    for (final controller in _textControllers.values) {
      controller.dispose();
    }
    for (final focusNode in _numberFocusNodes.values) {
      focusNode.dispose();
    }

    _textControllers.clear();
    _numberFocusNodes.clear();
    _boolValues.clear();
    _selectValues.clear();
    _obscureStates.clear();
    _fieldErrors.clear();
    _maskedSecrets.clear();

    final sortedEntries = schema.entries
        .map((entry) => MapEntry(
              entry.key,
              entry.value is Map<String, dynamic>
                  ? entry.value as Map<String, dynamic>
                  : Map<String, dynamic>.from(entry.value as Map? ?? const {}),
            ))
        .toList()
      ..sort((a, b) => a.key.compareTo(b.key));

    _fields = sortedEntries;

    for (final entry in sortedEntries) {
      final fieldName = entry.key;
      final field = entry.value;
      final type = (field['type'] as String?) ?? 'string';
      final secret = field['secret'] == true;
      final value =
          config.containsKey(fieldName) ? config[fieldName] : field['default'];

      switch (type) {
        case 'boolean':
          _boolValues[fieldName] = value == true;
          break;
        case 'select':
          final options = _stringOptions(field);
          final initialValue = value?.toString();
          _selectValues[fieldName] = options.contains(initialValue)
              ? initialValue
              : (initialValue?.isNotEmpty == true ? initialValue : null);
          break;
        case 'number':
          _textControllers[fieldName] =
              TextEditingController(text: _stringifyValue(value));
          final focusNode = FocusNode();
          focusNode.addListener(() {
            if (!focusNode.hasFocus) {
              _validateField(fieldName, field, updateState: true);
            }
          });
          _numberFocusNodes[fieldName] = focusNode;
          break;
        case 'string':
        default:
          if (secret && value == _secretMask) {
            _maskedSecrets.add(fieldName);
            _textControllers[fieldName] = TextEditingController();
          } else {
            _textControllers[fieldName] =
                TextEditingController(text: _stringifyValue(value));
          }
          _obscureStates[fieldName] = secret;
          break;
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text('Configure ${widget.packageName}'),
      content: SizedBox(
        width: 480,
        child: _buildContent(context),
      ),
      actions: [
        TextButton(
          onPressed: _saving ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: (_loading || _saving || _error != null) ? null : _save,
          child: _saving
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Save'),
        ),
      ],
    );
  }

  Widget _buildContent(BuildContext context) {
    final theme = Theme.of(context);

    if (_loading) {
      return const Padding(
        padding: EdgeInsets.symmetric(vertical: Spacing.lg),
        child: Center(
          child: SizedBox(
            width: 24,
            height: 24,
            child: CircularProgressIndicator(strokeWidth: 2),
          ),
        ),
      );
    }

    if (_error != null) {
      return Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(_error!, style: theme.textTheme.bodyMedium),
          const SizedBox(height: Spacing.md),
          OutlinedButton.icon(
            onPressed: _saving ? null : _load,
            icon: const Icon(Icons.refresh, size: 16),
            label: const Text('Retry'),
          ),
        ],
      );
    }

    if (_fields.isEmpty) {
      return Text(
        'This package does not expose configurable fields.',
        style: theme.textTheme.bodyMedium,
      );
    }

    return SingleChildScrollView(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (var i = 0; i < _fields.length; i++) ...[
            _buildField(_fields[i].key, _fields[i].value),
            if (i != _fields.length - 1) const SizedBox(height: Spacing.md),
          ],
        ],
      ),
    );
  }

  Widget _buildField(String fieldName, Map<String, dynamic> field) {
    final type = (field['type'] as String?) ?? 'string';
    final label = _buildLabel(fieldName, field['required'] == true);
    final description = field['description']?.toString();
    final errorText = _fieldErrors[fieldName];

    switch (type) {
      case 'boolean':
        return SwitchListTile(
          contentPadding: EdgeInsets.zero,
          title: Text(label),
          subtitle: description == null ? null : Text(description),
          value: _boolValues[fieldName] ?? false,
          onChanged: _saving
              ? null
              : (value) {
                  setState(() {
                    _boolValues[fieldName] = value;
                  });
                },
        );
      case 'select':
        final options = _stringOptions(field);
        final currentValue = _selectValues[fieldName];
        return DropdownButtonFormField<String>(
          initialValue: options.contains(currentValue) ? currentValue : null,
          decoration: InputDecoration(
            labelText: label,
            helperText: description,
            errorText: errorText,
          ),
          items: options
              .map(
                (option) => DropdownMenuItem<String>(
                  value: option,
                  child: Text(option),
                ),
              )
              .toList(),
          onChanged: _saving
              ? null
              : (value) {
                  setState(() {
                    _selectValues[fieldName] = value;
                    _fieldErrors.remove(fieldName);
                  });
                },
        );
      case 'number':
        return TextField(
          controller: _textControllers[fieldName],
          focusNode: _numberFocusNodes[fieldName],
          enabled: !_saving,
          keyboardType: const TextInputType.numberWithOptions(
            decimal: true,
            signed: true,
          ),
          inputFormatters: [
            FilteringTextInputFormatter.allow(RegExp(r'[0-9+\-\.]')),
          ],
          decoration: InputDecoration(
            labelText: label,
            helperText: _buildNumberHelperText(field, description),
            errorText: errorText,
          ),
          onChanged: (_) {
            if (_fieldErrors.containsKey(fieldName)) {
              setState(() => _fieldErrors.remove(fieldName));
            }
          },
        );
      case 'string':
      default:
        final secret = field['secret'] == true;
        final maskedSecret = _maskedSecrets.contains(fieldName);
        return TextField(
          controller: _textControllers[fieldName],
          enabled: !_saving,
          obscureText: _obscureStates[fieldName] ?? false,
          decoration: InputDecoration(
            labelText: label,
            helperText: secret && maskedSecret
                ? _mergeHelperText(description, '已设置，留空保持不变')
                : description,
            hintText: secret && maskedSecret ? '已设置，留空保持不变' : null,
            errorText: errorText,
            suffixIcon: secret
                ? IconButton(
                    icon: Icon(
                      (_obscureStates[fieldName] ?? false)
                          ? Icons.visibility_outlined
                          : Icons.visibility_off_outlined,
                      size: 16,
                    ),
                    onPressed: _saving
                        ? null
                        : () {
                            setState(() {
                              _obscureStates[fieldName] =
                                  !(_obscureStates[fieldName] ?? true);
                            });
                          },
                    visualDensity: VisualDensity.compact,
                  )
                : null,
          ),
          onChanged: (_) {
            if (_fieldErrors.containsKey(fieldName)) {
              setState(() => _fieldErrors.remove(fieldName));
            }
          },
        );
    }
  }

  Future<void> _save() async {
    final nextErrors = <String, String?>{};
    for (final entry in _fields) {
      final error = _validateField(entry.key, entry.value);
      if (error != null) {
        nextErrors[entry.key] = error;
      }
    }

    if (nextErrors.isNotEmpty) {
      setState(() {
        _fieldErrors
          ..clear()
          ..addAll(nextErrors);
      });
      return;
    }

    final payload = <String, dynamic>{};
    for (final entry in _fields) {
      final fieldName = entry.key;
      final field = entry.value;
      final type = (field['type'] as String?) ?? 'string';

      switch (type) {
        case 'boolean':
          payload[fieldName] = _boolValues[fieldName] ?? false;
          break;
        case 'select':
          final value = _selectValues[fieldName];
          if (value != null && value.isNotEmpty) {
            payload[fieldName] = value;
          }
          break;
        case 'number':
          final text = _textControllers[fieldName]!.text.trim();
          if (text.isNotEmpty) {
            payload[fieldName] = num.parse(text);
          }
          break;
        case 'string':
        default:
          final text = _textControllers[fieldName]!.text.trim();
          if (field['secret'] == true &&
              _maskedSecrets.contains(fieldName) &&
              text.isEmpty) {
            continue;
          }
          if (text.isNotEmpty) {
            payload[fieldName] = text;
          }
          break;
      }
    }

    setState(() => _saving = true);

    try {
      await ref
          .read(coreRepositoryProvider)
          .savePackageConfig(widget.packageName, payload);
      if (!mounted) {
        return;
      }
      Navigator.pop(context);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Saved configuration for ${widget.packageName}')),
      );
    } catch (e) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Failed to save configuration: $e')),
      );
      setState(() => _saving = false);
    }
  }

  String? _validateField(
    String fieldName,
    Map<String, dynamic> field, {
    bool updateState = false,
  }) {
    final type = (field['type'] as String?) ?? 'string';
    final required = field['required'] == true;

    String? error;
    switch (type) {
      case 'boolean':
        error = null;
        break;
      case 'select':
        final value = _selectValues[fieldName];
        if (required && (value == null || value.isEmpty)) {
          error = 'This field is required';
        }
        break;
      case 'number':
        final text = _textControllers[fieldName]!.text.trim();
        if (text.isEmpty) {
          if (required) {
            error = 'This field is required';
          }
          break;
        }
        final parsed = num.tryParse(text);
        if (parsed == null) {
          error = 'Enter a valid number';
          break;
        }
        final min = _toNum(field['min']);
        final max = _toNum(field['max']);
        if (min != null && parsed < min) {
          error = 'Must be at least $min';
        } else if (max != null && parsed > max) {
          error = 'Must be at most $max';
        }
        break;
      case 'string':
      default:
        final text = _textControllers[fieldName]!.text.trim();
        final keepMaskedSecret =
            field['secret'] == true && _maskedSecrets.contains(fieldName) && text.isEmpty;
        if (required && text.isEmpty && !keepMaskedSecret) {
          error = 'This field is required';
        }
        break;
    }

    if (updateState && mounted) {
      setState(() {
        if (error == null) {
          _fieldErrors.remove(fieldName);
        } else {
          _fieldErrors[fieldName] = error;
        }
      });
    }

    return error;
  }

  List<String> _stringOptions(Map<String, dynamic> field) {
    final options = field['options'];
    if (options is! List) {
      return const [];
    }
    return options.map((option) => option.toString()).toList();
  }

  String _buildLabel(String fieldName, bool required) {
    return required ? '$fieldName *' : fieldName;
  }

  String? _buildNumberHelperText(
    Map<String, dynamic> field,
    String? description,
  ) {
    final min = _toNum(field['min']);
    final max = _toNum(field['max']);
    final rangeText = switch ((min, max)) {
      (final min?, final max?) => 'Range: $min to $max',
      (final min?, null) => 'Min: $min',
      (null, final max?) => 'Max: $max',
      _ => null,
    };
    return _mergeHelperText(description, rangeText);
  }

  String? _mergeHelperText(String? first, String? second) {
    final values = [first, second]
        .whereType<String>()
        .where((value) => value.isNotEmpty)
        .toList();
    if (values.isEmpty) {
      return null;
    }
    return values.join('\n');
  }

  String _stringifyValue(dynamic value) {
    return value?.toString() ?? '';
  }

  num? _toNum(dynamic value) {
    if (value is num) {
      return value;
    }
    if (value is String) {
      return num.tryParse(value);
    }
    return null;
  }
}
