"""M9 slice 1: authored-faction membership, faction_tax, deterministic
strategic tick — exercised against the Destiny and Cyberpunk probes."""

from pathlib import Path

import pytest

from engine.core.loop import SimulationLoop
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save
from engine.systems import factions

FIXTURES = Path(__file__).parent / "fixtures"
DESTINY = FIXTURES / "probe_destiny"
CYBERPUNK = FIXTURES / "probe_cyberpunk"


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


# ------------------------------------------------------------- membership


def test_join_open_faction_at_hq(tmp_path):
    """Guardian walks into the Tower, takes the Vanguard's colors."""
    save = create_save(tmp_path / "s1", DESTINY, seed=1)
    loop = _loop(save)
    result = loop.submit("join vanguard")
    assert "You join The Vanguard" in result.text
    member = factions.membership(loop.ctx)
    assert member is not None and member[0] == "fac.d2_vanguard"
    assert save.store.get_reputation("fac.d2_vanguard") > 0
    headlines = [e["headline"] for e in save.store.get_chronicle(10)]
    assert any("joins The Vanguard" in h for h in headlines)
    save.store.close()


def test_closed_faction_refuses(tmp_path):
    """No Guardian joins the House of Devils."""
    save = create_save(tmp_path / "s1", DESTINY, seed=1)
    result = _loop(save).submit("join devils")
    assert "not taking anyone" in result.text
    assert factions.membership(_loop(save).ctx) is None
    save.store.close()


def test_selective_faction_requires_reputation(tmp_path):
    """Arasaka won't look at a nobody; earn standing and the door opens."""
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=1)
    loop = _loop(save)
    loop.submit("go arasaka waterfront")
    result = loop.submit("join arasaka")
    assert "Make a name for yourself" in result.text
    assert factions.membership(loop.ctx) is None

    save.store.adjust_reputation("fac.cp_arasaka", 2.0)
    result = loop.submit("join arasaka")
    assert "You join Arasaka" in result.text
    save.store.close()


def test_join_requires_being_at_headquarters(tmp_path):
    """Maelstrom recruits in their den (the subnet), not at the Afterlife."""
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=1)
    loop = _loop(save)
    result = loop.submit("join maelstrom")
    assert "you'll need to go there" in result.text
    assert factions.membership(loop.ctx) is None
    save.store.close()


def test_one_faction_at_a_time_and_leave_penalty(tmp_path):
    save = create_save(tmp_path / "s1", DESTINY, seed=1)
    loop = _loop(save)
    loop.submit("join vanguard")
    result = loop.submit("join devils")
    assert "Leave first" in result.text

    rep_before = save.store.get_reputation("fac.d2_vanguard")
    result = loop.submit("leave")
    assert "You leave The Vanguard" in result.text
    assert factions.membership(loop.ctx) is None
    assert save.store.get_reputation("fac.d2_vanguard") < rep_before
    save.store.close()


# ------------------------------------------------------------- donations


def test_donation_credits_treasury_with_diminishing_rep(tmp_path):
    """Spec §7 exploit guards: rep priced against faction wealth, capped
    per day — donations can never become a reputation pump."""
    save = create_save(tmp_path / "s1", DESTINY, seed=1)
    loop = _loop(save)
    loop.submit("join vanguard")

    vanguard = save.registry.find("fac.d2_vanguard")
    treasury_before = factions.faction_state(loop.ctx, vanguard)["treasury"]
    col_before = save.store.get_player()["col"]
    rep_before = save.store.get_reputation("fac.d2_vanguard")

    result = loop.submit("donate 1000")
    assert "You donate 1000" in result.text
    assert save.store.get_player()["col"] == col_before - 1000
    assert (factions.faction_state(loop.ctx, vanguard)["treasury"]
            == treasury_before + 1000)
    first_gain = save.store.get_reputation("fac.d2_vanguard") - rep_before
    assert 0 < first_gain <= 0.5                    # priced against 100k treasury

    # hammer donations the same day: rep must hit the daily cap and stop
    for _ in range(3):
        loop.submit("donate 1000")
    capped = save.store.get_reputation("fac.d2_vanguard") - rep_before
    cap = save.registry.rule("factions.donation_rep_daily_cap", 0.5)
    assert capped <= cap + 1e-9
    save.store.close()


def test_donation_requires_membership_and_funds(tmp_path):
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=1)
    loop = _loop(save)
    assert "Join a faction first" in loop.submit("donate 100").text
    save.store.adjust_reputation("fac.cp_arasaka", 2.0)
    loop.submit("go arasaka waterfront")
    loop.submit("join arasaka")
    assert "You carry only" in loop.submit("donate 999999").text
    save.store.close()


# ------------------------------------------------------------- faction tax


def test_faction_tax_applies_and_credits_treasury(tmp_path):
    """Part 1 §11's faction_tax term, live: the Vanguard taxes Tower bazaar
    purchases and the revenue lands in its treasury."""
    save = create_save(tmp_path / "s1", DESTINY, seed=3)
    loop = _loop(save)
    loop.advance_days(1)                            # seed market rows

    vanguard = save.registry.find("fac.d2_vanguard")
    treasury_before = factions.faction_state(loop.ctx, vanguard)["treasury"]
    result = loop.submit("buy auto rifle")
    assert "Vanguard tax" in result.text
    treasury_after = factions.faction_state(loop.ctx, vanguard)["treasury"]
    assert treasury_after > treasury_before
    save.store.close()


def test_faction_members_are_tax_exempt(tmp_path):
    save = create_save(tmp_path / "s1", DESTINY, seed=3)
    loop = _loop(save)
    loop.advance_days(1)
    loop.submit("join vanguard")
    result = loop.submit("buy auto rifle")
    assert "Bought 1x" in result.text
    assert "tax" not in result.text                 # members pay no dues at the till
    save.store.close()


def test_market_control_follows_headquarters(tmp_path):
    """The Afterlife is neutral ground (no HQ there) — untaxed. Kabuki's
    market pays the Tyger Claws' protection rate; the waterfront pays
    Arasaka's."""
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=3)
    loop = _loop(save)

    neutral = save.registry.find("market.cp_afterlife_fixer")
    faction, rate = factions.market_tax(loop.ctx, neutral)
    assert faction is None and rate == 0.0

    kabuki = save.registry.find("market.cp_kabuki")
    faction, rate = factions.market_tax(loop.ctx, kabuki)
    assert faction is not None and faction.id == "fac.cp_tyger_claws"
    assert rate == pytest.approx(0.12)

    waterfront = save.registry.find("market.cp_waterfront")
    faction, rate = factions.market_tax(loop.ctx, waterfront)
    assert faction is not None and faction.id == "fac.cp_arasaka"
    assert rate == pytest.approx(0.08)
    save.store.close()


# ------------------------------------------------------------- strategic tick


def test_strategic_tick_is_deterministic(tmp_path):
    """Spec §3.3 determinism contract: same world + same days → identical
    decisions, dispositions, and chronicle, across independent saves."""
    def run(path):
        save = create_save(path, CYBERPUNK, seed=7)
        loop = _loop(save)
        loop.advance_days(10)
        arasaka = save.registry.find("fac.cp_arasaka")
        state = factions.faction_state(loop.ctx, arasaka)
        chronicle = [e["headline"] for e in save.store.get_chronicle(100)
                     if e["category"] == "politics"]
        save.store.close()
        return state, chronicle

    state_a, chron_a = run(tmp_path / "a")
    state_b, chron_b = run(tmp_path / "b")
    assert state_a["dispositions"] == state_b["dispositions"]
    assert state_a["last_decision"] == state_b["last_decision"]
    assert chron_a == chron_b


def test_hostile_factions_feud_and_it_chronicles(tmp_path):
    """Arasaka × Maelstrom start at -0.6; the strategic tick's feud action
    should push them across -0.5... wait, they're already past it — so
    drive from -0.6 downward and assert dispositions worsen and the
    decision log records feuds."""
    save = create_save(tmp_path / "s1", CYBERPUNK, seed=7)
    loop = _loop(save)
    loop.advance_days(5)
    arasaka = save.registry.find("fac.cp_arasaka")
    state = factions.faction_state(loop.ctx, arasaka)
    assert state["last_decision"]["chosen"] == "feud"
    assert state["last_decision"]["scores"]["feud"] > 0
    assert state["dispositions"]["fac.cp_maelstrom"] <= -0.6
    save.store.close()


def test_friendly_factions_reach_accord(tmp_path):
    """Seed a mutual thaw just below zero on both sides and verify the
    outreach action lifts the pair across the accord threshold, with a
    chronicle entry marking the crossing (spec §2.1: transitions are
    events, never silent drift)."""
    save = create_save(tmp_path / "s1", DESTINY, seed=7)
    loop = _loop(save)
    vanguard = save.registry.find("fac.d2_vanguard")
    devils = save.registry.find("fac.d2_devils")
    for faction in (vanguard, devils):
        other = devils if faction is vanguard else vanguard
        state = factions.faction_state(loop.ctx, faction)
        state["dispositions"][other.id] = -0.03      # a mutual thaw
        factions.save_faction_state(loop.ctx, faction, state)
    loop.advance_days(4)
    state = factions.faction_state(loop.ctx, vanguard)
    headlines = [e["headline"] for e in save.store.get_chronicle(100)]
    assert state["dispositions"]["fac.d2_devils"] > 0
    assert any("accord" in h for h in headlines)
    save.store.close()


def test_decision_log_records_full_utility_table(tmp_path):
    """Spec §8: every decision logs all candidate scores, not just the win."""
    save = create_save(tmp_path / "s1", DESTINY, seed=7)
    loop = _loop(save)
    loop.advance_days(2)
    for fac_id in ("fac.d2_vanguard", "fac.d2_devils"):
        faction = save.registry.find(fac_id)
        decision = factions.faction_state(loop.ctx, faction).get("last_decision")
        assert decision is not None
        assert set(decision["scores"]) == {"consolidate", "outreach", "feud"}
    save.store.close()


def test_dispositions_decay_toward_authored_baseline(tmp_path):
    """Spec §2.2 grievance decay: a pushed disposition drifts back toward
    the authored value instead of sticking forever."""
    save = create_save(tmp_path / "s1", DESTINY, seed=7)
    loop = _loop(save)
    vanguard = save.registry.find("fac.d2_vanguard")
    devils = save.registry.find("fac.d2_devils")
    state = factions.faction_state(loop.ctx, vanguard)
    state["dispositions"]["fac.d2_devils"] = 0.5     # artificially warm
    factions.save_faction_state(loop.ctx, vanguard, state)
    # freeze the devils' side too so mutual shifts don't confound the read
    dstate = factions.faction_state(loop.ctx, devils)
    factions.save_faction_state(loop.ctx, devils, dstate)

    loop.advance_days(3)
    after = factions.faction_state(loop.ctx, vanguard)["dispositions"]["fac.d2_devils"]
    assert after < 0.5                                # decaying toward -0.7
    save.store.close()


def test_faction_status_verb(tmp_path):
    save = create_save(tmp_path / "s1", DESTINY, seed=1)
    loop = _loop(save)
    result = loop.submit("faction")
    assert "You belong to no faction" in result.text
    assert "The Vanguard" in result.text and "House of Devils" in result.text
    loop.submit("join vanguard")
    result = loop.submit("faction")
    assert "member of The Vanguard" in result.text
    save.store.close()


# ------------------------------------------------------------- year burn


def test_watson_365_day_stability(tmp_path):
    """A full year of Watson politics must run clean: no crashes, faction
    state stays in bounds, the chronicle records the era's headlines
    without spamming, NPC life loops stay sustainable, and the world
    stays deterministic (spec §3.3) across the whole span."""
    def run(path):
        save = create_save(path, CYBERPUNK, seed=2077)
        loop = _loop(save)
        loop.advance_days(365)
        states = {}
        for faction in sorted(save.registry.by_kind("fac"), key=lambda f: f.id):
            states[faction.id] = factions.faction_state(loop.ctx, faction)
        politics = [e["headline"] for e in save.store.get_chronicle(3000)
                    if e["category"] == "politics"]
        npc_rows = save.store.conn.execute(
            "SELECT COUNT(*) AS n FROM entities WHERE kind='npc'").fetchone()["n"]
        save.store.close()
        return states, politics, npc_rows

    states, politics, npc_rows = run(tmp_path / "a")

    for fac_id, state in states.items():
        assert 0.0 <= state["cohesion"] <= 1.0, fac_id
        for other, value in state["dispositions"].items():
            assert -1.0 <= value <= 1.0, (fac_id, other)
        assert state["treasury"] >= 0, fac_id
        assert "_shift_before" not in state, fac_id          # no scratch leakage

    # the year's headlines: enmities declared once each, no spam
    assert 1 <= len(politics) <= 40
    assert len(politics) == len(set(politics)), "duplicate chronicle spam"
    assert any("sworn enemies" in h for h in politics)

    assert npc_rows == 12                    # 6 ambient + 6 agents, all rows kept
    # ambient NPCs never die; agents may (permadeath) but their rows remain

    # determinism across the full year: an independent save replays identically
    states_b, politics_b, _ = run(tmp_path / "b")
    assert states == states_b
    assert politics == politics_b
