"""Combat resolution (§8), delivery-agnostic. M3 implementation.

Model: 1-second rounds. Positions are per-monster distances to the player
(a 1D simplification of the spec's 2D plane — flanking is modeled as an AI
bonus; full 2D positioning is an M6 refinement, documented deviation).

Delivery paths per the Technique schema:
  melee      — must close to range_m; defender in ready state may react
  thrown     — consumes the weapon item; partial recovery after victory
  projectile — consumes ranged.ammo per shot within max_range_m
  (beam/area land when content needs them)

Core timing rule: a defender reacts only when ready (not frozen in a
post-skill delay) AND its reaction_ms beats the attack's activation+execution.
The sword-skill freeze is a real cost: post_delay_ms // 1000 = frozen rounds.

No level scaling, ever. Permadeath per world rules applies. All constants
from rules.yaml `combat:`; every roll from rng stream "combat".

Encounter state persists in the entities table (id `encounter.player`) —
the world, including a fight you quit mid-swing, survives restarts.
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass, field
from typing import Any

from engine.persistence.store import Delta
from engine.systems import SystemContext

ENCOUNTER_ID = "encounter.player"

BASIC_STRIKE = {"name": "strike", "delivery": "melee", "hits": 1, "damage_multiplier": 1.0,
                "activation_ms": 300, "execution_ms": 300, "post_delay_ms": 300,
                "range_m": 1.5, "cooldown_s": 0, "ammo_per_use": 0}
BASIC_SHOT = {"name": "shot", "delivery": "projectile", "hits": 1, "damage_multiplier": 1.0,
              "activation_ms": 300, "execution_ms": 200, "post_delay_ms": 0,
              "cooldown_s": 0, "ammo_per_use": 1}
BASIC_THROW = {"name": "throw", "delivery": "thrown", "hits": 1, "damage_multiplier": 1.0,
               "activation_ms": 250, "execution_ms": 250, "post_delay_ms": 0,
               "cooldown_s": 0, "ammo_per_use": 0}


@dataclass
class RoundResult:
    events: list[str] = field(default_factory=list)
    deltas: list[Delta] = field(default_factory=list)
    done: bool = False
    outcome: str | None = None  # victory | death | fled


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    return []


# --------------------------------------------------------------------- setup


def get_encounter(ctx: SystemContext) -> dict[str, Any] | None:
    row = ctx.store.get_entity(ENCOUNTER_ID)
    return row["state"] if row and row["state"].get("monsters") else None


def start_duel(ctx: SystemContext, npc_id: str,
               location_id: str) -> tuple[dict[str, Any], list[str]]:
    """Player-vs-agent combat (§22.4): the SAME full state machine as
    monster combat — stances, techniques, timing gates, stamina — with the
    agent's live stats on the other side. state['duel'] marks the mode;
    victory/death branch to agent permadeath + corpse looting instead of
    populations + drops."""
    from engine.systems import agents as agents_system
    combatant = agents_system.duel_combatant(ctx, npc_id)
    state = {
        "species": npc_id,
        "duel": npc_id,
        "zone": None,
        "location": location_id,
        "monsters": [{"tag": combatant.name, "hp": combatant.stats.hp,
                      "dist": 4.0, "frozen": 0, "attacked_once": False}],
        "player": {"stamina": ctx.registry.rule("combat.stamina_max", 100),
                   "guard": "dodge", "frozen": 0, "cooldowns": {},
                   "pools": {name: spec.get("max", 100)
                             for name, spec in
                             (ctx.registry.rule("combat.pools", {}) or {}).items()}},
        "round": 0,
        "seconds": 0,
        "kills": 0,
    }
    _save(ctx, state)
    return state, [f"You square up against {combatant.name} (4m off)."]


def combatant_def(ctx: SystemContext, state: dict[str, Any]):
    """The thing being fought: a registry monster, or a duel adapter over
    an agent's live state."""
    if state.get("duel"):
        from engine.systems import agents as agents_system
        return agents_system.duel_combatant(ctx, state["duel"])
    return ctx.registry.get(state["species"])


def start_encounter(ctx: SystemContext, species_id: str, zone_id: str,
                    location_id: str) -> tuple[dict[str, Any], list[str]]:
    monster = ctx.registry.get(species_id)
    rng = ctx.rng.stream("combat")
    low, high = monster.behavior.pack_size
    count = rng.randint(low, high)
    start_dist = float(monster.behavior.perception_range_m)
    state = {
        "species": species_id,
        "zone": zone_id,
        "location": location_id,
        "monsters": [
            {"tag": f"{monster.name} {chr(65 + i)}" if count > 1 else monster.name,
             "hp": monster.stats.hp, "dist": round(start_dist + i * 1.5, 1),
             "frozen": 0, "attacked_once": False}
            for i in range(count)
        ],
        "player": {"stamina": ctx.registry.rule("combat.stamina_max", 100),
                   "guard": "dodge", "frozen": 0, "cooldowns": {},
                   "pools": {name: spec.get("max", 100)
                             for name, spec in
                             (ctx.registry.rule("combat.pools", {}) or {}).items()}},
        "round": 0,
        "seconds": 0,
        "kills": 0,
    }
    _save(ctx, state)
    plural = f"{count} {monster.name}s" if count > 1 else f"a {monster.name}"
    return state, [f"You engage {plural} ({int(start_dist)}m off)."]


def _save(ctx: SystemContext, state: dict[str, Any]) -> None:
    ctx.store.upsert_entity(ENCOUNTER_ID, "encounter", "encounter", state,
                            state.get("location"), 0)


def _clear(ctx: SystemContext) -> None:
    ctx.store.upsert_entity(ENCOUNTER_ID, "encounter", "encounter", {}, None, 0)


# --------------------------------------------------------------------- round


def resolve_round(ctx: SystemContext, state: dict[str, Any], intent: str,
                  argument: str | None) -> RoundResult:
    """One combat round: the player's declared action plus every monster's."""
    result = RoundResult()
    rng = ctx.rng.stream("combat")
    monster_def = combatant_def(ctx, state)
    player = ctx.store.get_player()
    state["round"] += 1
    state["seconds"] += 1
    pstate = state["player"]

    mods = _modifier_effects(ctx)
    if mods.get("action_lock"):
        result.events.append("Your body will not answer — you are paralyzed.")
    elif intent == "guard":
        stance = argument if argument in ("parry", "dodge", "block") else "dodge"
        pstate["guard"] = stance
        pstate["stamina"] = min(ctx.registry.rule("combat.stamina_max", 100),
                                pstate["stamina"] + 10)
        result.events.append(f"You set yourself to {stance}.")
    elif intent == "flee":
        if _try_flee(ctx, state, monster_def, rng, result):
            return result
    elif intent == "attack":
        _player_attack(ctx, state, monster_def, player, argument, mods, rng, result)

    # cooldowns and freezes thaw; breath and power come back
    pstate["cooldowns"] = {k: v - 1 for k, v in pstate["cooldowns"].items() if v > 1}
    if pstate["frozen"] > 0:
        pstate["frozen"] -= 1
    pstate["stamina"] = min(ctx.registry.rule("combat.stamina_max", 100),
                            pstate["stamina"]
                            + ctx.registry.rule("combat.stamina_regen_per_round", 3))
    for name, spec in (ctx.registry.rule("combat.pools", {}) or {}).items():
        pools = pstate.setdefault("pools", {})
        pools[name] = min(spec.get("max", 100),
                          pools.get(name, spec.get("max", 100)) + spec.get("regen", 0))

    # monsters act
    if _living(state):
        _monsters_act(ctx, state, monster_def, player, mods, rng, result)
    player = ctx.store.get_player()

    if player["hp"] <= 0:
        _resolve_death(ctx, state, monster_def, result)
    elif not _living(state):
        _resolve_victory(ctx, state, monster_def, player, rng, result)
    else:
        _save(ctx, state)
        result.events.append(_status_line(state, player))
    return result


# --------------------------------------------------------------- player side


def _player_attack(ctx, state, monster_def, player, argument, mods, rng, result):
    from engine.systems import skills as skills_system

    pstate = state["player"]
    if pstate["frozen"] > 0:
        result.events.append("You are still recovering from your last skill.")
        return

    weapon = _equipped_weapon(ctx)
    weapon_def = ctx.registry.find(weapon["def_id"]) if weapon else None
    # bare `attack` uses the weapon's natural mode: shoot, throw, or swing
    ranged_spec = getattr(weapon_def, "ranged", None) if weapon_def else None
    if ranged_spec is not None and ranged_spec.thrown:
        technique: dict[str, Any] = dict(BASIC_THROW)
    elif ranged_spec is not None:
        technique = dict(BASIC_SHOT)
    else:
        technique = dict(BASIC_STRIKE)
    if argument and argument.lower() == "ram":
        mounted = _mounted_vehicle(ctx)
        if mounted is None:
            result.events.append("You'd need to be riding something to ram.")
            return
        instance, vehicle_def = mounted
        top_speed = max(vehicle_def.speed_kmh.values(), default=10.0)
        # Latent-exploit guard (Test 4 secondary finding): raw
        # top_speed × 0.8 with no cost and no cap means any world that
        # ships a fast vehicle (bike, mech, starship) silently gets a
        # free, ammo-less, durability-free attack scaling linearly with
        # speed. Cap the damage and charge the vehicle's own hp per
        # ram — slamming your ride into a monster should hurt the ride.
        # Both knobs live in rules.yaml so worlds can retune.
        ram_cap = ctx.registry.rule("combat.ram_damage_cap", 40)
        ram_damage = min(ram_cap, round(top_speed * 0.8))
        self_damage_ratio = ctx.registry.rule("combat.ram_self_damage_ratio", 0.25)
        vstate = instance["state"]
        vehicle_hp = vstate.get("hp", (vehicle_def.stats or {}).get("hp", 100))
        self_damage = max(1, round(ram_damage * self_damage_ratio))
        vehicle_hp -= self_damage
        if vehicle_hp <= 0:
            vstate.update({"hp": 0, "mounted": False, "destroyed": True})
            ctx.store.upsert_entity(instance["id"], "vehicle", instance["def_id"],
                                    vstate, instance["location_id"], 0)
            result.events.append(f"The {vehicle_def.name} crumples from the impact "
                                 f"— you are thrown clear!")
            return
        vstate["hp"] = vehicle_hp
        ctx.store.upsert_entity(instance["id"], "vehicle", instance["def_id"],
                                vstate, instance["location_id"], 0)
        result.events.append(f"The {vehicle_def.name} groans from the impact "
                             f"({self_damage} damage).")
        technique = {"name": "ram", "delivery": "melee", "hits": 1,
                     "damage_multiplier": 1.0, "base_damage": ram_damage,
                     "activation_ms": 400, "execution_ms": 300, "post_delay_ms": 500,
                     "range_m": 2.5, "cooldown_s": 2, "ammo_per_use": 0}
        argument = None  # resolved; skip technique lookup
    if argument:
        known = skills_system.known_techniques(ctx.store, ctx.registry)
        chosen = known.get(argument.lower())
        if chosen is None:
            result.events.append(f"You don't know a technique called {argument!r}.")
            return
        if chosen.name.lower() in pstate["cooldowns"]:
            result.events.append(f"{chosen.name} is still on cooldown.")
            return
        technique = chosen.model_dump()

    target = min(_living(state), key=lambda m: m["dist"])
    delivery = technique.get("delivery", "melee")
    base_damage = technique.get("base_damage")
    ability = base_damage is not None  # spells/grenades: no weapon, no ammo

    # resource pool cost (mana, light, energy — defined per world in rules)
    resource = technique.get("resource")
    if resource:
        pools = pstate.setdefault("pools", {})
        cost = technique.get("resource_cost", 0.0)
        if pools.get(resource, 0) < cost:
            result.events.append(f"Not enough {resource} for {technique['name']}.")
            return
        pools[resource] = round(pools[resource] - cost, 1)

    if delivery == "melee":
        reach = technique.get("range_m", 1.5)
        if target["dist"] > reach:
            step = ctx.registry.rule("combat.player_speed_mps", 5.0)
            mounted = _mounted_vehicle(ctx)
            if mounted is not None:  # ride the throttle, not your legs
                step = max(step, max(mounted[1].speed_kmh.values(), default=0) / 3.6)
            target["dist"] = max(reach, round(target["dist"] - step, 1))
            result.events.append(f"You close in on {target['tag']} ({target['dist']}m).")
            return
    elif ability:
        max_range = technique.get("max_range_m") or 20.0
        if target["dist"] > max_range:
            result.events.append(f"{target['tag']} is out of range ({target['dist']}m).")
            return
    else:  # weapon-delivered thrown / projectile / beam
        ranged = getattr(weapon_def, "ranged", None) if weapon_def else None
        if ranged is None:
            result.events.append("Your weapon can't attack at range.")
            return
        max_range = technique.get("max_range_m") or ranged.max_range_m
        if target["dist"] > max_range:
            result.events.append(f"{target['tag']} is out of range ({target['dist']}m).")
            return
        if delivery == "projectile":
            ammo_needed = technique.get("ammo_per_use", 1)
            if ranged.ammo is None or not ctx.store.consume_item(
                    "player", ranged.ammo, ammo_needed):
                result.events.append("You are out of ammunition.")
                return
        elif delivery == "thrown":
            if not ctx.store.consume_item("player", weapon["def_id"], 1):
                result.events.append("Nothing left to throw.")
                return
            state.setdefault("thrown", []).append(weapon["def_id"])

    # who gets hit: area covers everything near the impact point (1D window)
    if delivery == "area":
        blast = technique.get("range_m", 3.0)
        targets = [m for m in _living(state) if abs(m["dist"] - target["dist"]) <= blast]
    else:
        targets = [target]

    # timing: beams are dodged on the tell or not at all; the rest race reaction
    total_ms = technique.get("activation_ms", 300) + technique.get("execution_ms", 300)
    dodge_chance = 0.05 if delivery == "beam" else (0.0 if delivery == "area" else 0.15)

    attack_stat = base_damage if ability else (
        (getattr(weapon_def, "stats", {}) or {}).get("attack", 1) if weapon_def else 1)
    attack_stat += mods.get("attack_add", 0)
    skill_id = (technique.get("parent_skill") if (ability or argument)
                else _weapon_skill(weapon_def))
    proficiency = (ctx.store.get_player_skill(skill_id) or 0.0) if skill_id else 0.0
    proficiency_factor = 0.8 + (proficiency / 1000.0) * 0.4

    landed_any = False
    for struck in targets:
        if (monster_def.stats.reaction_ms < total_ms and rng.random() < dodge_chance):
            result.events.append(f"{struck['tag']} slips aside from your {technique['name']}.")
            continue
        damage = attack_stat * technique.get("damage_multiplier", 1.0) * proficiency_factor
        damage *= mods.get("attack_mult", 1.0)
        damage *= 0.9 + 0.2 * rng.random()
        damage = max(1, round(damage * technique.get("hits", 1) - monster_def.stats.defense))
        struck["hp"] -= damage
        landed_any = True
        result.events.append(
            f"Your {technique['name']} hits {struck['tag']} for {damage} "
            f"({_hp_words(struck['hp'], monster_def.stats.hp)}).")
        if struck["hp"] <= 0:
            state["kills"] += 1
            result.events.append(f"{struck['tag']} shatters into fragments.")

    if landed_any and skill_id and ctx.store.get_player_skill(skill_id) is not None:
        skills_system.gain_proficiency(ctx.store, ctx.registry, skill_id,
                                       monster_def.level, ctx.store.get_player()["level"])

    if weapon is not None and not ability and delivery != "thrown":
        if ctx.store.adjust_durability(weapon["id"], -1) is None:
            result.events.append(f"Your {getattr(weapon_def, 'name', 'weapon')} shatters!")

    freeze = int(technique.get("post_delay_ms", 0)
                 * ctx.registry.rule("combat.post_skill_delay_multiplier", 1.0)) // 1000
    pstate["frozen"] += freeze
    cooldown = int(technique.get("cooldown_s", 0))
    if cooldown and technique["name"] != "strike":
        pstate["cooldowns"][technique["name"].lower()] = cooldown


def _try_flee(ctx, state, monster_def, rng, result) -> bool:
    speed = ctx.registry.rule("combat.player_speed_mps", 5.0)
    chance = max(0.05, min(0.95, 0.4 + (speed - monster_def.stats.speed) / 20.0))
    if rng.random() < chance:
        _clear(ctx)
        result.done, result.outcome = True, "fled"
        result.deltas.append(Delta(kind="player_history", payload={
            "kind": "combat", "summary": f"Fled from {monster_def.name}.",
            "refs": [state["species"]]}))
        result.events.append("You break away and run.")
        return True
    result.events.append("You fail to disengage!")
    return False


# --------------------------------------------------------------- monster side


def _player_reaction_ms(ctx) -> int:
    """The player's effective reaction time in ms — lower is faster/better.

    There's no authored player stat block (the player is pure runtime
    state in the save, not content in schemas.py), so this used to be a
    flat `combat.base_reaction_ms` constant applied identically to every
    player regardless of level, gear, or proficiency (Test 4 root cause).
    This builds a real, build-sensitive value instead: the base constant,
    improved by acrobatics proficiency (if the world defines that skill —
    it already existed with no combat wiring, per the playtest report)
    and by character level, floored so it never reaches 0.
    """
    base = float(ctx.registry.rule("combat.base_reaction_ms", 800))
    acrobatics = ctx.store.get_player_skill("skill.acrobatics")
    if acrobatics:
        base -= acrobatics * ctx.registry.rule(
            "combat.acrobatics_reaction_ms_per_proficiency", 0.3)
    level = ctx.store.get_player()["level"]
    base -= (level - 1) * ctx.registry.rule("combat.reaction_ms_per_level", 2)
    floor = ctx.registry.rule("combat.min_reaction_ms", 250)
    return max(floor, round(base))


def _monster_attack_windup_ms(monster_def) -> int:
    """How telegraphed this monster's own attack is, in ms — higher means
    more time for the player to react to it. Authored via
    `attack_windup_ms` in the monster's stats block; falls back to a
    speed-derived default (a faster monster telegraphs less) for monsters
    that don't specify one, so this no longer silently reuses the
    monster's defensive `reaction_ms` stat to also mean its own attack
    speed (Test 4 finding).
    """
    authored = monster_def.stats.attack_windup_ms
    if authored is not None:
        return authored
    return max(400, round(1200 - monster_def.stats.speed * 40))


def _monsters_act(ctx, state, monster_def, player, mods, rng, result):
    pstate = state["player"]
    armor = _armor_defense(ctx) + mods.get("defense_add", 0)
    base_reaction = _player_reaction_ms(ctx)
    total_ms = _monster_attack_windup_ms(monster_def)
    script = monster_def.behavior.ai_script
    attack_range = 6.0 if "spit" in script else 1.5
    living = _living(state)

    for monster in living:
        if monster["frozen"] > 0:
            monster["frozen"] -= 1
            continue
        if monster["dist"] > attack_range:
            monster["dist"] = max(attack_range,
                                  round(monster["dist"] - monster_def.stats.speed, 1))
            continue

        damage = float(monster_def.stats.attack)
        if "charge" in script and not monster["attacked_once"]:
            damage *= 1.5
        if "flank" in script and len(living) >= 2:
            damage *= 1.15
        if "boss" in script and monster["hp"] < monster_def.stats.hp * 0.25:
            damage *= 1.3
        monster["attacked_once"] = True

        # defender reaction: only when ready and fast enough
        if pstate["frozen"] == 0 and base_reaction < total_ms:
            if _defend(ctx, state, monster, damage, rng, result):
                continue
        damage *= 0.9 + 0.2 * rng.random()
        dealt = max(1, round(damage - armor))
        dealt = _vehicle_absorb(ctx, dealt, result)
        if dealt <= 0:
            continue
        new_hp = max(0, player["hp"] - dealt)
        ctx.store.update_player(hp=new_hp)
        player = ctx.store.get_player()
        result.events.append(f"{monster['tag']} hits you for {dealt}.")
        _damage_armor(ctx, result)
        if new_hp <= 0:
            return


def _defend(ctx, state, monster, incoming, rng, result) -> bool:
    """Returns True if the attack was fully negated."""
    pstate = state["player"]
    stance = pstate["guard"]
    costs = {"parry": 10, "dodge": 8, "block": 15}
    cost = ctx.registry.rule(f"combat.{stance}_stamina", costs[stance])
    if pstate["stamina"] < cost:
        return False
    pstate["stamina"] -= cost
    weapon_def = None
    weapon = _equipped_weapon(ctx)
    if weapon:
        weapon_def = ctx.registry.find(weapon["def_id"])
    skill_id = _weapon_skill(weapon_def)
    proficiency = (ctx.store.get_player_skill(skill_id) or 0.0) if skill_id else 0.0

    if stance == "parry":
        if rng.random() < 0.35 + proficiency / 2000.0:
            monster["frozen"] = 1  # the switch mechanic's opening
            result.events.append(f"You parry {monster['tag']} — it staggers open!")
            return True
    elif stance == "dodge":
        if rng.random() < 0.35:
            result.events.append(f"You dodge {monster['tag']}'s attack.")
            return True
    elif stance == "block":
        if rng.random() < 0.55:
            player = ctx.store.get_player()
            dealt = max(1, round(incoming * 0.5 - _armor_defense(ctx)))
            ctx.store.update_player(hp=max(0, player["hp"] - dealt))
            result.events.append(f"You block; {dealt} bleeds through.")
            _damage_armor(ctx, result)
            return True
    return False


# --------------------------------------------------------------- resolution


def _resolve_victory(ctx, state, monster_def, player, rng, result):
    from engine.systems import skills as skills_system

    if state.get("duel"):
        return _resolve_duel_victory(ctx, state, monster_def, player, rng, result)

    kills = state["kills"]
    xp_each = monster_def.xp
    if monster_def.level <= player["level"] - skills_system.LOW_LEVEL_GAP:
        xp_each //= 4
    result.deltas += skills_system.award_xp(ctx.store, ctx.registry, xp_each * kills)

    for monster in state["monsters"]:
        for drop in monster_def.drops:
            if rng.random() >= drop.chance:
                continue
            qty = drop.qty if isinstance(drop.qty, int) else rng.randint(*drop.qty)
            instance_id = f"iteminst.drop_{state['species'].split('.')[1]}_{rng.randrange(1 << 30):08x}"
            item_def = ctx.registry.find(drop.item)
            durability = (getattr(item_def, "stats", {}) or {}).get("durability_max")
            ctx.store.add_item_instance(instance_id, drop.item, "player",
                                        durability=durability, qty=qty)
            result.events.append(f"Looted {qty}x {getattr(item_def, 'name', drop.item)}.")

    recovery = ctx.registry.rule("combat.ranged.thrown_recovery_chance", 0.85)
    for def_id in state.get("thrown", []):
        if rng.random() < recovery:
            item_def = ctx.registry.find(def_id)
            durability = (getattr(item_def, "stats", {}) or {}).get("durability_max")
            ctx.store.add_item_instance(
                f"iteminst.recovered_{rng.randrange(1 << 30):08x}", def_id, "player",
                durability=durability)
            result.events.append(f"You recover your {getattr(item_def, 'name', def_id)}.")

    _reduce_population(ctx, state, kills)
    result.deltas.append(Delta(kind="player_history", payload={
        "kind": "combat",
        "summary": f"Slew {kills}x {monster_def.name}.",
        "refs": [state["species"]]}))
    if _is_boss(ctx, state["species"]):
        result.deltas.append(Delta(kind="chronicle", payload={
            "category": "boss_defeat",
            "headline": f"{monster_def.name} has fallen.",
            "actors": ["player"]}))
    _clear(ctx)
    result.done, result.outcome = True, "victory"
    result.events.append(f"The field falls quiet. (+{xp_each * kills} XP)")


def _resolve_duel_victory(ctx, state, monster_def, player, rng, result):
    """The agent dies for real — foreground fidelity is the ONE place a
    survivor-flagged named character can be killed (spec §22.4 + survivor
    rule: the named die on-screen or not at all). Corpse looting: their
    col and entire inventory transfer; reputation and faction standing
    take the hit."""
    from engine.systems import skills as skills_system

    npc_id = state["duel"]
    npc = ctx.registry.find(npc_id)
    row = ctx.store.get_entity(npc_id)
    agent_state = dict(row["state"]) if row else {}
    looted_col = agent_state.get("col", 0)
    agent_state.update({"alive": False, "hp": 0, "col": 0, "activity": "dead"})
    ctx.store.upsert_entity(npc_id, "npc", npc_id, agent_state,
                            state.get("location"), 0)

    if looted_col:
        ctx.store.update_player(col=player["col"] + looted_col)
        result.events.append(
            f"You take {looted_col} {ctx.registry.manifest.currency.name} "
            f"from the body.")
    for item in list(ctx.store.get_inventory(npc_id)):
        ctx.store.consume_item(npc_id, item["def_id"], item["qty"])
        item_def = ctx.registry.find(item["def_id"])
        durability = (getattr(item_def, "stats", {}) or {}).get("durability_max")
        ctx.store.add_item_instance(
            f"iteminst.loot_{rng.randrange(1 << 30):08x}", item["def_id"],
            "player", durability=durability, qty=item["qty"])
        result.events.append(
            f"Looted {item['qty']}x {getattr(item_def, 'name', item['def_id'])}.")

    result.deltas += skills_system.award_xp(ctx.store, ctx.registry, monster_def.xp)
    result.deltas.append(Delta(kind="reputation", payload={
        "scope_id": npc_id, "delta": -1.0}))
    faction = ctx.registry.find(npc.faction) if getattr(npc, "faction", None) else None
    if faction is not None:
        penalty = ctx.registry.rule("agents.player_kill_faction_rep", 0.5)
        result.deltas.append(Delta(kind="reputation", payload={
            "scope_id": faction.id, "delta": -penalty}))
        result.events.append(f"{faction.name} will hear about this.")
    of = f" of {faction.name}" if faction is not None else ""
    result.deltas.append(Delta(kind="chronicle", payload={
        "category": "street", "visibility": "public",
        "headline": f"{monster_def.name}{of} was cut down by "
                    f"{player['name']} in open combat.",
        "detail": "duel", "actors": ["player", npc_id]},
        location_id=state.get("location")))
    result.deltas.append(Delta(kind="player_history", payload={
        "kind": "combat", "summary": f"Killed {monster_def.name} in a duel.",
        "refs": [npc_id]}))
    _clear(ctx)
    result.done, result.outcome = True, "victory"
    result.events.append(f"{monster_def.name} falls. (+{monster_def.xp} XP)")


def _resolve_death(ctx, state, monster_def, result):
    """HP 0. Under permadeath the character is deleted from the world's story —
    this must actually work (§8). Worlds where death is a setback, not an
    ending (respawn shrines, resurrection), set death.permadeath: false and a
    death.respawn_location; the world's rules decide, never the engine."""
    if ctx.registry.rule("death.permadeath", True):
        ctx.store.update_player(hp=0, alive=0)
        player = ctx.store.get_player()
        result.deltas.append(Delta(kind="chronicle", payload={
            "category": "death",
            "headline": (f"{player['name']} was slain by {monster_def.name}."
                         if state.get("duel") else
                         f"{player['name']} was slain by a {monster_def.name}."),
            "actors": ["player"]}, location_id=state.get("location")))
        result.deltas.append(Delta(kind="player_history", payload={
            "kind": "milestone", "summary": "Died. The world goes on."}))
        result.events.append("Your HP hits zero. Everything goes white.")
    else:
        respawn = ctx.registry.rule("death.respawn_location",
                                    ctx.registry.manifest.entry_point.location)
        player = ctx.store.get_player()
        col_loss = int(player["col"] * ctx.registry.rule("death.col_loss_ratio", 0.0))
        ctx.store.update_player(hp=player["hp_max"], location_id=respawn,
                                col=player["col"] - col_loss)
        place = ctx.registry.find(respawn)
        place_name = getattr(place, "name", respawn) if place else respawn
        result.deltas.append(Delta(kind="player_history", payload={
            "kind": "milestone",
            "summary": f"Died to a {monster_def.name} and returned at {place_name}."}))
        result.events.append(f"Death takes you — briefly. You wake at {place_name}."
                             + (f" ({col_loss} {ctx.registry.manifest.currency.name} lost.)"
                                if col_loss else ""))
    _clear(ctx)
    result.done, result.outcome = True, "death"


# --------------------------------------------------------------- helpers


def _living(state) -> list[dict[str, Any]]:
    return [m for m in state["monsters"] if m["hp"] > 0]


def _mounted_vehicle(ctx) -> tuple[dict[str, Any], Any] | None:
    """(instance record, definition) of the vehicle the player is riding."""
    for row in ctx.store.conn.execute(
            "SELECT * FROM entities WHERE kind='vehicle' ORDER BY id"):
        record = dict(row)
        record["state"] = json.loads(record.pop("state_json"))
        if record["state"].get("owner") == "player" and record["state"].get("mounted"):
            definition = ctx.registry.find(record["def_id"])
            if definition is not None:
                return record, definition
    return None


def _vehicle_absorb(ctx, damage: int, result) -> int:
    """A mounted vehicle takes the hit first. Returns damage reaching the rider.
    Destruction dumps the rider off — and the wreck stays destroyed."""
    mounted = _mounted_vehicle(ctx)
    if mounted is None:
        return damage
    instance, definition = mounted
    state = instance["state"]
    hp = state.get("hp", (definition.stats or {}).get("hp", 100))
    armor = (definition.stats or {}).get("armor", 0)
    absorbed = max(1, damage - armor)
    hp -= absorbed
    if hp <= 0:
        state.update({"hp": 0, "mounted": False, "destroyed": True})
        ctx.store.upsert_entity(instance["id"], "vehicle", instance["def_id"],
                                state, instance["location_id"], 0)
        result.events.append(f"The {definition.name} is wrecked — you are thrown clear!")
        return max(0, -hp)  # overkill spills onto the rider
    state["hp"] = hp
    ctx.store.upsert_entity(instance["id"], "vehicle", instance["def_id"],
                            state, instance["location_id"], 0)
    result.events.append(f"The {definition.name} shudders ({absorbed} damage).")
    return 0


def _equipped_weapon(ctx) -> dict[str, Any] | None:
    for item in ctx.store.get_equipped("player"):
        definition = ctx.registry.find(item["def_id"])
        if definition is not None and definition.category.startswith("weapon"):
            return item
    return None


def _weapon_skill(weapon_def) -> str | None:
    if weapon_def is None or not weapon_def.requirements:
        return None
    return weapon_def.requirements.get("skill")


def _armor_defense(ctx) -> int:
    total = 0
    for item in ctx.store.get_equipped("player"):
        definition = ctx.registry.find(item["def_id"])
        if definition is not None and definition.category.startswith("armor"):
            total += (definition.stats or {}).get("defense", 0)
    return total


def _damage_armor(ctx, result) -> None:
    for item in ctx.store.get_equipped("player"):
        definition = ctx.registry.find(item["def_id"])
        if definition is not None and definition.category.startswith("armor"):
            if ctx.store.adjust_durability(item["id"], -1) is None:
                result.events.append(f"Your {definition.name} falls apart!")
            break


def _modifier_effects(ctx) -> dict[str, Any]:
    """Aggregate active player modifier effects relevant to combat."""
    totals: dict[str, Any] = {"attack_add": 0, "defense_add": 0, "attack_mult": 1.0,
                              "action_lock": False}
    for record in ctx.store.get_modifiers("player"):
        definition = ctx.registry.find(record["def_id"])
        if definition is None:
            continue
        for effect in list(definition.effects) + list(definition.side_effects):
            if effect.type == "stat_add" and effect.target == "attack":
                totals["attack_add"] += effect.value
            elif effect.type == "stat_add" and effect.target == "defense":
                totals["defense_add"] += effect.value
            elif effect.type == "stat_mult" and effect.target == "attack":
                totals["attack_mult"] *= effect.value
            elif effect.type == "action_lock" and effect.value:
                totals["action_lock"] = True
    return totals


def _reduce_population(ctx, state, kills: int) -> None:
    if state.get("duel") or not state.get("zone"):
        return
    zone_id = state.get("zone")
    if not zone_id:
        return
    runtime = ctx.store.get_entity(zone_id)
    if runtime is not None:
        populations = runtime["state"].get("populations", {})
        def_id, location = runtime["def_id"], runtime["location_id"]
    else:
        # first kill before ecology's first day-tick: seed from authored data
        populations, def_id, location = {}, zone_id, None
        for floor in ctx.registry.by_kind("floor"):
            for zone in floor.zones:
                if zone.id == zone_id:
                    populations = {p.species: p.current for p in zone.monster_populations}
                    def_id, location = floor.id, floor.id
    species = state["species"]
    if species in populations:
        populations[species] = max(0, populations[species] - kills)
        ctx.store.upsert_entity(zone_id, "zone", def_id,
                                {"populations": populations}, location, 0)


def _is_boss(ctx, species_id: str) -> bool:
    for floor in ctx.registry.by_kind("floor"):
        if floor.labyrinth is not None and floor.labyrinth.boss == species_id:
            return True
    return False


def _hp_words(hp: int, hp_max: int) -> str:
    """Preserve uncertainty: the player reads a cursor, not a number (§15)."""
    ratio = hp / hp_max if hp_max else 0
    if hp <= 0:
        return "destroyed"
    if ratio > 0.7:
        return "barely scratched"
    if ratio > 0.4:
        return "wounded"
    if ratio > 0.15:
        return "badly wounded"
    return "nearly broken"


def _status_line(state, player) -> str:
    monsters = ", ".join(f"{m['tag']} {m['dist']}m" for m in _living(state))
    pstate = state["player"]
    frozen = " [FROZEN]" if pstate["frozen"] > 0 else ""
    pools = "".join(f" | {name} {value:g}"
                    for name, value in sorted(pstate.get("pools", {}).items()))
    return (f"HP {player['hp']}/{player['hp_max']} | stamina {pstate['stamina']}{pools} | "
            f"guard {pstate['guard']}{frozen} || {monsters}")


def minutes_elapsed(state_seconds: int) -> int:
    return max(1, math.ceil(state_seconds / 60))
