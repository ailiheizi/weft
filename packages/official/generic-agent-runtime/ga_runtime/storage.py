from __future__ import annotations

import json
from pathlib import Path
from typing import Any


class RuntimeStore:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.root.mkdir(parents=True, exist_ok=True)
        self.state_path = self.root / "runtime-state.json"
        self.skill_dir = self.root / "skill-drafts"
        self.skill_dir.mkdir(parents=True, exist_ok=True)

    def load_state(self) -> dict[str, Any]:
        if not self.state_path.exists():
            return {
                "runs": [],
                "plans": [],
                "verifications": [],
                "crystallized_skills": [],
            }
        try:
            return json.loads(self.state_path.read_text("utf-8"))
        except Exception:
            return {
                "runs": [],
                "plans": [],
                "verifications": [],
                "crystallized_skills": [],
            }

    def save_state(self, state: dict[str, Any]) -> None:
        self.state_path.write_text(json.dumps(state, ensure_ascii=False, indent=2), encoding="utf-8")

    def append_state_entry(self, key: str, value: dict[str, Any]) -> dict[str, Any]:
        state = self.load_state()
        state.setdefault(key, []).append(value)
        self.save_state(state)
        return state

    def write_skill_draft(self, slug: str, content: str) -> Path:
        path = self.skill_dir / f"{slug}.md"
        path.write_text(content, encoding="utf-8")
        return path
