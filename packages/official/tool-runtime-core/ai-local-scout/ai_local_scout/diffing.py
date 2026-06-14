from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


PROFILE_KEYS = [
    "active_workspaces",
    "tools",
    "game_ecosystem",
    "installed_apps",
    "browser_activity",
    "local_signal_coverage",
    "llm_context",
]


def _sorted_strings(values: Any) -> list[str]:
    if not isinstance(values, list):
        return []
    return sorted({str(value) for value in values if str(value)})


def _set_diff(old_values: Any, new_values: Any) -> dict[str, list[str]]:
    old_set = set(_sorted_strings(old_values))
    new_set = set(_sorted_strings(new_values))
    return {
        "added": sorted(new_set - old_set),
        "removed": sorted(old_set - new_set),
    }


def _profile(report: dict, key: str) -> Any:
    derived = report.get("derived_profile")
    if not isinstance(derived, dict):
        return {}
    return derived.get(key, {})


def _run_id(report: dict) -> str | None:
    run_meta = report.get("run_meta")
    if not isinstance(run_meta, dict):
        return None
    run_id = run_meta.get("run_id")
    return str(run_id) if run_id is not None else None


def _workspace_paths(report: dict) -> list[str]:
    active = _profile(report, "active_workspaces")
    if not isinstance(active, dict):
        return []
    return _sorted_strings(active.get("workspace_paths"))


def _game_names(report: dict) -> list[str]:
    game_profile = _profile(report, "game_ecosystem")
    if not isinstance(game_profile, dict):
        return []
    games = game_profile.get("installed_games")
    if not isinstance(games, list):
        return []
    names = []
    for game in games:
        if isinstance(game, dict) and game.get("name"):
            names.append(str(game["name"]))
    return sorted(set(names))


def _game_platforms(report: dict) -> list[str]:
    game_profile = _profile(report, "game_ecosystem")
    if not isinstance(game_profile, dict):
        return []
    return _sorted_strings(game_profile.get("platforms"))


def _present_tools(report: dict) -> list[str]:
    tools = _profile(report, "tools")
    if not isinstance(tools, dict):
        return []
    present = []
    for name, value in tools.items():
        if isinstance(value, dict) and value.get("present") is True:
            present.append(str(name))
    return sorted(set(present))


def _present_signals(report: dict) -> list[str]:
    coverage = _profile(report, "local_signal_coverage")
    if not isinstance(coverage, dict):
        return []
    categories = coverage.get("categories")
    if not isinstance(categories, dict):
        return []
    present = []
    for name, value in categories.items():
        if isinstance(value, dict) and value.get("status") == "present":
            present.append(str(name))
    return sorted(set(present))


def _llm_context_list(report: dict, key: str) -> list[str]:
    context = _profile(report, "llm_context")
    if not isinstance(context, dict):
        return []
    return _sorted_strings(context.get(key))


def _changed_profiles(old: dict, new: dict) -> list[str]:
    changed = []
    for key in PROFILE_KEYS:
        if _profile(old, key) != _profile(new, key):
            changed.append(key)
    return changed


def _add_messages(prefix: str, values: list[str]) -> list[str]:
    return [f"{prefix}: {value}" for value in values]


def _meaningful_changes(old: dict, new: dict) -> list[str]:
    changes: list[str] = []
    changes.extend(_add_messages("New workspace detected", _set_diff(_workspace_paths(old), _workspace_paths(new))["added"]))
    changes.extend(_add_messages("Workspace no longer detected", _set_diff(_workspace_paths(old), _workspace_paths(new))["removed"]))
    changes.extend(_add_messages("New game detected", _set_diff(_game_names(old), _game_names(new))["added"]))
    changes.extend(_add_messages("Game no longer detected", _set_diff(_game_names(old), _game_names(new))["removed"]))
    changes.extend(_add_messages("New game platform detected", _set_diff(_game_platforms(old), _game_platforms(new))["added"]))
    changes.extend(_add_messages("Game platform no longer detected", _set_diff(_game_platforms(old), _game_platforms(new))["removed"]))
    changes.extend(_add_messages("Tool became present", _set_diff(_present_tools(old), _present_tools(new))["added"]))
    changes.extend(_add_messages("Tool no longer present", _set_diff(_present_tools(old), _present_tools(new))["removed"]))
    changes.extend(_add_messages("Signal became present", _set_diff(_present_signals(old), _present_signals(new))["added"]))
    changes.extend(_add_messages("Signal no longer present", _set_diff(_present_signals(old), _present_signals(new))["removed"]))
    changes.extend(_add_messages("New strong fact", _set_diff(_llm_context_list(old, "strong_facts"), _llm_context_list(new, "strong_facts"))["added"]))
    changes.extend(_add_messages("New weak hint", _set_diff(_llm_context_list(old, "weak_hints"), _llm_context_list(new, "weak_hints"))["added"]))
    return changes


def diff_reports(old: dict, new: dict) -> dict:
    path_changes = _set_diff(
        old.get("entities", {}).get("paths") if isinstance(old.get("entities"), dict) else [],
        new.get("entities", {}).get("paths") if isinstance(new.get("entities"), dict) else [],
    )
    entity_kind_changes = _set_diff(
        old.get("entities", {}).get("entity_kinds") if isinstance(old.get("entities"), dict) else [],
        new.get("entities", {}).get("entity_kinds") if isinstance(new.get("entities"), dict) else [],
    )
    changed_profiles = _changed_profiles(old, new)

    return {
        "old_run_id": _run_id(old),
        "new_run_id": _run_id(new),
        "summary": {
            "added_paths": len(path_changes["added"]),
            "removed_paths": len(path_changes["removed"]),
            "changed_profiles": changed_profiles,
            "meaningful_changes": _meaningful_changes(old, new),
        },
        "changes": {
            "paths": path_changes,
            "entity_kinds": entity_kind_changes,
            "profiles": {
                key: {
                    "changed": key in changed_profiles,
                    "before": _profile(old, key),
                    "after": _profile(new, key),
                }
                for key in PROFILE_KEYS
                if key in changed_profiles
            },
        },
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Diff two AI Local Scout JSON reports")
    parser.add_argument("old_report")
    parser.add_argument("new_report")
    parser.add_argument("--output", required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    old = json.loads(Path(args.old_report).read_text(encoding="utf-8"))
    new = json.loads(Path(args.new_report).read_text(encoding="utf-8"))
    diff = diff_reports(old, new)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(diff, indent=2, ensure_ascii=True), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
