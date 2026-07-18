"""The 8-step loop: costs, ticking, atomic commits, golden determinism."""

from engine.core.loop import SimulationLoop
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save, open_save


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


def test_wait_advances_clock(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=1)
    loop = _loop(save)
    result = loop.submit("wait 90")
    assert result.ok and result.minutes_elapsed == 90
    assert save.store.get_clock() == (0, 8 * 60 + 90)
    save.store.close()


def test_look_is_free_and_shows_location(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=1)
    result = _loop(save).submit("look")
    assert result.minutes_elapsed == 0
    assert "Hub Town" in result.text
    assert "Alice" in result.text  # co-located NPCs are perceivable
    assert "Dan" not in result.text  # Far Village NPCs are not
    save.store.close()


def test_travel_moves_player_and_costs_time(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=1)
    loop = _loop(save)
    result = loop.submit("go far village")
    assert result.ok
    assert save.store.get_player()["location_id"] == "loc.tw_village"
    assert result.minutes_elapsed == 10  # 10 min/km * (4 km diameter / 4)
    history = save.store.conn.execute(
        "SELECT * FROM player_history WHERE kind='travel'"
    ).fetchall()
    assert len(history) == 1
    save.store.close()


def test_invalid_input_costs_no_time(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=1)
    loop = _loop(save)
    before = save.store.get_clock()
    result = loop.submit("dance wildly")
    assert not result.ok
    assert save.store.get_clock() == before
    save.store.close()


def _run_script(tmp_path, testworld_path, name: str, seed: int) -> dict:
    save = create_save(tmp_path / name, testworld_path, seed=seed)
    loop = _loop(save)
    outputs = [loop.submit(cmd).text for cmd in
               ["look", "wait 200", "go far village", "wait 30", "status"]]
    state = {
        "clock": save.store.get_clock(),
        "player": save.store.get_player(),
        "chronicle": [dict(r) for r in save.store.get_chronicle()],
        "rng": save.store.load_rng(),
        "outputs": outputs,
    }
    save.store.close()
    return state


def test_golden_same_seed_same_script_identical_state(tmp_path, testworld_path):
    a = _run_script(tmp_path, testworld_path, "a", seed=123)
    b = _run_script(tmp_path, testworld_path, "b", seed=123)
    assert a == b


def test_headless_tick_survives_a_year(tmp_path, testworld_path):
    """`cardinal tick --days 365` acceptance check (§17), fixture-sized."""
    save = create_save(tmp_path / "s1", testworld_path, seed=9)
    result = _loop(save).advance_days(365)
    assert save.store.get_clock()[0] == 365
    assert result.minutes_elapsed == 365 * 1440
    save.store.close()
    # and the save still opens cleanly afterwards
    open_save(tmp_path / "s1").store.close()
