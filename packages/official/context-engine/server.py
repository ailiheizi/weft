#!/usr/bin/env python3
from __future__ import annotations

import json
import base64
import logging
import os
import re
import shlex
import subprocess
import tempfile
import threading
import time
import sys
import urllib.parse
import urllib.request
import uuid
from datetime import datetime, timezone
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

try:
    import ctypes
    import ctypes.wintypes
except Exception:  # pragma: no cover - platform import guard
    ctypes = None

try:
    import psutil
except Exception:  # pragma: no cover - optional runtime dependency
    psutil = None


LOGGER = logging.getLogger("context-engine-runtime")
logging.basicConfig(
    level=os.environ.get("WEFT_CONTEXT_ENGINE_LOG_LEVEL", "INFO").upper(),
    format="[context-engine-runtime] %(message)s",
)

PORT = int(os.environ.get("WEFT_CONTEXT_ENGINE_PORT", "43131"))
HTTPD: ThreadingHTTPServer | None = None
SEEN_EVENT_SIGNATURES: set[str] = set()
PENDING_EVENTS: list[dict[str, Any]] = []
PENDING_EVENTS_LOCK = threading.Lock()
POLL_INTERVAL_SECONDS = max(
    1,
    int(os.environ.get("WEFT_CONTEXT_ENGINE_POLL_INTERVAL_SECONDS", "1")),
)
POLL_STOP_EVENT = threading.Event()
POLL_THREAD: threading.Thread | None = None
CONTENT_MODE_VISUAL_THREAD: threading.Thread | None = None
AUDIO_SIDECAR_PROCESS: subprocess.Popen[str] | None = None
AUDIO_SIDECAR_LOCK = threading.Lock()
ACTIVITY_STATE_LOCK = threading.Lock()
ACTIVITY_STATE_DIR = ""
ACTIVITY_WINDOWS: list[dict[str, Any]] = []
ACTIVITY_CURRENT_WINDOW: dict[str, Any] | None = None
ACTIVITY_RECENT_EVENTS: list[dict[str, Any]] = []
LAST_NON_HOST_PC_STATUS: dict[str, Any] | None = None
LAST_CLIPBOARD_TEXT = ""
START_MONOTONIC = time.monotonic()
CONTENT_MODE_VISUAL_LOOP_WAIT_SECONDS = 1.0
CONTENT_MODE_TTL_MS = 90_000

ACTIVITY_WINDOW_SECONDS = 10
ACTIVITY_MAX_WINDOWS = 12
ACTIVITY_MAX_RECENT_EVENTS = 24
TERMINAL_CONTEXT_REFRESH_SECONDS = max(
    30,
    int(os.environ.get("WEFT_CONTEXT_ENGINE_TERMINAL_REFRESH_SECONDS", "120")),
)

PROACTIVE_MANIFEST_FILES = [
    "bilibili-companion.json",
    "browser-reading-companion.json",
]


def content_mode_enabled() -> bool:
    return str(os.environ.get("WEFT_COMPANION_CONTENT_MODE_ENABLED", "1")).strip().lower() in {"1", "true", "yes", "on"}

OFFICE_PROCESS_NAMES = {
    "winword.exe",
    "excel.exe",
    "powerpnt.exe",
    "wps.exe",
    "wpp.exe",
    "et.exe",
}

VIDEO_PROCESS_NAMES = {
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "brave.exe",
}

BROWSER_CONTEXT_EVENT_TYPES = {
    "active_url_changed",
    "reading_page_detected",
}

FOCUS_AUTHORITY_EVENT_TYPES = BROWSER_CONTEXT_EVENT_TYPES | {
    "game_context_detected",
    "video_context_detected",
}

MEETING_PROCESS_NAMES = {
    "zoom.exe",
    "teams.exe",
    "ms-teams.exe",
    "wemeetapp.exe",
    "lark.exe",
    "feishu.exe",
    "dingtalk.exe",
}

TERMINAL_PROCESS_NAMES = {
    "windowsterminal.exe",
    "pwsh.exe",
    "powershell.exe",
    "cmd.exe",
    "wt.exe",
}

GAME_PROCESS_HINTS = (
    "steam",
    "epicgames",
    "battle.net",
    "origin",
    "gog",
    "uplay",
    "leagueclient",
    "valorant",
    "genshin",
    "yuanshen",
    "minecraft",
    "pubg",
    "apex",
    "fortnite",
    "dota2",
    "overwatch",
    "elden",
    "cyberpunk",
    "gtav",
    "gta5",
    "marvelrivals",
    "wukong",
    "deadlock",
)

GAME_PROCESS_MAP = {
    "leagueclient.exe": "League of Legends",
    "league of legends.exe": "League of Legends",
    "valorant.exe": "Valorant",
    "valorant-win64-shipping.exe": "Valorant",
    "csgo.exe": "CS:GO",
    "cs2.exe": "CS2",
    "yuanshen.exe": "Genshin Impact",
    "genshinimpact.exe": "Genshin Impact",
    "overwatch.exe": "Overwatch",
    "overwatch2.exe": "Overwatch 2",
    "minecraft.exe": "Minecraft",
    "javaw.exe": "Minecraft",
    "pubg.exe": "PUBG",
    "tslgame.exe": "PUBG",
    "apex legends.exe": "Apex Legends",
    "r5apex.exe": "Apex Legends",
    "fortnite.exe": "Fortnite",
    "dota2.exe": "Dota2",
    "hearthstone.exe": "Hearthstone",
    "wow.exe": "World of Warcraft",
    "starcraft ii.exe": "StarCraft II",
    "sc2.exe": "StarCraft II",
    "diablo iv.exe": "Diablo IV",
    "d4.exe": "Diablo IV",
    "elden ring.exe": "Elden Ring",
    "cyberpunk2077.exe": "Cyberpunk 2077",
    "rocketleague.exe": "Rocket League",
    "gtav.exe": "GTA5",
    "gta5.exe": "GTA5",
    "witcher3.exe": "The Witcher 3",
    "sekiro.exe": "Sekiro",
    "darksouls3.exe": "Dark Souls III",
    "hollowknight.exe": "Hollow Knight",
    "celeste.exe": "Celeste",
    "stardewvalley.exe": "Stardew Valley",
    "terraria.exe": "Terraria",
    "among us.exe": "Among Us",
    "hades.exe": "Hades",
    "splitgate.exe": "Splitgate",
    "battlebit.exe": "BattleBit",
    "thefinalsrelease-win64-shipping.exe": "The Finals",
    "deadlock.exe": "Deadlock",
    "marvelrivals-win64-shipping.exe": "Marvel Rivals",
    "wukong.exe": "Black Myth: Wukong",
    "b1-win64-shipping.exe": "Black Myth: Wukong",
}

HOST_FOREGROUND_PROCESS_NAMES = {
    "electron.exe",
    "weft.exe",
    "weft-desktop.exe",
}

PS_HISTORY_FILE = Path.home() / "AppData" / "Roaming" / "Microsoft" / "Windows" / "PowerShell" / "PSReadLine" / "ConsoleHost_history.txt"
CLAUDE_PROJECTS_DIR = Path.home() / ".claude" / "projects"
CODEX_SESSIONS_DIR = Path.home() / ".codex" / "sessions"
MEANINGFUL_COMMAND_RE = re.compile(r"(git|npm|pnpm|yarn|cargo|python|pip|uv|node|codex|claude|cursor|pytest|vitest|tsx|vite|build|dev|test)", re.IGNORECASE)
CD_COMMAND_RE = re.compile(r"""^(?:Set-Location|sl|cd|pushd)\s+(?:"([^"]+)"|'([^']+)'|([A-Za-z]:[\\/][^;]+?))\s*(?:;|$)""", re.IGNORECASE)


def read_text_file(file_path: Path) -> str:
    try:
        return file_path.read_text("utf-8")
    except Exception:
        return ""


def dedupe_keep_order(items: list[str], limit: int) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for item in items:
        text = str(item or "").strip()
        if not text or text in seen:
            continue
        seen.add(text)
        result.append(text)
    return result[-limit:]


def read_powershell_history(limit: int = 50) -> list[str]:
    if not PS_HISTORY_FILE.exists() or not PS_HISTORY_FILE.is_file():
        return []
    try:
        lines = [line.strip() for line in PS_HISTORY_FILE.read_text("utf-8").splitlines()]
    except Exception:
        return []
    return [line for line in lines if line][-max(1, limit):]


def is_usable_project_path(value: str) -> bool:
    normalized = str(value or "").strip()
    if not normalized:
        return False
    try:
        path = Path(normalized)
        return path.exists() and path.is_dir()
    except Exception:
        return False


def find_git_root(start_path: str) -> str:
    normalized = str(start_path or "").strip()
    if not normalized:
        return ""
    try:
        current = Path(normalized).resolve()
    except Exception:
        return ""
    root = ""
    for _ in range(8):
        if (current / ".git").exists():
            root = str(current)
        if current.parent == current:
            break
        current = current.parent
    return root


def run_git_sync(args: list[str], cwd: str) -> str:
    if not cwd.strip():
        return ""
    try:
        result = subprocess.run(
            ["git", *args],
            cwd=cwd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=3,
            check=False,
        )
    except Exception:
        return ""
    if result.returncode != 0:
        return ""
    return result.stdout.strip()


def normalize_text_content(content: Any) -> str:
    if isinstance(content, str):
        return " ".join(content.split()).strip()
    if not isinstance(content, list):
        return ""
    parts: list[str] = []
    for item in content:
        if not isinstance(item, dict):
            continue
        if item.get("type") == "text" and isinstance(item.get("text"), str):
            parts.append(str(item["text"]))
        if item.get("type") == "input_text" and isinstance(item.get("text"), str):
            parts.append(str(item["text"]))
    return " ".join(" ".join(parts).split()).strip()


def newest_jsonl_file(root: Path, file_pattern: str) -> Path | None:
    if not root.exists():
        return None
    newest_path: Path | None = None
    newest_mtime = 0.0
    for entry in root.rglob(file_pattern):
        if not entry.is_file():
            continue
        try:
            mtime = entry.stat().st_mtime
        except Exception:
            continue
        if mtime > newest_mtime:
            newest_mtime = mtime
            newest_path = entry
    return newest_path


def read_jsonl_entries(file_path: Path, max_lines: int = 400) -> list[dict[str, Any]]:
    if not file_path.exists() or not file_path.is_file():
        return []
    try:
        lines = [line for line in file_path.read_text("utf-8").splitlines() if line.strip()]
    except Exception:
        return []
    entries: list[dict[str, Any]] = []
    for line in lines[-max(1, max_lines):]:
        try:
            parsed = json.loads(line)
        except Exception:
            continue
        if isinstance(parsed, dict):
            entries.append(parsed)
    return entries


def collect_codex_file_paths(function_call: dict[str, Any], cwd: str) -> list[str]:
    raw_arguments = str(function_call.get("arguments") or function_call.get("raw_arguments") or "")
    paths: list[str] = []
    for match in re.finditer(r'"(?:path|file_path|workdir)"\s*:\s*"([^"]+)"', raw_arguments):
        candidate = match.group(1).strip().replace("\\\\", "\\")
        if not candidate:
            continue
        if not re.match(r"^[A-Za-z]:[\\/]", candidate) and cwd:
            candidate = str((Path(cwd) / candidate).resolve())
        paths.append(candidate)
    return dedupe_keep_order(paths, 8)


def read_codex_code_session() -> dict[str, Any] | None:
    newest = newest_jsonl_file(CODEX_SESSIONS_DIR, "rollout-*.jsonl")
    if newest is None:
        return None
    try:
        age_s = int(time.time() - newest.stat().st_mtime)
    except Exception:
        age_s = 0
    entries = read_jsonl_entries(newest, 400)
    if not entries:
        return None

    cwd = ""
    last_user_msg = ""
    recent_files: list[str] = []
    recent_cmds: list[str] = []
    file_snapshots: list[str] = []
    reference_files: list[str] = []

    for entry in entries:
        if entry.get("type") == "session_meta" and isinstance(entry.get("payload"), dict):
            cwd = str(entry["payload"].get("cwd") or cwd).strip()
            continue
        payload = entry.get("payload") if isinstance(entry.get("payload"), dict) else {}
        if payload.get("type") == "message" and payload.get("role") == "user":
            content = payload.get("content")
            if isinstance(content, list):
                for item in content:
                    if isinstance(item, dict) and item.get("type") in {"input_text", "text"}:
                        text = str(item.get("text") or "").strip()
                        if text:
                            last_user_msg = " ".join(text.split())[:200]
            continue
        if payload.get("type") == "function_call":
            function_name = str(payload.get("name") or "").strip()
            arguments = str(payload.get("arguments") or "")
            if "shell_command" in function_name.lower():
                command_match = re.search(r'"command"\s*:\s*"([^"]+)', arguments)
                if command_match:
                    recent_cmds.append(command_match.group(1).strip())
            for file_path in collect_codex_file_paths(payload, cwd):
                recent_files.append(file_path)
                lower_name = function_name.lower()
                if "apply_patch" in lower_name or "write" in lower_name or "edit" in lower_name:
                    file_snapshots.append(file_path)
                else:
                    reference_files.append(file_path)

    recent_files = dedupe_keep_order(recent_files, 5)
    recent_cmds = dedupe_keep_order(recent_cmds, 5)
    file_snapshots = dedupe_keep_order(file_snapshots, 8)
    reference_files = dedupe_keep_order(reference_files, 8)
    if not any([cwd, last_user_msg, recent_files, recent_cmds, file_snapshots, reference_files]):
        return None
    return {
        "session_age_s": age_s,
        "session_file": str(newest),
        "cwd": cwd,
        "last_user_msg": last_user_msg,
        "recent_files": recent_files,
        "recent_cmds": recent_cmds,
        "file_snapshots": file_snapshots,
        "reference_files": reference_files,
    }


def read_claude_code_session() -> dict[str, Any] | None:
    newest = newest_jsonl_file(CLAUDE_PROJECTS_DIR, "*.jsonl")
    if newest is None:
        return None
    try:
        age_s = int(time.time() - newest.stat().st_mtime)
    except Exception:
        age_s = 0
    entries = read_jsonl_entries(newest, 300)
    if not entries:
        return None

    last_user_msg = ""
    recent_files: list[str] = []
    recent_cmds: list[str] = []
    file_snapshots: list[str] = []

    for entry in entries:
        if entry.get("type") == "user" and isinstance(entry.get("message"), dict):
            text = normalize_text_content(entry["message"].get("content"))
            if text:
                last_user_msg = text[:200]
            continue
        message = entry.get("message") if isinstance(entry.get("message"), dict) else {}
        content = message.get("content")
        if not (message.get("role") == "assistant" and isinstance(content, list)):
            continue
        for item in content:
            if not isinstance(item, dict) or item.get("type") != "tool_use":
                continue
            name = str(item.get("name") or "").strip()
            item_input = item.get("input") if isinstance(item.get("input"), dict) else {}
            file_path = str(item_input.get("file_path") or "").strip()
            if file_path:
                recent_files.append(file_path)
                if name in {"Edit", "Write"}:
                    file_snapshots.append(file_path)
            command = str(item_input.get("command") or "").strip()
            if name == "Bash" and command:
                recent_cmds.append(command.splitlines()[0][:120])

    recent_files = dedupe_keep_order(recent_files, 5)
    recent_cmds = dedupe_keep_order(recent_cmds, 5)
    file_snapshots = dedupe_keep_order(file_snapshots, 8)
    if not any([last_user_msg, recent_files, recent_cmds, file_snapshots]):
        return None
    return {
        "session_age_s": age_s,
        "session_file": str(newest),
        "last_user_msg": last_user_msg,
        "recent_files": recent_files,
        "recent_cmds": recent_cmds,
        "file_snapshots": file_snapshots,
    }


def project_dir_from_file_path(file_path: str) -> str:
    candidate = str(file_path or "").strip()
    if not candidate:
        return ""
    try:
        path = Path(candidate).resolve()
    except Exception:
        return ""
    current = path if path.is_dir() else path.parent
    if current.name.lower() in {"lib", "src", "public", "scripts", "skills", "node_modules"}:
        current = current.parent
    git_root = find_git_root(str(current))
    return git_root or str(current)


def pick_agent_project_path(primary_path: str, evidence_paths: list[str]) -> str:
    if is_usable_project_path(primary_path):
        git_root = find_git_root(primary_path)
        return git_root or str(Path(primary_path).resolve())

    counts: dict[str, int] = {}
    for evidence in evidence_paths:
        project_path = project_dir_from_file_path(evidence)
        if not project_path:
            continue
        counts[project_path] = counts.get(project_path, 0) + 1

    if not counts:
        return ""
    return max(counts.items(), key=lambda item: item[1])[0]


def infer_task_hint(meaningful_commands: list[str]) -> str:
    for command in reversed(meaningful_commands):
        lowered = command.lower()
        if lowered.startswith("git commit"):
            return f"recent commit: {command[10:].strip() or command}"
        if lowered.startswith("git "):
            return f"git work: {command}"
        if "codex" in lowered:
            return "working in Codex"
        if "claude" in lowered:
            return "working in Claude Code"
        if any(token in lowered for token in ("npm", "node", "pnpm", "yarn", "vite", "tsx")):
            return f"node work: {command}"
        if any(token in lowered for token in ("python", "pip", "uv", "pytest")):
            return f"python work: {command}"
    return ""


def generate_terminal_context(workspace_dir: Path, hint_cwd: str = "") -> dict[str, Any] | None:
    all_commands = read_powershell_history(50)
    meaningful = dedupe_keep_order(
        [command for command in all_commands if MEANINGFUL_COMMAND_RE.search(command)],
        12,
    )

    project_path = ""
    if is_usable_project_path(hint_cwd):
        project_path = str(Path(hint_cwd).resolve())

    if not project_path:
        for command in reversed(all_commands):
            match = CD_COMMAND_RE.match(command)
            if not match:
                continue
            candidate = str(match.group(1) or match.group(2) or match.group(3) or "").strip()
            if is_usable_project_path(candidate):
                project_path = str(Path(candidate).resolve())
                break

    codex_code = read_codex_code_session()
    claude_code = read_claude_code_session()

    if not project_path and isinstance(codex_code, dict):
        evidence = []
        evidence.extend(codex_code.get("recent_files") if isinstance(codex_code.get("recent_files"), list) else [])
        evidence.extend(codex_code.get("file_snapshots") if isinstance(codex_code.get("file_snapshots"), list) else [])
        evidence.extend(codex_code.get("reference_files") if isinstance(codex_code.get("reference_files"), list) else [])
        project_path = pick_agent_project_path(str(codex_code.get("cwd") or ""), [str(x) for x in evidence])

    if not project_path and isinstance(claude_code, dict):
        evidence = claude_code.get("recent_files") if isinstance(claude_code.get("recent_files"), list) else []
        project_path = pick_agent_project_path("", [str(x) for x in evidence])

    existing = read_terminal_context(workspace_dir)
    if not project_path and isinstance(existing, dict):
        project_path = str(existing.get("project_path") or "").strip()

    git_root = find_git_root(project_path)
    if git_root:
        project_path = git_root

    git_context = None
    if project_path:
        git_context = {
            "root": project_path,
            "branch": run_git_sync(["branch", "--show-current"], project_path),
            "recent_commits": run_git_sync(["log", "--oneline", "-5"], project_path),
            "status": run_git_sync(["status", "--short"], project_path),
            "last_modified": run_git_sync(["log", "--format=%ar", "-1"], project_path),
        }

    context = {
        "updated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "project_path": project_path,
        "task_hint": infer_task_hint(meaningful),
        "recent_commands": meaningful[-8:],
        "git": git_context,
        "claude_code": claude_code,
        "codex_code": codex_code,
    }

    terminal_context_path = workspace_dir / "terminal-context.json"
    terminal_context_path.parent.mkdir(parents=True, exist_ok=True)
    terminal_context_path.write_text(json.dumps(context, ensure_ascii=False, indent=2), encoding="utf-8")
    return context


def read_terminal_context(workspace_dir: Path) -> dict[str, Any] | None:
    terminal_context_path = workspace_dir / "terminal-context.json"
    if not terminal_context_path.exists() or not terminal_context_path.is_file():
        return None
    parsed = read_json_file(terminal_context_path)
    return parsed if isinstance(parsed, dict) else None


def basename_from_path(value: str) -> str:
    normalized = str(value or "").strip().rstrip("\\/")
    if not normalized:
        return ""
    return normalized.replace("/", "\\").split("\\")[-1].strip()


def plugin_dir() -> Path:
    return Path(os.environ.get("WEFT_PACKAGE_DIR", Path(__file__).resolve().parent)).resolve()


def runtime_root() -> Path:
    root = plugin_dir()
    return root.parents[2] if len(root.parents) >= 3 else root


def context_engine_data_dir() -> Path:
    path = plugin_dir() / "data"
    path.mkdir(parents=True, exist_ok=True)
    return path


def runtime_data_dir() -> Path:
    path = runtime_root() / "data"
    path.mkdir(parents=True, exist_ok=True)
    return path


def blink_once_timeout_seconds() -> float:
    try:
        return max(1.0, float(os.environ.get("WEFT_CONTEXT_ENGINE_BLINK_ONCE_TIMEOUT_SECONDS", "45")))
    except Exception:
        return 45.0


def read_shared_mode_state() -> dict[str, Any]:
    for path in (
        runtime_data_dir() / "weft-proactive-state.json",
        context_engine_data_dir() / "weft-proactive-state.json",
    ):
        if not path.exists() or not path.is_file():
            continue
        parsed = read_json_file(path)
        if isinstance(parsed, dict):
            return parsed
    return {}


def read_active_context_state() -> dict[str, Any]:
    parsed = read_json_file(runtime_data_dir() / "weft-active-context.json")
    if not isinstance(parsed, dict):
        return {}
    return {
        "session_id": str(parsed.get("session_id") or "").strip(),
        "workspace_id": str(parsed.get("workspace_id") or "").strip(),
        "workspace_dir": str(parsed.get("workspace_dir") or "").strip(),
    }


def normalize_game_index_path(value: object) -> str:
    text = str(value or "").strip()
    return text.replace("\\", "/").lower() if text else ""


def load_game_index() -> dict[str, Any]:
    parsed = read_json_file(runtime_data_dir() / "game-index.json")
    return parsed if isinstance(parsed, dict) else {}


def load_web_context_index() -> dict[str, Any]:
    for path in (
        runtime_data_dir() / "web-context-index.json",
        context_engine_data_dir() / "web-context-index.json",
    ):
        parsed = read_json_file(path)
        if isinstance(parsed, dict):
            return parsed
    return {}


def normalize_web_host(value: object) -> str:
    host = str(value or "").strip().lower().rstrip(".")
    if host.startswith("www."):
        host = host[4:]
    return host


def web_host_matches_suffix(host: str, suffix: object) -> bool:
    normalized_host = normalize_web_host(host)
    normalized_suffix = normalize_web_host(suffix)
    if not normalized_host or not normalized_suffix:
        return False
    return normalized_host == normalized_suffix or normalized_host.endswith(f".{normalized_suffix}")


def parse_web_context_url(url: object) -> dict[str, Any] | None:
    text = str(url or "").strip()
    if not text:
        return None
    candidate = text if "://" in text else f"https://{text}"
    parsed = urllib.parse.urlparse(candidate)
    if parsed.scheme and parsed.scheme not in {"http", "https"}:
        return None
    host = normalize_web_host(parsed.hostname or "")
    if not host:
        return None
    path = parsed.path or "/"
    segments = [segment for segment in path.split("/") if segment]
    return {
        "host": host,
        "path": path,
        "query": parsed.query,
        "segments": segments,
    }


def domain_semantics_for_host(index: dict[str, Any], host: str) -> dict[str, Any] | None:
    domains = index.get("domains") if isinstance(index.get("domains"), list) else []
    matches: list[dict[str, Any]] = []
    for entry in domains:
        if not isinstance(entry, dict):
            continue
        domain = str(entry.get("domain") or "").strip()
        if web_host_matches_suffix(host, domain):
            matches.append(entry)
    if not matches:
        return None
    return max(matches, key=lambda item: len(str(item.get("domain") or "")))


def route_match_score(route: dict[str, Any], parsed_url: dict[str, Any]) -> int:
    match = route.get("match") if isinstance(route.get("match"), dict) else {}
    host = str(parsed_url.get("host") or "")
    path = str(parsed_url.get("path") or "/")
    suffixes = match.get("domain_suffixes") if isinstance(match.get("domain_suffixes"), list) else []
    suffix = match.get("domain_suffix")
    candidate_suffixes = [suffix] if suffix else []
    candidate_suffixes.extend(suffixes)
    if candidate_suffixes and not any(web_host_matches_suffix(host, item) for item in candidate_suffixes):
        return -1

    score = 1
    prefixes = match.get("path_prefixes") if isinstance(match.get("path_prefixes"), list) else []
    if prefixes:
        matched_prefixes = [str(prefix or "") for prefix in prefixes if path.startswith(str(prefix or ""))]
        if not matched_prefixes:
            return -1
        score += max(len(prefix) for prefix in matched_prefixes)

    min_segments = match.get("path_min_segments")
    if min_segments is not None:
        try:
            required_segments = int(min_segments)
        except Exception:
            return -1
        if len(parsed_url.get("segments") or []) < required_segments:
            return -1
        score += required_segments
    return score


def match_web_context_index_url(url: object) -> dict[str, Any] | None:
    parsed_url = parse_web_context_url(url)
    if not parsed_url:
        return None
    index = load_web_context_index()
    if not index:
        return None

    host = str(parsed_url.get("host") or "")
    domain_semantics = domain_semantics_for_host(index, host) or {}
    routes = index.get("routes") if isinstance(index.get("routes"), list) else []
    best_route: dict[str, Any] | None = None
    best_score = -1
    for route in routes:
        if not isinstance(route, dict):
            continue
        score = route_match_score(route, parsed_url)
        if score > best_score:
            best_route = route
            best_score = score

    if best_route and best_score >= 0:
        semantics = best_route.get("semantics") if isinstance(best_route.get("semantics"), dict) else {}
        return {
            "site_kind": domain_semantics.get("site_kind"),
            "site_domain": domain_semantics.get("domain") or host,
            "platform": domain_semantics.get("platform"),
            **semantics,
            "match_source": "web_context_index:route",
            "match_rule_id": best_route.get("id"),
        }

    if domain_semantics:
        return {
            "site_kind": domain_semantics.get("site_kind"),
            "site_domain": domain_semantics.get("domain") or host,
            "platform": domain_semantics.get("platform"),
            "match_source": "web_context_index:domain",
        }
    return None


def enrich_event_with_web_context(event: dict[str, Any]) -> dict[str, Any]:
    event_type = str(event.get("event_type") or "").strip()
    if event_type not in BROWSER_CONTEXT_EVENT_TYPES:
        return event
    payload = event.get("payload") if isinstance(event.get("payload"), dict) else {}
    url = payload.get("url")
    match = match_web_context_index_url(url)
    if not match:
        return event
    return {
        **event,
        "payload": {
            **payload,
            **{key: value for key, value in match.items() if value not in (None, "", [])},
        },
    }


def match_game_index_entry(raw: dict[str, Any], process_name: str) -> dict[str, Any] | None:
    index = load_game_index()
    games = index.get("games") if isinstance(index.get("games"), list) else []
    if not games:
        return None
    detail = raw.get("detail") if isinstance(raw.get("detail"), dict) else {}
    process_path = normalize_game_index_path(
        detail.get("fg_process_path") or raw.get("fg_process_path") or raw.get("process_path")
    )
    normalized_process = str(process_name or "").strip().lower()

    if process_path:
        for game in games:
            if not isinstance(game, dict):
                continue
            paths = game.get("executable_paths") if isinstance(game.get("executable_paths"), list) else []
            normalized_paths = {normalize_game_index_path(path) for path in paths if normalize_game_index_path(path)}
            if process_path in normalized_paths:
                return {**game, "match_source": "game_index:path"}

    if normalized_process:
        for game in games:
            if not isinstance(game, dict):
                continue
            names = game.get("executable_names") if isinstance(game.get("executable_names"), list) else []
            normalized_names = {str(name or "").strip().lower() for name in names if str(name or "").strip()}
            if normalized_process in normalized_names:
                return {**game, "match_source": "game_index:exe"}
    return None


def load_core_config() -> dict[str, Any]:
    config_path = runtime_root() / "config" / "config.toml"
    try:
        import tomllib
        return tomllib.loads(config_path.read_text("utf-8"))
    except Exception:
        return {}


def core_port() -> int:
    config = load_core_config()
    return int(((config.get("core") or {}).get("port")) or 42617)


def physical_time_tags() -> dict[str, Any]:
    now = time.time()
    local_time = datetime.fromtimestamp(now).astimezone()
    return {
        "wall_time_utc": datetime.fromtimestamp(now, timezone.utc).isoformat(),
        "wall_time_local": local_time.isoformat(),
        "epoch_ms": int(now * 1000),
        "monotonic_ms": int((time.monotonic() - START_MONOTONIC) * 1000),
        "timezone": local_time.tzname() or "",
        "utc_offset_seconds": int(local_time.utcoffset().total_seconds()) if local_time.utcoffset() else 0,
    }


def call_builtin_tool(tool: str, args: dict[str, Any]) -> dict[str, Any]:
    import urllib.request

    normalized_tool = str(tool or "").strip()
    if not normalized_tool:
        return {}
    body = json.dumps(
        {
            "action": "execute_tool",
            "data": {
                "agent": "context-engine",
                "tool": normalized_tool,
                "args": args,
            },
        },
        ensure_ascii=False,
    ).encode("utf-8")
    request = urllib.request.Request(
        f"http://127.0.0.1:{core_port()}/api/plugins/tool-runtime-core/call",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=45) as response:
        raw = response.read().decode("utf-8")
    payload = json.loads(raw) if raw.strip() else {}
    if not isinstance(payload, dict):
        return {}
    if payload.get("status") != "ok":
        return {}
    data = payload.get("data")
    return data if isinstance(data, dict) else {}


def parse_json_or_last_object(text: str) -> dict[str, Any]:
    raw = str(text or "").strip()
    if not raw:
        return {}
    try:
        parsed = json.loads(raw)
        return parsed if isinstance(parsed, dict) else {}
    except Exception:
        pass
    start = raw.rfind("{")
    if start < 0:
        return {}
    try:
        parsed = json.loads(raw[start:])
        return parsed if isinstance(parsed, dict) else {}
    except Exception:
        return {}


def parse_blink_once_wrapper_output(stdout: bytes | str) -> dict[str, Any]:
    text = stdout.decode("utf-8", "replace") if isinstance(stdout, bytes) else str(stdout or "")
    outer = parse_json_or_last_object(text)
    encoded_payload = outer.get("payload_b64")
    if isinstance(encoded_payload, str) and encoded_payload.strip():
        try:
            parsed = json.loads(base64.b64decode(encoded_payload).decode("utf-8"))
            return parsed if isinstance(parsed, dict) else {}
        except Exception:
            return {}
    return outer


def subprocess_path(path: Path) -> str:
    text = str(path)
    if text.startswith("\\\\?\\"):
        return text[4:]
    return text


def blink_once_script_path() -> Path | None:
    candidates = [
        runtime_root() / "plugins" / "installed" / "tool-desktop" / "desktop-runtime" / "scripts" / "blink-once.js",
        runtime_root() / "plugins" / "installed" / "skills" / "desktop-runtime" / "scripts" / "blink-once.js",
    ]
    return next((candidate for candidate in candidates if candidate.exists()), None)


def run_blink_once_direct(args: dict[str, Any]) -> dict[str, Any]:
    if not VISION_PROBE_LOCK.acquire(blocking=False):
        return {}
    try:
        return run_blink_once_direct_unlocked(args)
    finally:
        VISION_PROBE_LOCK.release()


def run_blink_once_direct_unlocked(args: dict[str, Any]) -> dict[str, Any]:
    script = blink_once_script_path()
    if not script:
        return {}
    payload = {
        "task": str(args.get("task") or "look at the current screen"),
        "forceSystem": bool(args.get("force_system", True)),
        "fastMode": bool(args.get("fast_mode", False)),
        "screenIndex": int(args.get("screen_index") or 1),
        "prevSummaries": args.get("prev_summaries") if isinstance(args.get("prev_summaries"), list) else [],
    }
    try:
        completed = subprocess.run(
            ["node", subprocess_path(script), json.dumps(payload, ensure_ascii=False)],
            cwd=subprocess_path(script.parent.parent),
            capture_output=True,
            timeout=blink_once_timeout_seconds(),
            check=False,
        )
    except Exception:
        return {}
    if completed.returncode != 0:
        return {}
    return parse_blink_once_wrapper_output(completed.stdout)


def call_service_plugin(package_name: str, action: str, data: dict[str, Any]) -> dict[str, Any]:
    body = json.dumps({"action": action, "data": data}, ensure_ascii=False).encode("utf-8")
    request = urllib.request.Request(
        f"http://127.0.0.1:{core_port()}/api/plugins/{urllib.parse.quote(package_name)}/call",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=45) as response:
        raw = response.read().decode("utf-8")
    payload = json.loads(raw) if raw.strip() else {}
    return payload if isinstance(payload, dict) else {}


def call_companion_context_event(package_name: str, payload: dict[str, Any]) -> dict[str, Any]:
    normalized_package = str(package_name or "companion-core").strip() or "companion-core"
    body = json.dumps(
        {"action": "handle_context_event", "data": payload},
        ensure_ascii=False,
    ).encode("utf-8")
    url = f"http://127.0.0.1:{core_port()}/api/services/{urllib.parse.quote(normalized_plugin)}/dispatch-webhook"
    request = urllib.request.Request(
        url,
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=5) as response:
        raw = response.read().decode("utf-8")
    result = json.loads(raw) if raw.strip() else {}
    return result if isinstance(result, dict) else {}


def extract_tool_summary(result: dict[str, Any]) -> str:
    encoded = result.get("content_b64")
    if isinstance(encoded, str) and encoded.strip():
        try:
            decoded = base64.b64decode(encoded).decode("utf-8").strip()
            if decoded:
                return decoded[:200]
        except Exception:
            pass
    for key in ("content", "summary", "screen_description", "description"):
        value = result.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()[:200]
    return ""


def now_ms() -> int:
    return int(time.time() * 1000)


def ok(data: Any = None) -> dict[str, Any]:
    return {"status": "ok", "data": data}


def err(message: str) -> dict[str, Any]:
    return {"status": "error", "error": str(message)}


def default_workspace_dir() -> Path:
    return Path.home() / ".openclaw" / "workspace"


def default_global_sense_dir() -> Path:
    return runtime_root() / "data" / "global-sense"


def current_timestamp_iso() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def read_json_file(file_path: Path) -> dict[str, Any] | None:
    try:
        parsed = json.loads(file_path.read_text("utf-8"))
    except Exception:
        return None
    return parsed if isinstance(parsed, dict) else None


def write_json_file(file_path: Path, payload: dict[str, Any]) -> None:
    file_path.parent.mkdir(parents=True, exist_ok=True)
    file_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def trim_content(value: str, limit: int) -> str:
    text = str(value or "").strip()
    return text if len(text) <= limit else text[: max(0, limit - 3)] + "..."


def get_foreground_window_snapshot() -> tuple[str, str, int] | None:
    if ctypes is None or not hasattr(ctypes, "windll"):
        return None
    try:
        user32 = ctypes.windll.user32
        hwnd = user32.GetForegroundWindow()
        if not hwnd:
            return None

        length = int(user32.GetWindowTextLengthW(hwnd)) + 1
        title_buf = ctypes.create_unicode_buffer(max(1, length))
        user32.GetWindowTextW(hwnd, title_buf, len(title_buf))
        title = str(title_buf.value or "").strip()

        pid = ctypes.wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
        process_name = ""
        if psutil is not None and pid.value:
            try:
                process_name = str(psutil.Process(pid.value).name() or "").strip()
            except Exception:
                process_name = ""

        return title, process_name, int(pid.value or 0)
    except Exception:
        return None


def sample_foreground_status() -> dict[str, Any] | None:
    snapshot = get_foreground_window_snapshot()
    if not snapshot:
        return None

    title, process_name, pid = snapshot
    if not title and not process_name:
        return None

    normalized_process = str(process_name or "").strip().lower()
    return {
        "timestamp": time.time(),
        "updated_at": current_timestamp_iso(),
        "status": "active",
        "fg_title": title,
        "foreground": normalized_process,
        "detail": {
            "fg_window": title,
            "fg_process": normalized_process,
            "matched_by": normalized_process,
            "idle_seconds": 0,
        },
        "pid": pid,
    }


def refresh_pc_status(global_sense_dir: Path) -> dict[str, Any] | None:
    pc_status_path = global_sense_dir / "pc-status.json"
    sampled = sample_foreground_status()
    if isinstance(sampled, dict):
        pc_status_path.parent.mkdir(parents=True, exist_ok=True)
        pc_status_path.write_text(json.dumps(sampled, ensure_ascii=False, indent=2), encoding="utf-8")
        return sampled
    if pc_status_path.exists() and pc_status_path.is_file():
        cached = read_json_file(pc_status_path)
        if isinstance(cached, dict):
            return cached
    return None


def make_activity_window() -> dict[str, Any]:
    return {
        "start": current_timestamp_iso(),
        "end": None,
        "actions": [],
    }


def ensure_activity_state_loaded(global_sense_dir: Path) -> None:
    global ACTIVITY_STATE_DIR, ACTIVITY_WINDOWS, ACTIVITY_CURRENT_WINDOW, ACTIVITY_RECENT_EVENTS
    normalized_dir = str(global_sense_dir.resolve())
    if ACTIVITY_STATE_DIR == normalized_dir and isinstance(ACTIVITY_CURRENT_WINDOW, dict):
        return

    activity_dir = global_sense_dir / "activity"
    windows_path = activity_dir / "windows.json"
    recent_events_path = activity_dir / "recent-events.json"
    loaded_windows: list[dict[str, Any]] = []
    loaded_recent_events: list[dict[str, Any]] = []

    parsed_windows = read_json_file(windows_path)
    if isinstance(parsed_windows, dict):
        windows = parsed_windows.get("windows")
        if isinstance(windows, list):
            loaded_windows = [entry for entry in windows if isinstance(entry, dict)][-ACTIVITY_MAX_WINDOWS:]
        current_window = parsed_windows.get("current_window")
        if isinstance(current_window, dict):
            ACTIVITY_CURRENT_WINDOW = current_window
        else:
            ACTIVITY_CURRENT_WINDOW = make_activity_window()
    else:
        ACTIVITY_CURRENT_WINDOW = make_activity_window()

    parsed_recent_events = read_json_file(recent_events_path)
    if isinstance(parsed_recent_events, dict):
        events = parsed_recent_events.get("events")
        if isinstance(events, list):
            loaded_recent_events = [entry for entry in events if isinstance(entry, dict)][-ACTIVITY_MAX_RECENT_EVENTS:]

    ACTIVITY_STATE_DIR = normalized_dir
    ACTIVITY_WINDOWS = loaded_windows
    ACTIVITY_RECENT_EVENTS = loaded_recent_events


def persist_activity_state(global_sense_dir: Path) -> None:
    activity_dir = global_sense_dir / "activity"
    activity_dir.mkdir(parents=True, exist_ok=True)
    windows_path = activity_dir / "windows.json"
    recent_events_path = activity_dir / "recent-events.json"
    windows_path.write_text(
        json.dumps(
            {
                "updated_at": current_timestamp_iso(),
                "windows": ACTIVITY_WINDOWS[-ACTIVITY_MAX_WINDOWS:],
                "current_window": ACTIVITY_CURRENT_WINDOW or make_activity_window(),
            },
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )
    recent_events_path.write_text(
        json.dumps(
            {
                "updated_at": current_timestamp_iso(),
                "events": ACTIVITY_RECENT_EVENTS[-ACTIVITY_MAX_RECENT_EVENTS:],
            },
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )


def seal_activity_window_if_needed(now_ts: float) -> None:
    global ACTIVITY_CURRENT_WINDOW
    current_window = ACTIVITY_CURRENT_WINDOW if isinstance(ACTIVITY_CURRENT_WINDOW, dict) else make_activity_window()
    start_text = str(current_window.get("start") or "").strip()
    start_ts = None
    if start_text:
        try:
            start_ts = time.mktime(time.strptime(start_text, "%Y-%m-%dT%H:%M:%SZ"))
        except Exception:
            start_ts = None
    if start_ts is None:
        current_window = make_activity_window()
        ACTIVITY_CURRENT_WINDOW = current_window
        return
    if now_ts - start_ts < ACTIVITY_WINDOW_SECONDS:
        return
    if isinstance(current_window.get("actions"), list) and current_window["actions"]:
        current_window["end"] = current_timestamp_iso()
        ACTIVITY_WINDOWS.append(current_window)
        if len(ACTIVITY_WINDOWS) > ACTIVITY_MAX_WINDOWS:
            ACTIVITY_WINDOWS[:] = ACTIVITY_WINDOWS[-ACTIVITY_MAX_WINDOWS:]
    ACTIVITY_CURRENT_WINDOW = make_activity_window()


def append_activity_action(action_type: str, data: dict[str, Any]) -> None:
    global ACTIVITY_CURRENT_WINDOW
    current_window = ACTIVITY_CURRENT_WINDOW if isinstance(ACTIVITY_CURRENT_WINDOW, dict) else make_activity_window()
    actions = current_window.get("actions")
    if not isinstance(actions, list):
        current_window["actions"] = []
        actions = current_window["actions"]
    event = {
        "type": action_type,
        "data": data,
        "t": current_timestamp_iso(),
    }
    actions.append(event)
    ACTIVITY_CURRENT_WINDOW = current_window
    ACTIVITY_RECENT_EVENTS.append(event)
    if len(ACTIVITY_RECENT_EVENTS) > ACTIVITY_MAX_RECENT_EVENTS:
        ACTIVITY_RECENT_EVENTS[:] = ACTIVITY_RECENT_EVENTS[-ACTIVITY_MAX_RECENT_EVENTS:]


def classify_foreground_activity(raw_status: dict[str, Any]) -> tuple[str, dict[str, Any]] | None:
    detail = raw_status.get("detail") if isinstance(raw_status.get("detail"), dict) else {}
    fg_window = str(detail.get("fg_window") or raw_status.get("fg_title") or "").strip()
    process_name = str(detail.get("fg_process") or raw_status.get("foreground") or "").strip().lower()
    if not fg_window and not process_name:
        return None

    if process_name in OFFICE_PROCESS_NAMES or re.search(r"\.(docx?|xlsx?|pptx?|pdf|txt|md)$", fg_window, re.IGNORECASE):
        return ("file_edited", {"title": fg_window, "process": process_name})

    if any(token in fg_window.lower() for token in ("bilibili", "youtube", "优酷", "爱奇艺", "twitch", "抖音", "tiktok")):
        return ("video_watched", {"title": fg_window, "process": process_name})

    if process_name in VIDEO_PROCESS_NAMES:
        return ("url_visited", {"title": fg_window, "process": process_name})

    if process_name in MEETING_PROCESS_NAMES:
        return ("app_used", {"title": fg_window, "process": process_name, "appType": "meeting"})

    if process_name in TERMINAL_PROCESS_NAMES:
        return ("app_used", {"title": fg_window, "process": process_name, "appType": "terminal"})

    if process_name:
        return ("app_switched", {"title": fg_window, "process": process_name})
    return None


def refresh_activity_windows(raw_status: dict[str, Any] | None, global_sense_dir: Path) -> None:
    if not isinstance(raw_status, dict):
        return
    detail = raw_status.get("detail") if isinstance(raw_status.get("detail"), dict) else {}
    fg_window = str(detail.get("fg_window") or raw_status.get("fg_title") or "").strip()
    process_name = str(detail.get("fg_process") or raw_status.get("foreground") or "").strip().lower()
    if is_host_foreground_window(process_name, fg_window):
        return

    now_ts = time.time()
    with ACTIVITY_STATE_LOCK:
        ensure_activity_state_loaded(global_sense_dir)
        seal_activity_window_if_needed(now_ts)
        current_window = ACTIVITY_CURRENT_WINDOW if isinstance(ACTIVITY_CURRENT_WINDOW, dict) else make_activity_window()
        actions = current_window.get("actions") if isinstance(current_window.get("actions"), list) else []
        last_entry = actions[-1] if actions else None
        last_process = ""
        last_title = ""
        if isinstance(last_entry, dict):
            data = last_entry.get("data") if isinstance(last_entry.get("data"), dict) else {}
            last_process = str(data.get("process") or "").strip().lower()
            last_title = str(data.get("title") or "").strip()
        if last_process == process_name and last_title == fg_window:
            return
        classified = classify_foreground_activity(raw_status)
        if not classified:
            return
        action_type, data = classified
        append_activity_action(action_type, data)
        persist_activity_state(global_sense_dir)


def get_recent_activity_events(global_sense_dir: Path, limit: int = 8) -> list[dict[str, Any]]:
    with ACTIVITY_STATE_LOCK:
        ensure_activity_state_loaded(global_sense_dir)
        events = list(ACTIVITY_RECENT_EVENTS)
    return events[-max(1, limit):]


def format_recent_activity_summary(global_sense_dir: Path, limit: int = 8) -> list[str]:
    lines: list[str] = []
    for event in get_recent_activity_events(global_sense_dir, limit):
        event_type = str(event.get("type") or "").strip()
        data = event.get("data") if isinstance(event.get("data"), dict) else {}
        label = str(data.get("title") or data.get("appType") or data.get("process") or "").strip()
        if not event_type or not label:
            continue
        lines.append(f"{event_type}: {label}")
    return lines


def is_host_foreground_window(process_name: str, title: str) -> bool:
    normalized_process = str(process_name or "").strip().lower()
    normalized_title = str(title or "").strip().lower()
    if normalized_process in HOST_FOREGROUND_PROCESS_NAMES:
        return True
    if normalized_title in {"weft", "companion"}:
        return True
    if normalized_title.startswith("weft - ") or normalized_title.startswith("companion - "):
        return True
    return False


def foreground_process_name(raw_status: dict[str, Any] | None) -> str:
    if not isinstance(raw_status, dict):
        return ""
    detail = raw_status.get("detail") if isinstance(raw_status.get("detail"), dict) else {}
    return str(detail.get("fg_process") or raw_status.get("foreground") or "").strip().lower()


def event_allowed_by_foreground(event: dict[str, Any], raw_status: dict[str, Any] | None) -> bool:
    event_type = str(event.get("event_type") or "").strip()
    if event_type not in FOCUS_AUTHORITY_EVENT_TYPES:
        return True
    process_name = foreground_process_name(raw_status)
    if event_type in BROWSER_CONTEXT_EVENT_TYPES or event_type == "video_context_detected":
        return process_name in VIDEO_PROCESS_NAMES
    if event_type == "game_context_detected":
        return bool(match_game_index_entry(raw_status or {}, process_name))
    return True


def is_background_focus_event(event: dict[str, Any], raw_status: dict[str, Any] | None) -> bool:
    return not event_allowed_by_foreground(event, raw_status)


def remember_last_non_host_pc_status(raw_status: dict[str, Any] | None) -> None:
    global LAST_NON_HOST_PC_STATUS
    if not isinstance(raw_status, dict):
        return
    detail = raw_status.get("detail") if isinstance(raw_status.get("detail"), dict) else {}
    fg_window = str(detail.get("fg_window") or raw_status.get("fg_title") or "").strip()
    process_name = str(detail.get("fg_process") or raw_status.get("foreground") or "").strip().lower()
    if is_host_foreground_window(process_name, fg_window):
        return
    LAST_NON_HOST_PC_STATUS = raw_status


def normalize_event_payload(raw: dict[str, Any]) -> dict[str, Any] | None:
    event_type = str(raw.get("event_type") or raw.get("type") or "").strip()
    payload = raw.get("payload")
    if not event_type or not isinstance(payload, dict):
        return None

    return {
        "event_type": event_type,
        "payload": payload,
        "source": "external_event_file",
        "timestamp": str(raw.get("timestamp") or "").strip() or None,
    }


def event_sort_key(event: dict[str, Any], fallback_path: Path | None = None) -> tuple[str, int]:
    timestamp = str(event.get("timestamp") or "").strip()
    event_type = str(event.get("event_type") or "").strip()
    priority = 2 if event_type == "reading_page_detected" else 1 if event_type == "active_url_changed" else 0
    if timestamp:
        return (timestamp, priority)
    if fallback_path is not None:
        try:
            return (str(fallback_path.stat().st_mtime_ns), priority)
        except Exception:
            return ("", priority)
    return ("", priority)


def read_latest_browser_context_event(global_sense_dir: Path) -> dict[str, Any] | None:
    events_dir = global_sense_dir / "events"
    if not events_dir.exists() or not events_dir.is_dir():
        return None

    newest: dict[str, Any] | None = None
    newest_key = ("", -1)
    for entry in sorted(events_dir.glob("*.json")):
        parsed = read_json_file(entry)
        if not parsed:
            continue
        event = normalize_event_payload(parsed)
        if not event:
            continue
        if str(event.get("event_type") or "").strip() not in BROWSER_CONTEXT_EVENT_TYPES:
            continue
        key = event_sort_key(event, entry)
        if key > newest_key:
            newest = event
            newest_key = key
    return newest


def prefer_browser_context_event(
    source_status: dict[str, Any],
    current_activity: dict[str, Any] | None,
    global_sense_dir: Path,
) -> dict[str, Any] | None:
    if foreground_process_name(source_status) not in VIDEO_PROCESS_NAMES:
        return current_activity

    browser_event = read_latest_browser_context_event(global_sense_dir)
    if not isinstance(browser_event, dict):
        return current_activity
    return browser_event


def sample_clipboard_text() -> str:
    command = (
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; "
        "$text = Get-Clipboard -Raw -ErrorAction SilentlyContinue; "
        "if ($null -eq $text) { '' } else { [string]$text }"
    )
    try:
        result = subprocess.run(
            [
                "powershell.exe",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                command,
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=2,
            check=False,
        )
    except Exception:
        return ""
    if result.returncode != 0:
        return ""
    return str(result.stdout or "").replace("\r\n", "\n").strip()


def collect_clipboard_event() -> dict[str, Any] | None:
    global LAST_CLIPBOARD_TEXT
    text = sample_clipboard_text()
    if not text or len(text) < 20:
        return None
    if text == LAST_CLIPBOARD_TEXT:
        return None
    LAST_CLIPBOARD_TEXT = text
    return {
        "event_type": "clipboard_changed",
        "payload": {
            "text": text,
            "visible_length": len(text),
        },
        "source": "clipboard_collector",
        "timestamp": current_timestamp_iso(),
    }

def enrich_pc_status_event_with_workspace_context(
    event: dict[str, Any] | None,
    workspace_dir: Path,
) -> dict[str, Any] | None:
    if not event or not isinstance(event, dict):
        return event
    event_type = str(event.get("event_type") or "").strip()
    if event_type != "coding_context_detected":
        return event
    payload = event.get("payload") if isinstance(event.get("payload"), dict) else {}
    title = str(payload.get("title") or "").strip()
    cwd_match = re.search(r"([A-Za-z]:[\\/][^-|]+)", title)
    hint_cwd = cwd_match.group(1).strip() if cwd_match else ""
    terminal_context = read_terminal_context(workspace_dir) or generate_terminal_context(workspace_dir, hint_cwd)
    if not terminal_context:
        return event

    project_path = str(terminal_context.get("project_path") or "").strip()
    recent_commands = (
        terminal_context.get("recent_commands")
        if isinstance(terminal_context.get("recent_commands"), list)
        else []
    )
    git = terminal_context.get("git") if isinstance(terminal_context.get("git"), dict) else {}

    enriched_payload = dict(payload)
    if project_path:
        enriched_payload["project_path"] = project_path
        enriched_payload["project_name"] = basename_from_path(project_path)
    branch = str(git.get("branch") or "").strip()
    if branch:
        enriched_payload["branch"] = branch
    if recent_commands:
        enriched_payload["recent_commands"] = [str(entry) for entry in recent_commands if str(entry).strip()]
    codex_code = terminal_context.get("codex_code")
    if isinstance(codex_code, dict):
        enriched_payload["codex_code"] = codex_code
    claude_code = terminal_context.get("claude_code")
    if isinstance(claude_code, dict):
        enriched_payload["claude_code"] = claude_code

    return {
        **event,
        "payload": enriched_payload,
    }


def map_pc_status_to_event(raw: dict[str, Any]) -> dict[str, Any] | None:
    foreground = str(raw.get("foreground") or "").strip().lower()
    detail = raw.get("detail") if isinstance(raw.get("detail"), dict) else {}
    process_name = str(detail.get("fg_process") or foreground).strip().lower()
    title = str(detail.get("fg_window") or raw.get("fg_title") or "").strip()
    title_lower = title.lower()

    game_match = match_game_index_entry(raw, process_name)
    if game_match:
        return {
            "event_type": "game_context_detected",
            "payload": {
                "title": title,
                "process_name": process_name,
                "game_name": str(game_match.get("name") or "").strip(),
                "platform": game_match.get("platform"),
                "install_dir": game_match.get("install_dir"),
                "match_source": game_match.get("match_source"),
                "source": "pc-status",
            },
            "source": "pc-status",
            "timestamp": str(raw.get("updated_at") or "").strip() or None,
        }

    if process_name in OFFICE_PROCESS_NAMES:
        return {
            "event_type": "document_context_detected",
            "payload": {
                "kind": "document",
                "title": title,
                "process_name": process_name,
                "source": "pc-status",
            },
            "source": "pc-status",
            "timestamp": str(raw.get("updated_at") or "").strip() or None,
        }

    if process_name in MEETING_PROCESS_NAMES or any(
        token in title_lower
        for token in ("meeting", "zoom", "teams", "lark meeting", "鑵捐浼氳", "閽夐拤浼氳", "椋炰功浼氳")
    ):
        app_name = (
            "Zoom"
            if "zoom" in process_name or "zoom" in title_lower
            else "Teams"
            if "teams" in process_name or "teams" in title_lower
            else "Meeting"
        )
        return {
            "event_type": "meeting_context_detected",
            "payload": {
                "title": title,
                "process_name": process_name,
                "app_name": app_name,
                "source": "pc-status",
            },
            "source": "pc-status",
            "timestamp": str(raw.get("updated_at") or "").strip() or None,
        }

    if process_name in TERMINAL_PROCESS_NAMES and title:
        tool_name = "terminal"
        if "claude" in title_lower:
            tool_name = "Claude Code"
        elif "codex" in title_lower:
            tool_name = "Codex"
        elif "cursor" in title_lower:
            tool_name = "Cursor"
        return {
            "event_type": "coding_context_detected",
            "payload": {
                "title": title,
                "process_name": process_name,
                "tool_name": tool_name,
                "source": "pc-status",
            },
            "source": "pc-status",
            "timestamp": str(raw.get("updated_at") or "").strip() or None,
        }

    if process_name in VIDEO_PROCESS_NAMES and any(
        token in title_lower
        for token in ("youtube", "bilibili", "twitch", "tiktok", "浼橀叿", "鐖卞鑹?", "鎶栭煶")
    ):
        platform = "video"
        if "bilibili" in title_lower:
            platform = "B站"
        elif "youtube" in title_lower:
            platform = "YouTube"
        elif "twitch" in title_lower:
            platform = "Twitch"
        elif "tiktok" in title_lower or "鎶栭煶" in title_lower:
            platform = "抖音"
        return {
            "event_type": "video_context_detected",
            "payload": {
                "title": title,
                "process_name": process_name,
                "platform": platform,
                "source": "pc-status",
            },
            "source": "pc-status",
            "timestamp": str(raw.get("updated_at") or "").strip() or None,
        }

    if process_name or title:
        return {
            "event_type": "app_context_detected",
            "payload": {
                "title": title,
                "process_name": process_name,
                "source": "pc-status",
            },
            "source": "pc-status",
            "timestamp": str(raw.get("updated_at") or "").strip() or None,
        }

    return None


VISION_EVENT_TYPES = {"game_context_detected", "video_context_detected", "active_url_changed", "reading_page_detected"}
VISION_LAST_TIME_BY_EVENT: dict[str, float] = {}
VISION_PROBE_LOCK = threading.Lock()
CONTEXT_FRESHNESS_BY_SCENE: dict[str, dict[str, Any]] = {}
VISION_MIN_INTERVAL_BY_EVENT = {
    "game_context_detected": 20.0,
    "video_context_detected": 20.0,
    "active_url_changed": 20.0,
    "reading_page_detected": 20.0,
}


def context_scene_key(event_type: str, payload: dict[str, Any]) -> str:
    normalized = str(event_type or "").strip()
    title = str(payload.get("title") or "").strip()
    project_name = str(payload.get("project_name") or payload.get("projectName") or "").strip()
    project_path = str(payload.get("project_path") or payload.get("workspace_dir") or "").strip()
    game_name = str(payload.get("game_name") or payload.get("gameName") or "").strip()
    platform = str(payload.get("platform") or payload.get("app_name") or payload.get("tool_name") or payload.get("process_name") or "").strip()
    subject = title or project_name or game_name or platform or normalized or "unknown"

    if normalized == "coding_context_detected":
        subject = project_name or basename_from_path(project_path) or title or "coding"
        prefix = "workspace"
    elif normalized == "game_context_detected":
        subject = game_name or title or "game"
        prefix = "game"
    elif normalized in {"video_context_detected", "active_url_changed", "reading_page_detected"}:
        subject = str(payload.get("url") or payload.get("canonical_url") or title or "content").strip()
        prefix = "content"
    elif normalized == "meeting_context_detected":
        prefix = "meeting"
    elif normalized == "document_context_detected":
        prefix = "document"
    else:
        prefix = "activity"

    compact = re.sub(r"\s+", " ", subject).strip().lower()
    compact = compact.replace("\\", "/")
    if "/" in compact:
        compact = compact.rstrip("/").split("/")[-1] or compact
    return f"{prefix}:{trim_content(compact, 120)}"


def blink_once_probe(shared_mode: dict[str, Any]) -> dict[str, Any] | None:
    if str(shared_mode.get("mode_id") or "").strip() != "content_mode":
        return None
    status = str(shared_mode.get("status") or "").strip()
    if status not in {"active", "content"}:
        return None
    expires_at = int(float(shared_mode.get("expires_at") or 0))
    if expires_at and expires_at <= int(time.time() * 1000):
        return None
    visual = shared_mode.get("visual") if isinstance(shared_mode.get("visual"), dict) else {}
    if visual and not bool(visual.get("enabled")):
        return None
    enhancers = shared_mode.get("enhancers") if isinstance(shared_mode.get("enhancers"), dict) else {}
    probes = enhancers.get("probes") if isinstance(enhancers.get("probes"), list) else []
    probe = next(
        (entry for entry in probes if isinstance(entry, dict) and str(entry.get("tool") or "").strip() == "blink_once"),
        None,
    )
    if isinstance(probe, dict) and visual:
        interval_ms = int(float(visual.get("interval_ms") or 0))
        last_blink_at = int(float(visual.get("last_blink_at") or 0))
        return {**probe, "cooldown_ms": interval_ms or probe.get("cooldown_ms"), "last_blink_at": last_blink_at}
    return probe


def visual_summary_from_payload(payload: dict[str, Any]) -> str:
    summary = str(payload.get("screen_description") or "").strip()
    if summary:
        return summary
    probe_context = payload.get("probe_context") if isinstance(payload.get("probe_context"), dict) else {}
    vision = probe_context.get("vision") if isinstance(probe_context.get("vision"), dict) else {}
    return str(vision.get("summary") or "").strip()


def build_context_freshness(
    event_type: str,
    payload: dict[str, Any],
    *,
    selected_enhancer: str = "",
    visual_captured: bool = False,
) -> dict[str, Any]:
    now_seconds = time.time()
    now = int(now_seconds * 1000)
    scene_key = context_scene_key(event_type, payload)
    previous = CONTEXT_FRESHNESS_BY_SCENE.get(scene_key) if isinstance(CONTEXT_FRESHNESS_BY_SCENE.get(scene_key), dict) else {}
    visual_summary = visual_summary_from_payload(payload)
    previous_visual_at = int(previous.get("last_visual_at") or 0)
    last_visual_at = now if visual_captured or visual_summary else previous_visual_at
    last_seen_at = now
    previous_enriched_at = int(previous.get("last_enriched_at") or 0)
    last_enriched_at = now if selected_enhancer else previous_enriched_at
    min_interval = float(VISION_MIN_INTERVAL_BY_EVENT.get(event_type, 60.0)) * 1000.0
    stale_visual = bool(last_visual_at and now - last_visual_at >= min_interval)
    missing_visual = not last_visual_at
    needs_refresh = missing_visual or stale_visual
    refresh_reason = "missing_visual_evidence" if missing_visual else "stale_visual_evidence" if stale_visual else ""
    evidence_level = "visual" if last_visual_at else "metadata"
    staleness_ms = max(0, now - int(last_visual_at or previous.get("last_seen_at") or now))
    freshness = {
        "scene_key": scene_key,
        "scene_type": event_type,
        "last_seen_at": last_seen_at,
        "last_enriched_at": last_enriched_at,
        "last_visual_at": last_visual_at,
        "previous_enriched_at": previous_enriched_at,
        "evidence_level": evidence_level,
        "staleness_ms": int(staleness_ms),
        "needs_refresh": bool(needs_refresh),
        "refresh_reason": refresh_reason,
        "selected_enhancer": selected_enhancer,
    }
    CONTEXT_FRESHNESS_BY_SCENE[scene_key] = freshness
    return freshness


def attach_context_freshness(event: dict[str, Any], freshness: dict[str, Any]) -> dict[str, Any]:
    payload = event.get("payload") if isinstance(event.get("payload"), dict) else {}
    payload["context_freshness"] = freshness
    event["payload"] = payload
    return event


def persist_shared_visual_blink(shared_mode: dict[str, Any], blinked_at: int) -> None:
    if str(shared_mode.get("mode_id") or "").strip() != "content_mode":
        return
    for path in (
        runtime_data_dir() / "weft-proactive-state.json",
        context_engine_data_dir() / "weft-proactive-state.json",
    ):
        state = read_json_file(path)
        if not isinstance(state, dict):
            continue
        shared = state.get("shared") if isinstance(state.get("shared"), dict) else state
        if str(shared.get("mode_id") or "").strip() != "content_mode":
            continue
        visual = shared.get("visual") if isinstance(shared.get("visual"), dict) else {}
        shared["visual"] = {**visual, "last_blink_at": int(blinked_at)}
        shared["last_seen_at"] = int(blinked_at)
        shared["expires_at"] = int(blinked_at) + CONTENT_MODE_TTL_MS
        if isinstance(state.get("shared"), dict):
            state["shared"] = shared
        else:
            state = shared
        write_json_file(path, state)


def active_shared_content_mode() -> dict[str, Any]:
    if not content_mode_enabled():
        return {}
    shared_state = read_shared_mode_state()
    shared = shared_state.get("shared") if isinstance(shared_state.get("shared"), dict) else shared_state
    shared = shared if isinstance(shared, dict) else {}
    return shared if isinstance(blink_once_probe(shared), dict) else {}


def content_mode_visual_interval_seconds(shared_mode: dict[str, Any]) -> float:
    visual = shared_mode.get("visual") if isinstance(shared_mode.get("visual"), dict) else {}
    interval_ms = int(float(visual.get("interval_ms") or 0))
    if interval_ms <= 0:
        probe = blink_once_probe(shared_mode) or {}
        interval_ms = int(float(probe.get("cooldown_ms") or 20_000))
    return max(1.0, interval_ms / 1000.0)


def content_mode_visual_due(shared_mode: dict[str, Any], now_seconds: float | None = None) -> bool:
    now_seconds = float(now_seconds if now_seconds is not None else time.time())
    probe = blink_once_probe(shared_mode)
    if not isinstance(probe, dict):
        return False
    last_blink_at = float(probe.get("last_blink_at") or 0) / 1000.0
    return now_seconds - last_blink_at >= content_mode_visual_interval_seconds(shared_mode)


def content_mode_visual_blink_once(shared_mode: dict[str, Any] | None = None, *, debug: bool = False) -> dict[str, Any]:
    shared = shared_mode if isinstance(shared_mode, dict) else active_shared_content_mode()
    if not shared or not content_mode_visual_due(shared):
        return {"blinked": False, "reason": "not_due_or_inactive"}
    if debug and VISION_PROBE_LOCK.locked():
        return {"blinked": False, "reason": "probe_busy", "debug": True}
    try:
        tool_result = run_blink_once_direct({
            "task": "Describe the current screen for active content mode in one short Chinese sentence.",
            "force_system": False,
            "fast_mode": True,
        })
        if not tool_result:
            return {"blinked": False, "reason": "probe_busy_or_failed", **({"debug": True} if debug else {})}
        desc = extract_tool_summary(tool_result)
        timings = tool_result.get("timings") if isinstance(tool_result.get("timings"), dict) else {}
        ocr_text = str(tool_result.get("ocr_text") or "").strip()
        encoded_ocr = tool_result.get("ocr_text_b64")
        if not ocr_text and isinstance(encoded_ocr, str) and encoded_ocr.strip():
            try:
                ocr_text = base64.b64decode(encoded_ocr).decode("utf-8").strip()
            except Exception:
                ocr_text = ""
        completed_at = time.time()
        blinked_at = int(completed_at * 1000)
        persist_shared_visual_blink(shared, blinked_at)
        VISION_LAST_TIME_BY_EVENT[str(shared.get("scene_key") or "content_mode")] = completed_at
        if not desc:
            return {"blinked": True, "summary": "", "blinked_at": blinked_at, "warning": "empty_blink_result", "timings": timings}
        try:
            call_companion_context_event(
                "companion-core",
                {
                    "event_type": "blink_signal_detected",
                    "payload": {
                        "title": str(shared.get("scene_key") or "content_mode"),
                        "signal_class": "content_mode_visual",
                        "signal": "blink_once",
                        "screen_description": desc,
                        "ocr_text": trim_content(ocr_text, 4000),
                        "probe_timings": timings,
                        "source": "context-engine-content-mode-visual-loop",
                        "content_mode": shared,
                    },
                    "session_id": "content-mode-visual-loop",
                    "time_tags": physical_time_tags(),
                },
            )
        except Exception as error:
            LOGGER.warning("content mode visual context dispatch failed: %s", error)
        return {"blinked": True, "summary": desc, "ocr_text_len": len(ocr_text), "blinked_at": blinked_at, "timings": timings}
    except Exception as error:
        LOGGER.warning("content mode visual blink failed: %s", error)
        return {"blinked": False, "reason": "blink_failed"}


def enrich_event_with_vision(event: dict[str, Any] | None) -> dict[str, Any] | None:
    """Add screen_description to event payload via the formal blink_once tool."""
    if not event or not isinstance(event, dict):
        return event
    event_type = str(event.get("event_type") or "").strip()
    payload = event.get("payload") if isinstance(event.get("payload"), dict) else {}
    freshness = build_context_freshness(event_type, payload)
    if event_type not in VISION_EVENT_TYPES:
        return attach_context_freshness(event, freshness)
    if not content_mode_enabled():
        return attach_context_freshness(event, freshness)
    shared_mode = read_shared_mode_state()
    shared_payload = shared_mode.get("shared") if isinstance(shared_mode.get("shared"), dict) else shared_mode
    shared_mode = shared_payload if isinstance(shared_payload, dict) else {}
    probe = blink_once_probe(shared_mode)
    if not isinstance(probe, dict):
        return attach_context_freshness(event, freshness)
    if not freshness.get("needs_refresh"):
        return attach_context_freshness(event, freshness)
    now = time.time()
    min_interval = float(probe.get("cooldown_ms") or VISION_MIN_INTERVAL_BY_EVENT.get(event_type, 60.0) * 1000) / 1000.0
    last_time = max(
        float(VISION_LAST_TIME_BY_EVENT.get(str(freshness.get("scene_key") or event_type), 0.0)),
        float(probe.get("last_blink_at") or 0) / 1000.0,
    )
    if now - last_time < min_interval:
        return attach_context_freshness(event, freshness)

    try:
        tool_result = run_blink_once_direct({
            "task": f"Describe the current screen for {event_type} in one short Chinese sentence.",
            "force_system": False,
        })
        desc = extract_tool_summary(tool_result)
        timings = tool_result.get("timings") if isinstance(tool_result.get("timings"), dict) else {}
        ocr_text = str(tool_result.get("ocr_text") or "").strip()
        encoded_ocr = tool_result.get("ocr_text_b64")
        if not ocr_text and isinstance(encoded_ocr, str) and encoded_ocr.strip():
            try:
                ocr_text = base64.b64decode(encoded_ocr).decode("utf-8").strip()
            except Exception:
                ocr_text = ""
        if desc:
            payload["screen_description"] = desc
            payload["ocr_text"] = trim_content(ocr_text, 4000)
            probe_context = payload.get("probe_context") if isinstance(payload.get("probe_context"), dict) else {}
            probe_context["vision"] = {
                "summary": desc,
                "captured_at": current_timestamp_iso(),
                "source": "tool:blink_once",
                "tool": "blink_once",
                "event_type": event_type,
                "timings": timings,
                "ocr_text_len": len(ocr_text),
            }
            payload["probe_context"] = probe_context
            event["payload"] = payload
            freshness = build_context_freshness(event_type, payload, selected_enhancer="blink_once", visual_captured=True)
            VISION_LAST_TIME_BY_EVENT[str(freshness.get("scene_key") or event_type)] = now
            persist_shared_visual_blink(shared_mode, int(now * 1000))
    except Exception:
        pass
    attach_context_freshness(event, freshness)
    return event


def event_signature(event: dict[str, Any]) -> str:
    payload = event.get("payload") if isinstance(event.get("payload"), dict) else {}
    stable_payload = {**payload}
    stable_payload.pop("context_freshness", None)
    stable_payload.pop("probe_context", None)
    stable_payload.pop("screen_description", None)
    return json.dumps(
        {
            "source": event.get("source"),
            "event_type": event.get("event_type"),
            "payload": stable_payload,
        },
        ensure_ascii=False,
        sort_keys=True,
    )


def collect_pending_source_events(input_payload: dict[str, Any]) -> dict[str, Any]:
    workspace_dir = Path(
        str(input_payload.get("workspace_dir") or default_workspace_dir())
    ).resolve()
    global_sense_dir = Path(
        str(input_payload.get("global_sense_dir") or default_global_sense_dir())
    ).resolve()
    events: list[dict[str, Any]] = []

    if not read_terminal_context(workspace_dir):
        generate_terminal_context(workspace_dir)

    events_dir = global_sense_dir / "events"
    raw_status = refresh_pc_status(global_sense_dir)
    remember_last_non_host_pc_status(raw_status)
    refresh_activity_windows(raw_status, global_sense_dir)
    if events_dir.exists() and events_dir.is_dir():
        for entry in sorted(events_dir.glob("*.json")):
            parsed_event_file = read_json_file(entry)
            if not parsed_event_file:
                continue
            event = normalize_event_payload(parsed_event_file)
            if not event:
                continue
            if is_background_focus_event(event, raw_status):
                continue
            signature = event_signature(event)
            if signature in SEEN_EVENT_SIGNATURES:
                continue
            SEEN_EVENT_SIGNATURES.add(signature)
            events.append(enrich_event_with_web_context(event))

    clipboard_event = collect_clipboard_event()
    if clipboard_event:
        signature = event_signature(clipboard_event)
        if signature not in SEEN_EVENT_SIGNATURES:
            SEEN_EVENT_SIGNATURES.add(signature)
            events.append(clipboard_event)

    if raw_status:
        event = enrich_pc_status_event_with_workspace_context(
            map_pc_status_to_event(raw_status),
            workspace_dir,
        )
        if event:
            signature = event_signature(event)
            if signature not in SEEN_EVENT_SIGNATURES:
                SEEN_EVENT_SIGNATURES.add(signature)
                event = enrich_event_with_vision(event)
                events.append(event)

    return {"events": events}


def get_current_activity(input_payload: dict[str, Any]) -> dict[str, Any]:
    workspace_dir = Path(
        str(input_payload.get("workspace_dir") or default_workspace_dir())
    ).resolve()
    global_sense_dir = Path(
        str(input_payload.get("global_sense_dir") or default_global_sense_dir())
    ).resolve()
    raw = {}
    current_activity = None

    parsed = refresh_pc_status(global_sense_dir)
    remember_last_non_host_pc_status(parsed)
    refresh_activity_windows(parsed, global_sense_dir)
    if isinstance(parsed, dict):
        detail = parsed.get("detail") if isinstance(parsed.get("detail"), dict) else {}
        fg_window = str(detail.get("fg_window") or parsed.get("fg_title") or "").strip()
        process_name = str(detail.get("fg_process") or parsed.get("foreground") or "").strip().lower()
        source_status = parsed
        if is_host_foreground_window(process_name, fg_window) and isinstance(LAST_NON_HOST_PC_STATUS, dict):
            source_status = LAST_NON_HOST_PC_STATUS

        raw = source_status
        current_activity = enrich_pc_status_event_with_workspace_context(
            map_pc_status_to_event(source_status),
            workspace_dir,
        )
        current_activity = prefer_browser_context_event(
            source_status,
            current_activity,
            global_sense_dir,
        )
        current_activity = enrich_event_with_vision(current_activity)
        if isinstance(current_activity, dict):
            recent_activity = format_recent_activity_summary(global_sense_dir)
            payload = current_activity.get("payload") if isinstance(current_activity.get("payload"), dict) else {}
            current_activity = {
                **current_activity,
                "payload": {
                    **payload,
                    "recent_activity": recent_activity,
                },
                "raw": source_status,
            }

    return {
        "current_activity": current_activity,
        "workspace_dir": str(workspace_dir),
        "global_sense_dir": str(global_sense_dir),
        "raw": raw,
    }


def poll_sources(input_payload: dict[str, Any]) -> dict[str, Any]:
    collected = collect_pending_source_events(input_payload)
    events = collected.get("events") if isinstance(collected.get("events"), list) else []
    normalized_events = []
    for entry in events:
        if not isinstance(entry, dict):
            continue
        normalized_events.append({**entry, "time_tags": physical_time_tags()})
    if normalized_events:
        with PENDING_EVENTS_LOCK:
            PENDING_EVENTS.extend(normalized_events)
    return {"events": normalized_events}


def dispatch_companion_context_events(input_payload: dict[str, Any]) -> dict[str, Any]:
    companion_package = str(input_payload.get("companion_plugin") or "companion-core").strip() or "companion-core"
    payload = {
        "context_engine_plugin": str(input_payload.get("context_engine_plugin") or "context-engine").strip() or "context-engine",
        "memory_plugin": str(input_payload.get("memory_plugin") or "memory-runtime").strip(),
        "session_id": str(input_payload.get("session_id") or "").strip(),
        "workspace_id": str(input_payload.get("workspace_id") or "").strip(),
        "workspace_dir": str(input_payload.get("workspace_dir") or "").strip(),
        "event_type": "",
        "payload": {},
        "time_tags": physical_time_tags(),
    }
    return call_companion_context_event(companion_plugin, payload)


def poll_tick(input_payload: dict[str, Any]) -> dict[str, Any]:
    result = poll_sources(input_payload)
    events = result.get("events") if isinstance(result.get("events"), list) else []
    if events:
        try:
            dispatch_companion_context_events(input_payload)
        except Exception as error:
            LOGGER.warning("companion context dispatch failed: %s", error)
    return result


def drain_pending_events(_input_payload: dict[str, Any]) -> dict[str, Any]:
    with PENDING_EVENTS_LOCK:
        events = list(PENDING_EVENTS)
        PENDING_EVENTS.clear()
    return {"events": events}


def context_manifests_root() -> Path:
    return plugin_dir() / "context-manifests"


def read_proactive_manifests() -> list[dict[str, Any]]:
    manifests: list[dict[str, Any]] = []
    for file_name in PROACTIVE_MANIFEST_FILES:
        manifest_path = context_manifests_root() / file_name
        if not manifest_path.exists():
            continue
        parsed = read_json_file(manifest_path)
        if not parsed:
            continue
        parsed["_manifest_path"] = str(manifest_path)
        manifests.append(parsed)
    return manifests


def resolve_match_script(manifest: dict[str, Any]) -> Path | None:
    manifest_path = Path(str(manifest.get("_manifest_path") or "")).resolve()
    if not manifest_path.exists():
        return None

    scripts = manifest.get("scripts") if isinstance(manifest.get("scripts"), dict) else {}
    match_script = str(scripts.get("match") or "").strip()
    if not match_script:
        return None

    package_dir = str(manifest.get("packageDir") or "").strip()
    base_dir = manifest_path.parent
    if package_dir:
        base_dir = (manifest_path.parent / package_dir).resolve()
    return (base_dir / match_script).resolve()


def run_match_script(script_path: Path, event_payload: dict[str, Any]) -> dict[str, Any] | None:
    with tempfile.TemporaryDirectory(prefix="weft-context-event-") as temp_dir:
        event_path = Path(temp_dir) / "event.json"
        event_path.write_text(json.dumps(event_payload, ensure_ascii=False), encoding="utf-8")
        compat_script_path = Path(temp_dir) / script_path.name
        compat_script_path.write_text(
            "\ufeff" + script_path.read_text("utf-8"),
            encoding="utf-8",
        )
        result = subprocess.run(
            [
                "powershell.exe",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "[Console]::InputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $OutputEncoding = [System.Text.Encoding]::UTF8; &",
                str(compat_script_path),
                "-EventPath",
                str(event_path),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=os.environ.copy(),
            check=False,
        )
        if result.returncode != 0:
            return None

        stdout = result.stdout.strip()
        if not stdout:
            return None

        try:
            parsed = json.loads(stdout)
        except Exception:
            return None
        return parsed if isinstance(parsed, dict) else None


def build_suggestion(
    manifest: dict[str, Any],
    match_payload: dict[str, Any],
    event_type: str,
    event_payload: dict[str, Any],
    session_id: str | None,
) -> dict[str, Any]:
    return {
        "sessionId": session_id or None,
        "eventId": str(uuid.uuid4()),
        "skillId": str(match_payload.get("skill_id") or manifest.get("id") or "").strip(),
        "skillName": str(manifest.get("name") or manifest.get("id") or "").strip(),
        "eventType": str(match_payload.get("event_type") or event_type).strip(),
        "eventPayload": match_payload.get("event_payload") if isinstance(match_payload.get("event_payload"), dict) else event_payload,
        "timestamp": now_ms(),
    }


def handle_event(input_payload: dict[str, Any]) -> dict[str, Any]:
    event_type = str(input_payload.get("event_type") or "").strip()
    payload = input_payload.get("payload") if isinstance(input_payload.get("payload"), dict) else {}
    session_id = str(input_payload.get("session_id") or "").strip() or None
    if not event_type:
        return {"suggestions": []}

    event_body = {
        "type": event_type,
        "event_type": event_type,
        "payload": payload,
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime()),
    }
    suggestions: list[dict[str, Any]] = []

    for manifest in read_proactive_manifests():
        event_types = manifest.get("eventTypes") if isinstance(manifest.get("eventTypes"), list) else []
        if event_type not in [str(entry).strip() for entry in event_types]:
            continue

        script_path = resolve_match_script(manifest)
        if not script_path or not script_path.exists():
            continue

        match_payload = run_match_script(script_path, event_body)
        if not match_payload:
            continue
        if str(match_payload.get("decision") or "").strip() != "suggest":
            continue

        suggestions.append(
            build_suggestion(manifest, match_payload, event_type, payload, session_id)
        )

    return {"suggestions": suggestions}


def ingest_external_event(body: dict[str, Any]) -> dict[str, Any]:
    event_type = str(body.get("event_type") or body.get("type") or "").strip()
    payload = body.get("payload") if isinstance(body.get("payload"), dict) else {}
    session_id = str(body.get("session_id") or "").strip()
    if not event_type or not payload:
        raise ValueError("invalid external event payload")

    event = {
        "event_type": event_type,
        "payload": payload,
        "session_id": session_id or None,
        "source": "webhook",
        "timestamp": str(body.get("timestamp") or "").strip() or current_timestamp_iso(),
    }
    if is_background_focus_event(event, refresh_pc_status(default_global_sense_dir())):
        return {"accepted": False, "event_type": event_type, "reason": "background_browser_event"}
    signature = event_signature(event)
    if signature not in SEEN_EVENT_SIGNATURES:
        SEEN_EVENT_SIGNATURES.add(signature)
        with PENDING_EVENTS_LOCK:
            PENDING_EVENTS.append(enrich_event_with_web_context(event))
    return {"accepted": True, "event_type": event_type}


def _parse_repeated_env_args(name: str) -> list[str]:
    raw = str(os.environ.get(name, "") or "").strip()
    if not raw:
        return []
    values: list[str] = []
    for line in raw.splitlines():
        text = line.strip()
        if not text:
            continue
        parsed = shlex.split(text, posix=False)
        values.extend(item.strip().strip("\"'") for item in parsed if str(item).strip())
    return values


def _audio_enabled() -> bool:
    value = str(os.environ.get("WEFT_CONTEXT_ENGINE_AUDIO_ENABLED", "0") or "").strip().lower()
    return value in {"1", "true", "yes", "on"}


def _audio_runner_python() -> str:
    return str(os.environ.get("WEFT_CONTEXT_ENGINE_AUDIO_RUNNER_PYTHON") or sys.executable).strip() or sys.executable


def _packaged_python() -> str:
    pointer_path = plugin_dir() / "pydeps" / ".shared-python-exe"
    if not pointer_path.exists():
        return ""
    try:
        python_path = Path(pointer_path.read_text("utf-8").strip()).resolve()
    except Exception:
        return ""
    return str(python_path) if python_path.exists() else ""


def _audio_recorder_python() -> str:
    return (
        str(os.environ.get("WEFT_CONTEXT_ENGINE_AUDIO_RECORDER_PYTHON") or "").strip()
        or _packaged_python()
        or sys.executable
    )


def _audio_producer_python() -> str:
    configured = str(os.environ.get("WEFT_CONTEXT_ENGINE_AUDIO_PRODUCER_PYTHON") or "").strip()
    if configured:
        return configured
    packaged_python = _packaged_python()
    if packaged_python:
        return packaged_python
    venv_python = runtime_root() / "tmp" / "zipformer-audio-tagging" / "venv" / "Scripts" / "python.exe"
    if venv_python.exists():
        return str(venv_python)
    return sys.executable


def _audio_runner_script() -> Path:
    return plugin_dir() / "scripts" / "audio-event-live-runner.py"


def _audio_recorder_script() -> Path:
    return plugin_dir() / "scripts" / "audio-segment-recorder.py"


def _audio_producer_script() -> Path:
    return plugin_dir() / "scripts" / "audio-tagging-producer.py"


def _audio_model_path() -> Path:
    configured = str(os.environ.get("WEFT_CONTEXT_ENGINE_AUDIO_MODEL") or "").strip()
    if configured:
        return Path(configured).resolve()
    bundled_model = plugin_dir() / "models" / "zipformer-audio-tagging" / "model.int8.onnx"
    if bundled_model.exists():
        return bundled_model.resolve()
    return (
        runtime_root()
        / "tmp"
        / "zipformer-audio-tagging"
        / "extract"
        / "sherpa-onnx-zipformer-small-audio-tagging-2024-04-15"
        / "model.int8.onnx"
    ).resolve()


def _audio_labels_path() -> Path:
    configured = str(os.environ.get("WEFT_CONTEXT_ENGINE_AUDIO_LABELS") or "").strip()
    if configured:
        return Path(configured).resolve()
    bundled_labels = plugin_dir() / "models" / "zipformer-audio-tagging" / "class_labels_indices.csv"
    if bundled_labels.exists():
        return bundled_labels.resolve()
    return (
        runtime_root()
        / "tmp"
        / "zipformer-audio-tagging"
        / "extract"
        / "sherpa-onnx-zipformer-small-audio-tagging-2024-04-15"
        / "class_labels_indices.csv"
    ).resolve()


def _audio_segments_dir() -> Path:
    configured = str(os.environ.get("WEFT_CONTEXT_ENGINE_AUDIO_SEGMENTS_DIR") or "").strip()
    if configured:
        path = Path(configured).resolve()
    else:
        path = context_engine_data_dir() / "audio-segments"
    path.mkdir(parents=True, exist_ok=True)
    return path


def build_audio_sidecar_command() -> list[str] | None:
    runner_script = _audio_runner_script()
    recorder_script = _audio_recorder_script()
    producer_script = _audio_producer_script()
    model_path = _audio_model_path()
    labels_path = _audio_labels_path()

    required_paths = [runner_script, recorder_script, producer_script, model_path, labels_path]
    missing = [str(path) for path in required_paths if not path.exists()]
    if missing:
        LOGGER.info("audio sidecar disabled: missing assets %s", ", ".join(missing))
        return None

    command = [
        _audio_runner_python(),
        str(runner_script),
        "--recorder-python",
        _audio_recorder_python(),
        "--recorder-script",
        str(recorder_script),
        "--producer-python",
        _audio_producer_python(),
        "--producer-script",
        str(producer_script),
        "--segments-dir",
        str(_audio_segments_dir()),
        "--model",
        str(model_path),
        "--labels",
        str(labels_path),
        "--webhook-url",
        f"http://127.0.0.1:{PORT}/webhook",
        "--session-id",
        "",
    ]
    command.extend(_parse_repeated_env_args("WEFT_CONTEXT_ENGINE_AUDIO_RUNNER_EXTRA_ARG"))
    command.extend(_parse_repeated_env_args("WEFT_CONTEXT_ENGINE_AUDIO_RECORDER_EXTRA_ARG"))
    command.extend(_parse_repeated_env_args("WEFT_CONTEXT_ENGINE_AUDIO_PRODUCER_EXTRA_ARG"))
    return command


def ensure_audio_sidecar_started() -> bool:
    global AUDIO_SIDECAR_PROCESS
    if not _audio_enabled():
        return False

    with AUDIO_SIDECAR_LOCK:
        if AUDIO_SIDECAR_PROCESS and AUDIO_SIDECAR_PROCESS.poll() is None:
            return True

        command = build_audio_sidecar_command()
        if not command:
            return False

        try:
            AUDIO_SIDECAR_PROCESS = subprocess.Popen(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                errors="replace",
                cwd=str(runtime_root()),
            )
        except Exception as error:
            LOGGER.warning("audio sidecar failed to start: %s", error)
            AUDIO_SIDECAR_PROCESS = None
            return False

        LOGGER.info("audio sidecar started: %s", " ".join(command[:6]))
        return True


def stop_audio_sidecar() -> None:
    global AUDIO_SIDECAR_PROCESS
    with AUDIO_SIDECAR_LOCK:
        process = AUDIO_SIDECAR_PROCESS
        AUDIO_SIDECAR_PROCESS = None
    if process is None:
        return
    if process.poll() is not None:
        return
    try:
        process.terminate()
        process.wait(timeout=5)
    except Exception:
        try:
            process.kill()
            process.wait(timeout=5)
        except Exception:
            pass


def poll_loop() -> None:
    while not POLL_STOP_EVENT.is_set():
        try:
            poll_tick(read_active_context_state())
        except Exception as error:
            LOGGER.exception("context source polling failed: %s", error)
        POLL_STOP_EVENT.wait(POLL_INTERVAL_SECONDS)


def content_mode_visual_loop() -> None:
    while not POLL_STOP_EVENT.is_set():
        try:
            if content_mode_enabled():
                content_mode_visual_blink_once()
        except Exception as error:
            LOGGER.exception("content mode visual loop failed: %s", error)
        POLL_STOP_EVENT.wait(CONTENT_MODE_VISUAL_LOOP_WAIT_SECONDS)


def ensure_poll_thread_started() -> None:
    global POLL_THREAD, CONTENT_MODE_VISUAL_THREAD
    if POLL_THREAD and POLL_THREAD.is_alive():
        poll_running = True
    else:
        poll_running = False
    POLL_STOP_EVENT.clear()
    if not poll_running:
        POLL_THREAD = threading.Thread(
            target=poll_loop,
            name="context-engine-poll",
            daemon=True,
        )
        POLL_THREAD.start()
    if not content_mode_enabled():
        return
    if CONTENT_MODE_VISUAL_THREAD and CONTENT_MODE_VISUAL_THREAD.is_alive():
        return
    CONTENT_MODE_VISUAL_THREAD = threading.Thread(
        target=content_mode_visual_loop,
        name="context-engine-content-mode-visual",
        daemon=True,
    )
    CONTENT_MODE_VISUAL_THREAD.start()


def dispatch_action(action: str, data: dict[str, Any]) -> dict[str, Any]:
    normalized = action.strip()
    if normalized == "poll_sources":
        return ok(poll_sources(data))
    if normalized == "drain_pending_events":
        return ok(drain_pending_events(data))
    if normalized == "get_current_activity":
        return ok(get_current_activity(data))
    if normalized == "handle_event":
        return ok(handle_event(data))
    if normalized == "ingest_external_event":
        return ok(ingest_external_event(data))
    if normalized == "debug_content_mode_visual_blink_once":
        return ok(content_mode_visual_blink_once(data.get("shared_mode") if isinstance(data.get("shared_mode"), dict) else None, debug=True))
    return err(f"unknown action: {normalized}")


class ContextEngineHandler(BaseHTTPRequestHandler):
    server_version = "weft-context-engine-runtime/0.1.0"

    def do_GET(self) -> None:
        if self.path == "/health":
            self._write_json(
                HTTPStatus.OK,
                {
                    "ok": True,
                    "plugin": "context-engine",
                    "runtime": "service",
                    "port": PORT,
                    "plugin_dir": str(plugin_dir()),
                },
            )
            return

        self._write_json(HTTPStatus.NOT_FOUND, {"error": f"Unknown path: {self.path}"})

    def do_POST(self) -> None:
        if self.path == "/shutdown":
            self._write_json(HTTPStatus.OK, {"ok": True, "message": "shutdown requested"})
            self.close_connection = True
            if HTTPD is not None:
                HTTPD.shutdown()
            return

        if self.path != "/webhook":
            self._write_json(HTTPStatus.NOT_FOUND, {"error": f"Unknown path: {self.path}"})
            return

        body = self._read_json_body()
        action = str(body.get("action", "")).strip()
        data = body.get("data", {})
        if not isinstance(data, dict):
            data = {}

        if action == "ingest_external_event":
            try:
                self._write_json(HTTPStatus.OK, ok(ingest_external_event(data)))
            except Exception as error:
                LOGGER.exception("context engine external action ingest failed")
                self._write_json(HTTPStatus.OK, err(str(error)))
            return

        if not action:
            event_type = str(body.get("event_type") or "").strip()
            raw_type = str(body.get("type") or "").strip()
            payload = body.get("payload") if isinstance(body.get("payload"), dict) else None
            if payload is not None and (event_type or raw_type == "skill_event"):
                if not event_type and raw_type == "skill_event":
                    event_type = raw_type
                try:
                    self._write_json(HTTPStatus.OK, ok(ingest_external_event({
                        "event_type": event_type,
                        "payload": payload,
                        "session_id": body.get("session_id"),
                        "timestamp": body.get("timestamp"),
                    })))
                except Exception as error:
                    LOGGER.exception("context engine external webhook ingest failed")
                    self._write_json(HTTPStatus.OK, err(str(error)))
                return

        try:
            self._write_json(HTTPStatus.OK, dispatch_action(action, data))
        except Exception as error:
            LOGGER.exception("context engine action failed: %s", action)
            self._write_json(HTTPStatus.OK, err(str(error)))

    def log_message(self, format: str, *args: Any) -> None:
        LOGGER.info("%s - %s", self.address_string(), format % args)

    def _read_json_body(self) -> dict[str, Any]:
        transfer_encoding = str(self.headers.get("Transfer-Encoding") or "").strip().lower()
        if "chunked" in transfer_encoding:
            raw = self._read_chunked_body()
            if not raw:
                return {}
            try:
                parsed = json.loads(raw.decode("utf-8"))
                return parsed if isinstance(parsed, dict) else {}
            except json.JSONDecodeError:
                return {}

        raw_length = self.headers.get("Content-Length", "0").strip()
        try:
            length = int(raw_length or "0")
        except ValueError:
            length = 0

        raw = self.rfile.read(length) if length > 0 else b"{}"
        if not raw:
            return {}
        try:
            parsed = json.loads(raw.decode("utf-8"))
            return parsed if isinstance(parsed, dict) else {}
        except json.JSONDecodeError:
            return {}

    def _read_chunked_body(self) -> bytes:
        chunks: list[bytes] = []

        while True:
            line = self.rfile.readline()
            if not line:
                break

            chunk_size_raw = line.strip().split(b";", 1)[0]
            if not chunk_size_raw:
                continue

            try:
                chunk_size = int(chunk_size_raw, 16)
            except ValueError:
                break

            if chunk_size == 0:
                # Consume trailing CRLF plus optional trailer headers.
                while True:
                    trailer = self.rfile.readline()
                    if trailer in (b"", b"\r\n", b"\n"):
                        break
                break

            chunk = self.rfile.read(chunk_size)
            if chunk:
                chunks.append(chunk)

            # Consume the CRLF after each chunk.
            self.rfile.read(2)

        return b"".join(chunks)

    def _write_json(self, status: HTTPStatus, payload: dict[str, Any]) -> None:
        encoded = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status.value)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)


def main() -> None:
    global HTTPD
    HTTPD = ThreadingHTTPServer(("127.0.0.1", PORT), ContextEngineHandler)
    ensure_poll_thread_started()
    ensure_audio_sidecar_started()
    LOGGER.info("context-engine runtime listening on http://127.0.0.1:%s", PORT)
    try:
        HTTPD.serve_forever(poll_interval=0.5)
    finally:
        POLL_STOP_EVENT.set()
        stop_audio_sidecar()
        HTTPD.server_close()


if __name__ == "__main__":
    main()
