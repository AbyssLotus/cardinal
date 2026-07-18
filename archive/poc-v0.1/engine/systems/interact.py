"""Generic interaction system: skill checks against world objects.

`use/hack/pick/open/press <device>` resolves as:
  success chance = 0.6 + (proficiency - difficulty) / 200, clamped [0.05, 0.95]
  (interactions without a skill always succeed — a plain lever)

Outcomes are authored world data (InteractionOutcome): device state changes,
messages, and effects the engine applies — teleport, give_item, npc_state,
chronicle. This one primitive is hacking, lockpicking, shrine prayers, and
jack-in/jack-out netrunning (a terminal that teleports your presence into a
net-architecture location and an exit node that brings you back).

Deterministic: all rolls from rng stream "interact".
"""

from __future__ import annotations

from typing import Any

from engine.persistence.store import Delta
from engine.systems import SystemContext


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    return []


def find_device(ctx: SystemContext, name: str, location_id: str):
    """Match a device by name or id fragment at the player's location."""
    needle = name.lower()
    for device in sorted(ctx.registry.by_kind("device"), key=lambda d: d.id):
        if device.location != location_id:
            continue
        if needle in device.name.lower() or needle in device.id:
            return device
    return None


def find_interaction(device, verb: str):
    for interaction in device.interactions:
        if interaction.verb == verb:
            return interaction
    # `use` is the universal fallback verb
    if verb == "use" and device.interactions:
        return device.interactions[0]
    return None


def device_state(ctx: SystemContext, device) -> str:
    runtime = ctx.store.get_entity(device.id)
    if runtime is None:
        return device.initial_state
    return runtime["state"].get("state", device.initial_state)


def resolve(ctx: SystemContext, device, interaction,
            now_minutes: int | None = None) -> tuple[list[Delta], list[str]]:
    messages: list[str] = []
    deltas: list[Delta] = []

    if interaction.requires_state is not None:
        current = device_state(ctx, device)
        if current != interaction.requires_state:
            return [], [f"The {device.name} is {current}; nothing happens."]

    if interaction.requires_item is not None:
        owned = any(i["def_id"] == interaction.requires_item
                    for i in ctx.store.get_inventory("player"))
        if not owned:
            item_def = ctx.registry.find(interaction.requires_item)
            needed = getattr(item_def, "name", interaction.requires_item)
            return [], [f"You need a {needed} for that."]

    # Lockout after repeated failure (Test 5 playtest finding): without
    # this, failure on a skill-checked device costs nothing but in-game
    # time, so any lock in any world is brute-forceable by retry-spam —
    # the 5% success floor guarantees it. Engine-level default so world
    # authors don't have to hand-write consequences onto every device;
    # tunable/disable-able per world via rules.yaml `interact:` keys.
    lockout_after = ctx.registry.rule("interact.lockout_after_fails", 3)
    runtime = ctx.store.get_entity(device.id)
    dstate = dict(runtime["state"]) if runtime else {}
    if interaction.skill is not None and lockout_after > 0 and now_minutes is not None:
        locked_until = dstate.get("lockout_until")
        if locked_until is not None and now_minutes < locked_until:
            return [], [f"The {device.name} refuses you — its countermeasures "
                        f"are still up. Give it time."]

    if interaction.skill is not None:
        proficiency = ctx.store.get_player_skill(interaction.skill) or 0.0
        chance = max(0.05, min(0.95, 0.6 + (proficiency - interaction.difficulty) / 200))
        succeeded = ctx.rng.stream("interact").random() < chance
    else:
        succeeded = True

    outcome = interaction.success if succeeded else interaction.failure
    if outcome.message:
        messages.append(outcome.message)
    elif succeeded:
        messages.append(f"You {interaction.verb} the {device.name}.")
    else:
        messages.append(f"You fail to {interaction.verb} the {device.name}.")

    if succeeded and interaction.skill is not None:
        from engine.systems import skills as skills_system
        skills_system.gain_proficiency(ctx.store, ctx.registry, interaction.skill,
                                       target_level=1,
                                       actor_level=ctx.store.get_player()["level"])

    # track the fail streak; trip the lockout when it crosses the threshold
    if interaction.skill is not None and lockout_after > 0:
        if succeeded:
            dstate.pop("fail_streak", None)
            dstate.pop("lockout_until", None)
        else:
            streak = dstate.get("fail_streak", 0) + 1
            dstate["fail_streak"] = streak
            if streak >= lockout_after and now_minutes is not None:
                lockout_minutes = ctx.registry.rule("interact.lockout_minutes", 60)
                dstate["fail_streak"] = 0
                dstate["lockout_until"] = now_minutes + lockout_minutes
                messages.append(f"The {device.name} locks down against your "
                                f"fumbling. It will take time to try again.")
        if outcome.set_state is not None:
            dstate["state"] = outcome.set_state
        ctx.store.upsert_entity(device.id, "device", device.id,
                                dstate, device.location, 0)
    elif outcome.set_state is not None:
        dstate["state"] = outcome.set_state
        ctx.store.upsert_entity(device.id, "device", device.id,
                                dstate, device.location, 0)

    for effect in outcome.effects:
        deltas += _apply_effect(ctx, effect, messages)
    return deltas, messages


def _apply_effect(ctx: SystemContext, effect: dict[str, Any],
                  messages: list[str]) -> list[Delta]:
    kind = effect.get("type")
    if kind == "teleport":
        destination = effect["to"]
        place = ctx.registry.find(destination)
        place_name = getattr(place, "name", destination) if place else destination
        messages.append(f"The world drops away — you are at {place_name}.")
        return [
            Delta(kind="player_update", payload={"location_id": destination}),
            Delta(kind="player_history", payload={
                "kind": "travel", "summary": f"Transferred to {place_name}.",
                "refs": [destination]}),
        ]
    if kind == "give_item":
        item_id = effect["item"]
        qty = effect.get("qty", 1)
        item_def = ctx.registry.find(item_id)
        durability = (getattr(item_def, "stats", {}) or {}).get("durability_max")
        instance_id = f"iteminst.dev_{ctx.rng.stream('interact').randrange(1 << 30):08x}"
        ctx.store.add_item_instance(instance_id, item_id, "player",
                                    durability=durability, qty=qty)
        messages.append(f"Acquired {qty}x {getattr(item_def, 'name', item_id)}.")
        return []
    if kind == "npc_state":
        target = effect["target"]
        runtime = ctx.store.get_entity(target)
        state = runtime["state"] if runtime else {}
        state.update(effect.get("set", {}))
        return [Delta(kind="entity_state",
                      payload={"id": target, "kind": "npc", "def_id": target,
                               "state": state},
                      location_id=runtime["location_id"] if runtime else None)]
    if kind == "chronicle":
        return [Delta(kind="chronicle", payload={
            "category": effect.get("category", "discovery"),
            "headline": effect.get("headline", ""),
            "detail": effect.get("detail", ""),
            "visibility": effect.get("visibility", "public")})]
    return []
