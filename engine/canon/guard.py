"""Canon-tier tagging & contradiction checks (§12).

Every generated entity must pass `check()` before entering the world:
no id collisions with confirmed canon, valid tier tagging, no rule
violations. Deep semantic contradiction checks grow through M5.
"""

from __future__ import annotations

from engine.core.registry import Registry
from engine.schemas import Entity


class CanonViolation(Exception):
    pass


def check(entity: Entity, registry: Registry) -> None:
    existing = registry.find(entity.id)
    if existing is not None and existing.canon_tier == "confirmed":
        raise CanonViolation(
            f"{entity.id}: collides with confirmed canon — definitions may not be replaced"
        )
    if entity.canon_tier == "confirmed" and existing is None:
        raise CanonViolation(
            f"{entity.id}: generated content may not claim 'confirmed' tier"
        )
