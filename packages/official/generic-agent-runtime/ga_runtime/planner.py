from __future__ import annotations

from typing import Any


def build_plan(task: str, session_id: str = "", workspace_id: str = "") -> dict[str, Any]:
    trimmed = task.strip()
    return {
        "task": trimmed,
        "session_id": session_id,
        "workspace_id": workspace_id,
        "phases": [
            {
                "name": "explore",
                "goal": "understand environment, constraints, and missing capabilities",
            },
            {
                "name": "execute",
                "goal": "perform the smallest viable sequence of actions to complete the task",
            },
            {
                "name": "verify",
                "goal": "check whether the result truly matches user intent",
            },
            {
                "name": "crystallize",
                "goal": "extract a reusable skill or SOP from the successful path",
            },
        ],
        "steps": [
            {
                "id": "explore-1",
                "phase": "explore",
                "description": "Identify task type, required environment, and reusable prior patterns.",
            },
            {
                "id": "execute-1",
                "phase": "execute",
                "description": "Run a minimal task loop and collect an execution trace.",
            },
            {
                "id": "verify-1",
                "phase": "verify",
                "description": "Validate outcome against task intent and failure criteria.",
            },
            {
                "id": "crystallize-1",
                "phase": "crystallize",
                "description": "Convert the successful execution path into a reusable skill draft.",
            },
        ],
    }
