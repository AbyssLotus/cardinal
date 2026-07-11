"""M6 hardening: golden combat, invariants (§17), migrations, telemetry."""

import re

import yaml

from engine import telemetry
from engine.core.loop import SimulationLoop
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence import migrations
from engine.persistence.saves import create_save, open_save
from engine.systems.combat import get_encounter


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


def _fight_out(loop, command="attack", max_rounds=200):
    outputs = []
    for _ in range(max_rounds):
        outputs.append(loop.submit(command).text)
        if get_encounter(loop.ctx) is None:
            break
    return outputs


# ------------------------------------------------------------------ golden


def test_golden_combat_same_seed_identical_transcript(tmp_path, testworld_path):
    """Same seed + same combat script ⇒ byte-identical fight, end state and all."""
    transcripts, states = [], []
    for name in ("a", "b"):
        save = create_save(tmp_path / name, testworld_path, seed=77)
        loop = _loop(save)
        loop.submit("hunt slime")
        transcript = _fight_out(loop)
        transcripts.append("\n".join(transcript))
        states.append({
            "player": save.store.get_player(),
            "inventory": save.store.get_inventory("player"),
            "skills": save.store.get_player_skills(),
            "zone": save.store.get_entity("zone.tw_field"),
        })
        save.store.close()
    assert transcripts[0] == transcripts[1]
    assert states[0] == states[1]


# ------------------------------------------------------------------ invariants


def test_no_level_scaling_ever(tmp_path, testworld_path):
    """§8: a level-2 slime hits a level-40 player with the same numbers it
    hits a level-1 player. Same seed, only player level differs — every
    monster damage line must be identical."""
    hits_by_level = {}
    for level in (1, 40):
        save = create_save(tmp_path / f"lvl{level}", testworld_path, seed=31)
        with save.store.transaction():
            save.store.update_player(level=level, hp=500, hp_max=500)
        loop = _loop(save)
        loop.submit("hunt slime")
        transcript = "\n".join(_fight_out(loop))
        hits_by_level[level] = re.findall(r"Slime hits you for (\d+)", transcript)
        save.store.close()
    assert hits_by_level[1] == hits_by_level[40]
    assert hits_by_level[1], "expected the slime to land at least one hit"


def test_col_conservation(tmp_path, testworld_path):
    """§17: currency is neither created nor destroyed outside defined
    sources (sells, quest rewards) and sinks (buys). Every col movement in a
    play session must reconcile against the ledger the engine reported."""
    save = create_save(tmp_path / "s1", testworld_path, seed=5)
    loop = _loop(save)
    loop.advance_days(1)  # markets + quests come online
    start = save.store.get_player()["col"]
    ledger = 0

    text = loop.submit("buy ration 2").text
    spent = int(re.search(r"for (\d+) ", text).group(1))
    ledger -= spent

    text = loop.submit("sell ration 4").text
    earned = int(re.search(r"for (\d+) ", text).group(1))
    ledger += earned

    # earn the quest's 25-col reward
    for _ in range(12):
        loop.submit("hunt slime")
        _fight_out(loop)
        if any(i["def_id"] == "item.tw_goo"
               for i in save.store.get_inventory("player")):
            break
        loop.submit("wait 600")
    loop.submit("go far village")
    text = loop.submit("give goo").text
    if "presses" in text:
        ledger += 25

    assert save.store.get_player()["col"] == start + ledger
    save.store.close()


# ------------------------------------------------------------------ migrations


def test_old_save_format_migrates_on_open(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=3)
    save.store.close()

    # forge a pre-versioned save: format 0 and a pre-M2 schema (no modifiers
    # table, no equipped/qty columns)
    meta_file = tmp_path / "s1" / "meta.yaml"
    meta = yaml.safe_load(meta_file.read_text())
    del meta["save_format"]
    meta_file.write_text(yaml.safe_dump(meta, sort_keys=False))

    import sqlite3
    conn = sqlite3.connect(tmp_path / "s1" / "state.db")
    conn.execute("DROP TABLE modifiers")
    conn.commit()
    conn.close()

    reopened = open_save(tmp_path / "s1")
    assert reopened.meta["save_format"] == migrations.CURRENT_SAVE_FORMAT
    # the migration recreated what was missing — modifier queries work again
    assert reopened.store.get_modifiers("player") == []
    reopened.store.close()

    # and the meta file was persisted, so the migration never re-runs
    meta = yaml.safe_load(meta_file.read_text())
    assert meta["save_format"] == migrations.CURRENT_SAVE_FORMAT


def test_newer_save_format_refuses_to_open(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=3)
    save.store.close()
    meta_file = tmp_path / "s1" / "meta.yaml"
    meta = yaml.safe_load(meta_file.read_text())
    meta["save_format"] = migrations.CURRENT_SAVE_FORMAT + 5
    meta_file.write_text(yaml.safe_dump(meta, sort_keys=False))
    import pytest
    from engine.persistence.saves import SaveError
    with pytest.raises(SaveError, match="newer than this engine"):
        open_save(tmp_path / "s1")


# ------------------------------------------------------------------ telemetry


def test_telemetry_report_shape_and_vitals(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=9)
    loop = _loop(save)
    loop.advance_days(60)
    data = telemetry.report(save.store, save.registry)

    assert data["clock"]["day"] == 60
    assert data["npcs"]["count"] == 5
    # the balance invariant that matters most: NPC life loops are sustainable —
    # nobody starves or collapses over two months of routine
    assert data["npcs"]["needs"]["food"]["min"] > 0.0
    assert data["npcs"]["needs"]["rest"]["min"] > 0.0
    # the balance bug the first year-long run caught: working a schedule
    # must sustain income, not let it bleed to zero
    assert data["npcs"]["needs"]["income"]["min"] > 0.0
    assert data["quests_by_state"]  # lifecycle ran
    assert data["markets"]["rows"] > 0
    assert data["populations"]["mon.tw_slime"]["ratio"] is not None
    save.store.close()
