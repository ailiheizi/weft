from __future__ import annotations

from typing import Any

from .bridge import WeftToolBridge


def run_task(
    task: str,
    session_id: str = "",
    workspace_id: str = "",
    bridge: WeftToolBridge | None = None,
    tool: str = "",
    args: dict[str, Any] | None = None,
) -> dict[str, Any]:
    trimmed = task.strip()
    explicit_tool = tool.strip()
    explicit_args = args or {}
    loop = [
        {"turn": 1, "stage": "observe", "note": "received task and normalized intent"},
        {"turn": 1, "stage": "plan", "note": "built a minimal execution plan"},
    ]

    if explicit_tool and bridge is not None:
        tool_result = bridge.execute_tool(explicit_tool, explicit_args)
        loop.append(
            {
                "turn": 1,
                "stage": "act",
                "note": f"executed WEFT tool '{explicit_tool}' through WEFT-core package call API",
                "tool": explicit_tool,
                "args": explicit_args,
                "result": tool_result,
            }
        )
        loop.append(
            {
                "turn": 1,
                "stage": "reflect",
                "note": "captured real tool execution trace for later crystallization",
            }
        )
        return {
            "task": trimmed,
            "session_id": session_id,
            "workspace_id": workspace_id,
            "loop": loop,
            "result": {
                "status": "tool_executed",
                "summary": f"Executed WEFT tool '{explicit_tool}' from isolated GenericAgent runtime.",
                "tool": explicit_tool,
                "tool_result": tool_result,
            },
        }

    loop.extend(
        [
            {"turn": 1, "stage": "act", "note": "prototype runtime does not execute external tools yet"},
            {"turn": 1, "stage": "reflect", "note": "captured execution trace for later crystallization"},
        ]
    )
    return {
        "task": trimmed,
        "session_id": session_id,
        "workspace_id": workspace_id,
        "loop": loop,
        "result": {
            "status": "prototype_complete",
            "summary": "The isolated GenericAgent-style runtime completed its planning loop and produced a trace without mutating WEFT core state.",
        },
    }
