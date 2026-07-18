"""M5: Floor 1 content depth and the LLM narrator's headless guarantee."""

from engine.core.loop import SimulationLoop
from engine.core.registry import load_world
from engine.narrator.base import PerceptionContext
from engine.narrator.llm_narrator import LlmNarrator
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save
from engine.persistence.store import Delta


def test_floor_one_has_fifteen_named_npcs(aincrad_path):
    registry = load_world(aincrad_path)
    named = [n for n in registry.by_kind("npc")]
    assert len(named) == 15
    # every resident has a life: a schedule or an active goal
    for npc in named:
        assert npc.schedule or npc.goals, f"{npc.id} has nothing to do"


def test_llm_narrator_degrades_to_plain(aincrad_path):
    """The engine must run headless: with the LLM path unavailable, rendering
    is byte-identical to the deterministic plain narrator."""
    narrator = LlmNarrator()
    narrator._disabled = True  # simulate missing package/credentials/API failure
    deltas = [Delta(kind="chronicle", payload={"headline": "A storm lashes Floor 1."})]
    perception = PerceptionContext(clock_label="Day 3, 09:00",
                                   location_snapshot={"name": "Horunka Village"})
    assert narrator.render(deltas, perception) == PlainNarrator().render(deltas, perception)


def test_llm_narrator_context_is_perception_only(aincrad_path):
    """The prompt payload contains only perceived data — no store access,
    no hidden state, nothing the player couldn't legitimately know."""
    narrator = LlmNarrator.__new__(LlmNarrator)  # skip client construction
    perception = PerceptionContext(
        clock_label="Day 1, 13:00",
        location_snapshot={"name": "Town of Beginnings"},
        present_npcs=[{"id": "npc.argo", "name": "Argo the Rat"}],
    )
    deltas = [Delta(kind="player_history",
                    payload={"kind": "travel", "summary": "Arrived."})]
    context = narrator._build_context(deltas, perception)
    assert "Argo the Rat" in context
    assert "Town of Beginnings" in context
    assert "goal" not in context.lower()   # NPC internals never enter the prompt


def test_a_month_on_floor_one(tmp_path, aincrad_path):
    """v0.1 definition of done: a month passes on Floor 1 and the world
    has visibly lived — quests ran their course, NPCs moved and remembered."""
    save = create_save(tmp_path / "s1", aincrad_path, seed=42)
    loop = SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())
    loop.advance_days(30)

    quests = {q["def_id"]: q["state"] for q in save.store.get_quests()}
    assert "quest.secret_medicine_of_the_forest" in quests
    assert "quest.wolves_at_the_treeline" in quests
    assert quests["quest.secret_medicine_of_the_forest"] in ("expired", "npc_resolved")

    chronicle = save.store.get_chronicle(200)
    assert len(chronicle) >= 5

    # Kell runs his freight route: at day 30, 13:00 he's mid-route in the city
    kell = save.store.get_entity("npc.kell_the_wagoneer")
    assert kell["location_id"] == "loc.town_of_beginnings"
    # and the inn crowd has talked enough to remember each other
    memories = save.store.conn.execute(
        "SELECT COUNT(*) AS n FROM npc_memory").fetchone()["n"]
    assert memories > 20
    save.store.close()
