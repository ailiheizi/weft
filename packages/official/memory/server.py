#!/usr/bin/env python3
"""
Hermes-style curated memory runtime for WEFT.

This is intentionally a standalone service runtime prototype:
- file-backed MEMORY.md + USER.md persistence
- frozen system-prompt snapshot captured at service start
- live add/replace/remove/read operations
- lightweight injection/exfiltration scanning
- Windows-safe file locking
"""

from __future__ import annotations

import json
import logging
import os
import re
import tempfile
import time
import traceback
from contextlib import contextmanager
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any, Dict, List, Optional
from urllib.parse import parse_qs, urlparse

from hermes_port.builtin_memory_provider import BuiltinMemoryProvider
from hermes_port.holographic_provider import HolographicMemoryProvider
from hermes_port.memory_manager import MemoryManager

if os.name == "nt":
    import msvcrt
else:
    import fcntl


LOGGER = logging.getLogger("memory-runtime")
logging.basicConfig(
    level=os.environ.get("WEFT_MEMORY_RUNTIME_LOG_LEVEL", "INFO").upper(),
    format="[memory-runtime] %(message)s",
)

PORT = int(os.environ.get("WEFT_MEMORY_RUNTIME_PORT", "43129"))
ENTRY_DELIMITER = "\n---\n"
MEMORY_CHAR_LIMIT = int(os.environ.get("WEFT_MEMORY_RUNTIME_MEMORY_LIMIT", "2200"))
USER_CHAR_LIMIT = int(os.environ.get("WEFT_MEMORY_RUNTIME_USER_LIMIT", "1375"))

_MEMORY_THREAT_PATTERNS = [
    (r"ignore\s+(previous|all|above|prior)\s+instructions", "prompt_injection"),
    (r"you\s+are\s+now\s+", "role_hijack"),
    (r"do\s+not\s+tell\s+the\s+user", "deception_hide"),
    (r"system\s+prompt\s+override", "sys_prompt_override"),
    (r"disregard\s+(your|all|any)\s+(instructions|rules|guidelines)", "disregard_rules"),
    (r"authorized_keys", "ssh_backdoor"),
    (r"curl\s+[^\n]*\$\{?\w*(KEY|TOKEN|SECRET|PASSWORD|CREDENTIAL|API)", "exfil_curl"),
    (r"wget\s+[^\n]*\$\{?\w*(KEY|TOKEN|SECRET|PASSWORD|CREDENTIAL|API)", "exfil_wget"),
]

_INVISIBLE_CHARS = {
    "\u200b",
    "\u200c",
    "\u200d",
    "\u2060",
    "\ufeff",
    "\u202a",
    "\u202b",
    "\u202c",
    "\u202d",
    "\u202e",
}


def ok(data: Any = None) -> Dict[str, Any]:
    return {"status": "ok", "data": data}


def err(message: str) -> Dict[str, Any]:
    return {"status": "error", "error": str(message)}


def get_memory_dir() -> Path:
    configured = os.environ.get("WEFT_MEMORY_RUNTIME_DIR", "").strip()
    if configured:
        return Path(configured).expanduser().resolve()

    plugin_dir = os.environ.get("WEFT_PACKAGE_DIR", "").strip()
    if plugin_dir:
        return Path(plugin_dir).resolve() / "data"

    return Path.cwd() / "data"


def get_runtime_debug_dir() -> Path:
    plugin_dir = os.environ.get("WEFT_PACKAGE_DIR", "").strip()
    if plugin_dir:
        return Path(plugin_dir).resolve() / "runtime-debug"
    return Path.cwd() / "runtime-debug"


def write_debug_file(name: str, payload: Dict[str, Any]) -> None:
    debug_dir = get_runtime_debug_dir()
    debug_dir.mkdir(parents=True, exist_ok=True)
    (debug_dir / name).write_text(
        json.dumps(payload, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


COMPAT_MEMORY_PREFIX = "[compat-memory "
COMPAT_MEMORY_PATTERN = re.compile(
    r"^\[compat-memory agent=(?P<agent>[^\s\]]+) key=(?P<key>[^\s\]]+) "
    r"category=(?P<category>[^\s\]]+) stored_at=(?P<stored_at>\d+)\]\n(?P<content>.*)$",
    re.S,
)


def _build_compat_memory_entry(agent: str, key: str, content: str, category: str, stored_at: int) -> str:
    return (
        f"[compat-memory agent={agent} key={key} category={category} stored_at={stored_at}]\n"
        f"{content}"
    )


def _parse_compat_memory_entry(entry: str) -> Optional[Dict[str, Any]]:
    match = COMPAT_MEMORY_PATTERN.match(entry.strip())
    if not match:
        return None
    return {
        "agent": match.group("agent"),
        "key": match.group("key"),
        "category": match.group("category"),
        "content": match.group("content").strip(),
        "updated_at": int(match.group("stored_at")),
    }


def _list_compat_memory_entries(agent: str, category: str = "") -> List[Dict[str, Any]]:
    parsed_entries: List[Dict[str, Any]] = []
    for entry in STORE.read("memory")["entries"]:
        parsed = _parse_compat_memory_entry(entry)
        if parsed is None:
            continue
        if parsed["agent"] != agent:
            continue
        if category and parsed["category"] != category:
            continue
        parsed_entries.append(parsed)
    parsed_entries.sort(key=lambda item: int(item["updated_at"]), reverse=True)
    return parsed_entries


def store_memory_record(agent: str, key: str, content: str, category: str) -> Dict[str, Any]:
    agent = agent.strip()
    key = key.strip()
    content = content.strip()
    category = category.strip() or "core"
    if not agent:
        return err("agent is required")
    if not key:
        return err("key is required")
    if not content:
        return err("content is required")

    stored_at = int(time.time() * 1000)
    marker = f"[compat-memory agent={agent} key={key} "
    next_entry = _build_compat_memory_entry(agent, key, content, category, stored_at)
    existing = next((item for item in _list_compat_memory_entries(agent) if item["key"] == key), None)

    if existing is None:
        result = STORE.add("memory", next_entry)
        replaced = False
    else:
        result = STORE.replace("memory", marker, next_entry)
        replaced = True

    if not result.get("success"):
        return err(str(result.get("error", "memory write failed")))

    return ok({"stored": True, "replaced": replaced})


def list_memory_records(agent: str, category: str = "") -> Dict[str, Any]:
    agent = agent.strip()
    category = category.strip()
    if not agent:
        return err("agent is required")
    return ok({"memories": _list_compat_memory_entries(agent, category)})


def forget_memory_record(agent: str, key: str) -> Dict[str, Any]:
    agent = agent.strip()
    key = key.strip()
    if not agent:
        return err("agent is required")
    if not key:
        return err("key is required")

    marker = f"[compat-memory agent={agent} key={key} "
    result = STORE.remove("memory", marker)
    if not result.get("success"):
        return ok({"removed": False})
    return ok({"removed": True})


def recall_memory_records(agent: str, query: str, limit: int) -> Dict[str, Any]:
    agent = agent.strip()
    query = query.strip().lower()
    if not agent:
        return err("agent is required")
    if not query:
        return list_memory_records(agent)

    matches = []
    for record in _list_compat_memory_entries(agent):
        haystack = " ".join([record["key"], record["category"], record["content"]]).lower()
        if query in haystack:
            matches.append(record)
    matches.sort(key=lambda record: int(record["updated_at"]), reverse=True)
    return ok({"memories": matches[: max(0, limit)]})


def _dispatch_memory_tool(action: str, data: Dict[str, Any]) -> Dict[str, Any]:
    action = action.strip().lower()
    if action == "list":
        return list_memory_records(str(data.get("agent", "")), str(data.get("category", "")))
    if action == "recall":
        return recall_memory_records(
            str(data.get("agent", "")),
            str(data.get("query", "")),
            int(data.get("limit", 10) or 10),
        )
    if action == "store":
        return store_memory_record(
            str(data.get("agent", "")),
            str(data.get("key", "")),
            str(data.get("content", "")),
            str(data.get("category", "core")),
        )
    if action == "forget":
        return forget_memory_record(
            str(data.get("agent", "")),
            str(data.get("key", "")),
        )
    return err(f"unknown action: {action}")


def dispatch_action(action: str, data: Dict[str, Any]) -> Dict[str, Any]:
    action = action.strip().lower()
    if action in {"list", "recall", "store", "forget"}:
        return _dispatch_memory_tool(action, data)

    if MEMORY_MANAGER.has_tool(action):
        raw_result = MEMORY_MANAGER.handle_tool_call(action, data)
        try:
            parsed = json.loads(raw_result)
        except Exception:
            return err(f"memory tool '{action}' returned invalid JSON")
        if isinstance(parsed, dict) and parsed.get("error"):
            return err(str(parsed["error"]))
        return ok(parsed)

    return err(f"unknown action: {action}")


def _scan_memory_content(content: str) -> Optional[str]:
    for char in _INVISIBLE_CHARS:
        if char in content:
            return (
                f"Blocked: content contains invisible unicode character "
                f"U+{ord(char):04X} (possible injection)."
            )

    for pattern, pattern_id in _MEMORY_THREAT_PATTERNS:
        if re.search(pattern, content, re.IGNORECASE):
            return (
                f"Blocked: content matches threat pattern '{pattern_id}'. "
                "Memory entries must not contain prompt injection or exfiltration payloads."
            )

    return None


class MemoryStore:
    def __init__(self, memory_char_limit: int = MEMORY_CHAR_LIMIT, user_char_limit: int = USER_CHAR_LIMIT):
        self.memory_entries: List[str] = []
        self.user_entries: List[str] = []
        self.memory_char_limit = memory_char_limit
        self.user_char_limit = user_char_limit
        self._system_prompt_snapshot: Dict[str, str] = {"memory": "", "user": ""}

    def load_from_disk(self) -> None:
        memory_dir = get_memory_dir()
        memory_dir.mkdir(parents=True, exist_ok=True)
        self.memory_entries = self._dedupe(self._read_file(self._path_for("memory")))
        self.user_entries = self._dedupe(self._read_file(self._path_for("user")))
        self._system_prompt_snapshot = {
            "memory": self._render_block("memory", self.memory_entries),
            "user": self._render_block("user", self.user_entries),
        }

    @staticmethod
    def _dedupe(entries: List[str]) -> List[str]:
        return list(dict.fromkeys(entry for entry in entries if entry.strip()))

    @staticmethod
    def _read_file(path: Path) -> List[str]:
        if not path.exists():
            return []
        raw = path.read_text(encoding="utf-8").strip()
        if not raw:
            return []
        return [entry.strip() for entry in raw.split(ENTRY_DELIMITER) if entry.strip()]

    @staticmethod
    def _write_file(path: Path, entries: List[str]) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        content = ENTRY_DELIMITER.join(entries).strip()
        with tempfile.NamedTemporaryFile(
            "w",
            encoding="utf-8",
            delete=False,
            dir=str(path.parent),
        ) as handle:
            handle.write(content)
            temp_path = Path(handle.name)
        temp_path.replace(path)

    @staticmethod
    @contextmanager
    def _file_lock(path: Path):
        lock_path = path.with_suffix(path.suffix + ".lock")
        lock_path.parent.mkdir(parents=True, exist_ok=True)
        handle = open(lock_path, "a+b")
        try:
            handle.seek(0)
            if handle.tell() == 0:
                handle.write(b"0")
                handle.flush()
            handle.seek(0)

            if os.name == "nt":
                msvcrt.locking(handle.fileno(), msvcrt.LK_LOCK, 1)
            else:
                fcntl.flock(handle.fileno(), fcntl.LOCK_EX)

            yield
        finally:
            try:
                handle.seek(0)
                if os.name == "nt":
                    msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
                else:
                    fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
            finally:
                handle.close()

    @staticmethod
    def _path_for(target: str) -> Path:
        memory_dir = get_memory_dir()
        if target == "user":
            return memory_dir / "USER.md"
        return memory_dir / "MEMORY.md"

    def _entries_for(self, target: str) -> List[str]:
        return self.user_entries if target == "user" else self.memory_entries

    def _set_entries(self, target: str, entries: List[str]) -> None:
        if target == "user":
            self.user_entries = entries
        else:
            self.memory_entries = entries

    def _char_limit(self, target: str) -> int:
        return self.user_char_limit if target == "user" else self.memory_char_limit

    def _char_count(self, target: str) -> int:
        entries = self._entries_for(target)
        if not entries:
            return 0
        return len(ENTRY_DELIMITER.join(entries))

    def _reload_target(self, target: str) -> None:
        self._set_entries(target, self._dedupe(self._read_file(self._path_for(target))))

    def save_to_disk(self, target: str) -> None:
        self._write_file(self._path_for(target), self._entries_for(target))

    def _render_block(self, target: str, entries: List[str]) -> str:
        label = "USER" if target == "user" else "MEMORY"
        if not entries:
            return f"[{label}]\n(empty)"
        return f"[{label}]\n" + ENTRY_DELIMITER.join(entries)

    def snapshot(self) -> Dict[str, str]:
        return dict(self._system_prompt_snapshot)

    def format_for_system_prompt(self, target: str) -> Optional[str]:
        block = self._system_prompt_snapshot.get(target, "")
        return block if block else None

    def read(self, target: str) -> Dict[str, Any]:
        self._reload_target(target)
        return {
            "success": True,
            "target": target,
            "entries": self._entries_for(target),
            "usage": f"{self._char_count(target)}/{self._char_limit(target)}",
        }

    def add(self, target: str, content: str) -> Dict[str, Any]:
        content = content.strip()
        if not content:
            return {"success": False, "error": "Content cannot be empty."}

        scan_error = _scan_memory_content(content)
        if scan_error:
            return {"success": False, "error": scan_error}

        with self._file_lock(self._path_for(target)):
            self._reload_target(target)
            entries = self._entries_for(target)
            if content in entries:
                return self._success_response(target, "Entry already exists.")

            next_entries = entries + [content]
            next_total = len(ENTRY_DELIMITER.join(next_entries))
            limit = self._char_limit(target)
            if next_total > limit:
                return {
                    "success": False,
                    "error": (
                        f"Memory at {self._char_count(target)}/{limit} chars. "
                        f"Adding this entry would exceed the limit."
                    ),
                }

            entries.append(content)
            self._set_entries(target, entries)
            self.save_to_disk(target)

        return self._success_response(target, "Entry added.")

    def replace(self, target: str, old_text: str, new_content: str) -> Dict[str, Any]:
        old_text = old_text.strip()
        new_content = new_content.strip()
        if not old_text:
            return {"success": False, "error": "old_text cannot be empty."}
        if not new_content:
            return {"success": False, "error": "new_content cannot be empty."}

        scan_error = _scan_memory_content(new_content)
        if scan_error:
            return {"success": False, "error": scan_error}

        with self._file_lock(self._path_for(target)):
            self._reload_target(target)
            entries = self._entries_for(target)
            matches = [(index, entry) for index, entry in enumerate(entries) if old_text in entry]
            if not matches:
                return {"success": False, "error": f"No entry matched '{old_text}'."}

            unique_texts = {entry for _, entry in matches}
            if len(matches) > 1 and len(unique_texts) > 1:
                return {
                    "success": False,
                    "error": f"Multiple entries matched '{old_text}'. Be more specific.",
                }

            updated = entries.copy()
            updated[matches[0][0]] = new_content
            if len(ENTRY_DELIMITER.join(updated)) > self._char_limit(target):
                return {
                    "success": False,
                    "error": "Replacement would exceed the configured character limit.",
                }

            self._set_entries(target, updated)
            self.save_to_disk(target)

        return self._success_response(target, "Entry replaced.")

    def remove(self, target: str, old_text: str) -> Dict[str, Any]:
        old_text = old_text.strip()
        if not old_text:
            return {"success": False, "error": "old_text cannot be empty."}

        with self._file_lock(self._path_for(target)):
            self._reload_target(target)
            entries = self._entries_for(target)
            matches = [(index, entry) for index, entry in enumerate(entries) if old_text in entry]
            if not matches:
                return {"success": False, "error": f"No entry matched '{old_text}'."}

            unique_texts = {entry for _, entry in matches}
            if len(matches) > 1 and len(unique_texts) > 1:
                return {
                    "success": False,
                    "error": f"Multiple entries matched '{old_text}'. Be more specific.",
                }

            entries.pop(matches[0][0])
            self._set_entries(target, entries)
            self.save_to_disk(target)

        return self._success_response(target, "Entry removed.")

    def _success_response(self, target: str, message: str) -> Dict[str, Any]:
        self._reload_target(target)
        return {
            "success": True,
            "message": message,
            "target": target,
            "entries": self._entries_for(target),
            "usage": f"{self._char_count(target)}/{self._char_limit(target)}",
        }


STORE = MemoryStore()
STORE.load_from_disk()
MEMORY_MANAGER = MemoryManager()
MEMORY_MANAGER.add_provider(
    BuiltinMemoryProvider(
        memory_store=STORE,
        memory_enabled=True,
        user_profile_enabled=True,
    )
)
MEMORY_MANAGER.add_provider(HolographicMemoryProvider())
HTTPD: Optional[ThreadingHTTPServer] = None
PROMPT_SNAPSHOT: str = ""


class MemoryRuntimeHandler(BaseHTTPRequestHandler):
    server_version = "weft-memory-runtime/0.1.0"

    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        if parsed.path == "/health":
            payload = {
                "ok": True,
                "plugin": "memory-runtime",
                "backend": "hermes-style",
                "memory_entries": len(STORE.read("memory")["entries"]),
                "user_entries": len(STORE.read("user")["entries"]),
                "providers": MEMORY_MANAGER.provider_names,
                "memory_dir": str(get_memory_dir()),
            }
            self._write_json(HTTPStatus.OK, payload)
            return

        if parsed.path == "/memory":
            target = self._normalize_target(parse_qs(parsed.query).get("target", ["memory"])[0])
            self._write_json(HTTPStatus.OK, STORE.read(target))
            return

        if parsed.path == "/snapshot":
            self._write_json(HTTPStatus.OK, STORE.snapshot())
            return

        if parsed.path == "/prompt":
            self._write_json(
                HTTPStatus.OK,
                {
                    "prompt": PROMPT_SNAPSHOT,
                    "providers": MEMORY_MANAGER.provider_names,
                },
            )
            return

        self._write_json(HTTPStatus.NOT_FOUND, {"error": f"Unknown path: {parsed.path}"})

    def do_POST(self) -> None:
        parsed = urlparse(self.path)
        body = self._read_json_body()
        if body is None:
            self._write_json(
                HTTPStatus.BAD_REQUEST,
                {"success": False, "error": "Invalid JSON body."},
            )
            return

        if parsed.path == "/webhook":
            action = str(body.get("action", "")).strip()
            data = body.get("data", {})
            if not isinstance(data, dict):
                data = {}
            self._write_json(HTTPStatus.OK, dispatch_action(action, data))
            return

        if parsed.path == "/memory":
            action = str(body.get("action", "read")).strip().lower()
            target = self._normalize_target(body.get("target", "memory"))
            if action == "add":
                self._write_json(HTTPStatus.OK, STORE.add(target, str(body.get("content", ""))))
                return
            if action == "replace":
                self._write_json(
                    HTTPStatus.OK,
                    STORE.replace(
                        target,
                        str(body.get("old_text", "")),
                        str(body.get("new_content", "")),
                    ),
                )
                return
            if action == "remove":
                self._write_json(HTTPStatus.OK, STORE.remove(target, str(body.get("old_text", ""))))
                return
            if action == "read":
                self._write_json(HTTPStatus.OK, STORE.read(target))
                return

            self._write_json(HTTPStatus.BAD_REQUEST, {"success": False, "error": f"Unknown action '{action}'."})
            return

        if parsed.path == "/shutdown":
            self._write_json(HTTPStatus.OK, {"ok": True, "message": "shutdown requested"})
            self.close_connection = True
            if HTTPD is not None:
                HTTPD.shutdown()
            return

        self._write_json(HTTPStatus.NOT_FOUND, {"error": f"Unknown path: {parsed.path}"})

    def log_message(self, format: str, *args: Any) -> None:
        LOGGER.info("%s - %s", self.address_string(), format % args)

    def _normalize_target(self, value: Any) -> str:
        return "user" if str(value).strip().lower() == "user" else "memory"

    def _read_json_body(self) -> Optional[Dict[str, Any]]:
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
            return parsed if isinstance(parsed, dict) else None
        except json.JSONDecodeError:
            return None

    def _write_json(self, status: HTTPStatus, payload: Dict[str, Any]) -> None:
        encoded = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status.value)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)


def main() -> None:
    global HTTPD, PROMPT_SNAPSHOT
    memory_dir = get_memory_dir()
    memory_dir.mkdir(parents=True, exist_ok=True)
    write_debug_file(
        "boot.json",
        {
            "status": "booting",
            "pid": os.getpid(),
            "cwd": os.getcwd(),
            "plugin_dir": os.environ.get("WEFT_PACKAGE_DIR", ""),
            "memory_dir": str(memory_dir),
            "port": PORT,
        },
    )
    try:
        MEMORY_MANAGER.initialize_all(session_id="memory-runtime-service", platform="service")
        PROMPT_SNAPSHOT = MEMORY_MANAGER.build_system_prompt()
        HTTPD = ThreadingHTTPServer(("127.0.0.1", PORT), MemoryRuntimeHandler)
        LOGGER.info("memory-runtime listening on http://127.0.0.1:%s", PORT)
        LOGGER.info("memory-runtime data dir: %s", memory_dir)
        write_debug_file(
            "boot.json",
            {
                "status": "listening",
                "pid": os.getpid(),
                "cwd": os.getcwd(),
                "plugin_dir": os.environ.get("WEFT_PACKAGE_DIR", ""),
                "memory_dir": str(memory_dir),
                "port": PORT,
            },
        )
        try:
            HTTPD.serve_forever(poll_interval=0.5)
        finally:
            MEMORY_MANAGER.shutdown_all()
            HTTPD.server_close()
    except Exception as error:
        write_debug_file(
            "crash.json",
            {
                "status": "crashed",
                "pid": os.getpid(),
                "cwd": os.getcwd(),
                "plugin_dir": os.environ.get("WEFT_PACKAGE_DIR", ""),
                "memory_dir": str(memory_dir),
                "port": PORT,
                "error": str(error),
                "traceback": traceback.format_exc(),
            },
        )
        raise


if __name__ == "__main__":
    main()
