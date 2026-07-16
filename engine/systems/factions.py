"""Faction runtime — M9 slice 1: authored-faction membership & politics.

Implements the first slice of the faction spec (v0.3) in the dependency
order it mandates (§24.4): membership in *authored* factions before
agent integration or runtime founding. What lands here:

- Runtime faction state (treasury, cohesion, dispositions) persisted in
  the entities table, seeded from the authored YAML on first touch.
- A **deterministic strategic tick** (spec §3): no RNG anywhere in this
  module. Each faction, on its stagger day, scores a fixed action
  catalog — consolidate / outreach / feud — and executes the top score;
  ties break by catalog order. The full utility table is logged into
  faction state (`last_decision`) per the spec's telemetry contract
  (§8: decision logs are non-negotiable).
- Player membership (join/leave/donate/status verbs resolved by the
  loop through helpers here), with the spec's exploit guards (§7):
  donation reputation is priced against faction wealth and capped per
  day window, so no money-pump-to-rep-pump chaining.
- `faction_tax` finally entering the pricing formula (Part 1 §11 /
  Part 2 §19.2): markets in a faction's headquarters settlement carry
  its tax rate on buys; revenue credits the faction treasury; members
  of the controlling faction are exempt.
- Dispositions decay toward their authored baseline (grievance decay,
  spec §2.2) so opinion moves are tamper-evident drifts, not raw
  mutable numbers.

Tunables (rules.yaml `factions:` — all optional):
  strategic_tick_days (1), disposition_decay (0.02),
  selective_join_reputation (1.0), join_reputation (0.5),
  leave_reputation_penalty (1.0), donation_rep_scale (1.0),
  donation_rep_daily_cap (0.5), feud_threshold (-0.2),
  accord_threshold (0.0)
"""

from __future__ import annotations

from typing import Any

from engine.persistence.store import Delta
from engine.systems import SystemContext

MEMBER_ID = "factionmember.player"

# fixed catalog order — the deterministic tie-break (spec §3.2)
ACTION_CATALOG = ("consolidate", "outreach", "feud")


# ------------------------------------------------------------- state access


def _stable_hash(text: str) -> int:
    """Deterministic across processes (unlike hash())."""
    value = 0
    for char in text:
        value = (value * 31 + ord(char)) % 997
    return value


def faction_state(ctx: SystemContext, faction) -> dict[str, Any]:
    """Runtime state for a faction, seeded from authored YAML on first touch."""
    runtime = ctx.store.get_entity(faction.id)
    if runtime is not None:
        return dict(runtime["state"])
    state = {
        "treasury": faction.treasury_col,
        "cohesion": 0.6,
        "dispositions": {rel.with_: rel.disposition for rel in faction.relations},
    }
    return state


def save_faction_state(ctx: SystemContext, faction, state: dict[str, Any]) -> None:
    ctx.store.upsert_entity(faction.id, "faction", faction.id, state,
                            faction.headquarters, 0)


def membership(ctx: SystemContext) -> tuple[str, dict[str, Any]] | None:
    """(faction_id, membership state) for the player, or None.

    The faction id lives inside `state` (not the row's def_id) because
    upsert_entity's ON CONFLICT never updates def_id — state is the only
    field that can be reliably cleared on leave."""
    row = ctx.store.get_entity(MEMBER_ID)
    if row is None or not row["state"].get("faction"):
        return None
    return row["state"]["faction"], dict(row["state"])


def find_faction(ctx: SystemContext, name: str):
    needle = name.lower()
    for faction in sorted(ctx.registry.by_kind("fac"), key=lambda f: f.id):
        if needle in faction.name.lower() or needle in faction.id:
            return faction
    return None


def controlling_faction(ctx: SystemContext, settlement_id: str):
    """The faction whose headquarters sits in this settlement — market-tax
    authority. Deterministic tie-break: largest membership, then id."""
    candidates = [f for f in ctx.registry.by_kind("fac")
                  if f.headquarters == settlement_id]
    if not candidates:
        return None
    return sorted(candidates, key=lambda f: (-f.membership_count, f.id))[0]


def market_tax(ctx: SystemContext, market,
               trader_faction: str | None = "__player__") -> tuple[Any, float]:
    """(controlling faction, tax rate) for a market — (None, 0.0) if untaxed.
    Members of the controlling faction are exempt (the perk of dues).

    trader_faction identifies who is trading: the default sentinel means
    "the player" (membership looked up from the save); agents pass their
    own faction id (or None for independents)."""
    faction = controlling_faction(ctx, market.settlement)
    if faction is None:
        return None, 0.0
    rate = float((faction.policies or {}).get("tax_rate", 0.0))
    if rate <= 0:
        return None, 0.0
    if trader_faction == "__player__":
        member = membership(ctx)
        trader_faction = member[0] if member is not None else None
    if trader_faction == faction.id:
        return faction, 0.0
    return faction, rate


def credit_treasury(ctx: SystemContext, faction, amount: int) -> None:
    if amount == 0:
        return
    state = faction_state(ctx, faction)
    state["treasury"] = state.get("treasury", faction.treasury_col) + amount
    save_faction_state(ctx, faction, state)


# --------------------------------------------------------------- strategic tick


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    if granularity != "day":
        return []
    factions = sorted(ctx.registry.by_kind("fac"), key=lambda f: f.id)
    if not factions:
        return []
    rule = ctx.registry.rule
    interval = max(1, rule("factions.strategic_tick_days", 1))
    decay = rule("factions.disposition_decay", 0.02)
    deltas: list[Delta] = []

    for faction in factions:
        # stagger by id hash so factions don't all act on the same day
        if (day + _stable_hash(faction.id)) % interval != 0:
            continue
        state = faction_state(ctx, faction)
        _decay_dispositions(faction, state, decay)
        deltas += _decide(ctx, faction, state, day)
        save_faction_state(ctx, faction, state)
    return deltas


def _decay_dispositions(faction, state: dict[str, Any], decay: float) -> None:
    """Grievance decay (spec §2.2): dispositions drift toward the authored
    baseline, so pushes in either direction fade unless renewed."""
    baselines = {rel.with_: rel.disposition for rel in faction.relations}
    dispositions = state.setdefault("dispositions", {})
    for other, value in list(dispositions.items()):
        baseline = baselines.get(other, 0.0)
        if abs(value - baseline) <= decay:
            dispositions[other] = baseline
        else:
            dispositions[other] = round(
                value + (decay if value < baseline else -decay), 4)
        if dispositions[other] > -0.5:
            state.get("enemies_declared", {}).pop(other, None)


def _decide(ctx: SystemContext, faction, state: dict[str, Any],
            day: int) -> list[Delta]:
    """Deterministic action selection (spec §3.2). Scores the fixed catalog,
    executes the top score, logs the full utility table."""
    cohesion = state.get("cohesion", 0.6)
    dispositions = state.get("dispositions", {})
    aggressive = faction.type in ("gang", "raider", "cult", "horde")
    drive = max((entry.get("priority", 0.0) for entry in faction.agenda), default=0.3)

    scores: dict[str, float] = {}
    scores["consolidate"] = round(0.35 + max(0.0, 0.65 - cohesion), 4)

    outreach_target, best_disp = None, -1.0
    for other_id in sorted(dispositions):
        value = dispositions[other_id]
        if value > best_disp and value > -0.5:
            outreach_target, best_disp = other_id, value
    outreach_score = 0.0
    if outreach_target:
        outreach_score = 0.3 + best_disp * 0.4 + drive * 0.1
        if best_disp >= 0.95:
            outreach_score *= 0.4   # the friendship has nowhere left to grow
    scores["outreach"] = round(outreach_score, 4)

    feud_target, worst_disp = None, 1.0
    for other_id in sorted(dispositions):
        value = dispositions[other_id]
        if value < worst_disp:
            feud_target, worst_disp = other_id, value
    feud_threshold = ctx.registry.rule("factions.feud_threshold", -0.2)
    feud_score = 0.0
    if feud_target is not None and worst_disp < feud_threshold:
        feud_score = 0.3 + abs(worst_disp) * 0.5 + (0.15 if aggressive else 0.0)
        if worst_disp <= -0.95:
            feud_score *= 0.4       # the feud has nowhere left to go
    scores["feud"] = round(feud_score, 4)

    chosen = max(ACTION_CATALOG, key=lambda a: (scores[a], -ACTION_CATALOG.index(a)))
    state["last_decision"] = {"day": day, "chosen": chosen, "scores": scores}

    if chosen == "consolidate":
        return _act_consolidate(faction, state)
    if chosen == "outreach":
        deltas = _act_shift(ctx, faction, state, outreach_target, +0.04,
                            crossing=ctx.registry.rule("factions.accord_threshold", 0.0),
                            upward=True,
                            headline=f"{faction.name} reaches an accord with {{other}}.")
        if not deltas:
            # a second, higher rung: closing ranks at +0.5 is alliance-grade
            # news even when the accord line was never crossed from below
            before = state.get("_shift_before", None)
            after = state["dispositions"].get(outreach_target)
            if before is not None and before < 0.5 <= after:
                other = ctx.registry.find(outreach_target)
                deltas = [Delta(kind="chronicle", payload={
                    "category": "politics", "visibility": "public",
                    "headline": f"{faction.name} closes ranks with "
                                f"{getattr(other, 'name', outreach_target)}.",
                    "detail": f"disposition {before} -> {after}"})]
        state.pop("_shift_before", None)
        return deltas
    # war weariness (spec §4.1): feuding drains cohesion, so feuds exhaust
    # into consolidation instead of grinding silently at the floor forever
    state["cohesion"] = round(max(0.0, state.get("cohesion", 0.6) - 0.03), 4)
    deltas = _act_shift(ctx, faction, state, feud_target, -0.05,
                        crossing=-0.5, upward=False,
                        headline=f"{faction.name} feuds openly with {{other}}.")
    state.pop("_shift_before", None)
    return deltas


def _act_consolidate(faction, state: dict[str, Any]) -> list[Delta]:
    before = state.get("cohesion", 0.6)
    state["cohesion"] = round(min(1.0, before + 0.05), 4)
    if before < 0.3:
        return [Delta(kind="chronicle", payload={
            "category": "politics", "visibility": "public",
            "headline": f"{faction.name} regroups and consolidates its ranks.",
            "detail": f"cohesion {before} -> {state['cohesion']}"})]
    return []


def _act_shift(ctx: SystemContext, faction, state: dict[str, Any], other_id: str,
               step: float, crossing: float, upward: bool,
               headline: str) -> list[Delta]:
    """Move a pairwise disposition (both directions), emitting a chronicle
    entry only when the value crosses the given threshold — treaty-style
    transitions are events, not silent drift (spec §2.1)."""
    other = ctx.registry.find(other_id)
    if other is None:
        return []
    before = state["dispositions"].get(other_id, 0.0)
    after = round(max(-1.0, min(1.0, before + step)), 4)
    state["dispositions"][other_id] = after
    state["_shift_before"] = before

    other_state = faction_state(ctx, other)
    mirrored = other_state.setdefault("dispositions", {}).get(faction.id, 0.0)
    other_state["dispositions"][faction.id] = round(
        max(-1.0, min(1.0, mirrored + step)), 4)
    save_faction_state(ctx, other, other_state)

    crossed = (before < crossing <= after) if upward else (before > crossing >= after)
    if crossed:
        return [Delta(kind="chronicle", payload={
            "category": "politics", "visibility": "public",
            "headline": headline.format(other=other.name),
            "detail": f"disposition {before} -> {after}"})]
    if not upward and after <= -1.0:
        # bottoming out is itself an event — but only once per enmity;
        # the flag clears if relations ever climb back past -0.5
        declared = state.setdefault("enemies_declared", {})
        if not declared.get(other_id):
            declared[other_id] = True
            other_state.setdefault("enemies_declared", {})[faction.id] = True
            save_faction_state(ctx, other, other_state)
            return [Delta(kind="chronicle", payload={
                "category": "politics", "visibility": "public",
                "headline": f"{faction.name} and {other.name} are now sworn enemies.",
                "detail": f"disposition {before} -> {after}"})]
    return []


# --------------------------------------------------------------- player verbs


def resolve_join(ctx: SystemContext, name: str, player: dict,
                 day: int) -> tuple[list[Delta], list[str]]:
    faction = find_faction(ctx, name)
    if faction is None:
        return [], [f"You know of no faction called {name!r}."]
    existing = membership(ctx)
    if existing is not None:
        current = ctx.registry.find(existing[0])
        return [], [f"You already wear {getattr(current, 'name', existing[0])}'s "
                    f"colors. Leave first."]
    hq = faction.headquarters
    if hq is not None and player["location_id"] != hq:
        place = ctx.registry.find(hq)
        return [], [f"{faction.name} recruits at {getattr(place, 'name', hq)}; "
                    f"you'll need to go there."]
    recruitment = (faction.policies or {}).get("recruitment", "open")
    if recruitment == "closed":
        return [], [f"{faction.name} is not taking anyone."]
    if recruitment == "selective":
        needed = ctx.registry.rule("factions.selective_join_reputation", 1.0)
        if ctx.store.get_reputation(faction.id) < needed:
            return [], [f"{faction.name} looks you over and passes. Make a name "
                        f"for yourself first."]
    ctx.store.upsert_entity(MEMBER_ID, "faction_member", MEMBER_ID,
                            {"faction": faction.id, "role": "member",
                             "joined_day": day},
                            player["location_id"], day)
    join_rep = ctx.registry.rule("factions.join_reputation", 0.5)
    deltas = [
        Delta(kind="reputation", payload={"scope_id": faction.id, "delta": join_rep}),
        Delta(kind="chronicle", payload={
            "category": "politics", "visibility": "public",
            "headline": f"A new member joins {faction.name}.",
            "detail": "the player took the colors"}),
    ]
    return deltas, [f"You join {faction.name} as a member."]


def resolve_leave(ctx: SystemContext, player: dict) -> tuple[list[Delta], list[str]]:
    existing = membership(ctx)
    if existing is None:
        return [], ["You belong to no faction."]
    faction_id, _ = existing
    faction = ctx.registry.find(faction_id)
    name = getattr(faction, "name", faction_id)
    ctx.store.upsert_entity(MEMBER_ID, "faction_member", MEMBER_ID,
                            {}, player["location_id"], 0)
    penalty = ctx.registry.rule("factions.leave_reputation_penalty", 1.0)
    deltas = [
        Delta(kind="reputation", payload={"scope_id": faction_id, "delta": -penalty}),
        Delta(kind="chronicle", payload={
            "category": "politics", "visibility": "public",
            "headline": f"A member walks away from {name}.",
            "detail": "the player renounced the colors"}),
    ]
    return deltas, [f"You leave {name}. They won't forget it."]


def resolve_donate(ctx: SystemContext, amount: int, player: dict,
                   day: int) -> tuple[list[Delta], list[str]]:
    existing = membership(ctx)
    if existing is None:
        return [], ["Donate to whom? Join a faction first."]
    faction_id, member_state = existing
    faction = ctx.registry.find(faction_id)
    if amount <= 0:
        return [], ["Donate a positive amount."]
    if player["col"] < amount:
        currency = ctx.registry.manifest.currency.name
        return [], [f"You carry only {player['col']} {currency}."]

    state = faction_state(ctx, faction)
    treasury_before = state.get("treasury", faction.treasury_col)
    ctx.store.update_player(col=player["col"] - amount)
    state["treasury"] = treasury_before + amount
    save_faction_state(ctx, faction, state)

    # Exploit guards (spec §7): reputation priced against faction wealth —
    # rich factions are unimpressed — and capped per day window, so a
    # currency exploit can never chain into a reputation pump.
    scale = ctx.registry.rule("factions.donation_rep_scale", 1.0)
    raw_gain = scale * amount / (amount + max(1, treasury_before))
    cap = ctx.registry.rule("factions.donation_rep_daily_cap", 0.5)
    if member_state.get("donated_day") == day:
        already = member_state.get("donated_rep_today", 0.0)
    else:
        already = 0.0
    gain = round(max(0.0, min(raw_gain, cap - already)), 4)
    member_state["donated_day"] = day
    member_state["donated_rep_today"] = round(already + gain, 4)
    ctx.store.upsert_entity(MEMBER_ID, "faction_member", MEMBER_ID,
                            member_state, player["location_id"], day)

    currency = ctx.registry.manifest.currency.name
    messages = [f"You donate {amount} {currency} to {faction.name}."]
    deltas: list[Delta] = []
    if gain > 0:
        deltas.append(Delta(kind="reputation",
                            payload={"scope_id": faction_id, "delta": gain}))
    else:
        messages.append("The quartermaster barely nods — your generosity has "
                        "made its impression for today.")
    return deltas, messages


def status_report(ctx: SystemContext) -> list[str]:
    lines: list[str] = []
    existing = membership(ctx)
    if existing is not None:
        faction = ctx.registry.find(existing[0])
        name = getattr(faction, "name", existing[0])
        rep = ctx.store.get_reputation(existing[0])
        lines.append(f"You are a {existing[1].get('role', 'member')} of {name} "
                     f"(standing {rep:+.2f}).")
    else:
        lines.append("You belong to no faction.")
    for faction in sorted(ctx.registry.by_kind("fac"), key=lambda f: f.id):
        state = faction_state(ctx, faction)
        moods = ", ".join(
            f"{getattr(ctx.registry.find(k), 'name', k)} {v:+.2f}"
            for k, v in sorted(state.get("dispositions", {}).items()))
        lines.append(f"{faction.name}: treasury {state.get('treasury', 0)}, "
                     f"cohesion {state.get('cohesion', 0.6):.2f}"
                     + (f" — relations: {moods}" if moods else ""))
    return lines
