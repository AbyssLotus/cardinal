"""Agent NPCs (spec §22) — player-type actors. The world moves without a player.

The load-bearing rule from the spec: **no shadow systems.** Agent kills
decrement the same zone populations the player's hunts do; agent trades
move the same market supply/demand indices through the same
`economy.trade_cost` / `record_trade` math; agent deaths are permadeath
with a chronicle entry; agent faction members pay real dues into the
same treasuries and their deaths move the same inter-faction
dispositions the strategic tick reads. Downstream systems cannot tell
an agent-caused outcome from a player-caused one.

Execution model (spec §22.2, background fidelity):
- Agents act on the hour tick, in sorted-id order, staggered by an id
  hash so the district doesn't move in lockstep. A per-tick action
  budget (`agents.max_actions_per_tick`) bounds cost on large worlds.
- Combat resolves through an expected-value contest (level + a seeded,
  persisted RNG stream), not the full state machine — foreground
  fidelity (full combat when the player is watching) is the M8 follow-up.
- Everything is reproducible: all randomness comes from
  ctx.rng.stream("agents"), which persists in the save like every other
  stream. Same seed, same year, same deaths.

Policy archetypes (worlds assign + parameterize via `policy_params`):
- hunter_gatherer — hunts a zone's real population, hauls loot to a
  market, sells it (moving indices), pays faction dues on revenue.
- merchant — watches prices across a route and arbitrages the spread at
  a wholesale ratio, creating the cross-market drift of §20.2
  organically. Pays faction dues on sale revenue.
- aggressor — patrols turf; fights co-located agents of hostile
  factions. Deaths transfer col + inventory to the victor (robbery is
  real), chronicle publicly, and worsen the two factions' dispositions
  (member actions move standing, §24.1).
- passive — holds state; acts only when acted upon.

Tunables (rules.yaml `agents:`): act_every_hours (3),
max_actions_per_tick (32), hostility_threshold (-0.3),
engage_chance (0.35), kill_disposition_hit (0.1),
sell_threshold (5), merchant_lot (10).
"""

from __future__ import annotations

from typing import Any

from engine.persistence.store import Delta
from engine.systems import SystemContext
from engine.systems import economy as economy_system
from engine.systems import factions as factions_system

WHOLESALE_RATIO = 0.85   # agents trade as merchants, both directions


def _stable_hash(text: str) -> int:
    value = 0
    for char in text:
        value = (value * 31 + ord(char)) % 997
    return value


def _agents(ctx: SystemContext):
    return sorted((n for n in ctx.registry.by_kind("npc")
                   if getattr(n, "actor_class", "ambient") == "agent"),
                  key=lambda n: n.id)


def agent_state(ctx: SystemContext, npc) -> dict[str, Any]:
    runtime = ctx.store.get_entity(npc.id)
    if runtime is not None:
        return dict(runtime["state"])
    hp_max = 60 + npc.combat.level * 12
    state = {"alive": True, "hp": hp_max, "hp_max": hp_max,
             "level": npc.combat.level, "col": npc.col, "activity": "idle"}
    for item_id in npc.loadout:
        ctx.store.add_item_instance(
            f"iteminst.{npc.id.split('.')[1]}_{item_id.split('.')[1]}",
            item_id, npc.id, qty=1)
    return state


def _save(ctx: SystemContext, npc, state: dict[str, Any], location: str,
          day: int) -> None:
    ctx.store.upsert_entity(npc.id, "npc", npc.id, state, location, day)


def _location(ctx: SystemContext, npc) -> str:
    runtime = ctx.store.get_entity(npc.id)
    return (runtime["location_id"] if runtime else None) or npc.location


# ------------------------------------------------------------------- tick


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    if granularity != "hour":
        return []
    roster = _agents(ctx)
    if not roster:
        return []
    rule = ctx.registry.rule
    cadence = max(1, rule("agents.act_every_hours", 3))
    budget = rule("agents.max_actions_per_tick", 32)
    rng = ctx.rng.stream("agents")
    deltas: list[Delta] = []

    agent_defs = {npc.id: npc for npc in roster}
    # Fair scheduling: rotate the roster's starting index each tick so the
    # action budget doesn't permanently starve whoever sorts last. The
    # rotation STRIDES BY THE BUDGET (not by 1): advancing one position per
    # tick while consuming `budget` positions aliases against the cadence
    # hash and systematically starves ~12% of large rosters (measured at
    # both 106 and 1006 agents). Striding by the budget marches the window
    # through the whole roster every ceil(len/budget) ticks.
    start = ((day * 24 + hour) * budget) % len(roster)
    rotated = roster[start:] + roster[:start]

    for npc in rotated:
        if budget <= 0:
            break
        if (hour + _stable_hash(npc.id)) % cadence != 0:
            continue
        state = agent_state(ctx, npc)
        if not state.get("alive", True):
            continue
        if state.get("recovering_until_day", -1) >= day:
            continue                       # licking wounds; sits the day out
        budget -= 1
        location = _location(ctx, npc)
        policy = npc.policy or "passive"
        if policy == "quest_taker":
            deltas += _act_quest_taker(ctx, npc, state, location, day, rng)
        elif policy == "hunter_gatherer":
            deltas += _act_hunter(ctx, npc, state, location, day, rng)
        elif policy == "merchant":
            deltas += _act_merchant(ctx, npc, state, location, day, rng)
        elif policy == "aggressor":
            deltas += _act_aggressor(ctx, npc, state, location, day, hour, rng,
                                     agent_defs)
        # passive: holds state
        # light recovery between actions — agents heal like the player does
        if state.get("alive", True) and state["hp"] < state["hp_max"]:
            state["hp"] = min(state["hp_max"], state["hp"] + 4)
            _save(ctx, npc, state, _location(ctx, npc), day)
    return deltas


# ---------------------------------------------------------------- quest_taker


def _requirement_items(quest):
    """(item_id, qty) pairs for obtain/deliver requirements; None if the
    quest has requirement types an agent can't execute yet."""
    pairs = []
    for requirement in quest.requirements:
        spec = requirement.get("obtain") or requirement.get("deliver")
        if not spec:
            return None
        pairs.append((spec["item"], spec.get("qty", 1)))
    return pairs


def _act_quest_taker(ctx, npc, state, location, day, rng) -> list[Delta]:
    """Claim -> acquire -> deliver, all through the real systems: items are
    bought from real markets (moving indices), delivery happens at the
    source NPC's actual location, rewards pay real col. Claiming and
    fulfilment are chronicle-visible moments (spec §23)."""
    instances = {q["def_id"]: q for q in ctx.store.get_quests()}
    mine = None
    for quest in sorted(ctx.registry.by_kind("quest"), key=lambda q: q.id):
        instance = instances.get(quest.id)
        if instance is not None and instance["state"] == "available" \
                and instance.get("assignee") == npc.id:
            mine = (quest, instance)
            break

    if mine is None:
        return _claim_best_open_quest(ctx, npc, state, instances, day)

    quest, instance = mine
    pairs = _requirement_items(quest)
    if pairs is None:                      # shouldn't happen post-claim; release
        return []

    missing = []
    for item_id, qty in pairs:
        held = sum(i["qty"] for i in ctx.store.get_inventory(npc.id)
                   if i["def_id"] == item_id)
        if held < qty:
            missing.append((item_id, qty - held))

    if not missing:
        # deliver at the source's real location
        source_loc = ctx.registry.resolve_location(
            _source_location(ctx, quest.source) or "") or \
            _source_location(ctx, quest.source)
        if source_loc and location != source_loc:
            state["activity"] = f"delivering ({quest.name})"
            _save(ctx, npc, state, source_loc, day)
            return []
        for item_id, qty in pairs:
            ctx.store.consume_item(npc.id, item_id, qty)
        reward_col = sum(r.get("col", 0) for r in quest.rewards)
        state["col"] += reward_col
        state["activity"] = "collecting a bounty"
        _pay_dues(ctx, npc, state, reward_col)
        _save(ctx, npc, state, location, day)
        source = ctx.registry.find(quest.source)
        return [
            Delta(kind="quest_state", payload={
                "instance_id": instance["instance_id"], "def_id": quest.id,
                "state": "completed", "available_day": instance["available_day"],
                "expires_day": instance["expires_day"], "assignee": npc.id}),
            Delta(kind="chronicle", payload={
                "category": "discovery", "visibility": "public",
                "headline": f"{npc.name} fulfilled "
                            f"{getattr(source, 'name', quest.source)}'s contract: "
                            f"{quest.name}.",
                "detail": f"reward {reward_col}",
                "actors": [npc.id, quest.source]}),
        ]

    # acquire the first missing item from a real market
    item_id, need = missing[0]
    good = ctx.registry.find(item_id)
    if good is None or getattr(good, "market", None) is None:
        state["activity"] = f"hunting down {item_id} (unbuyable)"
        _save(ctx, npc, state, location, day)
        return []
    market = _market_at(ctx, location)
    if market is None:
        target = _nearest_market_settlement(ctx)
        if target is None:
            return []
        state["activity"] = "heading to market"
        _save(ctx, npc, state, target, day)
        return []
    _, cost = economy_system.trade_cost(ctx, market.id, good, need,
                                        player_buys=True, ratio=WHOLESALE_RATIO)
    if state["col"] < cost:
        state["activity"] = "short on funds for the contract"
        _save(ctx, npc, state, location, day)
        return []
    state["col"] -= cost
    ctx.store.add_item_instance(
        f"iteminst.{npc.id.split('.')[1]}_{rng.randrange(1 << 30):08x}",
        item_id, npc.id, qty=need)
    economy_system.record_trade(ctx, market.id, good, need, player_buys=True)
    state["activity"] = f"gathering supplies ({quest.name})"
    _save(ctx, npc, state, location, day)
    return []


def _claim_best_open_quest(ctx, npc, state, instances, day) -> list[Delta]:
    """Score open contracts by expected margin; claim the best profitable
    one. Claiming is a chronicle-visible moment."""
    best, best_margin = None, 0
    for quest in sorted(ctx.registry.by_kind("quest"), key=lambda q: q.id):
        instance = instances.get(quest.id)
        if instance is None or instance["state"] != "available" \
                or instance.get("assignee"):
            continue
        pairs = _requirement_items(quest)
        if pairs is None:
            continue
        cost = 0
        feasible = True
        for item_id, qty in pairs:
            good = ctx.registry.find(item_id)
            if good is None or getattr(good, "market", None) is None:
                feasible = False
                break
            cost += good.market.base_value_col * qty
        if not feasible:
            continue
        reward = sum(r.get("col", 0) for r in quest.rewards)
        margin = reward - cost * 1.2       # wholesale + travel slack
        if margin > best_margin:
            best, best_margin = (quest, instance), margin
    if best is None:
        state["activity"] = "scanning the boards"
        _save(ctx, npc, state, ctx.store.get_entity(npc.id)["location_id"]
              if ctx.store.get_entity(npc.id) else npc.location, day)
        return []
    quest, instance = best
    state["activity"] = f"took a contract ({quest.name})"
    _save(ctx, npc, state,
          (ctx.store.get_entity(npc.id) or {"location_id": npc.location})["location_id"]
          or npc.location, day)
    source = ctx.registry.find(quest.source)
    return [
        Delta(kind="quest_state", payload={
            "instance_id": instance["instance_id"], "def_id": quest.id,
            "state": "available", "available_day": instance["available_day"],
            "expires_day": instance["expires_day"], "assignee": npc.id}),
        Delta(kind="chronicle", payload={
            "category": "discovery", "visibility": "public",
            "headline": f"{npc.name} took the contract: {quest.name}.",
            "detail": quest.purpose, "actors": [npc.id, quest.source]}),
    ]


def _source_location(ctx, npc_id: str):
    runtime = ctx.store.get_entity(npc_id)
    if runtime is not None and runtime["location_id"]:
        return runtime["location_id"]
    definition = ctx.registry.find(npc_id)
    return getattr(definition, "location", None) if definition else None


def _nearest_market_settlement(ctx):
    for location in sorted(ctx.registry.by_kind("loc"), key=lambda l: l.id):
        if getattr(location, "market", None):
            return location.id
    return None


# ------------------------------------------------------------- hunter_gatherer


def _act_hunter(ctx, npc, state, location, day, rng) -> list[Delta]:
    params = npc.policy_params
    species = params.get("species")
    zone_id = params.get("zone")
    sell_at = params.get("sell_at", npc.location)
    threshold = ctx.registry.rule("agents.sell_threshold", 5)
    monster = ctx.registry.find(species) if species else None
    if monster is None or zone_id is None:
        state["activity"] = "idle (bad hunt config)"
        _save(ctx, npc, state, location, day)
        return []

    carrying = sum(i["qty"] for i in ctx.store.get_inventory(npc.id)
                   if i["def_id"] in {d.item for d in monster.drops})
    if carrying >= threshold:
        if location != sell_at:
            state["activity"] = "hauling loot"
            _save(ctx, npc, state, sell_at, day)   # background travel
            return []
        return _sell_inventory(ctx, npc, state, sell_at, day,
                               {d.item for d in monster.drops})

    home = params.get("home", npc.location)
    if location != home:
        state["activity"] = "heading home"
        _save(ctx, npc, state, home, day)
        return []

    # hunt the real population
    population = _zone_population(ctx, zone_id, species)
    if population < 1:
        state["activity"] = "finding nothing to hunt"
        _save(ctx, npc, state, location, day)
        return []
    win_chance = max(0.1, min(0.95,
                              0.5 + (state["level"] - monster.level) * 0.08))
    state["activity"] = f"hunting {monster.name}"
    if rng.random() < win_chance:
        kills = monster.behavior.pack_size[0]
        _reduce_zone_population(ctx, zone_id, species, kills)
        for _ in range(kills):
            for drop in monster.drops:
                if rng.random() >= drop.chance:
                    continue
                qty = drop.qty if isinstance(drop.qty, int) else rng.randint(*drop.qty)
                ctx.store.add_item_instance(
                    f"iteminst.{npc.id.split('.')[1]}_{rng.randrange(1 << 30):08x}",
                    drop.item, npc.id, qty=qty)
        _save(ctx, npc, state, location, day)
        return []
    # lost the fight — real damage, real permadeath (unless named)
    damage = max(5, monster.stats.attack - 4 + rng.randint(0, 6))
    state["hp"] -= damage
    if state["hp"] <= 0:
        if npc.survivor:
            return _survivor_escapes(ctx, npc, state, location, day,
                                     from_what=f"a {monster.name} pack")
        return _agent_dies(ctx, npc, state, location, day,
                           cause=f"was torn apart by {monster.name}s")
    _save(ctx, npc, state, location, day)
    return []


def _sell_inventory(ctx, npc, state, settlement, day,
                    def_ids: set[str]) -> list[Delta]:
    market = _market_at(ctx, settlement)
    if market is None:
        return []
    earned_total = 0
    for row in list(ctx.store.get_inventory(npc.id)):
        if row["def_id"] not in def_ids or row["qty"] <= 0:
            continue
        good = ctx.registry.find(row["def_id"])
        if good is None:
            continue
        qty = row["qty"]
        _, earned = economy_system.trade_cost(ctx, market.id, good, qty,
                                              player_buys=False,
                                              ratio=WHOLESALE_RATIO)
        ctx.store.consume_item(npc.id, row["def_id"], qty)
        economy_system.record_trade(ctx, market.id, good, qty, player_buys=False)
        earned_total += earned
    earned_total = _pay_market_skim(ctx, npc, market, earned_total)
    state["col"] += earned_total
    state["activity"] = "selling loot"
    _pay_dues(ctx, npc, state, earned_total)
    _save(ctx, npc, state, settlement, day)
    return []


# ----------------------------------------------------------------- merchant


def _act_merchant(ctx, npc, state, location, day, rng) -> list[Delta]:
    params = npc.policy_params
    good = ctx.registry.find(params.get("good", ""))
    route = params.get("route", [])
    lot = ctx.registry.rule("agents.merchant_lot", 10)
    if good is None or len(route) < 2:
        state["activity"] = "idle (bad trade config)"
        _save(ctx, npc, state, location, day)
        return []

    prices = {}
    for stop in route:
        market = _market_at(ctx, stop)
        if market is not None:
            prices[stop] = economy_system.current_price(ctx, market.id, good)
    if len(prices) < 2:
        # a broken route is a visible state, not a silent no-op — otherwise
        # misconfigured agents are indistinguishable from starved ones
        state["activity"] = "idle (route has <2 markets)"
        _save(ctx, npc, state, location, day)
        return []
    cheap = min(prices, key=lambda s: (prices[s], s))
    dear = max(prices, key=lambda s: (prices[s], s))

    holding = sum(i["qty"] for i in ctx.store.get_inventory(npc.id)
                  if i["def_id"] == good.id)
    if holding >= lot:
        # go sell where it's dear
        if location != dear:
            state["activity"] = f"hauling {good.name}"
            _save(ctx, npc, state, dear, day)
            return []
        market = _market_at(ctx, dear)
        _, earned = economy_system.trade_cost(ctx, market.id, good, holding,
                                              player_buys=False,
                                              ratio=WHOLESALE_RATIO)
        ctx.store.consume_item(npc.id, good.id, holding)
        economy_system.record_trade(ctx, market.id, good, holding,
                                    player_buys=False)
        earned = _pay_market_skim(ctx, npc, market, earned)
        state["col"] += earned
        state["activity"] = "selling stock"
        _pay_dues(ctx, npc, state, earned)
        _save(ctx, npc, state, dear, day)
        return []

    # go buy where it's cheap — only if the spread is worth the ratio
    if prices[dear] * WHOLESALE_RATIO <= prices[cheap] * 1.02:
        state["activity"] = "waiting for a spread"
        _save(ctx, npc, state, location, day)
        return []
    if location != cheap:
        state["activity"] = "scouting prices"
        _save(ctx, npc, state, cheap, day)
        return []
    market = _market_at(ctx, cheap)
    _, cost = economy_system.trade_cost(ctx, market.id, good, lot,
                                        player_buys=True, ratio=WHOLESALE_RATIO)
    if state["col"] < cost:
        state["activity"] = "counting eddies"
        _save(ctx, npc, state, location, day)
        return []
    state["col"] -= cost
    ctx.store.add_item_instance(
        f"iteminst.{npc.id.split('.')[1]}_{rng.randrange(1 << 30):08x}",
        good.id, npc.id, qty=lot)
    economy_system.record_trade(ctx, market.id, good, lot, player_buys=True)
    state["activity"] = "buying stock"
    _save(ctx, npc, state, cheap, day)
    return []


# ---------------------------------------------------------------- aggressor


def _act_aggressor(ctx, npc, state, location, day, hour, rng,
                   agent_defs) -> list[Delta]:
    params = npc.policy_params
    turf = params.get("turf", [npc.location])

    # fight a co-located hostile agent (at most one engagement per day)
    if state.get("last_fight_day") != day:
        rival = _hostile_agent_here(ctx, npc, location, agent_defs)
        if rival is not None and rng.random() < ctx.registry.rule(
                "agents.engage_chance", 0.35):
            state["last_fight_day"] = day
            return _agent_fight(ctx, npc, state, rival, location, day, rng)

    # otherwise patrol the turf
    if turf:
        index = (day * 24 + hour) // max(1, ctx.registry.rule(
            "agents.act_every_hours", 3)) % len(turf)
        target = turf[index]
        state["activity"] = "patrolling"
        _save(ctx, npc, state, target, day)
    return []


def _hostile_agent_here(ctx, npc, location, agent_defs):
    """Find a co-located hostile agent using the entities table's location
    index (one SQL lookup) instead of scanning the whole roster — the
    difference between O(here) and O(world) per aggressor action."""
    if npc.faction is None:
        return None
    my_faction = ctx.registry.find(npc.faction)
    if my_faction is None:
        return None
    threshold = ctx.registry.rule("agents.hostility_threshold", -0.3)
    my_state = factions_system.faction_state(ctx, my_faction)
    dispositions = my_state.get("dispositions", {})
    for row in ctx.store.entities_at(location, kind="npc"):
        other = agent_defs.get(row["id"])
        if other is None or other.id == npc.id:
            continue
        if other.faction in (None, npc.faction):
            continue
        if not row["state"].get("alive", True):
            continue
        if dispositions.get(other.faction, 0.0) <= threshold:
            return other
    return None


def _agent_fight(ctx, npc, state, rival, location, day, rng) -> list[Delta]:
    rival_state = agent_state(ctx, rival)
    my_level = state["level"]
    their_level = rival_state["level"]
    win_chance = max(0.15, min(0.85, 0.5 + (my_level - their_level) * 0.07))
    place = ctx.registry.find(location)
    place_name = getattr(place, "name", location)

    if rng.random() < win_chance:
        winner, w_state, loser, l_state = npc, state, rival, rival_state
    else:
        winner, w_state, loser, l_state = rival, rival_state, npc, state

    damage = 25 + rng.randint(0, 20)
    l_state["hp"] -= damage
    deltas: list[Delta] = []
    if l_state["hp"] <= 0:
        # robbery is real: col and inventory transfer to the victor
        w_state["col"] += l_state.get("col", 0)
        l_state["col"] = 0
        for row in list(ctx.store.get_inventory(loser.id)):
            ctx.store.consume_item(loser.id, row["def_id"], row["qty"])
            ctx.store.add_item_instance(
                f"iteminst.{winner.id.split('.')[1]}_{rng.randrange(1 << 30):08x}",
                row["def_id"], winner.id, qty=row["qty"])
        if loser.survivor:
            deltas += _survivor_escapes(ctx, loser, l_state, location, day,
                                        from_what=winner.name)
            deltas += _kill_moves_standing(ctx, winner, loser)  # blood still spilled
            w_state["activity"] = "fighting"
            _save(ctx, npc, state, location, day)
            _save(ctx, rival, rival_state,
                  ctx.store.get_entity(rival.id)["location_id"]
                  if rival is loser else location, day)
            return deltas
        deltas += _agent_dies(ctx, loser, l_state, location, day,
                              cause=f"was killed by {winner.name}",
                              save_state=False)
        deltas += _kill_moves_standing(ctx, winner, loser)
    else:
        w_state["hp"] = max(1, w_state["hp"] - (10 + rng.randint(0, 10)))
        deltas.append(Delta(kind="chronicle", payload={
            "category": "street", "visibility": "public",
            "headline": f"{winner.name} and {loser.name} trade gunfire "
                        f"at {place_name}.",
            "detail": f"{loser.name} withdrew wounded"}))
    w_state["activity"] = "fighting"
    l_state["activity"] = "fighting"
    _save(ctx, npc, state, location, day)
    _save(ctx, rival, rival_state, location, day)
    return deltas


def _kill_moves_standing(ctx, winner, loser) -> list[Delta]:
    """Member actions move faction standing (spec §24.1): a kill worsens
    the two factions' dispositions in both directions."""
    if winner.faction is None or loser.faction is None:
        return []
    hit = ctx.registry.rule("agents.kill_disposition_hit", 0.1)
    for a_id, b_id in ((winner.faction, loser.faction),
                       (loser.faction, winner.faction)):
        faction = ctx.registry.find(a_id)
        if faction is None:
            continue
        f_state = factions_system.faction_state(ctx, faction)
        current = f_state.setdefault("dispositions", {}).get(b_id, 0.0)
        f_state["dispositions"][b_id] = round(max(-1.0, current - hit), 4)
        factions_system.save_faction_state(ctx, faction, f_state)
    return []


def _survivor_escapes(ctx, npc, state, location, day,
                      from_what: str) -> list[Delta]:
    """Named characters don't die off-screen: they escape at 1 hp, wounded,
    and retreat home to recover for agents.survivor_recovery_days."""
    state["alive"] = True
    state["hp"] = 1
    state["activity"] = "recovering"
    state["recovering_until_day"] = day + ctx.registry.rule(
        "agents.survivor_recovery_days", 4)
    home = (npc.policy_params.get("turf") or [npc.location])[0] \
        if npc.policy == "aggressor" else npc.policy_params.get("home", npc.location)
    place = ctx.registry.find(location)
    _save(ctx, npc, state, home, day)
    faction = ctx.registry.find(npc.faction) if npc.faction else None
    of = f" of {faction.name}" if faction is not None else ""
    return [Delta(kind="chronicle", payload={
        "category": "street", "visibility": "public",
        "headline": f"{npc.name}{of} barely escapes {from_what} at "
                    f"{getattr(place, 'name', location)}.",
        "detail": "wounded, gone to ground"})]


def _agent_dies(ctx, npc, state, location, day, cause: str,
                save_state: bool = True) -> list[Delta]:
    """Permadeath (spec §22.3): a dead agent stays dead and the world hears."""
    state["alive"] = False
    state["hp"] = 0
    state["activity"] = "dead"
    _save(ctx, npc, state, location, day)
    place = ctx.registry.find(location)
    faction = ctx.registry.find(npc.faction) if npc.faction else None
    of = f" of {faction.name}" if faction is not None else ""
    return [Delta(kind="chronicle", payload={
        "category": "street", "visibility": "public",
        "headline": f"{npc.name}{of} {cause} at "
                    f"{getattr(place, 'name', location)}.",
        "detail": "permadeath"})]


# ------------------------------------------------------------------ helpers


def _pay_market_skim(ctx, npc, market, revenue: int) -> int:
    """The protection racket is real: the faction controlling a market
    skims agent sale revenue at its tax rate (its own members exempt).
    Returns the revenue after the skim."""
    if revenue <= 0:
        return revenue
    faction, rate = factions_system.market_tax(ctx, market,
                                               trader_faction=npc.faction)
    if faction is None or rate <= 0:
        return revenue
    cut = round(revenue * rate)
    if cut > 0:
        factions_system.credit_treasury(ctx, faction, cut)
    return revenue - cut


def _pay_dues(ctx, npc, state, revenue: int) -> None:
    """Faction members tithe their trade revenue at the faction's tax rate —
    treasuries finally move without a player in the world."""
    if revenue <= 0 or npc.faction is None:
        return
    faction = ctx.registry.find(npc.faction)
    if faction is None:
        return
    rate = float((faction.policies or {}).get("tax_rate", 0.0))
    if rate <= 0:
        return
    cut = round(revenue * rate)
    if cut > 0:
        state["col"] -= cut
        factions_system.credit_treasury(ctx, faction, cut)


def _market_at(ctx, settlement_id: str):
    location = ctx.registry.find(settlement_id)
    market_id = getattr(location, "market", None)
    return ctx.registry.find(market_id) if market_id else None


def _zone_population(ctx, zone_id: str, species: str) -> float:
    runtime = ctx.store.get_entity(zone_id)
    if runtime is not None and "populations" in runtime["state"]:
        return runtime["state"]["populations"].get(species, 0.0)
    for floor in ctx.registry.by_kind("floor"):
        for zone in floor.zones:
            if zone.id == zone_id:
                for pop in zone.monster_populations:
                    if pop.species == species:
                        return pop.current
    return 0.0


def _reduce_zone_population(ctx, zone_id: str, species: str, kills: int) -> None:
    runtime = ctx.store.get_entity(zone_id)
    if runtime is not None:
        populations = dict(runtime["state"].get("populations", {}))
        location = runtime["location_id"]
    else:
        populations, location = {}, None
        for floor in ctx.registry.by_kind("floor"):
            for zone in floor.zones:
                if zone.id == zone_id:
                    populations = {p.species: p.current
                                   for p in zone.monster_populations}
    if species in populations:
        populations[species] = max(0, populations[species] - kills)
        ctx.store.upsert_entity(zone_id, "zone", zone_id,
                                {"populations": populations}, location, 0)
