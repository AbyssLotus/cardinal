"""Save lifecycle, atomic commits, state survival across reopen."""

import pytest

from engine.persistence.saves import SaveError, create_save, open_save
from engine.persistence.store import Delta, Store


def test_create_and_reopen_save(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=7)
    player = save.store.get_player()
    assert player["location_id"] == "loc.tw_hub"
    assert player["col"] == 500
    inventory = save.store.get_inventory("player")
    assert {(i["def_id"], i["qty"]) for i in inventory} == {
        ("item.tw_stick", 1), ("item.tw_ration", 3),
        ("item.tw_sling", 1), ("item.tw_pebble", 20),
    }
    # stick got durability from its definition and was auto-equipped (first weapon)
    stick = next(i for i in inventory if i["def_id"] == "item.tw_stick")
    assert stick["durability"] == 200
    assert stick["equipped"] == 1
    sling = next(i for i in inventory if i["def_id"] == "item.tw_sling")
    assert sling["equipped"] == 0
    # starting skills seeded
    assert save.store.get_player_skills() == {"skill.tw_slinging": 0.0}
    assert save.store.get_clock() == (0, 8 * 60)
    # NPCs materialized as runtime entities; chronicle seeded
    assert save.store.get_entity("npc.tw_alice")["location_id"] == "loc.tw_hub"
    assert any("Hub Town founded" in e["headline"] for e in save.store.get_chronicle())
    save.store.close()

    reopened = open_save(tmp_path / "s1")
    assert reopened.store.get_player()["col"] == 500
    assert reopened.meta["seed"] == 7
    reopened.store.close()


def test_create_refuses_overwrite(tmp_path, testworld_path):
    create_save(tmp_path / "s1", testworld_path, seed=1).store.close()
    with pytest.raises(SaveError, match="already exists"):
        create_save(tmp_path / "s1", testworld_path, seed=2)


def test_commit_is_atomic(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=1)
    good = Delta(kind="player_update", payload={"col": 999})
    bad = Delta(kind="no_such_kind")
    with pytest.raises(ValueError):
        with save.store.transaction():
            save.store.apply_deltas([good, bad], day=0, hour=8)
    # the good delta was rolled back along with the bad one
    assert save.store.get_player()["col"] == 500
    save.store.close()


def test_rng_state_roundtrip(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=42)
    first = [save.rng.stream("combat").random() for _ in range(3)]
    with save.store.transaction():
        save.store.save_rng(save.rng.dump_states())
    upcoming = [save.rng.stream("combat").random() for _ in range(3)]
    save.store.close()

    reopened = open_save(tmp_path / "s1")
    resumed = [reopened.rng.stream("combat").random() for _ in range(3)]
    assert resumed == upcoming  # continues exactly where the saved state left off
    assert resumed != first
    reopened.store.close()


def test_modifier_lifecycle_via_deltas(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=1)
    gain = Delta(kind="modifier_add", payload={
        "owner_id": "player", "def_id": "mod.tw_slime_rot", "expires_day": 5.0,
    })
    chrome = Delta(kind="modifier_add", payload={
        "owner_id": "npc.tw_bob", "def_id": "mod.tw_chrome_arm",
        "state": {"humanity_spent": 0.1},
    })
    with save.store.transaction():
        save.store.apply_deltas([gain, chrome], day=0, hour=9)

    player_mods = save.store.get_modifiers("player")
    assert [m["def_id"] for m in player_mods] == ["mod.tw_slime_rot"]
    assert player_mods[0]["expires_day"] == 5.0
    bob_mods = save.store.get_modifiers("npc.tw_bob")
    assert bob_mods[0]["state"] == {"humanity_spent": 0.1}

    cure = Delta(kind="modifier_remove",
                 payload={"owner_id": "player", "def_id": "mod.tw_slime_rot"})
    with save.store.transaction():
        save.store.apply_deltas([cure], day=1, hour=9)
    assert save.store.get_modifiers("player") == []
    # deactivated, not deleted — history survives for consequences/narration
    assert len(save.store.get_modifiers("player", active_only=False)) == 1
    save.store.close()


def test_store_requires_delta_handler(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=1)
    with pytest.raises(ValueError, match="no delta handler"):
        save.store.apply_deltas([Delta(kind="bogus")], 0, 0)
    save.store.close()
