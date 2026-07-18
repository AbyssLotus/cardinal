"""Quest lifecycle & expiry (§13).

Quests are NPC goals exposed to the world; they run whether or not the
player participates. Day-tick:
  - authored quest defs without an instance become `available` (chronicled)
  - `npc_fallback.chance_npc_resolves_per_day` rolls — the world doesn't wait
  - past `expires_day`, the quest fails: failure.world_effects apply and the
    outcome enters the chronicle regardless of player awareness

§23 additions: quest instances carry an `assignee` (player | npc.<id> |
NULL = open). quest_taker agents claim and execute contracts through real
actions (see agents.py). If a claiming agent dies mid-quest, the contract
reopens or fails per the quest's `on_assignee_death`. `npc_fallback` is
DEPRECATED: the coin-flip only rolls in worlds with no quest_taker agents,
and will be removed once all shipped worlds have takers.

All text comes from quest data (purpose, failure.outcome, npc_fallback.outcome);
the engine composes, never authors.
"""

from __future__ import annotations

from engine.persistence.store import Delta
from engine.systems import SystemContext

TERMINAL_STATES = {"completed", "failed", "expired", "npc_resolved"}


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    if granularity != "day":
        return []
    rng = ctx.rng.stream("quests")
    deltas: list[Delta] = []
    instances = {q["instance_id"]: q for q in ctx.store.get_quests()}

    for quest in sorted(ctx.registry.by_kind("quest"), key=lambda q: q.id):
        instance_id = quest.id
        instance = instances.get(instance_id)

        if instance is None:
            expires = day + quest.duration_days if quest.duration_days else None
            deltas.append(Delta(kind="quest_state", payload={
                "instance_id": instance_id, "def_id": quest.id, "state": "available",
                "available_day": day, "expires_day": expires,
            }))
            source = ctx.registry.find(quest.source)
            source_name = getattr(source, "name", quest.source) if source else quest.source
            source_loc = _npc_location(ctx, quest.source)
            deltas.append(Delta(
                kind="chronicle",
                payload={
                    "category": "discovery",
                    "headline": f"{source_name} seeks help: {quest.purpose}",
                    "detail": f"({quest.name})",
                    "actors": [quest.source],
                },
                location_id=source_loc,
            ))
            continue

        if instance["state"] in TERMINAL_STATES:
            continue

        # §23: a dead assignee releases the contract — reopen or fail per
        # the quest's definition; the outcome the old coin-flip couldn't say.
        assignee = instance.get("assignee")
        if assignee and assignee != "player":
            holder = ctx.store.get_entity(assignee)
            if holder is not None and not holder["state"].get("alive", True):
                holder_def = ctx.registry.find(assignee)
                holder_name = getattr(holder_def, "name", assignee)
                if quest.on_assignee_death == "fail":
                    deltas += _terminate(ctx, quest, instance, "failed",
                                         f"{quest.name} died with {holder_name}.",
                                         day)
                    for effect in quest.failure.world_effects:
                        deltas += _apply_world_effect(ctx, effect)
                else:
                    deltas.append(Delta(kind="quest_state", payload={
                        "instance_id": instance["instance_id"], "def_id": quest.id,
                        "state": "available",
                        "available_day": instance["available_day"],
                        "expires_day": instance["expires_day"], "assignee": None}))
                    deltas.append(Delta(kind="chronicle", payload={
                        "category": "discovery",
                        "headline": f"The contract lapses with {holder_name}'s "
                                    f"death: {quest.purpose}",
                        "detail": f"({quest.name}: reopened)",
                        "actors": [quest.source]},
                        location_id=_npc_location(ctx, quest.source)))
                continue

        # DEPRECATED (§23): the fallback coin-flip only exists for worlds
        # with no quest_taker agents; real takers replace it entirely.
        fallback_chance = 0.0
        if not _world_has_takers(ctx):
            fallback_chance = quest.npc_fallback.get("chance_npc_resolves_per_day", 0.0)
        if fallback_chance and rng.random() < fallback_chance:
            deltas += _terminate(ctx, quest, instance, "npc_resolved",
                                 quest.npc_fallback.get(
                                     "outcome", f"{quest.name} was seen to without fanfare."),
                                 day)
            continue

        if instance["expires_day"] is not None and day >= instance["expires_day"]:
            deltas += _terminate(ctx, quest, instance, "expired",
                                 quest.failure.outcome or f"{quest.name} went unanswered.",
                                 day)
            for effect in quest.failure.world_effects:
                deltas += _apply_world_effect(ctx, effect)
    return deltas


def _world_has_takers(ctx: SystemContext) -> bool:
    return any(getattr(n, "actor_class", "ambient") == "agent"
               and n.policy == "quest_taker"
               for n in ctx.registry.by_kind("npc"))


def _terminate(ctx: SystemContext, quest, instance, state: str,
               outcome: str, day: int) -> list[Delta]:
    return [
        Delta(kind="quest_state", payload={
            "instance_id": instance["instance_id"], "def_id": quest.id, "state": state,
            "available_day": instance["available_day"], "expires_day": instance["expires_day"],
            "assignee": instance.get("assignee"),
        }),
        Delta(
            kind="chronicle",
            payload={
                "category": "discovery" if state == "npc_resolved" else "disaster",
                "headline": outcome,
                "detail": f"({quest.name}: {state})",
                "actors": [quest.source],
            },
            location_id=_npc_location(ctx, quest.source),
        ),
    ]


def _apply_world_effect(ctx: SystemContext, effect: dict) -> list[Delta]:
    if effect.get("type") == "npc_state":
        target = effect["target"]
        runtime = ctx.store.get_entity(target)
        state = runtime["state"] if runtime else {}
        state.update(effect.get("set", {}))
        location = runtime["location_id"] if runtime else None
        return [Delta(kind="entity_state",
                      payload={"id": target, "kind": "npc", "def_id": target, "state": state},
                      location_id=location)]
    return []  # other effect types (economy shocks, faction shifts) land in M4


def _npc_location(ctx: SystemContext, npc_id: str) -> str | None:
    runtime = ctx.store.get_entity(npc_id)
    if runtime is not None:
        return runtime["location_id"]
    definition = ctx.registry.find(npc_id)
    return getattr(definition, "location", None) if definition else None
