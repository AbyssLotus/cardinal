"""M3: combat, skills, loot, ammo, permadeath — headless and deterministic."""

from engine.core.loop import SimulationLoop
from engine.core.registry import load_world
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save
from engine.systems import combat


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


def _fight_out(loop, command="attack", max_rounds=200):
    """Submit combat rounds until the encounter resolves; returns final result."""
    from engine.systems.combat import get_encounter

    result = None
    for _ in range(max_rounds):
        result = loop.submit(command)
        if get_encounter(loop.ctx) is None:
            break
    return result


def test_melee_hunt_and_kill(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=11)
    loop = _loop(save)
    result = loop.submit("hunt slime")
    assert "engage" in result.text
    assert result.minutes_elapsed > 0            # searching the wilds took time

    final = _fight_out(loop)
    assert "quiet" in final.text                  # victory line
    player = save.store.get_player()
    assert player["xp"] > 0 and player["alive"] == 1
    # world consequence: the zone has one fewer slime
    zone = save.store.get_entity("zone.tw_field")
    assert zone["state"]["populations"]["mon.tw_slime"] < 50
    # weapon took durability wear
    stick = next(i for i in save.store.get_inventory("player")
                 if i["def_id"] == "item.tw_stick")
    assert stick["durability"] < 200
    # the kill is in player history
    rows = save.store.conn.execute(
        "SELECT * FROM player_history WHERE kind='combat'").fetchall()
    assert any("Slew" in r["summary"] for r in rows)
    save.store.close()


def test_projectile_combat_consumes_ammo_and_grows_skill(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=7)
    loop = _loop(save)
    loop.submit("equip sling")
    loop.submit("hunt slime")
    final = _fight_out(loop, command="attack sling shot")
    assert "quiet" in final.text
    pebbles = [i for i in save.store.get_inventory("player")
               if i["def_id"] == "item.tw_pebble"]
    remaining = pebbles[0]["qty"] if pebbles else 0
    assert remaining < 20                          # shots consumed ammo
    assert save.store.get_player_skill("skill.tw_slinging") > 0.0
    save.store.close()


def test_unknown_technique_rejected(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=3)
    loop = _loop(save)
    loop.submit("hunt slime")
    result = loop.submit("attack starburst stream")
    assert "don't know a technique" in result.text
    save.store.close()


def test_permadeath_is_real(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=13)
    loop = _loop(save)
    with save.store.transaction():
        save.store.update_player(hp=2)             # one hit from the void
    loop.submit("hunt slime")
    final = _fight_out(loop)
    player = save.store.get_player()
    if player["alive"] == 0:                       # the expected outcome
        assert player["hp"] == 0
        chronicle = [e["headline"] for e in save.store.get_chronicle()]
        assert any("slain" in h for h in chronicle)
        # the dead take no actions
        refused = loop.submit("look")
        assert not refused.ok or "dead" in refused.text.lower()
    else:
        # freak seed where a 2 HP player wins — still a valid world, but flag it
        raise AssertionError("seed 13 unexpectedly survived; pick a crueler seed")
    save.store.close()


def test_combat_locks_out_world_actions(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=11)
    loop = _loop(save)
    loop.submit("hunt slime")
    blocked = loop.submit("go far village")
    assert not blocked.ok and "attack, guard, or flee" in blocked.text
    save.store.close()


def test_guard_and_flee(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=11)
    loop = _loop(save)
    loop.submit("hunt slime")
    result = loop.submit("guard parry")
    assert "set yourself to parry" in result.text
    # flee until it succeeds (deterministic under the seed)
    for _ in range(20):
        result = loop.submit("flee")
        from engine.systems.combat import get_encounter
        if get_encounter(loop.ctx) is None:
            break
    assert "break away" in result.text
    rows = save.store.conn.execute(
        "SELECT * FROM player_history WHERE kind='combat'").fetchall()
    assert any("Fled" in r["summary"] for r in rows)
    save.store.close()


def test_xp_levels_up_eventually(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=19)
    loop = _loop(save)
    # slimes are 25 xp; level 2 needs 100 — grind, resting to heal between fights
    for _ in range(10):
        loop.submit("hunt slime")
        _fight_out(loop)
        if save.store.get_player()["level"] >= 2:
            break
        if save.store.get_player()["alive"] == 0:
            break
        loop.submit("wait 600")                    # passive regen between hunts
    player = save.store.get_player()
    assert player["alive"] == 1
    assert player["level"] >= 2
    milestones = save.store.conn.execute(
        "SELECT * FROM player_history WHERE kind='milestone'").fetchall()
    assert any("level 2" in r["summary"] for r in milestones)
    save.store.close()


def test_defend_gate_varies_with_player_build(tmp_path, aincrad_path):
    """Regression test for the Test 4 playtest finding: the guard-stance
    defend gate used to be `combat.base_reaction_ms` (a flat constant, never
    modified by anything) compared against a value derived from the
    *monster's* reaction_ms — meaning some monsters (dire_wolf, and the
    Floor 1 boss illfang) were mathematically undefendable by ANY player,
    at ANY level, with ANY gear, forever. Nothing about the player's build
    could ever change the outcome.

    This asserts the fix directly: a higher-level, acrobatics-trained
    player has a strictly better (lower) reaction time than a fresh
    level-1 character with no acrobatics, so the gate is no longer static.
    """
    save = create_save(tmp_path / "s1", aincrad_path, seed=1)
    ctx = _loop(save).ctx

    fresh_reaction = combat._player_reaction_ms(ctx)

    save.store.update_player(level=50)
    save.store.upsert_player_skill("skill.acrobatics", 1000.0)
    trained_reaction = combat._player_reaction_ms(ctx)

    assert trained_reaction < fresh_reaction              # build now matters
    assert trained_reaction >= ctx.registry.rule("combat.min_reaction_ms", 250)
    save.store.close()


def test_no_monster_is_unconditionally_undefendable(tmp_path, aincrad_path):
    """Every Aincrad monster must be defendable by a sufficiently trained
    player. Before the fix, dire_wolf and illfang's attack windup was
    derived from their own reaction_ms in a way that made
    `base_reaction_ms < total_ms` false for every possible player value —
    a mechanical impossibility, not just a hard fight. A veteran build
    (high level + max acrobatics) must now be able to beat that gate for
    every monster in the bestiary, including the floor boss.
    """
    save = create_save(tmp_path / "s1", aincrad_path, seed=1)
    ctx = _loop(save).ctx
    save.store.update_player(level=90)
    save.store.upsert_player_skill("skill.acrobatics", 1000.0)
    veteran_reaction = combat._player_reaction_ms(ctx)

    registry = load_world(aincrad_path)
    undefendable = []
    for monster_def in registry.by_kind("mon"):
        windup = combat._monster_attack_windup_ms(monster_def)
        if not (veteran_reaction < windup):
            undefendable.append(monster_def.id)
    assert not undefendable, f"still mechanically undefendable: {undefendable}"
    save.store.close()


def test_fresh_player_reaction_matches_prior_default(tmp_path, aincrad_path):
    """A brand-new level-1, no-acrobatics character should reaction-check
    at exactly the old flat constant (800ms in Aincrad) — the fix changes
    what improves the number, not the unbuilt baseline, so starting
    difficulty is unchanged for a fresh save.
    """
    save = create_save(tmp_path / "s1", aincrad_path, seed=1)
    ctx = _loop(save).ctx
    assert combat._player_reaction_ms(ctx) == 800
    save.store.close()
