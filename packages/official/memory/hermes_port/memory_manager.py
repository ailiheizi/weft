from __future__ import annotations

import json
import logging
from typing import Any, Dict, List, Optional

from hermes_port.memory_provider import MemoryProvider

LOGGER = logging.getLogger("memory-runtime")


class MemoryManager:
    """Orchestrate builtin memory plus at most one external provider."""

    def __init__(self) -> None:
        self._providers: List[MemoryProvider] = []
        self._tool_to_provider: Dict[str, MemoryProvider] = {}
        self._has_external = False

    def add_provider(self, provider: MemoryProvider) -> None:
        is_builtin = provider.name == "builtin"
        if not is_builtin:
            if self._has_external:
                existing = next((item.name for item in self._providers if item.name != "builtin"), "unknown")
                LOGGER.warning(
                    "Rejected memory provider '%s' because external provider '%s' is already active.",
                    provider.name,
                    existing,
                )
                return
            self._has_external = True

        self._providers.append(provider)
        for schema in provider.get_tool_schemas():
            tool_name = str(schema.get("name", "")).strip()
            if tool_name and tool_name not in self._tool_to_provider:
                self._tool_to_provider[tool_name] = provider
            elif tool_name:
                LOGGER.warning(
                    "Memory tool name conflict for '%s': keeping %s, ignoring %s",
                    tool_name,
                    self._tool_to_provider[tool_name].name,
                    provider.name,
                )

    @property
    def providers(self) -> List[MemoryProvider]:
        return list(self._providers)

    @property
    def provider_names(self) -> List[str]:
        return [provider.name for provider in self._providers]

    def get_provider(self, name: str) -> Optional[MemoryProvider]:
        return next((provider for provider in self._providers if provider.name == name), None)

    def build_system_prompt(self) -> str:
        blocks: List[str] = []
        for provider in self._providers:
            try:
                block = provider.system_prompt_block()
            except Exception as error:
                LOGGER.warning("Memory provider '%s' system_prompt_block failed: %s", provider.name, error)
                continue
            if block and block.strip():
                blocks.append(block)
        return "\n\n".join(blocks)

    def prefetch_all(self, query: str, *, session_id: str = "") -> str:
        parts: List[str] = []
        for provider in self._providers:
            try:
                block = provider.prefetch(query, session_id=session_id)
            except Exception as error:
                LOGGER.debug("Memory provider '%s' prefetch failed: %s", provider.name, error)
                continue
            if block and block.strip():
                parts.append(block)
        return "\n\n".join(parts)

    def queue_prefetch_all(self, query: str, *, session_id: str = "") -> None:
        for provider in self._providers:
            try:
                provider.queue_prefetch(query, session_id=session_id)
            except Exception as error:
                LOGGER.debug("Memory provider '%s' queue_prefetch failed: %s", provider.name, error)

    def sync_all(self, user_content: str, assistant_content: str, *, session_id: str = "") -> None:
        for provider in self._providers:
            try:
                provider.sync_turn(user_content, assistant_content, session_id=session_id)
            except Exception as error:
                LOGGER.warning("Memory provider '%s' sync_turn failed: %s", provider.name, error)

    def get_all_tool_schemas(self) -> List[Dict[str, Any]]:
        seen = set()
        schemas: List[Dict[str, Any]] = []
        for provider in self._providers:
            try:
                provider_schemas = provider.get_tool_schemas()
            except Exception as error:
                LOGGER.warning("Memory provider '%s' get_tool_schemas failed: %s", provider.name, error)
                continue
            for schema in provider_schemas:
                tool_name = str(schema.get("name", "")).strip()
                if tool_name and tool_name not in seen:
                    seen.add(tool_name)
                    schemas.append(schema)
        return schemas

    def get_all_tool_names(self) -> set[str]:
        return set(self._tool_to_provider.keys())

    def has_tool(self, tool_name: str) -> bool:
        return tool_name in self._tool_to_provider

    def handle_tool_call(self, tool_name: str, args: Dict[str, Any], **kwargs) -> str:
        provider = self._tool_to_provider.get(tool_name)
        if provider is None:
            return json.dumps({"error": f"No memory provider handles tool '{tool_name}'"})
        try:
            return provider.handle_tool_call(tool_name, args, **kwargs)
        except Exception as error:
            LOGGER.error("Memory provider '%s' handle_tool_call(%s) failed: %s", provider.name, tool_name, error)
            return json.dumps({"error": f"Memory tool '{tool_name}' failed: {error}"})

    def initialize_all(self, session_id: str, **kwargs) -> None:
        for provider in self._providers:
            try:
                provider.initialize(session_id=session_id, **kwargs)
            except Exception as error:
                LOGGER.warning("Memory provider '%s' initialize failed: %s", provider.name, error)

    def on_memory_write(self, action: str, target: str, content: str) -> None:
        for provider in self._providers:
            if provider.name == "builtin":
                continue
            try:
                provider.on_memory_write(action, target, content)
            except Exception as error:
                LOGGER.debug("Memory provider '%s' on_memory_write failed: %s", provider.name, error)

    def on_pre_compress(self, messages: List[Dict[str, Any]]) -> str:
        parts: List[str] = []
        for provider in self._providers:
            try:
                result = provider.on_pre_compress(messages)
            except Exception as error:
                LOGGER.debug("Memory provider '%s' on_pre_compress failed: %s", provider.name, error)
                continue
            if result and result.strip():
                parts.append(result)
        return "\n\n".join(parts)

    def on_delegation(self, task: str, result: str, *, child_session_id: str = "", **kwargs) -> None:
        for provider in self._providers:
            try:
                provider.on_delegation(task, result, child_session_id=child_session_id, **kwargs)
            except Exception as error:
                LOGGER.debug("Memory provider '%s' on_delegation failed: %s", provider.name, error)

    def shutdown_all(self) -> None:
        for provider in reversed(self._providers):
            try:
                provider.shutdown()
            except Exception as error:
                LOGGER.warning("Memory provider '%s' shutdown failed: %s", provider.name, error)
