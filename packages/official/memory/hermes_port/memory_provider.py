from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any, Dict, List


class MemoryProvider(ABC):
    @property
    @abstractmethod
    def name(self) -> str:
        """Short identifier for this provider."""

    @abstractmethod
    def is_available(self) -> bool:
        """Return whether the provider is ready for use."""

    @abstractmethod
    def initialize(self, session_id: str, **kwargs) -> None:
        """Initialize provider state for a session."""

    def system_prompt_block(self) -> str:
        return ""

    def prefetch(self, query: str, *, session_id: str = "") -> str:
        return ""

    def queue_prefetch(self, query: str, *, session_id: str = "") -> None:
        return None

    def sync_turn(self, user_content: str, assistant_content: str, *, session_id: str = "") -> None:
        return None

    @abstractmethod
    def get_tool_schemas(self) -> List[Dict[str, Any]]:
        """Return tool schemas this provider exposes."""

    def handle_tool_call(self, tool_name: str, args: Dict[str, Any], **kwargs) -> str:
        raise NotImplementedError(f"Provider {self.name} does not handle tool {tool_name}")

    def shutdown(self) -> None:
        return None

    def on_turn_start(self, turn_number: int, message: str, **kwargs) -> None:
        return None

    def on_session_end(self, messages: List[Dict[str, Any]]) -> None:
        return None

    def on_pre_compress(self, messages: List[Dict[str, Any]]) -> str:
        return ""

    def on_delegation(self, task: str, result: str, *, child_session_id: str = "", **kwargs) -> None:
        return None

    def get_config_schema(self) -> List[Dict[str, Any]]:
        return []

    def save_config(self, values: Dict[str, Any], hermes_home: str) -> None:
        return None

    def on_memory_write(self, action: str, target: str, content: str) -> None:
        return None
