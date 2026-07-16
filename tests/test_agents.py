"""Agent NPCs (spec §22): the world must change and move without a player.

Every test here runs pure world-simulation — advance_days only, zero
player actions — and asserts the agents' effects landed in the SAME
stores the player's actions use (no shadow systems)."""

from pathlib import Path

from engine.core.loop import SimulationLoop
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save
from engine.systems import factions

CYBERPUNK = Path(__file__).parent / "fixtures" / "probe_cyberpunk"


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


def _run(tmp_path, days, seed=2077):
    save = create_save(tmp_path / "s", CYBERPUNK, seed=seed)
    loop = _loop(save)
    loop.advance_days(days)
    return save, loop


def test_agent_hunts_deplete_real_populations(tmp_path):
    """Scorpion's kills come out of the same population counter the
    player's hunts decrement — same zone entity, same key."""
    save, _ = _run(tmp_path, 20)
    zone = save.store.get_entity("zone.cp_northside_ruins")
    assert zone is not None
    assert zone["state"]["populations"]["mon.cp_scav"] < 90
    # the zone nobody hunts regrows/holds at capacity untouched by agents
    save.store.close()


def test_agent_trades_move_real_market_indices(tmp_path):
    """Loot dumps and merchant arbitrage move the same supply/demand
    indices player trades move — cross-market drift, organically."""
    save, _ = _run(tmp_path, 20)
    moved = [dict(r) for r in save.store.conn.execute(
        "SELECT * FROM markets WHERE ABS(supply_idx - 1.0) > 0.02 "
        "OR ABS(demand_idx - 1.0) > 0.02")]
    assert len(moved) >= 2, moved
    markets_touched = {r["market_id"] for r in moved}
    assert len(markets_touched) >= 2          # drift crossed settlements
    save.store.close()


def test_agent_death_is_permadeath_with_chronicle(tmp_path):
    """Aggressor turf wars produce real deaths: the dead stay dead, the
    chronicle hears about it, and the victim's col is robbed."""
    save, loop = _run(tmp_path, 120)
    states = {aid: save.store.get_entity(aid)["state"]
              for aid in ("npc.cp_dum_dum", "npc.cp_ryo", "npc.cp_rita",
                          "npc.cp_scorpion")}
    dead = [aid for aid, s in states.items() if not s.get("alive", True)]
    assert dead, "120 days of turf war produced no deaths"
    headlines = [e["headline"] for e in save.store.get_chronicle(2000)
                 if e["category"] == "street"]
    assert any("was killed by" in h for h in headlines)
    for aid in dead:
        assert states[aid]["hp"] == 0
        assert states[aid]["col"] == 0        # robbed by the victor

    # permadeath: dead agents never act again
    activity_at_death = {aid: states[aid]["activity"] for aid in dead}
    loop.advance_days(30)
    for aid in dead:
        after = save.store.get_entity(aid)["state"]
        assert not after.get("alive", True)
        assert after["activity"] == "dead" or after["activity"] == activity_at_death[aid]
    save.store.close()


def test_faction_treasuries_move_without_a_player(tmp_path):
    """Dues + the Kabuki protection racket: treasuries change from agent
    activity alone. Maelstrom tithes Scorpion's hauls; the Tyger Claws
    skim non-member sales at their market."""
    save, loop = _run(tmp_path, 30)
    maelstrom = factions.faction_state(
        loop.ctx, save.registry.find("fac.cp_maelstrom"))
    tygers = factions.faction_state(
        loop.ctx, save.registry.find("fac.cp_tyger_claws"))
    assert maelstrom["treasury"] > 15000      # tithes from Scorpion's loot sales
    assert tygers["treasury"] > 60000         # racket skim on Kabuki sales
    save.store.close()


def test_agent_kills_move_faction_standing(tmp_path):
    """Member deaths at rival hands worsen the two factions' dispositions
    (spec §24.1) — verified via the same state the strategic tick reads.
    Decay pulls back toward baseline, so catch it early: run day by day
    until a cross-faction kill lands, then check within the same day."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=2077)
    loop = _loop(save)
    baselines = {("fac.cp_mox", "fac.cp_tyger_claws"): -0.7,
                 ("fac.cp_tyger_claws", "fac.cp_mox"): -0.7,
                 ("fac.cp_maelstrom", "fac.cp_tyger_claws"): -0.4,
                 ("fac.cp_tyger_claws", "fac.cp_maelstrom"): -0.4}
    seen_dip = False
    for _ in range(120):
        loop.advance_days(1)
        for (a, b), baseline in baselines.items():
            state = factions.faction_state(loop.ctx, save.registry.find(a))
            if state["dispositions"].get(b, baseline) < baseline - 0.05:
                seen_dip = True
                break
        if seen_dip:
            break
    assert seen_dip, "no kill ever moved a faction disposition below baseline"
    save.store.close()


def test_world_simulation_is_deterministic_with_agents(tmp_path):
    """Spec §3.3/§22: the whole moving world — hunts, trades, gunfights,
    deaths — replays identically from the same seed."""
    def run(path):
        save, _ = _run(path, 45)
        rows = [dict(r) for r in save.store.conn.execute(
            "SELECT id, state_json, location_id FROM entities "
            "WHERE kind IN ('npc','faction','zone') ORDER BY id")]
        markets = [dict(r) for r in save.store.conn.execute(
            "SELECT * FROM markets ORDER BY market_id, item_def")]
        chronicle = [e["headline"] for e in save.store.get_chronicle(3000)]
        save.store.close()
        return rows, markets, chronicle

    assert run(tmp_path / "a") == run(tmp_path / "b")


def test_world_moves_without_player(tmp_path):
    """The headline requirement: after 90 untouched days, Watson is a
    different place — populations hunted, markets drifted, treasuries
    grown, streets bloodied — and every change came through the real
    systems."""
    save, loop = _run(tmp_path, 90)

    zone = save.store.get_entity("zone.cp_northside_ruins")
    assert zone["state"]["populations"]["mon.cp_scav"] < 90

    drifted = save.store.conn.execute(
        "SELECT COUNT(*) AS n FROM markets WHERE ABS(supply_idx-1.0) > 0.02").fetchone()["n"]
    assert drifted >= 1

    treasuries_moved = 0
    for fid in ("fac.cp_arasaka", "fac.cp_maelstrom", "fac.cp_mox",
                "fac.cp_tyger_claws"):
        faction = save.registry.find(fid)
        state = factions.faction_state(loop.ctx, faction)
        if state["treasury"] != faction.treasury_col:
            treasuries_moved += 1
    assert treasuries_moved >= 1

    street = [e for e in save.store.get_chronicle(3000)
              if e["category"] == "street"]
    assert street, "90 days and the streets stayed silent"
    save.store.close()
