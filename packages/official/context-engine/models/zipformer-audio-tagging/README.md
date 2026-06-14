Bundled Zipformer audio tagging assets for `context-engine`.

Contents:
- `model.int8.onnx`
- `class_labels_indices.csv`
- `README.upstream.md`

Source:
- Upstream project: `https://github.com/k2-fsa/icefall`
- Model family: sherpa-onnx zipformer audio tagging

Packaging rule:
- Keep the default packaged asset set minimal.
- The int8 model is the default runtime model.
- Do not depend on `tmp/zipformer-audio-tagging` for normal package startup.
