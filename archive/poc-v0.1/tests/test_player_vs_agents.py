"""§22.4 player interaction surface: agents as duel targets (full combat
state machine, foreground fidelity — where survivors CAN die), direct
trade against real agent inventory/col, and opposed steal checks."""

from pathlib import Path

from engine.core.loop import SimulationLoop
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save
from engine.systems import agents, combat

CYBERPUNK = Path(__file__).parent / "fixtures" / "probe_cyberpunk"


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


def _prep_agent(save, loop, npc_id, hp=12, col=250, level=None):
    npc = save.registry.find(npc_id)
    state = agents.agent_state(loop.ctx, npc)
    state["hp"] = hp
    state["col"] = col
    if level is not None:
        state["level"] = level
    agents._save(loop.ctx, npc, state, "loc.cp_afterlife", 0)
    return npc


def _buff_player(save):
    save.store.update_player(hp=5000, hp_max=5000, level=40)


def _duel_to_the_end(loop, save, rounds=60):
    for _ in range(rounds):
        if combat.get_encounter(loop.ctx) is None:
            return
        loop.submit("attack")


def test_player_duels_and_loots_an_agent(tmp_path):
    """attack <agent> runs the FULL combat state machine; victory is
    permadeath for the agent, corpse looting (col + entire inventory),
    a public chronicle entry, and a personal reputation hit."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=9)
    loop = _loop(save)
    _buff_player(save)
    blister = _prep_agent(save, loop, "npc.cp_blister", hp=12, col=333)
    save.store.add_item_instance("iteminst.swag", "item.cp_bullets",
                                 "npc.cp_blister", qty=7)

    col_before = save.store.get_player()["col"]
    result = loop.submit("attack blister")
    assert "square up against Blister" in result.text
    assert combat.get_encounter(loop.ctx) is not None   # a real encounter

    _duel_to_the_end(loop, save)
    assert combat.get_encounter(loop.ctx) is None       # resolved, cleared

    corpse = save.store.get_entity("npc.cp_blister")["state"]
    assert corpse["alive"] is False and corpse["col"] == 0
    assert save.store.get_player()["col"] == col_before + 333
    looted = sum(i["qty"] for i in save.store.get_inventory("player")
                 if i["def_id"] == "item.cp_bullets")
    assert looted >= 7
    headlines = [e["headline"] for e in save.store.get_chronicle(2000)]
    assert any("cut down" in h and "Blister" in h for h in headlines)
    assert save.store.get_reputation("npc.cp_blister") < 0
    save.store.close()


def test_survivors_die_in_foreground_combat(tmp_path):
    """The survivor rule's other half: Rita cannot die in background EV
    resolution, but a duel with the player watching is exactly where the
    named CAN fall."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=9)
    loop = _loop(save)
    _buff_player(save)
    rita = save.registry.find("npc.cp_rita")
    assert rita.survivor is True
    state = agents.agent_state(loop.ctx, rita)
    state["hp"] = 10
    agents._save(loop.ctx, rita, state, "loc.cp_afterlife", 0)

    loop.submit("attack rita")
    _duel_to_the_end(loop, save)
    assert save.store.get_entity("npc.cp_rita")["state"]["alive"] is False
    save.store.close()


def test_killing_a_faction_member_moves_faction_standing(tmp_path):
    """Cutting down a Tyger Claw is not a private matter: the player's
    standing with the faction drops."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=9)
    loop = _loop(save)
    _buff_player(save)
    ryo = save.registry.find("npc.cp_ryo")
    state = agents.agent_state(loop.ctx, ryo)
    state["hp"] = 10
    agents._save(loop.ctx, ryo, state, "loc.cp_afterlife", 0)

    result = loop.submit("attack ryo")
    _duel_to_the_end(loop, save)
    assert save.store.get_reputation("fac.cp_tyger_claws") < 0
    save.store.close()


def test_direct_trade_moves_real_inventory_and_col(tmp_path):
    """buy from / sell to an agent trades against their ACTUAL inventory
    and purse — both sides' balances move, no market involved."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=9)
    loop = _loop(save)
    wei = _prep_agent(save, loop, "npc.cp_wei", hp=100, col=500)
    save.store.add_item_instance("iteminst.stock", "item.cp_bullets",
                                 "npc.cp_wei", qty=20)
    save.store.update_player(col=100)

    result = loop.submit("barter wei")
    assert "shows you their goods" in result.text
    assert "9mm Rounds" in result.text

    ammo_before = sum(i["qty"] for i in save.store.get_inventory("player")
                      if i["def_id"] == "item.cp_bullets")
    result = loop.submit("buy bullets 5 from wei")
    assert "sells you 5x" in result.text
    assert sum(i["qty"] for i in save.store.get_inventory("player")
               if i["def_id"] == "item.cp_bullets") == ammo_before + 5
    wei_state = save.store.get_entity("npc.cp_wei")["state"]
    assert wei_state["col"] == 510                       # 5 x 2 col
    assert save.store.get_player()["col"] == 90
    assert sum(i["qty"] for i in save.store.get_inventory("npc.cp_wei")
               if i["def_id"] == "item.cp_bullets") == 15

    result = loop.submit("sell bullets 3 to wei")
    assert "buys 3x" in result.text
    assert save.store.get_player()["col"] == 96
    assert save.store.get_entity("npc.cp_wei")["state"]["col"] == 504
    assert sum(i["qty"] for i in save.store.get_inventory("player")
               if i["def_id"] == "item.cp_bullets") == ammo_before + 2
    save.store.close()


def test_trade_respects_agent_solvency(tmp_path):
    """An agent that can't cover the price doesn't buy — their col is a
    real constraint, not a facade."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=9)
    loop = _loop(save)
    _prep_agent(save, loop, "npc.cp_wei", hp=100, col=0)
    save.store.add_item_instance("iteminst.mine", "item.cp_shard",
                                 "player", qty=1)
    result = loop.submit("sell shard to wei")
    assert "can't cover" in result.text
    assert sum(i["qty"] for i in save.store.get_inventory("player")
               if i["def_id"] == "item.cp_shard") == 1
    save.store.close()


def test_steal_success_skims_the_purse(tmp_path):
    """A high-level player against a low-level mark: the opposed check
    lands, col moves quietly, no fight."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=9)
    loop = _loop(save)
    save.store.update_player(level=40)                   # chance clamps to 0.9
    _prep_agent(save, loop, "npc.cp_wei", hp=100, col=1000, level=1)

    stolen = False
    for _ in range(5):                                   # p(all fail) = 1e-5
        result = loop.submit("steal wei")
        if "fingers find" in result.text:
            stolen = True
            break
        if combat.get_encounter(loop.ctx) is not None:   # caught — bail out
            loop.submit("flee")
    assert stolen
    assert save.store.get_player()["col"] > 0
    assert save.store.get_entity("npc.cp_wei")["state"]["col"] < 1000
    save.store.close()


def test_steal_failure_starts_a_fight(tmp_path):
    """Caught red-handed: reputation hit and the mark comes at you —
    through the same duel machinery."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=9)
    loop = _loop(save)
    _buff_player(save)
    save.store.update_player(level=1)                    # chance floors at 0.05
    _prep_agent(save, loop, "npc.cp_blister", hp=200, col=100, level=90)

    caught = False
    for _ in range(20):                                  # p(never caught) ~ 1e-26
        result = loop.submit("steal blister")
        if "catches your hand" in result.text:
            caught = True
            break
    assert caught
    assert combat.get_encounter(loop.ctx) is not None
    assert save.store.get_reputation("npc.cp_blister") < 0
    save.store.close()


def test_multi_round_duel_lets_the_agent_fight_back(tmp_path):
    """Regression: a duel target that survives round one must act on the
    player through the monster-side machinery (the adapter's empty
    ai_script used to crash _monsters_act). The agent lands real hits."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=9)
    loop = _loop(save)
    _buff_player(save)
    _prep_agent(save, loop, "npc.cp_blister", hp=200, col=0, level=8)

    loop.submit("attack blister")
    hp_start = save.store.get_player()["hp"]
    for _ in range(12):
        if combat.get_encounter(loop.ctx) is None:
            break
        loop.submit("attack")
    assert save.store.get_player()["hp"] < hp_start   # he hit back
    save.store.close()
