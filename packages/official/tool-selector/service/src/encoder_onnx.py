"""ONNX-based encoder for fast inference (5ms vs 2000ms PyTorch)."""

import numpy as np
import onnxruntime as ort
from pathlib import Path
from transformers import AutoTokenizer

# GitHub Release URL for model assets (manual download).
MODEL_DOWNLOAD_URL = "https://github.com/ailiheizi/universal-selector/releases/tag/v0.1.0"
_REQUIRED_FILES = {
    "int8": ["model_int8.onnx", "tokenizer.json", "tokenizer_config.json"],
    "full": ["model.onnx", "model.onnx.data", "tokenizer.json", "tokenizer_config.json"],
}


def check_models_available(model_dir: str = None, use_int8: bool = True) -> dict:
    """Check if model files are present. Returns status dict for UI display.

    Returns:
        {
            "available": bool,
            "model_dir": str,
            "missing": [str],  # list of missing file names
            "download_url": str,
            "message": str,    # human-readable status
        }
    """
    if model_dir is None:
        model_dir = str(Path(__file__).resolve().parent.parent / "models")
    model_dir = Path(model_dir)
    variant = "int8" if use_int8 else "full"
    needed = _REQUIRED_FILES[variant]
    missing = [f for f in needed if not (model_dir / f).exists()]

    if not missing:
        return {
            "available": True,
            "model_dir": str(model_dir),
            "missing": [],
            "download_url": MODEL_DOWNLOAD_URL,
            "message": "语义选择引擎已就绪",
        }
    else:
        return {
            "available": False,
            "model_dir": str(model_dir),
            "missing": missing,
            "download_url": MODEL_DOWNLOAD_URL,
            "message": (
                f"语义选择引擎需要下载模型文件({', '.join(missing)})。\n"
                f"请从以下地址下载并放入 {model_dir}:\n"
                f"{MODEL_DOWNLOAD_URL}"
            ),
        }


class OnnxEncoder:
    """Fast ONNX encoder for universal selector. Drop-in replacement for SelectorEncoder."""

    def __init__(self, model_dir: str = None, use_int8: bool = True, max_length: int = 128):
        """Load ONNX model + tokenizer.

        Args:
            model_dir: Path to directory containing model.onnx/model_int8.onnx + tokenizer files.
                       Defaults to models/ under package root.
            use_int8: Use INT8 quantized model (faster, slightly less accurate).
            max_length: Max token length for input texts (64 for short queries, 128 for longer text).
        """
        self.max_length = max_length
        if model_dir is None:
            model_dir = str(Path(__file__).resolve().parent.parent / "models")

        model_dir = Path(model_dir)
        model_file = "model_int8.onnx" if use_int8 else "model.onnx"
        model_path = model_dir / model_file

        if not model_path.exists():
            status = check_models_available(str(model_dir), use_int8)
            raise FileNotFoundError(status["message"])

        # Load tokenizer
        self.tokenizer = AutoTokenizer.from_pretrained(str(model_dir))

        # Load ONNX session
        sess_options = ort.SessionOptions()
        sess_options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
        sess_options.intra_op_num_threads = 4

        self.session = ort.InferenceSession(
            str(model_path),
            sess_options,
            providers=["CPUExecutionProvider"],
        )

    def encode(self, text: str) -> np.ndarray:
        """Encode single text to 384d vector. ~5ms on CPU."""
        return self.encode_batch([text])[0]

    def encode_batch(self, texts: list[str]) -> np.ndarray:
        """Encode batch of texts to (N, 384) vectors."""
        # ONNX model was exported with fixed max_length=64 position embeddings.
        # Batch with different lengths causes dimension mismatch.
        # Encode one by one for robustness.
        results = []
        for text in texts:
            inputs = self.tokenizer(
                text, return_tensors="np", padding="max_length",
                truncation=True, max_length=min(self.max_length, 64)
            )

            outputs = self.session.run(None, {
                "input_ids": inputs["input_ids"].astype(np.int64),
                "attention_mask": inputs["attention_mask"].astype(np.int64),
            })

            # Mean pooling
            token_embeddings = outputs[0]  # (1, seq, 384)
            mask = inputs["attention_mask"][:, :, np.newaxis].astype(np.float32)
            pooled = (token_embeddings * mask).sum(axis=1) / mask.sum(axis=1).clip(min=1e-9)

            # L2 normalize
            norms = np.linalg.norm(pooled, axis=1, keepdims=True)
            results.append((pooled / norms)[0])

        return np.array(results, dtype=np.float32)
