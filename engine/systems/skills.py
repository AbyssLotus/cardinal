"""Skill usage & proficiency growth (§9). M3 milestone — stub."""

from __future__ import annotations

from engine.persistence.store import Delta
from engine.systems import SystemContext


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    return []
