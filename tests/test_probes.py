"""Genre probes: the same engine runs Night City, the Cosmodrome, and Elwynn.

Each probe world exercises a different power system in pure data:
  cyberpunk — firearms (undodgeable rounds, magazines of real ammo), permadeath
  destiny   — Light pool abilities (area grenade, hitscan super), resurrection
  wow       — mana-limited casting, graveyard respawn

No engine code knows any of these settings exist.
"""

from pathlib import Path

from engine.core.loop import SimulationLoop
from engine.core.registry import load_world
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save
from engine.systems.combat import get_encounter

FIXTURES = Path(__file__).parent / "fixtures"
CYBERPUNK = FIXTURES / "probe_cyberpunk"
DESTINY = FIXTURES / "probe_destiny"
WOW = FIXTURES / "probe_wow"


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


def _fight_out(loop, command="attack", max_rounds=200):
    result = None
    for _ in range(max_rounds):
        result = loop.submit(command)
        if get_encounter(loop.ctx) is None:
            break
    return result


def test_probe_worlds_validate():
    for path in (CYBERPUNK, DESTINY, WOW):
        registry = load_world(path)
        assert registry.manifest.name


def test_world_vocabulary_is_data():
    assert load_world(CYBERPUNK).manifest.currency.name == "Eurodollars"
    assert load_world(DESTINY).manifest.region_label == "Sector"
    assert load_world(WOW).manifest.currency.id == "gold"


def test_cyberpunk_gunfight(tmp_path):
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=21)
    loop = _loop(save)
    result = loop.submit("hunt scav")
    assert "engage" in result.text
    final = _fight_out(loop)  # bare attack = shoot: pistol is a ranged weapon
    player = save.store.get_player()
    assert player["alive"] in (0, 1)
    if player["alive"]:
        assert "quiet" in final.text
        bullets = [i for i in save.store.get_inventory("player")
                   if i["def_id"] == "item.cp_bullets"]
        spent = 30 - (bullets[0]["qty"] if bullets else 0)
        assert spent > 0                          # shots consumed real rounds
        assert save.store.get_player_skill("skill.cp_handguns") > 0
    save.store.close()


def test_cyberpunk_double_tap_spends_two_rounds(tmp_path):
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=8)
    loop = _loop(save)
    loop.submit("hunt scav")
    before = next(i["qty"] for i in save.store.get_inventory("player")
                  if i["def_id"] == "item.cp_bullets")
    result = loop.submit("attack double tap")
    if "hits" in result.text or "slips aside" in result.text:
        after = sum(i["qty"] for i in save.store.get_inventory("player")
                    if i["def_id"] == "item.cp_bullets")
        assert before - after == 2
    save.store.close()


def test_destiny_grenade_hits_the_pack_and_spends_light(tmp_path):
    save = create_save(tmp_path / "s1", DESTINY, seed=5)
    loop = _loop(save)
    loop.submit("hunt dreg")
    state = get_encounter(loop.ctx)
    pack_size = len(state["monsters"])
    assert pack_size >= 3                          # dregs travel in packs
    result = loop.submit("attack solar grenade")
    hits = result.text.count("Your Solar Grenade hits")
    assert hits >= 2                               # area delivery, multiple struck
    state = get_encounter(loop.ctx)
    if state is not None:
        assert state["player"]["pools"]["light"] <= 100 - 60 + 4  # cost minus regen
    save.store.close()


def test_destiny_guardians_resurrect(tmp_path):
    save = create_save(tmp_path / "s1", DESTINY, seed=9)
    loop = _loop(save)
    with save.store.transaction():
        save.store.update_player(hp=3, col=1000)
    loop.submit("hunt dreg")
    final = _fight_out(loop, command="guard block", max_rounds=60)  # stand and die
    player = save.store.get_player()
    assert player["alive"] == 1                    # death is not the end here
    assert player["hp"] == player["hp_max"]        # revived whole
    assert player["location_id"] == "loc.d2_tower"  # back at the respawn point
    assert player["col"] == 950                    # 5% glimmer tax
    assert "wake at The Tower" in final.text
    save.store.close()


def test_wow_fireball_costs_mana_and_kills(tmp_path):
    save = create_save(tmp_path / "s1", WOW, seed=4)
    loop = _loop(save)
    loop.submit("hunt murloc")
    result = loop.submit("attack fireball")
    assert "Fireball" in result.text
    state = get_encounter(loop.ctx)
    if state is not None:
        assert state["player"]["pools"]["mana"] < 120
        _fight_out(loop, command="attack fireball", max_rounds=40)
    player = save.store.get_player()
    assert player["alive"] == 1
    assert save.store.get_player_skill("skill.wow_fire") > 0
    save.store.close()


def test_wow_out_of_mana_is_a_real_wall(tmp_path):
    save = create_save(tmp_path / "s1", WOW, seed=4)
    loop = _loop(save)
    loop.submit("hunt murloc")
    state = get_encounter(loop.ctx)
    if state is not None:
        state["player"]["pools"]["mana"] = 5
        from engine.systems.combat import _save as save_state
        with save.store.transaction():
            save_state(loop.ctx, state)
        result = loop.submit("attack fireball")
        assert "Not enough mana" in result.text
    save.store.close()
