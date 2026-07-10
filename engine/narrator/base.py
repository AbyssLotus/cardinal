"""Narrator interface (§15).

The narrator is strictly read-only: it receives committed, perception-filtered
deltas plus context, and returns prose. It has no tool access and no ability
to mutate state. The engine must run headless with the narrator disabled —
LLM output is never a dependency of state computation.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any

from engine.persistence.store import Delta


@dataclass
class PerceptionContext:
    """Everything the player can legitimately perceive this turn (§15)."""

    clock_label: str
    location_snapshot: dict[str, Any] = field(default_factory=dict)
    present_npcs: list[dict[str, Any]] = field(default_factory=list)
    player_status: dict[str, Any] = field(default_factory=dict)
    recent_history: list[dict[str, Any]] = field(default_factory=list)


class Narrator(ABC):
    @abstractmethod
    def render(self, deltas: list[Delta], perception: PerceptionContext) -> str:
        """Turn committed, perceivable deltas into player-facing text."""
