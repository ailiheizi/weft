from __future__ import annotations

from typing import Any


def ok(data: Any = None) -> dict[str, Any]:
    return {"status": "ok", "data": data}


def err(message: str) -> dict[str, Any]:
    return {"status": "error", "error": str(message)}
