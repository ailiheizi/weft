"""Universal Selector JSON-RPC service.

Long-lived process that loads ONNX model + libraries on startup, then accepts
JSON-line requests on stdin and writes JSON-line responses to stdout.

Protocol:
  Request:  {"id": 1, "method": "select", "params": {...}}\n
  Response: {"id": 1, "result": [...]}\n
  Error:    {"id": 1, "error": "message"}\n
  Ready:    {"ready": true, "libraries": [...]}\n

Usage:
  python server.py --libraries-dir /path/to/libraries
"""

import sys
import os
import json
import traceback
from pathlib import Path

# Add universal-selector src to path for imports
SCRIPT_DIR = Path(__file__).parent
# Try to find universal-selector in common locations
SELECTOR_PATHS = [
    SCRIPT_DIR / ".." / ".." / ".." / ".." / "ref" / "universal-selector",
    Path(os.environ.get("UNIVERSAL_SELECTOR_PATH", "")) if os.environ.get("UNIVERSAL_SELECTOR_PATH") else None,
    Path.home() / "code" / "research" / "universal-selector",
    Path("D:/windows/code/project/research/universal-selector"),
]

selector_path = None
for p in SELECTOR_PATHS:
    if p and p.exists() and (p / "src" / "selector.py").exists():
        selector_path = p
        break

if selector_path:
    sys.path.insert(0, str(selector_path))
else:
    # Fallback: assume src modules are copied alongside this script
    sys.path.insert(0, str(SCRIPT_DIR))


def load_selector(libraries_dir: str):
    """Load ONNX encoder and selector with all available libraries."""
    # We need to import from universal-selector as a package.
    # The issue: src/selector.py does `from .encoder import SelectorEncoder` which
    # pulls in sentence_transformers (PyTorch). We patch it before importing.
    import importlib
    import types

    project_path = None
    for p in SELECTOR_PATHS:
        if p and p.exists() and (p / "src" / "selector.py").exists():
            project_path = p
            break

    if project_path is None:
        raise ImportError(
            "Cannot find universal-selector project. "
            "Ensure it exists at one of: " +
            str([str(p) for p in SELECTOR_PATHS if p])
        )

    # Create a fake 'src' package and a stub 'src.encoder' module
    # so that `from .encoder import SelectorEncoder` doesn't fail
    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = [str(project_path / "src")]
    src_pkg.__package__ = "src"
    sys.modules.setdefault("src", src_pkg)

    # Stub encoder module with a dummy SelectorEncoder
    encoder_stub = types.ModuleType("src.encoder")
    encoder_stub.SelectorEncoder = object  # placeholder type
    sys.modules.setdefault("src.encoder", encoder_stub)

    # Now import the real modules
    sys.path.insert(0, str(project_path))
    from src.encoder_onnx import OnnxEncoder
    from src.selector import UniversalSelector

    # Find ONNX model — look in project_path first, then service/models/
    model_dir = None
    candidate = project_path / "models" / "onnx"
    if candidate.exists():
        model_dir = str(candidate)
    else:
        candidate = SCRIPT_DIR / "models"
        if candidate.exists():
            model_dir = str(candidate)

    if model_dir:
        encoder = OnnxEncoder(model_dir=model_dir, use_int8=True)
    else:
        encoder = OnnxEncoder(use_int8=True)

    sel = UniversalSelector(encoder)

    # Auto-load all libraries from the libraries directory
    lib_dir = Path(libraries_dir)
    loaded = []
    if lib_dir.is_dir():
        for subdir in sorted(lib_dir.iterdir()):
            if subdir.is_dir() and (subdir / "descriptions.jsonl").exists():
                try:
                    sel.load_library(subdir.name, str(subdir))
                    loaded.append(subdir.name)
                except Exception as e:
                    sys.stderr.write(f"Warning: failed to load library {subdir.name}: {e}\n")
                    sys.stderr.flush()

    return sel, loaded


def handle_request(selector, method: str, params: dict):
    """Dispatch a method call and return the result."""
    if method == "select":
        query = params.get("query", "")
        library = params.get("library", "tools")
        top_k = int(params.get("top_k", 3))
        include_skills = params.get("include_skills", False)
        results = selector.select(query, library, top_k=top_k)
        # Optionally attach skill.md content to each result.
        # Lookup order for "mcp:server:tool": skills/{server}.md (server-level
        # TOOL.md) first, then skills/{tool}.md. For plain ids: skills/{id}.md.
        if include_skills and results:
            skills_dir = SCRIPT_DIR / "skills"
            for item in results:
                tool_id = item.get("id", "")
                candidates = []
                if tool_id.startswith("mcp:"):
                    parts = tool_id.split(":", 2)
                    if len(parts) == 3:
                        candidates.append(parts[1])  # server-level doc
                        candidates.append(parts[2])  # tool-level doc
                    else:
                        candidates.append(tool_id)
                else:
                    candidates.append(tool_id)
                for skill_name in candidates:
                    skill_path = skills_dir / f"{skill_name}.md"
                    if skill_path.exists():
                        item["skill"] = skill_path.read_text(encoding="utf-8")
                        break
        return results

    elif method == "select_multi":
        queries = params.get("queries", {})
        top_k = int(params.get("top_k", 1))
        results = selector.select_multi(queries, top_k=top_k)
        return results

    elif method == "list_libraries":
        return list(selector.libraries.keys()) if hasattr(selector, 'libraries') else []

    elif method == "list_presets":
        # Return the curated MCP server preset catalog.
        presets_path = SCRIPT_DIR / "mcp_presets.json"
        if not presets_path.exists():
            return {"presets": []}
        with open(presets_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        return data

    elif method == "list_tools":
        # List all tools in a library with metadata
        library = params.get("library", "tools")
        lib_dir = _get_libraries_dir()
        jsonl_path = lib_dir / library / "descriptions.jsonl"
        if not jsonl_path.exists():
            return {"tools": [], "count": 0}
        tools = []
        with open(jsonl_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    tools.append(json.loads(line))
        return {"tools": tools, "count": len(tools)}

    elif method == "register":
        # Register a single tool to a library
        library = params.get("library", "tools")
        tool_id = params.get("id", "")
        name = params.get("name", tool_id)
        description = params.get("description", "")
        source = params.get("source", "manual")
        mcp_server = params.get("mcp_server", "")
        if not tool_id:
            raise ValueError("id is required")
        if not description:
            raise ValueError("description is required")

        lib_dir = _get_libraries_dir()
        tool_dir = lib_dir / library
        tool_dir.mkdir(parents=True, exist_ok=True)
        jsonl_path = tool_dir / "descriptions.jsonl"

        # Read existing, remove duplicate id
        entries = []
        if jsonl_path.exists():
            with open(jsonl_path, "r", encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if line:
                        entry = json.loads(line)
                        if entry.get("id") != tool_id:
                            entries.append(entry)

        new_entry = {"id": tool_id, "name": name, "description": description, "source": source}
        if mcp_server:
            new_entry["mcp_server"] = mcp_server
        entries.append(new_entry)

        with open(jsonl_path, "w", encoding="utf-8") as f:
            for entry in entries:
                f.write(json.dumps(entry, ensure_ascii=False) + "\n")

        return {"status": "registered", "id": tool_id, "total": len(entries)}

    elif method == "unregister":
        # Remove a tool from a library
        library = params.get("library", "tools")
        tool_id = params.get("id", "")
        if not tool_id:
            raise ValueError("id is required")

        lib_dir = _get_libraries_dir()
        jsonl_path = lib_dir / library / "descriptions.jsonl"
        if not jsonl_path.exists():
            return {"status": "not_found"}

        entries = []
        removed = False
        with open(jsonl_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    entry = json.loads(line)
                    if entry.get("id") == tool_id:
                        removed = True
                    else:
                        entries.append(entry)

        with open(jsonl_path, "w", encoding="utf-8") as f:
            for entry in entries:
                f.write(json.dumps(entry, ensure_ascii=False) + "\n")

        return {"status": "removed" if removed else "not_found", "total": len(entries)}

    elif method == "rebuild":
        # Rebuild embeddings for a library from descriptions.jsonl
        library = params.get("library", "tools")
        lib_dir = _get_libraries_dir()
        tool_dir = lib_dir / library
        jsonl_path = tool_dir / "descriptions.jsonl"
        if not jsonl_path.exists():
            raise ValueError(f"no descriptions.jsonl in library '{library}'")

        items = []
        with open(jsonl_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    items.append(json.loads(line))

        if not items:
            raise ValueError(f"library '{library}' has no entries")

        import numpy as np
        texts = [item["description"] for item in items]
        embeddings = selector.encoder.encode_batch(texts)
        np.save(str(tool_dir / "embeddings.npy"), embeddings)

        # Reload the library
        selector.load_library(library, str(tool_dir))
        return {"status": "rebuilt", "library": library, "count": len(items)}

    elif method == "sync_mcp":
        # Sync tools from Core's mcp-client (get_tools) into the selector library
        import urllib.request
        core_port = int(params.get("core_port", 17830))
        token = params.get("token", "")
        library = params.get("library", "tools")

        # If no token passed, read it from data/runtime-token (selector runs
        # on the same machine as Core). Resolve project root from WEFT_PACKAGE_DIR
        # (packages/installed/tool-selector → up 3 levels) or env var.
        if not token:
            candidates = []
            pkg_dir = os.environ.get("WEFT_PACKAGE_DIR", "")
            if pkg_dir:
                root = Path(pkg_dir).resolve()
                for _ in range(4):
                    candidates.append(root / "data" / "runtime-token")
                    root = root.parent
            candidates.append(Path.cwd() / "data" / "runtime-token")
            for tok_path in candidates:
                try:
                    if tok_path.exists():
                        token = tok_path.read_text(encoding="utf-8").strip()
                        if token:
                            break
                except Exception:
                    continue

        # Also keep existing non-mcp entries (source != "mcp")
        keep_sources = {"virtual", "skill", "always-on", "manual"}

        # Read existing non-mcp entries to preserve them
        lib_dir = _get_libraries_dir()
        tool_dir = lib_dir / library
        tool_dir.mkdir(parents=True, exist_ok=True)
        jsonl_path = tool_dir / "descriptions.jsonl"
        preserved = []
        if jsonl_path.exists():
            with open(jsonl_path, "r", encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if line:
                        entry = json.loads(line)
                        if entry.get("source", "") in keep_sources:
                            preserved.append(entry)

        # Call Core API to get MCP tools (via weft-claw proxy)
        url = f"http://127.0.0.1:{core_port}/api/apps/weft-claw/run"
        payload = json.dumps({
            "capability": "ext.mcp",
            "action": "get_tools",
            "app": "weft-claw",
            "data": {"agent": "default", "include_diagnostics": False}
        }).encode("utf-8")
        headers = {"Content-Type": "application/json"}
        if token:
            headers["Authorization"] = f"Bearer {token}"

        try:
            req = urllib.request.Request(url, data=payload, headers=headers, method="POST")
            with urllib.request.urlopen(req, timeout=10) as resp:
                resp_data = json.loads(resp.read().decode("utf-8"))
        except Exception as e:
            raise ValueError(f"failed to fetch MCP tools from Core: {e}")

        # Parse response: result.response.data.tools
        result = resp_data.get("result", {})
        response = result.get("response", {})
        data = response.get("data", {})
        mcp_tools = data.get("tools", [])

        # Convert to selector entries
        new_mcp_entries = []
        for tool in mcp_tools:
            server_name = tool.get("server", "unknown")
            tool_name = tool.get("name", "")
            description = tool.get("description", tool_name)
            if not tool_name:
                continue
            entry_id = f"mcp:{server_name}:{tool_name}"
            new_mcp_entries.append({
                "id": entry_id,
                "name": tool_name,
                "description": description,
                "source": "mcp",
                "mcp_server": server_name,
            })

        # Merge: preserved (non-mcp) + new mcp entries
        all_entries = preserved + new_mcp_entries

        with open(jsonl_path, "w", encoding="utf-8") as f:
            for entry in all_entries:
                f.write(json.dumps(entry, ensure_ascii=False) + "\n")

        # Rebuild embeddings
        if all_entries:
            import numpy as np
            texts = [e["description"] for e in all_entries]
            embeddings = selector.encoder.encode_batch(texts)
            np.save(str(tool_dir / "embeddings.npy"), embeddings)
            selector.load_library(library, str(tool_dir))

        return {
            "status": "synced",
            "preserved": len(preserved),
            "mcp_tools_added": len(new_mcp_entries),
            "total": len(all_entries),
        }

    elif method == "build_library":
        library_path = params.get("library_path", "")
        if not library_path:
            raise ValueError("library_path is required")

        try:
            from src.builder import build_library
        except ImportError:
            from builder import build_library

        build_library(library_path)
        # Reload the library after building
        lib_name = Path(library_path).name
        selector.load_library(lib_name, library_path)
        return {"status": "built", "library": lib_name}

    else:
        raise ValueError(f"unknown method: {method}")


# Global libraries dir — set during startup, used by handle_request
_LIBRARIES_DIR = None

def _get_libraries_dir() -> Path:
    global _LIBRARIES_DIR
    if _LIBRARIES_DIR:
        return Path(_LIBRARIES_DIR)
    return SCRIPT_DIR / "libraries"


def main():
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--libraries-dir", default="./libraries")
    parser.add_argument("--http", action="store_true", help="Run as HTTP server instead of stdin/stdout")
    parser.add_argument("--port", type=int, default=17860, help="HTTP port (default: 17860)")
    args = parser.parse_args()

    # Load model and libraries
    try:
        global _LIBRARIES_DIR
        _LIBRARIES_DIR = args.libraries_dir
        selector, loaded_libs = load_selector(args.libraries_dir)
    except Exception as e:
        # Fatal: can't start without the model
        error_msg = json.dumps({"ready": False, "error": str(e)})
        sys.stdout.write(error_msg + "\n")
        sys.stdout.flush()
        sys.exit(1)

    if args.http:
        run_http_server(selector, loaded_libs, args.port)
    else:
        run_stdio_server(selector, loaded_libs)


def run_http_server(selector, loaded_libs, port):
    """Run as a simple HTTP JSON API server."""
    from http.server import HTTPServer, BaseHTTPRequestHandler

    class SelectorHandler(BaseHTTPRequestHandler):
        def do_POST(self):
            content_length = int(self.headers.get('Content-Length', 0))
            body = self.rfile.read(content_length).decode('utf-8')
            try:
                request = json.loads(body)
            except json.JSONDecodeError as e:
                self._respond(400, {"error": f"invalid JSON: {e}"})
                return

            method = request.get("method", "select")
            params = request.get("params", request)  # allow flat or nested

            try:
                result = handle_request(selector, method, params)
                self._respond(200, {"result": result})
            except Exception as e:
                self._respond(500, {"error": str(e)})

        def do_GET(self):
            self._respond(200, {"ready": True, "libraries": loaded_libs})

        def _respond(self, status, data):
            body = json.dumps(data, ensure_ascii=False).encode('utf-8')
            self.send_response(status)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(body)))
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(body)

        def do_OPTIONS(self):
            self.send_response(200)
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Access-Control-Allow-Methods', 'POST, GET, OPTIONS')
            self.send_header('Access-Control-Allow-Headers', 'Content-Type')
            self.end_headers()

        def log_message(self, format, *args):
            pass  # Suppress request logging

    server = HTTPServer(('127.0.0.1', port), SelectorHandler)
    print(f"Selector HTTP server running on http://127.0.0.1:{port}")
    print(f"Libraries: {loaded_libs}")
    sys.stdout.flush()
    server.serve_forever()


def run_stdio_server(selector, loaded_libs):
    """Original stdin/stdout JSON-line mode."""
    # Signal ready
    ready_msg = json.dumps({"ready": True, "libraries": loaded_libs})
    sys.stdout.write(ready_msg + "\n")
    sys.stdout.flush()

    # Main loop: read JSON lines from stdin
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
        except json.JSONDecodeError as e:
            err_resp = json.dumps({"id": None, "error": f"invalid JSON: {e}"})
            sys.stdout.write(err_resp + "\n")
            sys.stdout.flush()
            continue

        req_id = request.get("id")
        method = request.get("method", "")
        params = request.get("params", {})

        try:
            result = handle_request(selector, method, params)
            response = json.dumps({"id": req_id, "result": result}, ensure_ascii=False)
        except Exception as e:
            tb = traceback.format_exc()
            sys.stderr.write(f"Error handling {method}: {tb}\n")
            sys.stderr.flush()
            response = json.dumps({"id": req_id, "error": str(e)})

        sys.stdout.write(response + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
