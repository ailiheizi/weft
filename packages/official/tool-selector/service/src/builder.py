"""Build embeddings for a library from descriptions.jsonl."""

import json
import sys
from pathlib import Path

import numpy as np

from .encoder import SelectorEncoder


def build_library(library_path: str, model_name: str = "paraphrase-multilingual-MiniLM-L12-v2"):
    """Load descriptions.jsonl, encode descriptions, save as embeddings.npy."""
    lib_path = Path(library_path)
    descriptions_file = lib_path / "descriptions.jsonl"
    embeddings_file = lib_path / "embeddings.npy"

    # Load descriptions
    descriptions = []
    with open(descriptions_file, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                item = json.loads(line)
                descriptions.append(item["description"])

    print(f"Loaded {len(descriptions)} descriptions from {descriptions_file}")

    # Encode
    encoder = SelectorEncoder(model_name)
    embeddings = encoder.encode_batch(descriptions)

    # Save
    np.save(str(embeddings_file), embeddings)
    print(f"Saved embeddings ({embeddings.shape}) to {embeddings_file}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python -m src.builder <library_path>")
        sys.exit(1)
    build_library(sys.argv[1])
