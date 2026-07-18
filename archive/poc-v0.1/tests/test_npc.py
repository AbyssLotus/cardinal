"""M2: NPC agents live — needs decay, schedules run, memories form,
goals accrue effort, quests expire with consequences, all without a player."""

from engine.core.loop import SimulationLoop
from engine.memory.npc_memory import memories_of
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save


def _ticked_save(tmp_path, testworld_path, days: int, seed: int = 5):
    save = create_save(tmp_path / "s1", testworld_path, seed=seed)
    loop = SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())
    loop.advance_days(days)
    return save


def test_needs_decay_and_recovery(tmp_path, testworld_path):
    save = _ticked_save(tmp_path, testworld_path, days=2)
    alice = save.store.get_entity("npc.tw_alice")
    needs = alice["state"]["needs"]
    # decayed from 1.0 but recovered by eating/sleeping — never pinned at 0
    assert 0.0 < needs["food"] <= 1.0
    assert 0.0 < needs["rest"] <= 1.0
    assert alice["state"]["activity"] is not None
    save.store.close()


def test_schedules_position_npcs(tmp_path, testworld_path):
    save = _ticked_save(tmp_path, testworld_path, days=1)
    # everyone is where their life puts them, not where the YAML spawned them
    for npc_id, expected in [("npc.tw_alice", "loc.tw_hub"), ("npc.tw_dan", "loc.tw_village")]:
        assert save.store.get_entity(npc_id)["location_id"] == expected
    save.store.close()


def test_socializing_builds_memories(tmp_path, testworld_path):
    save = _ticked_save(tmp_path, testworld_path, days=3)
    # evening socialize blocks at the hub: Alice and Bob know each other now
    alice_memories = memories_of(save.store, "npc.tw_alice")
    assert any(m["kind"] == "conversation" and m["subject_id"] == "npc.tw_bob"
               for m in alice_memories)
    save.store.close()


def test_goals_accrue_effort(tmp_path, testworld_path):
    save = _ticked_save(tmp_path, testworld_path, days=3)
    goal = save.store.get_goal("npc.tw_bob", "goal.tw_bob_masterwork")
    assert goal is not None and goal["status"] == "active"
    assert goal["progress"].get("effort", 0) > 0
    save.store.close()


def test_quest_becomes_available_then_expires(tmp_path, testworld_path):
    save = _ticked_save(tmp_path, testworld_path, days=6)
    quests = {q["instance_id"]: q for q in save.store.get_quests()}
    errand = quests["quest.tw_errand"]
    assert errand["state"] == "expired"           # duration 3, nobody helped
    # failure world_effects applied: Dan is glum
    assert save.store.get_entity("npc.tw_dan")["state"]["mood"] == "glum"
    # both lifecycle events entered the chronicle
    headlines = [e["headline"] for e in save.store.get_chronicle(100)]
    assert any("parcel must reach Hub Town" in h for h in headlines)   # availability
    assert any("never made it" in h for h in headlines)                # authored failure
    save.store.close()


def test_playerless_month_produces_a_different_world(tmp_path, testworld_path):
    """The v0.1 definition-of-done, fixture-sized: after a month the chronicle,
    NPC memories, and NPC state differ materially from day zero."""
    save = _ticked_save(tmp_path, testworld_path, days=30)
    chronicle = save.store.get_chronicle(200)
    assert len(chronicle) > 2                     # more than the founding seeds
    memory_count = save.store.conn.execute(
        "SELECT COUNT(*) AS n FROM npc_memory").fetchone()["n"]
    assert memory_count > 0
    # zone populations were simulated and bookkept
    ticked = save.store.conn.execute("SELECT COUNT(*) AS n FROM zone_ticks").fetchone()["n"]
    assert ticked == 2                            # both testworld zones
    save.store.close()
