"""The 8-step simulation loop (§1.2) — a literal, inspectable pipeline.

INTERPRET → VALIDATE → COST → ADVANCE → TICK → RESOLVE → COMMIT → NARRATE

Steps 1–7 are deterministic given a seed. Step 8 is generative but read-only:
narration never creates, modifies, or invents world state.
"""

from __future__ import annotations

from dataclasses import dataclass

from engine.actions import parser, validator
from engine.actions.actions import Action
from engine.core.clock import TickBoundary, WorldClock
from engine.core.events import EventBus
from engine.core.registry import Registry
from engine.core.rng import RngManager
from engine.narrator import perception
from engine.narrator.base import Narrator
from engine.persistence.store import Delta, Store
from engine.systems import TICK_ORDER, SystemContext
from engine.systems import (  # noqa: F401  (imported for the tick table)
    economy, ecology, factions, npc, quests, weather, worldevents,
)

_SYSTEM_MODULES = {
    "weather": weather,
    "ecology": ecology,
    "npc": npc,
    "economy": economy,
    "quests": quests,
    "factions": factions,
    "worldevents": worldevents,
}


@dataclass
class TurnResult:
    text: str
    deltas: list[Delta]
    minutes_elapsed: int
    ok: bool = True


class SimulationLoop:
    def __init__(self, registry: Registry, store: Store, rng: RngManager, narrator: Narrator):
        self.registry = registry
        self.store = store
        self.rng = rng
        self.narrator = narrator
        self.bus = EventBus()
        self.ctx = SystemContext(registry=registry, store=store, rng=rng, bus=self.bus)

        day, minute = store.get_clock()
        hours = registry.manifest.calendar.hours_per_day
        self.clock = WorldClock(day=day, minute=minute, minutes_per_day=hours * 60)

        saved_rng = store.load_rng()
        if saved_rng:
            rng.load_states(saved_rng)

    # ------------------------------------------------------------------ turn

    def submit(self, text: str) -> TurnResult:
        # 1. INTERPRET
        try:
            actions = parser.parse(text, self.registry)
        except parser.ParseError as e:
            return TurnResult(str(e), [], 0, ok=False)

        player = self.store.get_player()
        if player is None:
            return TurnResult("No player exists in this save.", [], 0, ok=False)

        all_deltas: list[Delta] = []
        total_minutes = 0
        include_status = False

        for action in actions:
            # 2. VALIDATE
            try:
                validator.validate(action, player, self.registry)
            except validator.ValidationFailure as e:
                return TurnResult(str(e), [], 0, ok=False)

            # 3. COST
            minutes = self._cost(action, player)

            # 4. ADVANCE
            boundaries = self.clock.advance(minutes)
            total_minutes += minutes

            # 5. TICK — all regions, not just the player's
            deltas = self._tick(boundaries)

            # 6. RESOLVE — against the updated world state
            deltas += self._resolve(action, player)
            if action.intent == "status":
                include_status = True

            all_deltas += deltas
            player = self.store.get_player() or player  # not yet committed; see below

        # 7. COMMIT — atomic
        with self.store.transaction():
            self.store.apply_deltas(all_deltas, self.clock.day, self.clock.hour)
            self.store.set_clock(self.clock.day, self.clock.minute)
            self.store.save_rng(self.rng.dump_states())

        # 8. NARRATE — committed deltas only, filtered by perception
        committed_player = self.store.get_player()
        location_id = committed_player["location_id"] if committed_player else ""
        visible = perception.filter_deltas(all_deltas, location_id)
        context = perception.build_context(
            self.store, self.registry, self.clock.label(), include_status=include_status
        )
        return TurnResult(self.narrator.render(visible, context), all_deltas, total_minutes)

    def advance_days(self, days: int) -> TurnResult:
        """Headless world advance (`cardinal tick`) — the acceptance test for
        'the world exists independently of the player'."""
        minutes = days * self.clock.minutes_per_day
        boundaries = self.clock.advance(minutes)
        deltas = self._tick(boundaries)
        with self.store.transaction():
            self.store.apply_deltas(deltas, self.clock.day, self.clock.hour)
            self.store.set_clock(self.clock.day, self.clock.minute)
            self.store.save_rng(self.rng.dump_states())
        return TurnResult(f"Advanced {days} day(s) to {self.clock.label()}.", deltas, minutes)

    # ------------------------------------------------------------------ steps

    def _cost(self, action: Action, player: dict) -> int:
        rule = self.registry.rule
        if action.intent == "wait":
            return int(action.parameters["minutes"])
        if action.intent == "travel":
            per_km = rule("time_costs.travel_per_km_road", 12)
            current = self.registry.find(player["location_id"])
            floor = self.registry.find(getattr(current, "floor", "")) if current else None
            # M1 nominal distance: a quarter of the floor's diameter. Real zone
            # geometry arrives with the combat/positioning work in M3.
            distance_km = getattr(floor, "diameter_km", 8) / 4 if floor else 2
            return max(1, int(per_km * distance_km))
        return 0  # look / status are perceptual, not world actions

    def _tick(self, boundaries: list[TickBoundary]) -> list[Delta]:
        deltas: list[Delta] = []
        for boundary in boundaries:
            for name in TICK_ORDER:
                deltas += _SYSTEM_MODULES[name].tick(
                    self.ctx, boundary.granularity, boundary.day, boundary.hour
                )
        return deltas

    def _resolve(self, action: Action, player: dict) -> list[Delta]:
        if action.intent == "travel":
            destination = self.registry.get(action.target)
            return [
                Delta(kind="player_update", payload={"location_id": action.target}),
                Delta(
                    kind="player_history",
                    payload={
                        "kind": "travel",
                        "summary": f"Traveled to {getattr(destination, 'name', action.target)}.",
                        "refs": [action.target],
                    },
                ),
            ]
        return []  # wait / look / status change no state
