"""Internal event bus: system → system signals.

Synchronous, in-process, deterministic dispatch order (subscription order).
Events are transient signals inside a single turn; durable facts belong in
the chronicle, not here.
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any, Callable


@dataclass(frozen=True)
class Event:
    kind: str
    payload: dict[str, Any] = field(default_factory=dict)


class EventBus:
    def __init__(self):
        self._subscribers: dict[str, list[Callable[[Event], None]]] = defaultdict(list)

    def subscribe(self, kind: str, handler: Callable[[Event], None]) -> None:
        self._subscribers[kind].append(handler)

    def emit(self, kind: str, **payload: Any) -> None:
        event = Event(kind, payload)
        for handler in self._subscribers[kind]:
            handler(event)
