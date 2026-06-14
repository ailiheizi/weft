from __future__ import annotations

import json
from typing import Any, Dict, List

from hermes_port.memory_provider import MemoryProvider


class BuiltinMemoryProvider(MemoryProvider):
    def __init__(
        self,
        memory_store=None,
        memory_enabled: bool = True,
        user_profile_enabled: bool = True,
    ) -> None:
        self._store = memory_store
        self._memory_enabled = memory_enabled
        self._user_profile_enabled = user_profile_enabled

    @property
    def name(self) -> str:
        return "builtin"

    def is_available(self) -> bool:
        return True

    def initialize(self, session_id: str, **kwargs) -> None:
        if self._store is not None:
            self._store.load_from_disk()

    def system_prompt_block(self) -> str:
        if self._store is None:
            return ""

        parts: List[str] = []
        if self._memory_enabled:
            memory_block = self._store.format_for_system_prompt("memory")
            if memory_block:
                parts.append(memory_block)
        if self._user_profile_enabled:
            user_block = self._store.format_for_system_prompt("user")
            if user_block:
                parts.append(user_block)
        return "\n\n".join(parts)

    def get_tool_schemas(self) -> List[Dict[str, Any]]:
        return []

    def handle_tool_call(self, tool_name: str, args: Dict[str, Any], **kwargs) -> str:
        return json.dumps({"error": "Built-in memory tool is handled by the runtime service"})

    @property
    def store(self):
        return self._store
