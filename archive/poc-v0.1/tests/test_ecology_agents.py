"""§20.3 ecology fixes (logistic regrowth, recolonization, scarcity
chronicles) and the survivor mechanic for named agents."""

from pathlib import Path

from engine.core.loop import SimulationLoop
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save
from engine.systems import agents

CYBERPUNK = Path(__file__).parent / "fixtures" / "probe_cyberpunk"
ZONE = "zone.cp_northside_ruins"
SPECIES = "mon.cp_scav"
CAPACITY = 90


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


def _fresh(tmp_path, name="s"):
    save = create_save(tmp_path / name, CYBERPUNK, seed=5)
    return save, _loop(save)


def _set_population(save, value):
    save.store.upsert_entity(ZONE, "zone", "floor.cp_watson",
                             {"populations": {SPECIES: value,
                                              }}, "floor.cp_watson", 0)


def _population(save):
    return save.store.get_entity(ZONE)["state"]["populations"][SPECIES]


def _days_to_recover(tmp_path, name, start, target_ratio=0.9):
    save, loop = _fresh(tmp_path, name)
    # park the hunter so regrowth is measured clean: kill is fine too, but
    # simplest is to point the measurement at the zone Scorpion doesn't hunt
    _set_population(save, start)
    days = 0
    while _population(save) < CAPACITY * target_ratio and days < 2000:
        loop.advance_days(1)
        days += 1
        # undo Scorpion's kills between measurements so we measure pure regrowth
        pop = _population(save)
        hunted = save.store.get_entity(ZONE)["state"]["populations"]
        # (no correction needed if pop only grew)
    save.store.close()
    return days


def test_logistic_regrowth_recovery_scales_with_depth(tmp_path):
    """Spec §27 `test_logistic_regrowth`: recovery from 5% capacity must
    take strictly longer than recovery from 50% — near-extinction now
    costs the world something. Measured on the un-hunted alleys zone to
    isolate regrowth from agent hunting pressure."""
    def recover(name, start_ratio):
        save, loop = _fresh(tmp_path, name)
        save.store.upsert_entity(
            "zone.cp_alleys", "zone", "floor.cp_watson",
            {"populations": {SPECIES: 60 * start_ratio}}, "floor.cp_watson", 0)
        days = 0
        while days < 1000:
            loop.advance_days(1)
            days += 1
            pop = save.store.get_entity("zone.cp_alleys")["state"]["populations"][SPECIES]
            if pop >= 60 * 0.9:
                break
        save.store.close()
        return days

    from_deep = recover("deep", 0.05)
    from_half = recover("half", 0.50)
    assert from_deep > from_half, (from_deep, from_half)


def test_extinction_recolonizes_after_grace_period(tmp_path):
    """A zeroed species stays gone for ecology.recolonize_days, then
    reseeds at 2% of capacity with a chronicle entry."""
    save, loop = _fresh(tmp_path)
    save.store.upsert_entity("zone.cp_alleys", "zone", "floor.cp_watson",
                             {"populations": {SPECIES: 0}}, "floor.cp_watson", 0)
    loop.advance_days(30)
    pop = save.store.get_entity("zone.cp_alleys")["state"]["populations"][SPECIES]
    assert pop == 0                                   # still inside the grace period

    loop.advance_days(70)                             # past the 90-day default
    pop = save.store.get_entity("zone.cp_alleys")["state"]["populations"][SPECIES]
    assert pop > 0
    headlines = [e["headline"] for e in save.store.get_chronicle(3000)]
    assert any("seen again" in h for h in headlines)
    save.store.close()


def test_scarcity_and_recovery_chronicle_for_territorial_species(tmp_path):
    """The Test 1 gap closed: a non-predatory species hunted to the brink
    now reaches the story layer, and so does its recovery."""
    save, loop = _fresh(tmp_path)
    save.store.upsert_entity("zone.cp_alleys", "zone", "floor.cp_watson",
                             {"populations": {SPECIES: 4}}, "floor.cp_watson", 0)
    loop.advance_days(1)
    headlines = [e["headline"] for e in save.store.get_chronicle(3000)]
    assert any("grow scarce" in h for h in headlines)

    loop.advance_days(60)                             # logistic climb past 50%
    headlines = [e["headline"] for e in save.store.get_chronicle(3000)]
    assert any("thriving again" in h for h in headlines)
    save.store.close()


# ------------------------------------------------------------- survivors


class _RiggedRng:
    """Forces the named agent to lose the fight and take lethal damage."""

    def __init__(self, lose_first_roll):
        self._first = lose_first_roll

    def random(self):
        return self._first          # engagement/win rolls

    def randint(self, a, b):
        return b                    # max damage

    def randrange(self, n):
        return 7


def test_survivor_agents_escape_death_in_background_combat(tmp_path):
    """Rita Wheeler is marked survivor: a lethal EV fight robs and wounds
    her, but she escapes at 1 hp, retreats home, and sits out recovering.
    Named characters don't die off-screen."""
    save, loop = _fresh(tmp_path)
    loop.advance_days(1)                              # seed agent states
    rita = save.registry.find("npc.cp_rita")
    ryo = save.registry.find("npc.cp_ryo")
    rita_state = agents.agent_state(loop.ctx, rita)
    rita_state["hp"] = 5
    rita_state["col"] = 100
    agents._save(loop.ctx, rita, rita_state, "loc.cp_kabuki_market", 1)

    # rig the contest: Ryo (npc) beats Rita (rival) — random() = 0.0 < win_chance
    deltas = agents._agent_fight(loop.ctx, ryo, agents.agent_state(loop.ctx, ryo),
                                 rita, "loc.cp_kabuki_market", 1, _RiggedRng(0.0))
    after = save.store.get_entity("npc.cp_rita")["state"]
    assert after["alive"] is True                     # survived the "kill"
    assert after["hp"] == 1
    assert after["col"] == 0                          # still robbed blind
    assert after["recovering_until_day"] >= 1
    assert save.store.get_entity("npc.cp_rita")["location_id"] == "loc.cp_lizzies_bar"
    assert any("barely escapes" in d.payload.get("headline", "")
               for d in deltas if d.kind == "chronicle")
    save.store.close()


def test_mortal_agents_still_die(tmp_path):
    """The survivor flag is an exception, not the rule — an unmarked agent
    in the same rigged fight dies for real."""
    save, loop = _fresh(tmp_path)
    loop.advance_days(1)
    ryo = save.registry.find("npc.cp_ryo")            # no survivor flag
    rita = save.registry.find("npc.cp_rita")
    ryo_state = agents.agent_state(loop.ctx, ryo)
    ryo_state["hp"] = 5
    agents._save(loop.ctx, ryo, ryo_state, "loc.cp_kabuki_market", 1)

    # rig: Rita (npc) beats Ryo (rival)
    deltas = agents._agent_fight(loop.ctx, rita, agents.agent_state(loop.ctx, rita),
                                 ryo, "loc.cp_kabuki_market", 1, _RiggedRng(0.0))
    after = save.store.get_entity("npc.cp_ryo")["state"]
    assert after["alive"] is False
    assert any("was killed by" in d.payload.get("headline", "")
               for d in deltas if d.kind == "chronicle")
    save.store.close()


def test_survivor_holds_over_a_full_year(tmp_path):
    """The lore guarantee, end to end: 365 simulated days of Watson turf
    war and Rita Wheeler is still standing."""
    save = create_save(tmp_path / "year", CYBERPUNK, seed=2077)
    loop = _loop(save)
    loop.advance_days(365)
    rita = save.store.get_entity("npc.cp_rita")["state"]
    assert rita.get("alive", True) is True
    save.store.close()
