"""Universal selector using cosine similarity matching."""

import json
from pathlib import Path

import numpy as np

# SelectorEncoder is optional — only needed for PyTorch training/building.
# OnnxEncoder (used in production) also satisfies the encoder interface.
try:
    from .encoder import SelectorEncoder
except (ImportError, ModuleNotFoundError):
    SelectorEncoder = None


class UniversalSelector:
    """Selects the best matching items from a library based on semantic similarity."""

    def __init__(self, encoder: SelectorEncoder):
        self.encoder = encoder
        self.libraries: dict[str, dict] = {}

    def load_library(self, name: str, path: str):
        """Load a library from descriptions.jsonl + embeddings.npy."""
        lib_path = Path(path)
        descriptions_file = lib_path / "descriptions.jsonl"
        embeddings_file = lib_path / "embeddings.npy"

        items = []
        with open(descriptions_file, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    items.append(json.loads(line))

        embeddings = np.load(str(embeddings_file))

        self.libraries[name] = {
            "items": items,
            "embeddings": embeddings,
        }

    def select(self, query: str, library: str, top_k: int = 1) -> list[dict]:
        """Select top-k matching items from a library for a given query."""
        if library not in self.libraries:
            raise ValueError(f"Library '{library}' not loaded.")

        lib = self.libraries[library]
        query_vec = self.encoder.encode(query)

        # Cosine similarity
        embeddings = lib["embeddings"]
        query_norm = query_vec / (np.linalg.norm(query_vec) + 1e-10)
        emb_norms = embeddings / (np.linalg.norm(embeddings, axis=1, keepdims=True) + 1e-10)
        scores = emb_norms @ query_norm

        # Get top-k indices
        top_indices = np.argsort(scores)[::-1][:top_k]

        results = []
        for idx in top_indices:
            item = lib["items"][idx].copy()
            item["score"] = float(scores[idx])
            results.append(item)

        return results

    def select_multi(self, queries: dict[str, str], top_k: int = 1) -> dict:
        """Select from multiple libraries. queries = {library_name: query_text}."""
        results = {}
        for library, query in queries.items():
            results[library] = self.select(query, library, top_k)
        return results
