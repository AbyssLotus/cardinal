"""Action type definitions.

An Action is the sole unit of player intent the loop resolves. Natural
language never reaches the simulation — only these structured objects do.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class Action:
    intent: str                       # "wait" | "look" | "travel" | "status" | ...
    target: str | None = None         # entity id where applicable
    parameters: dict[str, Any] = field(default_factory=dict)
    raw_input: str = ""
