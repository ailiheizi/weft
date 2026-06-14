from __future__ import annotations

import re
from typing import Any


def _slugify(value: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-")
    return slug or "generic-agent-skill"


def crystallize_skill(task: str, run_result: dict[str, Any], verification: dict[str, Any]) -> dict[str, Any]:
    title = task.strip() or "GenericAgent Skill"
    slug = _slugify(title)
    lines = [
        f"# {title}",
        "",
        "## Intent",
        "",
        task.strip(),
        "",
        "## Execution Trace",
        "",
    ]
    for item in run_result.get("loop", []):
        lines.append(f"- turn {item.get('turn')}: {item.get('stage')} -> {item.get('note')}")
    lines.extend([
        "",
        "## Verification",
        "",
        f"- verdict: {verification.get('verdict', 'UNKNOWN')}",
        f"- notes: {verification.get('notes', '')}",
        "",
        "## Reuse Guidance",
        "",
        "- reuse this draft as a WEFT skill or SOP after manual review",
        "- keep this runtime isolated until real-tool execution is proven stable",
    ])
    return {
        "title": title,
        "slug": slug,
        "content": "\n".join(lines).strip() + "\n",
    }
