from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


def _report(*, run_id: str, paths: list[str], derived_profile: dict) -> dict:
    return {
        "run_meta": {"run_id": run_id},
        "entities": {
            "paths": paths,
            "entity_kinds": ["claude_entrypoint", "steam_game_manifest"],
        },
        "derived_profile": derived_profile,
    }


def test_diff_reports_extracts_meaningful_changes() -> None:
    from ai_local_scout.diffing import diff_reports

    old = _report(
        run_id="old-run",
        paths=["C:/Users/Admin/.claude/CLAUDE.md"],
        derived_profile={
            "active_workspaces": {"workspace_paths": ["D:/old"]},
            "game_ecosystem": {"installed_games": [{"name": "Dota 2"}], "platforms": ["steam"]},
            "tools": {"claude": {"present": True}, "codex": {"present": False}},
            "local_signal_coverage": {"categories": {"knowledge": {"status": "absent"}}},
            "llm_context": {"strong_facts": ["Claude context exists"], "weak_hints": []},
        },
    )
    new = _report(
        run_id="new-run",
        paths=["C:/Users/Admin/.claude/CLAUDE.md", "D:/weft/AGENTS.md"],
        derived_profile={
            "active_workspaces": {"workspace_paths": ["D:/old", "D:/weft"]},
            "game_ecosystem": {"installed_games": [{"name": "Dota 2"}, {"name": "Helldivers 2"}], "platforms": ["steam", "epic"]},
            "tools": {"claude": {"present": True}, "codex": {"present": True}},
            "local_signal_coverage": {"categories": {"knowledge": {"status": "present"}}},
            "llm_context": {"strong_facts": ["Claude context exists", "Codex config exists"], "weak_hints": ["Gaming setup detected"]},
        },
    )

    diff = diff_reports(old, new)

    assert diff["old_run_id"] == "old-run"
    assert diff["new_run_id"] == "new-run"
    assert diff["summary"]["added_paths"] == 1
    assert diff["summary"]["removed_paths"] == 0
    assert "active_workspaces" in diff["summary"]["changed_profiles"]
    assert "tools" in diff["summary"]["changed_profiles"]
    assert "New workspace detected: D:/weft" in diff["summary"]["meaningful_changes"]
    assert "New game detected: Helldivers 2" in diff["summary"]["meaningful_changes"]
    assert "New game platform detected: epic" in diff["summary"]["meaningful_changes"]
    assert "Tool became present: codex" in diff["summary"]["meaningful_changes"]
    assert "Signal became present: knowledge" in diff["summary"]["meaningful_changes"]
    assert "New strong fact: Codex config exists" in diff["summary"]["meaningful_changes"]
    assert diff["changes"]["paths"]["added"] == ["D:/weft/AGENTS.md"]


def test_diff_cli_writes_json_output(tmp_path: Path) -> None:
    old_path = tmp_path / "old.json"
    new_path = tmp_path / "new.json"
    output_path = tmp_path / "diff.json"
    old_path.write_text(json.dumps(_report(run_id="old", paths=[], derived_profile={})), encoding="utf-8")
    new_path.write_text(json.dumps(_report(run_id="new", paths=["D:/weft"], derived_profile={})), encoding="utf-8")

    result = subprocess.run(
        [sys.executable, "-m", "ai_local_scout.diffing", str(old_path), str(new_path), "--output", str(output_path)],
        cwd=Path(__file__).resolve().parents[1],
        text=True,
        capture_output=True,
    )

    assert result.returncode == 0, result.stderr
    payload = json.loads(output_path.read_text(encoding="utf-8"))
    assert payload["summary"]["added_paths"] == 1
    assert payload["changes"]["paths"]["added"] == ["D:/weft"]
