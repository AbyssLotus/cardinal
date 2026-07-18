"""Simulation systems. Each module exposes:

    tick(ctx, granularity, day, hour) -> list[Delta]

where granularity is "minute" | "hour" | "day" (§6). Systems read world
definitions from ctx.registry, runtime state from ctx.store, and randomness
from ctx.rng.stream("<system>") — never module-level RNG.

M1 ships these as wired stubs; M2–M4 fill them in.
"""

from __future__ import annotations

from dataclasses import dataclass

from engine.core.events import EventBus
from engine.core.registry import Registry
from engine.core.rng import RngManager
from engine.persistence.store import Store


@dataclass
class SystemContext:
    registry: Registry
    store: Store
    rng: RngManager
    bus: EventBus


# Deterministic tick order: same order every turn, every save.
TICK_ORDER = [
    "weather",
    "ecology",
    "npc",
    "agents",
    "economy",
    "quests",
    "factions",
    "worldevents",
]
