"""§24.5 dynamic entities / runtime faction founding, and §25 the
surveillance & information layer."""

from pathlib import Path

from engine.core.loop import SimulationLoop
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save, open_save
from engine.systems import factions

CYBERPUNK = Path(__file__).parent / "fixtures" / "probe_cyberpunk"


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


# ------------------------------------------------------- §24.5 founding


def test_found_mints_a_dynamic_faction(tmp_path):
    """'found <name>' creates a fac.rt_* entity: registry-visible, seeded
    from the founder's col, HQ at the founding site, founder as leader,
    chronicled publicly."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=4)
    loop = _loop(save)
    save.store.update_player(col=1000)

    result = loop.submit("found The Night Owls")
    assert "You found The Night Owls" in result.text

    faction = save.registry.find("fac.rt_the_night_owls")
    assert faction is not None
    assert faction.headquarters == "loc.cp_afterlife"
    assert save.store.get_player()["col"] == 500       # founding cost seeded
    state = factions.faction_state(loop.ctx, faction)
    assert state["treasury"] == 500
    assert state["leader"] == "player"
    member = factions.membership(loop.ctx)
    assert member == ("fac.rt_the_night_owls",
                      member[1]) and member[1]["role"] == "leader"
    headlines = [e["headline"] for e in save.store.get_chronicle(100)]
    assert any("A new banner rises" in h for h in headlines)
    # it participates in the world like any faction
    result = loop.submit("faction")
    assert "The Night Owls" in result.text
    save.store.close()


def test_dynamic_faction_survives_save_reopen(tmp_path):
    """§24.5's whole point: the minted definition lives in the save DB and
    rehydrates into the registry overlay on open."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=4)
    loop = _loop(save)
    save.store.update_player(col=1000)
    loop.submit("found The Night Owls")
    save.store.close()

    reopened = open_save(tmp_path / "s")
    assert reopened.registry.find("fac.rt_the_night_owls") is not None
    assert reopened.registry.find("fac.rt_the_night_owls").name == "The Night Owls"
    reopened.store.close()


def test_founding_requires_funds_and_freedom(tmp_path):
    save = create_save(tmp_path / "s", CYBERPUNK, seed=4)
    loop = _loop(save)
    save.store.update_player(col=10)
    assert "seed money" in loop.submit("found Broke Boys").text

    save.store.update_player(col=2000)
    loop.submit("found The Night Owls")
    result = loop.submit("found Second Banner")
    assert "Leave first" in result.text
    save.store.close()


def test_faction_lifecycle_with_treasury_conservation(tmp_path):
    """§27 test_faction_lifecycle (the implemented slice): found →
    donate → leader walks away → leaderless countdown → dissolution,
    with treasury conservation — every col paid in comes back out at
    dissolution."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=4)
    loop = _loop(save)
    save.store.update_player(col=2000)

    loop.submit("found The Night Owls")                 # -500 (into treasury)
    loop.submit("donate 300")                           # -300 (into treasury)
    col_before_dissolution = save.store.get_player()["col"]
    assert col_before_dissolution == 1200

    loop.submit("leave")
    loop.advance_days(1)
    headlines = [e["headline"] for e in save.store.get_chronicle(200)]
    assert any("stands leaderless" in h for h in headlines)

    loop.advance_days(8)                                # past the 7-day countdown
    headlines = [e["headline"] for e in save.store.get_chronicle(300)]
    assert any("dissolves" in h for h in headlines)
    assert save.registry.find("fac.rt_the_night_owls") is None   # overlay cleared
    # conservation: 500 + 300 refunded to the last leader
    assert save.store.get_player()["col"] == col_before_dissolution + 800
    save.store.close()


# ---------------------------------------------------------- §25 info layer


def _park_socializer(save, loop, npc_id, location="loc.cp_afterlife"):
    runtime = save.store.get_entity(npc_id)
    state = dict(runtime["state"]) if runtime else {}
    state["activity"] = "socialize"
    save.store.upsert_entity(npc_id, "npc", npc_id, state, location, 0)


def test_eavesdrop_sources_real_memory(tmp_path):
    """§27 test_eavesdrop_sources_real_memory: every overheard line
    derives from an existing npc_memory row — there is no fabrication
    channel."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=4)
    loop = _loop(save)
    _park_socializer(save, loop, "npc.cp_rogue")
    save.store.add_npc_memory("npc.cp_rogue", 0, 20, "conversation",
                              "Talked with Nix about who runs Kabuki.",
                              subject_id="npc.cp_runner", valence=0.1)
    save.store.add_npc_memory("npc.cp_rogue", 0, 21, "conversation",
                              "Talked with Judy about the news: a shipment "
                              "went missing.", subject_id="npc.cp_judy",
                              valence=0.1)
    save.store.update_player(level=5)
    save.store.upsert_player_skill("skill.hiding", 400.0)   # near-certain check

    heard = None
    for _ in range(6):
        result = loop.submit("listen rogue")
        if "catch fragments" in result.text:
            heard = result.text
            break
    assert heard is not None
    real_rows = [r["summary"] for r in save.store.conn.execute(
        "SELECT summary FROM npc_memory WHERE npc_id='npc.cp_rogue'")]
    for line in heard.splitlines():
        if line.strip().startswith('"...'):
            content = line.strip()[4:].rstrip('"')
            assert any(content in row for row in real_rows), content
    save.store.close()


def test_eavesdrop_failure_alerts_the_target(tmp_path):
    save = create_save(tmp_path / "s", CYBERPUNK, seed=4)
    loop = _loop(save)
    _park_socializer(save, loop, "npc.cp_rogue")
    # no hiding skill: 40% failure — hammer until caught
    caught = False
    for _ in range(30):
        result = loop.submit("listen rogue")
        if "catches you leaning in" in result.text:
            caught = True
            break
    assert caught
    assert save.store.get_reputation("npc.cp_rogue") < 0
    save.store.close()


def test_buy_info_debits_player_credits_npc(tmp_path):
    """§25.2: paid intel moves real balances and journals the fact."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=4)
    loop = _loop(save)
    save.store.update_player(col=600)

    result = loop.submit("buy info from rogue")
    assert "leans close" in result.text
    assert "Tyger Claws collect" in result.text          # the authored fact
    assert save.store.get_player()["col"] == 100
    rogue = save.store.get_entity("npc.cp_rogue")["state"]
    assert rogue.get("col", 0) >= 500                    # credited for real
    journal = save.store.conn.execute(
        "SELECT * FROM player_history WHERE kind='intel'").fetchall()
    assert journal
    save.store.close()


def test_buy_info_requires_funds(tmp_path):
    save = create_save(tmp_path / "s", CYBERPUNK, seed=4)
    loop = _loop(save)
    save.store.update_player(col=5)
    result = loop.submit("buy info from rogue")
    assert "wants 500" in result.text
    assert save.store.get_player()["col"] == 5
    save.store.close()


def test_bar_gossip_surfaces_chronicle(tmp_path):
    """§25.4: hanging around socializing NPCs leaks the chronicle —
    rate-limited, low-chance, sourced from real public entries."""
    save = create_save(tmp_path / "s", CYBERPUNK, seed=4)
    loop = _loop(save)
    loop.advance_days(3)                                 # chronicle has content
    _park_socializer(save, loop, "npc.cp_rogue")

    overheard = []
    for _ in range(40):
        result = loop.submit("wait 10")
        _park_socializer(save, loop, "npc.cp_rogue")     # keep her talking
        for line in result.text.splitlines():
            if line.startswith("You overhear:"):
                overheard.append(line)
    assert overheard                                      # the channel works
    headlines = [e["headline"] for e in save.store.get_chronicle(50)]
    for line in overheard:
        quoted = line.split('"')[1]
        assert quoted in headlines                        # real entries only
    # rate limit: never more than gossip.max_per_day per day
    assert len(overheard) <= 2 * (1 + 40 * 10 // (24 * 60) + 1)
    save.store.close()
