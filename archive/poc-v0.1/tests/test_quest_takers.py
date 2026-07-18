"""§23 Quest Assignment & Competition: agents claim, execute, and complete
contracts through the real systems; assignee death is a first-class
outcome; the player can race a claimed contract; npc_fallback is dead
in worlds with takers."""

from pathlib import Path

from engine.core.loop import SimulationLoop
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save

CYBERPUNK = Path(__file__).parent / "fixtures" / "probe_cyberpunk"


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


def _quest(save, def_id):
    return {q["def_id"]: q for q in save.store.get_quests()}.get(def_id)


def _bankrupt_nix(save, loop):
    """Seed Nix's runtime state with zero col so he claims contracts but
    stalls buying requirements — a deterministic mid-quest freeze."""
    from engine.systems import agents
    nix = save.registry.find("npc.cp_runner")
    state = agents.agent_state(loop.ctx, nix)
    state["col"] = 0
    agents._save(loop.ctx, nix, state, nix.location, 0)


def test_agent_claims_and_completes_contract(tmp_path):
    """Nix scores the ammo run (400 col reward vs ~20 col of bullets),
    claims it (chronicled), buys the bullets from a real market (moving
    its indices), walks them to Rogue, and collects — all with zero
    player actions."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=11)
    loop = _loop(save)
    loop.advance_days(12)

    instance = _quest(save, "quest.cp_ammo_run")
    assert instance is not None
    assert instance["state"] == "completed"
    assert instance["assignee"] == "npc.cp_runner"

    headlines = [e["headline"] for e in save.store.get_chronicle(3000)]
    assert any("took the contract: Ammo Run" in h for h in headlines)
    assert any("fulfilled" in h and "Ammo Run" in h for h in headlines)

    nix = save.store.get_entity("npc.cp_runner")["state"]
    assert nix["col"] > 500 - 50                    # reward covered the buy
    row = save.store.get_market_row("market.cp_afterlife_fixer", "item.cp_bullets")
    assert row is not None and row["demand_idx"] > 1.0   # real market motion
    save.store.close()


def test_npc_fallback_never_fires_with_takers_present(tmp_path):
    """quest.cp_ammo_run authors chance_npc_resolves_per_day: 1.0 — under
    the old system it would 'resolve without fanfare' on day one. With a
    quest_taker in the world the deprecated coin-flip must never roll."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=11)
    loop = _loop(save)
    loop.advance_days(3)
    instance = _quest(save, "quest.cp_ammo_run")
    assert instance["state"] != "npc_resolved"
    save.store.close()


def test_assignee_death_reopens_contract(tmp_path):
    """on_assignee_death: reopen — the claiming agent dies, the contract
    returns to the open pool with a chronicle entry (spec §27
    test_quest_assignee_death: no orphaned instances)."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=11)
    loop = _loop(save)
    # broke couriers still claim, but stall at "short on funds" — freezing
    # the contract mid-execution so the death path is deterministic
    _bankrupt_nix(save, loop)
    loop.advance_days(2)
    instance = _quest(save, "quest.cp_ammo_run")
    assert instance["assignee"] == "npc.cp_runner"
    assert instance["state"] == "available"         # claimed, not finished

    # kill Nix mid-quest
    row = save.store.get_entity("npc.cp_runner")
    state = dict(row["state"])
    state.update({"alive": False, "hp": 0})
    save.store.upsert_entity("npc.cp_runner", "npc", "npc.cp_runner",
                             state, row["location_id"], 2)
    loop.advance_days(1)

    instance = _quest(save, "quest.cp_ammo_run")
    assert instance["state"] == "available"
    assert not instance["assignee"]                 # back in the open pool
    headlines = [e["headline"] for e in save.store.get_chronicle(3000)]
    assert any("contract lapses" in h for h in headlines)
    save.store.close()


def test_assignee_death_can_fail_contract(tmp_path):
    """on_assignee_death: fail — Susie's time-sensitive shard dies with
    its courier."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=11)
    loop = _loop(save)
    # force-claim Last Words for Nix so the death path is deterministic
    loop.advance_days(1)
    instance = _quest(save, "quest.cp_last_words")
    save.store.upsert_quest(instance["instance_id"], "quest.cp_last_words",
                            "available", instance["available_day"],
                            instance["expires_day"], assignee="npc.cp_runner")
    row = save.store.get_entity("npc.cp_runner")
    if row is None:
        save.store.upsert_entity("npc.cp_runner", "npc", "npc.cp_runner",
                                 {"alive": False, "hp": 0}, "loc.cp_afterlife", 1)
    else:
        state = dict(row["state"])
        state.update({"alive": False, "hp": 0})
        save.store.upsert_entity("npc.cp_runner", "npc", "npc.cp_runner",
                                 state, row["location_id"], 1)
    loop.advance_days(1)
    instance = _quest(save, "quest.cp_last_words")
    assert instance["state"] == "failed"
    save.store.close()


def test_player_outraces_a_claimed_contract(tmp_path):
    """§23 competition: Nix claims the ammo run, the player turns in the
    bullets first — first completed turn-in wins, and the loss is
    visible drama in the chronicle."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=11)
    loop = _loop(save)
    _bankrupt_nix(save, loop)                       # stall Nix mid-quest
    loop.advance_days(2)
    instance = _quest(save, "quest.cp_ammo_run")
    assert instance["assignee"] == "npc.cp_runner"
    assert instance["state"] == "available"

    # the player shows up with the goods before Nix finishes
    save.store.add_item_instance("iteminst.race", "item.cp_bullets", "player", qty=10)
    result = loop.submit("talk rogue")
    assert "[claimed by Nix]" in result.text        # the race is visible
    result = loop.submit("give bullets")
    instance = _quest(save, "quest.cp_ammo_run")
    assert instance["state"] == "completed"
    assert instance["assignee"] == "player"

    headlines = [e["headline"] for e in save.store.get_chronicle(3000)]
    assert any("under their nose" in h for h in headlines)

    # the loser never double-collects
    loop.advance_days(5)
    assert _quest(save, "quest.cp_ammo_run")["state"] == "completed"
    save.store.close()
