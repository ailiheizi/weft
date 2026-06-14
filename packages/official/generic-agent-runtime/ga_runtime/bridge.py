from __future__ import annotations

import json
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import ProxyHandler, Request, build_opener


class WeftToolBridge:
    def __init__(self, base_url: str, package_name: str = "tool-runtime-core") -> None:
        self.base_url = base_url.rstrip("/")
        self.package_name = package_name.strip() or "tool-runtime-core"
        self.opener = build_opener(ProxyHandler({}))

    def execute_tool(self, tool: str, args: dict[str, Any]) -> dict[str, Any]:
        payload = {
            "action": "execute_tool",
            "data": {
                "tool": tool,
                "args": args,
            },
        }
        response = self._post_json(
            f"{self.base_url}/api/plugins/{self.package_name}/call",
            payload,
        )
        if isinstance(response, dict):
            return response
        return {"result": response}

    def _post_json(self, url: str, payload: dict[str, Any]) -> Any:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        request = Request(url, data=body, headers={"Content-Type": "application/json"})
        try:
            with self.opener.open(request, timeout=20) as response:
                return json.loads(response.read().decode("utf-8"))
        except HTTPError as error:
            details = error.read().decode("utf-8", errors="replace")
            return {
                "status": "error",
                "error": f"HTTP {error.code}",
                "details": details,
            }
        except URLError as error:
            return {
                "status": "error",
                "error": f"URL error: {error}",
            }
        except Exception as error:
            return {
                "status": "error",
                "error": str(error),
            }
