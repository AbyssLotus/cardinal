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


def test_failed_hacks_trip_a_lockout(tmp_path):
    """Regression test for the Test 5 playtest finding: failure on a
    skill-checked device used to cost nothing but in-game time, so any
    lock in any world was brute-forceable by pure retry-spam (the 5%
    success-chance floor guarantees eventual success). The engine now
    locks a device down after `interact.lockout_after_fails` consecutive
    failures (default 3) for `interact.lockout_minutes` (default 60),
    without every world author having to hand-write failure consequences
    on every device.
    """
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=2)
    loop = _loop(save)

    # get to the vault: jack into the subnet first
    for _ in range(15):
        loop.submit("hack terminal")
        if save.store.get_player()["location_id"] == "loc.cp_subnet_shallows":
            break
    assert save.store.get_player()["location_id"] == "loc.cp_subnet_shallows"

    # Force guaranteed failure so the streak is deterministic.
    device = save.registry.find("device.cp_datavault")
    hack = next(i for i in device.interactions if i.verb == "hack")
    hack.difficulty = 100000  # clamps success chance to the 5% floor... 

    locked = False
    failures = 0
    for _ in range(30):
        result = loop.submit("hack vault")
        if "locks down" in result.text:
            locked = True
            break
        failures += 1
    assert locked, "30 straight failures never tripped a lockout"
    assert failures <= 10                      # tripped promptly, not eventually

    # while locked, attempts are refused outright — no roll, no crack
    result = loop.submit("hack vault")
    assert "are still up" in result.text
    runtime = save.store.get_entity("device.cp_datavault")
    assert runtime["state"].get("state", "locked") != "emptied"
    save.store.close()


def test_lockout_expires_with_time(tmp_path):
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=2)
    loop = _loop(save)
    for _ in range(15):
        loop.submit("hack terminal")
        if save.store.get_player()["location_id"] == "loc.cp_subnet_shallows":
            break
    device = save.registry.find("device.cp_datavault")
    hack = next(i for i in device.interactions if i.verb == "hack")
    hack.difficulty = 100000

    for _ in range(30):
        if "locks down" in loop.submit("hack vault").text:
            break
    loop.submit("wait 120")                    # sleep off the default 60-min lockout
    result = loop.submit("hack vault")
    assert "are still up" not in result.text  # a real attempt happened again
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


def test_ram_is_capped_and_costs_the_vehicle(tmp_path, testworld_path):
    """Regression test for the Test 4 secondary (latent) finding: ram
    damage used to be a raw `top_speed × 0.8` with no cap and no cost —
    dormant only because this world's fastest vehicle is slow. Any world
    shipping a fast vehicle would inherit a free, ammo-less,
    durability-free attack scaling linearly with speed. Ram damage is
    now capped (combat.ram_damage_cap) and each ram costs the vehicle a
    fraction of the damage dealt (combat.ram_self_damage_ratio).
    """
    from engine.systems import combat

    save = create_save(tmp_path / "s1", testworld_path, seed=17)
    loop = _loop(save)
    loop.submit("mount autocart")

    # simulate a fast vehicle: crank the definition's top speed sky-high
    cart_def = save.registry.find("vehicle.tw_autocart")
    original = dict(cart_def.speed_kmh)
    cart_def.speed_kmh["road"] = 500.0                  # a starship on wheels
    try:
        hp_before = save.store.get_entity("vehicleinst.start_0")["state"].get("hp", 200)
        loop.submit("hunt slime")
        result = loop.submit("attack ram")
        cap = save.registry.rule("combat.ram_damage_cap", 40)
        # no landed hit may exceed the cap: 500 km/h × 0.8 = 400 uncapped
        import re
        for hit in re.findall(r"hits .* for (\d+)", result.text):
            assert int(hit) <= cap
        hp_after = save.store.get_entity("vehicleinst.start_0")["state"]["hp"]
        assert hp_after < hp_before                     # the ram cost the ride
    finally:
        cart_def.speed_kmh.clear()
        cart_def.speed_kmh.update(original)
    save.store.close()


def test_mount_requires_skill(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=6)
    loop = _loop(save)
    with save.store.transaction():
        save.store.conn.execute("DELETE FROM player_skills")
    result = loop.submit("mount autocart")
    assert "don't know how to operate it" in result.text
    save.store.close()
