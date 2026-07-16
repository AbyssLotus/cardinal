"""NPC agents: goals, schedules, decisions (§7).

Per hour-tick, every named NPC:
  1. decays needs (rates from rules.yaml npc.needs_decay_per_hour)
  2. scores candidate activities:
       score = schedule_adherence + need_urgency + goal_priority + idle
  3. executes the winner (move, eat, sleep, socialize, pursue goal)
  4. emits world effects as deltas: entity_state (position/needs/activity),
     npc_memory (conversations), goal_progress

Everything is deterministic: NPCs are processed in sorted-id order, all
randomness comes from ctx.rng.stream("npc"), and all weights/rates are
world data — engine defaults only kick in when rules.yaml is silent.
"""

from __future__ import annotations

from typing import Any

from engine.persistence.store import Delta
from engine.systems import SystemContext

_DEFAULT_DECAY = {"food": 0.03, "rest": 0.05, "income": 0.01}
_DEFAULT_WEIGHTS = {"schedule": 0.5, "need": 1.2, "goal": 0.4, "idle": 0.1}


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    if granularity == "day":
        return _daily_memory_pass(ctx, day)
    if granularity != "hour":
        return []
    rule = ctx.registry.rule
    decay_rates = rule("npc.needs_decay_per_hour", _DEFAULT_DECAY)
    urgent_below = rule("npc.need_urgent_threshold", 0.35)
    weights = {**_DEFAULT_WEIGHTS, **(rule("npc.weights", {}) or {})}
    socialize_chance = rule("npc.socialize_memory_chance", 0.5)
    rng = ctx.rng.stream("npc")

    deltas: list[Delta] = []
    for npc in sorted(ctx.registry.by_kind("npc"), key=lambda n: n.id):
        if getattr(npc, "actor_class", "ambient") != "ambient":
            continue  # agents run their own loop (agents.py); setpieces are scripted
        runtime = ctx.store.get_entity(npc.id)
        if runtime is None or not runtime["state"].get("alive", True):
            continue
        state: dict[str, Any] = runtime["state"]
        location: str = runtime["location_id"] or getattr(npc, "location", "")

        # 1. needs decay
        needs = state.get("needs", {})
        for need, rate in decay_rates.items():
            needs[need] = max(0.0, round(needs.get(need, 1.0) - rate, 4))

        # 2. score candidates — fixed construction order keeps ties deterministic
        candidates: list[tuple[float, str, str]] = [(weights["idle"], "idle", location)]

        block = _schedule_block(npc, hour)
        scheduled_activity = None
        if block is not None:
            at = ctx.registry.resolve_location(block.at) or location
            scheduled_activity = block.activity
            candidates.append((weights["schedule"], block.activity, at))

        if needs.get("food", 1.0) < urgent_below:
            candidates.append(((1.0 - needs["food"]) * weights["need"], "eat", location))
        if needs.get("rest", 1.0) < urgent_below or (block is None and _is_night(hour)):
            urgency = max(1.0 - needs.get("rest", 1.0), 0.3)
            home = state.get("home", location)
            candidates.append((urgency * weights["need"], "sleep", home))

        for goal in getattr(npc, "goals", []):
            if goal.status == "active":
                candidates.append((goal.priority * weights["goal"], f"pursue:{goal.id}", location))
                break  # one goal candidate: the highest-priority authored first

        score, activity, target = max(candidates, key=lambda c: c[0])

        # 3./4. execute and emit effects
        if activity == "eat":
            needs["food"] = 1.0
        elif activity == "sleep":
            needs["rest"] = min(1.0, round(needs.get("rest", 1.0) + 0.2, 4))
        elif activity.startswith("pursue:"):
            deltas.append(Delta(kind="goal_progress", payload={
                "npc_id": npc.id, "goal_id": activity.split(":", 1)[1], "effort_add": 1,
            }))
        elif activity == "socialize":
            deltas += _socialize(ctx, npc, target, socialize_chance, rng)
        if activity == scheduled_activity and "income" in needs:
            # an hour worked is an hour earned — schedules sustain livelihoods
            needs["income"] = min(1.0, round(
                needs["income"] + rule("npc.income_per_work_hour", 0.05), 4))

        state["needs"] = needs
        state["activity"] = activity
        deltas.append(Delta(
            kind="entity_state",
            payload={"id": npc.id, "kind": "npc", "def_id": npc.id, "state": state},
            location_id=target,
        ))
    return deltas


def _schedule_block(npc, hour: int):
    for block in getattr(npc, "schedule", []):
        start = int(block.from_.split(":")[0])
        end = int(block.to.split(":")[0])
        if start <= hour < end:
            return block
    return None


def _is_night(hour: int) -> bool:
    return hour >= 22 or hour < 6


def _daily_memory_pass(ctx: SystemContext, day: int) -> list[Delta]:
    """§5.2 daily work: salience decay and rumor propagation.

    Memories decay in salience (never deleted) unless flagged permanent.
    Co-located NPCs pass their strongest memories along as rumors with
    reduced certainty — this is how reputation spreads."""
    ctx.store.conn.execute(
        "UPDATE npc_memory SET salience = ROUND(salience * 0.97, 4) WHERE decays = 1")

    rng = ctx.rng.stream("rumors")
    chance = ctx.registry.rule("npc.rumor_spread_chance", 0.4)
    deltas: list[Delta] = []
    by_location: dict[str, list[str]] = {}
    for npc in sorted(ctx.registry.by_kind("npc"), key=lambda n: n.id):
        if getattr(npc, "actor_class", "ambient") != "ambient":
            continue  # agents run their own loop (agents.py); setpieces are scripted
        runtime = ctx.store.get_entity(npc.id)
        if runtime is not None and runtime["location_id"]:
            by_location.setdefault(runtime["location_id"], []).append(npc.id)

    for location in sorted(by_location):
        group = by_location[location]
        if len(group) < 2:
            continue
        for source in group:
            strong = ctx.store.conn.execute(
                "SELECT * FROM npc_memory WHERE npc_id=? AND kind != 'rumor' "
                "AND ABS(valence) >= 0.5 ORDER BY id DESC LIMIT 2",
                (source,)).fetchall()
            for memory in strong:
                for listener in group:
                    if listener == source or rng.random() >= chance:
                        continue
                    already = ctx.store.conn.execute(
                        "SELECT 1 FROM npc_memory WHERE npc_id=? AND summary=? LIMIT 1",
                        (listener, memory["summary"])).fetchone()
                    if already:
                        continue
                    certainty = round((memory["rumor_certainty"] or 1.0) * 0.75, 2)
                    deltas.append(Delta(kind="npc_memory", payload={
                        "npc_id": listener, "kind": "rumor",
                        "subject_id": memory["subject_id"],
                        "valence": round(memory["valence"] * 0.8, 2),
                        "summary": memory["summary"],
                        "rumor_certainty": certainty}))
    return deltas


def _socialize(ctx: SystemContext, npc, location: str, chance: float, rng) -> list[Delta]:
    """Co-located NPCs exchange pleasantries; some of it sticks as memory.
    Rumor propagation proper (mutating certainty) is M4 — this seeds the
    social graph those rumors will travel."""
    deltas: list[Delta] = []
    for other in ctx.store.entities_at(location, kind="npc"):
        if other["id"] == npc.id:
            continue
        if rng.random() >= chance:
            continue
        other_def = ctx.registry.find(other["id"])
        other_name = getattr(other_def, "name", other["id"]) if other_def else other["id"]
        # §25.1: conversations carry topics, not just pleasantries — sourced
        # from the speaker's authored knowledge or the recent public
        # chronicle, so eavesdropping yields usable intel.
        summary = f"Talked with {other_name}."
        knowledge = list(getattr(npc, "knowledge", []) or [])
        if knowledge and rng.random() < 0.5:
            entry = knowledge[rng.randrange(len(knowledge))]
            topic = getattr(entry, "topic", None) or "something they know"
            summary = f"Talked with {other_name} about {topic}."
        else:
            recent = [e for e in ctx.store.get_chronicle(12)
                      if e.get("visibility") in (None, "public", "regional")]
            if recent and rng.random() < 0.4:
                event = recent[rng.randrange(len(recent))]
                summary = (f"Talked with {other_name} about the news: "
                           f"{event['headline']}")
        deltas.append(Delta(kind="npc_memory", payload={
            "npc_id": npc.id,
            "kind": "conversation",
            "subject_id": other["id"],
            "valence": 0.1,
            "summary": summary,
        }))
    return deltas


def seed_runtime_state(ctx: SystemContext, day: int) -> list[Delta]:
    """Called once at save creation: materialize every named NPC as a runtime entity."""
    deltas = []
    for npc in sorted(ctx.registry.by_kind("npc"), key=lambda n: n.id):
        if getattr(npc, "actor_class", "ambient") != "ambient":
            continue  # agents run their own loop (agents.py); setpieces are scripted
        home = getattr(npc, "location", None)
        deltas.append(
            Delta(
                kind="entity_state",
                payload={
                    "id": npc.id,
                    "kind": "npc",
                    "def_id": npc.id,
                    "state": {
                        "needs": dict(getattr(npc, "needs", {})),
                        "home": home,
                        "alive": True,
                    },
                },
                location_id=home,
            )
        )
    return deltas
