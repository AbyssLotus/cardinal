"""Skill usage & proficiency growth (§9), XP and leveling.

Proficiency grows per validated use (`growth.per_use`), halved against
much-lower-level targets. Techniques are never stored — a technique is
"known" iff its parent skill's proficiency has reached the unlock threshold,
so the skill file is always the single source of truth.

The XP curve is a world-data formula (`rules.leveling.xp_curve`), evaluated
with only `level` in scope.
"""

from __future__ import annotations

from engine.core.registry import Registry
from engine.persistence.store import Delta, Store
from engine.systems import SystemContext

LOW_LEVEL_GAP = 5  # targets this far below the actor teach little


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    return []


def gain_proficiency(store: Store, registry: Registry, skill_id: str,
                     target_level: int, actor_level: int) -> float | None:
    """One validated use of a skill. Returns the new proficiency, or None if
    the actor doesn't have the skill."""
    current = store.get_player_skill(skill_id)
    if current is None:
        return None
    skill = registry.find(skill_id)
    if skill is None:
        return current
    gain = skill.growth.per_use
    if skill.growth.diminishing_vs_lower_level and target_level <= actor_level - LOW_LEVEL_GAP:
        gain *= 0.5
    new = min(float(skill.proficiency_max), round(current + gain, 2))
    store.upsert_player_skill(skill_id, new)
    return new


def known_techniques(store: Store, registry: Registry) -> dict[str, object]:
    """Technique name (lowercase) -> Technique, for every unlock the player's
    proficiencies have crossed."""
    known: dict[str, object] = {}
    for skill_id, proficiency in store.get_player_skills().items():
        skill = registry.find(skill_id)
        if skill is None:
            continue
        for unlock in getattr(skill, "unlocks", []):
            if proficiency >= unlock.at:
                technique = registry.find(unlock.grants)
                if technique is not None:
                    known[technique.name.lower()] = technique
    return known


def xp_to_next(registry: Registry, level: int) -> int:
    formula = registry.rule("leveling.xp_curve", "100 * level ** 1.8")
    return int(eval(formula, {"__builtins__": {}}, {"level": level}))  # noqa: S307 — world data, no builtins


def award_xp(store: Store, registry: Registry, amount: int) -> list[Delta]:
    """Adds XP, resolves any level-ups, returns the resulting deltas."""
    player = store.get_player()
    xp = player["xp"] + amount
    level = player["level"]
    hp_max = player["hp_max"]
    hp_per_level = registry.rule("leveling.hp_per_level", 10)
    deltas: list[Delta] = []
    while xp >= xp_to_next(registry, level):
        xp -= xp_to_next(registry, level)
        level += 1
        hp_max += hp_per_level
        deltas.append(Delta(kind="player_history", payload={
            "kind": "milestone", "summary": f"Reached level {level}.",
        }))
    deltas.insert(0, Delta(kind="player_update", payload={
        "xp": xp, "level": level, "hp_max": hp_max,
    }))
    return deltas
