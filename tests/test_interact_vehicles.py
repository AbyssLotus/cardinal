"""The interaction primitive (devices, skill checks, effects) and the
vehicle runtime (mount, ride, fuel, combat absorption, ram)."""

from pathlib import Path

from engine.core.loop import SimulationLoop
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save
from engine.systems.combat import get_encounter

FIXTURES = Path(__file__).parent / "fixtures"
CYBERPUNK = FIXTURES / "probe_cyberpunk"


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


def _until(loop, command, predicate, tries=15):
    result = None
    for _ in range(tries):
        result = loop.submit(command)
        if predicate():
            return result
    return result


# ------------------------------------------------------------- netrunning


def test_jack_in_raid_jack_out(tmp_path):
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=31)
    loop = _loop(save)

    # jack in: skill-checked hack, requires the cyberdeck, teleports presence
    _until(loop, "hack terminal",
           lambda: save.store.get_player()["location_id"] == "loc.cp_subnet_shallows")
    assert save.store.get_player()["location_id"] == "loc.cp_subnet_shallows"

    # fight ICE with a quickhack (beam, RAM-costed) — combat inside the net
    loop.submit("hunt wisp")
    assert get_encounter(loop.ctx) is not None
    for _ in range(60):
        loop.submit("attack zap")
        if get_encounter(loop.ctx) is None:
            break
    assert get_encounter(loop.ctx) is None
    assert save.store.get_player()["alive"] == 1

    # crack the data vault (state-gated, loot effect, secret chronicle)
    def vault_emptied():
        runtime = save.store.get_entity("device.cp_datavault")
        return runtime is not None and runtime["state"].get("state") == "emptied"
    _until(loop, "hack vault", vault_emptied, tries=25)
    assert vault_emptied()
    shards = sum(i["qty"] for i in save.store.get_inventory("player")
                 if i["def_id"] == "item.cp_datashard")
    assert shards >= 3
    secrets = save.store.conn.execute(
        "SELECT * FROM chronicle WHERE visibility='secret'").fetchall()
    assert any("data vault" in r["headline"] for r in secrets)

    # the vault stays cracked: state gate refuses a second run
    result = loop.submit("hack vault")
    assert "emptied" in result.text

    # jack out via the exit node
    result = loop.submit("use exit node")
    assert save.store.get_player()["location_id"] == "loc.cp_afterlife"
    assert "Meat again" in result.text

    # the run trained the skill
    assert save.store.get_player_skill("skill.cp_netrunning") > 0
    save.store.close()


def test_interaction_requires_tool(tmp_path):
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=2)
    loop = _loop(save)
    with save.store.transaction():
        save.store.conn.execute(
            "DELETE FROM item_instances WHERE def_id='item.cp_cyberdeck'")
    result = loop.submit("hack terminal")
    assert "You need a Fuyutsuki Cyberdeck" in result.text
    assert save.store.get_player()["location_id"] == "loc.cp_afterlife"
    save.store.close()


def test_unknown_device(tmp_path):
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=2)
    result = _loop(save).submit("hack mainframe")
    assert "no 'mainframe' here" in result.text
    save.store.close()


# ------------------------------------------------------------- vehicles


def test_mount_ride_burn_fuel(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=6)
    loop = _loop(save)
    result = loop.submit("mount autocart")
    assert "You mount the Autocart" in result.text
    result = loop.submit("go far village")
    assert result.minutes_elapsed == 2            # 1 km at 25 km/h, not 10 min afoot
    assert "aboard the Autocart" in result.text
    cart = save.store.get_entity("vehicleinst.start_0")
    assert cart["location_id"] == "loc.tw_village"  # the cart came along
    assert cart["state"]["fuel"] == 9.8             # 1 km × 0.2/km burned
    result = loop.submit("dismount")
    assert "You dismount" in result.text
    # walking again: full pedestrian price back
    assert loop.submit("go hub").minutes_elapsed == 10
    save.store.close()


def test_vehicle_absorbs_hits_and_rams(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=17)
    loop = _loop(save)
    loop.submit("mount autocart")
    loop.submit("hunt slime")
    result = None
    for _ in range(40):
        result = loop.submit("attack ram")
        if get_encounter(loop.ctx) is None:
            break
    assert get_encounter(loop.ctx) is None
    player = save.store.get_player()
    assert player["alive"] == 1
    assert player["hp"] == player["hp_max"]        # the cart took every hit
    cart = save.store.get_entity("vehicleinst.start_0")
    assert cart["state"]["hp"] <= 200              # and has the dents to show it
    save.store.close()


def test_ram_needs_a_ride(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=17)
    loop = _loop(save)
    loop.submit("hunt slime")
    result = loop.submit("attack ram")
    assert "riding something" in result.text
    save.store.close()


def test_mount_requires_skill(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=6)
    loop = _loop(save)
    with save.store.transaction():
        save.store.conn.execute("DELETE FROM player_skills")
    result = loop.submit("mount autocart")
    assert "don't know how to operate it" in result.text
    save.store.close()
