from __future__ import annotations

from typing import Any


def verify_task(task: str, run_result: dict[str, Any]) -> dict[str, Any]:
    status = run_result.get("result", {}).get("status")
    verdict = "PASS" if status == "prototype_complete" else "FAIL"
    return {
        "task": task.strip(),
        "verdict": verdict,
        "checks": [
            {
                "name": "runtime completed loop",
                "passed": status == "prototype_complete",
            },
            {
                "name": "result summary present",
                "passed": bool(run_result.get("result", {}).get("summary")),
            },
        ],
        "notes": "Prototype verification checks structural completion only. Real tool execution verification is intentionally out of scope for this isolated experiment.",
    }
