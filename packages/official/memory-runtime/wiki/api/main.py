#!/usr/bin/env python3
# pyright: reportMissingTypeArgument=false, reportUnknownParameterType=false, reportUnknownVariableType=false, reportUnknownMemberType=false, reportAny=false, reportMissingParameterType=false, reportUnusedCallResult=false, reportUnannotatedClassAttribute=false, reportImplicitOverride=false
from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

HOST = "127.0.0.1"
PORT = 18000


def iso_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def resolve_data_root() -> Path:
    configured = os.environ.get("SUPAVAULT_DATA_DIR", "").strip()
    if configured:
        root = Path(configured)
    else:
        root = Path(__file__).resolve().parent.parent / "data"
    root.mkdir(parents=True, exist_ok=True)
    return root


def resolve_seed_path() -> Path:
    return resolve_data_root() / "wiki-seed.json"


def default_seed() -> dict:
    return {
        "pages": [
            {
                "id": "workspace-overview",
                "title": "Workspace Overview",
                "summary": "What this workspace is for and how the team uses it.",
                "content": "# Workspace Overview\n\nThis workspace wiki is running from the bundled minimal local runtime.\n\n## What you can do\n- Browse seeded pages\n- Verify the Electron canvas mounts a real service\n- Extend this payload with richer indexing later\n",
                "tags": ["workspace", "overview"],
            },
            {
                "id": "current-focus",
                "title": "Current Focus",
                "summary": "Tracks the current engineering focus for the active workspace.",
                "content": "# Current Focus\n\nThe current focus is making the workspace wiki path functional end-to-end in Electron.\n\n## Backing services\n- Web UI on port 3000\n- API on port 18000\n",
                "tags": ["focus", "engineering"],
            },
            {
                "id": "usage-notes",
                "title": "Usage Notes",
                "summary": "Notes for validating the minimal wiki runtime.",
                "content": "# Usage Notes\n\nOpen the right canvas in chat view to confirm the wiki renders instead of the old placeholder.\n\nSearch is local and seeded for now, but the service is real.\n",
                "tags": ["notes", "validation"],
            },
        ]
    }


def load_pages() -> list[dict]:
    seed_path = resolve_seed_path()
    if not seed_path.exists():
        seed_path.write_text(json.dumps(default_seed(), ensure_ascii=False, indent=2), encoding="utf-8")
    payload = json.loads(seed_path.read_text(encoding="utf-8"))
    pages = payload.get("pages")
    return pages if isinstance(pages, list) else []


def build_view(workspace_id: str, query: str) -> dict:
    normalized_query = query.strip().lower()
    pages = load_pages()
    filtered: list[dict] = []
    for page in pages:
      title = str(page.get("title", ""))
      summary = str(page.get("summary", ""))
      content = str(page.get("content", ""))
      haystack = f"{title}\n{summary}\n{content}".lower()
      if normalized_query and normalized_query not in haystack:
          continue
      filtered.append(
          {
              "id": str(page.get("id", "")),
              "title": title,
              "summary": summary,
              "updatedAt": iso_now(),
              "workspaceId": workspace_id,
              "tags": page.get("tags", []),
          }
      )
    selected_page_id = filtered[0]["id"] if filtered else None
    return {
        "workspaceId": workspace_id,
        "query": query,
        "pages": filtered,
        "selectedPageId": selected_page_id,
        "updatedAt": iso_now(),
    }


def build_page(page_id: str, workspace_id: str) -> dict | None:
    for page in load_pages():
        if str(page.get("id", "")) != page_id:
            continue
        return {
            "id": str(page.get("id", "")),
            "title": str(page.get("title", "")),
            "summary": str(page.get("summary", "")),
            "content": str(page.get("content", "")),
            "workspaceId": workspace_id,
            "updatedAt": iso_now(),
            "tags": page.get("tags", []),
        }
    return None


class Handler(BaseHTTPRequestHandler):
    server_version = "WorkspaceWikiApi/0.1"

    def log_message(self, format: str, *args) -> None:
        return

    def send_json(self, payload: dict, status: int = HTTPStatus.OK) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(body)

    def do_OPTIONS(self) -> None:
        self.send_response(HTTPStatus.NO_CONTENT)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()

    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        if parsed.path == "/docs":
            self.send_json({"ok": True, "service": "workspace-wiki-api", "docs": True})
            return
        if parsed.path == "/health":
            self.send_json({"ok": True, "service": "workspace-wiki-api", "pages": len(load_pages())})
            return
        if parsed.path == "/v1/palace/wiki-view":
            params = parse_qs(parsed.query)
            workspace_id = (params.get("workspace_id") or ["workspace-global"])[0]
            query = (params.get("query") or [""])[0]
            self.send_json({"ok": True, "data": build_view(workspace_id, query)})
            return
        if parsed.path.startswith("/v1/palace/wiki-page/"):
            page_id = parsed.path.rsplit("/", 1)[-1]
            params = parse_qs(parsed.query)
            workspace_id = (params.get("workspace_id") or ["workspace-global"])[0]
            page = build_page(page_id, workspace_id)
            if page is None:
                self.send_json({"ok": False, "error": "page_not_found"}, HTTPStatus.NOT_FOUND)
                return
            self.send_json({"ok": True, "data": page})
            return
        self.send_json({"ok": False, "error": "not_found", "path": parsed.path}, HTTPStatus.NOT_FOUND)


def main() -> None:
    server = ThreadingHTTPServer((HOST, PORT), Handler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
