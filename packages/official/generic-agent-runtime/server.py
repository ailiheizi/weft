#!/usr/bin/env python3
from __future__ import annotations

import json
import logging
import os
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

from ga_runtime import WeftToolBridge, RuntimeStore, build_plan, crystallize_skill, err, ok, run_task, verify_task


LOGGER = logging.getLogger("generic-agent-runtime")
logging.basicConfig(
    level=os.environ.get("WEFT_GENERIC_AGENT_RUNTIME_LOG_LEVEL", "INFO").upper(),
    format="[generic-agent-runtime] %(message)s",
)

PORT = int(os.environ.get("WEFT_GENERIC_AGENT_RUNTIME_PORT", "43133"))
HTTPD: ThreadingHTTPServer | None = None
WEFT_CORE_BASE_URL = os.environ.get("WEFT_CORE_BASE_URL", "http://127.0.0.1:17830").rstrip("/")


def plugin_dir() -> Path:
    return Path(os.environ.get("WEFT_PACKAGE_DIR", Path(__file__).resolve().parent)).resolve()


STORE = RuntimeStore(plugin_dir() / "data")
BRIDGE = WeftToolBridge(WEFT_CORE_BASE_URL)


def dispatch_action(action: str, data: dict[str, Any]) -> dict[str, Any]:
    action = action.strip().lower()
    task = str(data.get("task", "")).strip()
    session_id = str(data.get("session_id", "")).strip()
    workspace_id = str(data.get("workspace_id", "")).strip()

    if action == "health":
        return ok({"healthy": True, "plugin": "generic-agent-runtime"})

    if action == "plan_task":
        if not task:
            return err("plan_task requires task")
        plan = build_plan(task, session_id, workspace_id)
        STORE.append_state_entry("plans", plan)
        return ok(plan)

    if action == "run_task":
        if not task:
            return err("run_task requires task")
        tool = str(data.get("tool", "")).strip()
        tool_args = data.get("args") if isinstance(data.get("args"), dict) else {}
        result = run_task(task, session_id, workspace_id, bridge=BRIDGE, tool=tool, args=tool_args)
        STORE.append_state_entry("runs", result)
        return ok(result)

    if action == "verify_task":
        run_result = data.get("run_result")
        if not task or not isinstance(run_result, dict):
            return err("verify_task requires task and run_result")
        result = verify_task(task, run_result)
        STORE.append_state_entry("verifications", result)
        return ok(result)

    if action == "crystallize_skill":
        run_result = data.get("run_result")
        verification = data.get("verification")
        if not task or not isinstance(run_result, dict) or not isinstance(verification, dict):
            return err("crystallize_skill requires task, run_result, and verification")
        draft = crystallize_skill(task, run_result, verification)
        path = STORE.write_skill_draft(draft["slug"], draft["content"])
        record = {
            **draft,
            "path": str(path),
        }
        STORE.append_state_entry("crystallized_skills", record)
        return ok(record)

    if action == "get_runtime_state":
        return ok(STORE.load_state())

    return err(f"unknown action: {action}")


class Handler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:  # noqa: N802
        if self.path == "/health":
            self._write_json(HTTPStatus.OK, {"status": "ok", "plugin": "generic-agent-runtime"})
            return
        self._write_json(HTTPStatus.NOT_FOUND, {"status": "error", "error": "not found"})

    def do_POST(self) -> None:  # noqa: N802
        if self.path != "/webhook":
            self._write_json(HTTPStatus.NOT_FOUND, {"status": "error", "error": "not found"})
            return

        try:
            content_length = int(self.headers.get("Content-Length", "0"))
            raw = self.rfile.read(content_length).decode("utf-8") if content_length > 0 else "{}"
            payload = json.loads(raw or "{}")
        except Exception as error:
            self._write_json(HTTPStatus.BAD_REQUEST, err(f"invalid json: {error}"))
            return

        action = str(payload.get("action", "")).strip()
        data = payload.get("data") if isinstance(payload.get("data"), dict) else {}
        result = dispatch_action(action, data)
        status = HTTPStatus.OK if result.get("status") == "ok" else HTTPStatus.BAD_REQUEST
        self._write_json(status, result)

    def log_message(self, format: str, *args: Any) -> None:  # noqa: A003
        LOGGER.info("%s", format % args)

    def _write_json(self, status: HTTPStatus, payload: dict[str, Any]) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def main() -> None:
    global HTTPD
    HTTPD = ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
    LOGGER.info("generic-agent-runtime listening on 127.0.0.1:%s", PORT)
    try:
        HTTPD.serve_forever()
    except KeyboardInterrupt:
        LOGGER.info("generic-agent-runtime interrupted; shutting down")
    finally:
        HTTPD.server_close()


if __name__ == "__main__":
    main()
