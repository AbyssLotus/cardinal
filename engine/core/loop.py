"""The 8-step simulation loop (§1.2) — a literal, inspectable pipeline.

INTERPRET → VALIDATE → COST → ADVANCE → TICK → RESOLVE → COMMIT → NARRATE

Steps 1–7 are deterministic given a seed. Step 8 is generative but read-only:
narration never creates, modifies, or invents world state.
"""

from __future__ import annotations

import json
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
from engine.systems import combat, interact, skills as skills_system
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


class _TurnAbort(Exception):
    """Raised inside the turn transaction to roll the whole turn back."""


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

        # Combat routing: an active encounter narrows the verb set and runs
        # on 1-second rounds instead of the minute-cost pipeline.
        encounter = combat.get_encounter(self.ctx)
        action = actions[0]
        if encounter is not None:
            if action.intent in ("attack", "guard", "flee"):
                return self._combat_turn(action, encounter)
            if action.intent not in ("look", "status", "skills"):
                return TurnResult("Not while something is trying to kill you — "
                                  "attack, guard, or flee.", [], 0, ok=False)
        elif action.intent == "attack":
            argument = action.parameters.get("argument")
            if not argument:
                return TurnResult("Attack what? (hunt <creature> to seek one out)",
                                  [], 0, ok=False)
            species = parser._resolve_monster(argument, self.registry)
            if species is None:
                return TurnResult(f"You know of no creature called {argument!r}.",
                                  [], 0, ok=False)
            actions = [Action("hunt", target=species, raw_input=text)]
        elif action.intent in ("guard", "flee"):
            return TurnResult("Nothing here is attacking you.", [], 0, ok=False)

        self._notes: list[str] = []
        all_deltas: list[Delta] = []
        total_minutes = 0
        include_status = False

        # Steps 2-7 run inside ONE transaction: ticks apply incrementally so
        # each boundary sees the world the previous one produced, and a failed
        # turn rolls back whole — at most the un-narrated turn is lost (§14).
        try:
            with self.store.transaction():
                for action in actions:
                    # 2. VALIDATE
                    try:
                        validator.validate(action, player, self.registry)
                    except validator.ValidationFailure as e:
                        raise _TurnAbort(str(e))

                    # 3. COST
                    minutes = self._cost(action, player)

                    # 4. ADVANCE
                    boundaries = self.clock.advance(minutes)
                    total_minutes += minutes

                    # 5. TICK — all regions, not just the player's
                    all_deltas += self._tick_and_apply(boundaries)

                    # 6. RESOLVE — against the updated world state
                    resolved = self._resolve(action, player)
                    self.store.apply_deltas(resolved, self.clock.day, self.clock.hour)
                    all_deltas += resolved
                    if action.intent == "status":
                        include_status = True
                    player = self.store.get_player() or player

                # 7. COMMIT — transaction closes atomically here
                self.store.set_clock(self.clock.day, self.clock.minute)
                self.store.save_rng(self.rng.dump_states())
        except _TurnAbort as abort:
            self._reload_clock()  # in-memory clock may have advanced; DB is truth
            return TurnResult(str(abort), [], 0, ok=False)

        # 8. NARRATE — committed deltas only, filtered by perception
        committed_player = self.store.get_player()
        location_id = committed_player["location_id"] if committed_player else ""
        visible = perception.filter_deltas(all_deltas, location_id)
        context = perception.build_context(
            self.store, self.registry, self.clock.label(), include_status=include_status
        )
        text_out = self.narrator.render(visible, context)
        if self._notes:
            text_out = text_out + "\n" + "\n".join(self._notes)
        return TurnResult(text_out, all_deltas, total_minutes)

    def _combat_turn(self, action: Action, state: dict) -> TurnResult:
        argument = action.parameters.get("argument") or action.parameters.get("stance")
        minutes = 0
        with self.store.transaction():
            result = combat.resolve_round(self.ctx, state, action.intent, argument)
            self.store.apply_deltas(result.deltas, self.clock.day, self.clock.hour)
            if result.done:
                # the fight's accumulated seconds hit the world clock at once
                minutes = combat.minutes_elapsed(state["seconds"])
                boundaries = self.clock.advance(minutes)
                self._tick_and_apply(boundaries)
                self.store.set_clock(self.clock.day, self.clock.minute)
            self.store.save_rng(self.rng.dump_states())
        return TurnResult("\n".join(result.events), result.deltas, minutes)

    def advance_days(self, days: int) -> TurnResult:
        """Headless world advance (`cardinal tick`) — the acceptance test for
        'the world exists independently of the player'."""
        minutes = days * self.clock.minutes_per_day
        boundaries = self.clock.advance(minutes)
        with self.store.transaction():
            deltas = self._tick_and_apply(boundaries)
            self.store.set_clock(self.clock.day, self.clock.minute)
            self.store.save_rng(self.rng.dump_states())
        return TurnResult(f"Advanced {days} day(s) to {self.clock.label()}.", deltas, minutes)

    # ------------------------------------------------------------------ steps

    def _cost(self, action: Action, player: dict) -> int:
        rule = self.registry.rule
        if action.intent == "wait":
            return int(action.parameters["minutes"])
        if action.intent == "travel":
            distance = self._floor_diameter(player) / 4
            vehicle = self._mounted_vehicle()
            if vehicle is not None:
                speed = self._vehicle_speed(vehicle, "road")
                if speed > 0:
                    return max(1, int(distance / speed * 60))
            per_km = rule("time_costs.travel_per_km_road", 12)
            # nominal distance: a quarter of the floor's diameter — real zone
            # geometry is an M6 refinement
            return max(1, int(per_km * distance))
        if action.intent == "hunt":
            per_km = rule("time_costs.travel_per_km_wilderness", 25)
            return max(1, int(per_km * self._floor_diameter(player) / 8))
        if action.intent == "interact":
            found = interact.find_device(self.ctx, action.parameters["name"],
                                         player["location_id"])
            if found is not None:
                interaction = interact.find_interaction(found, action.parameters["verb"])
                if interaction is not None:
                    return interaction.time_minutes
        if action.intent in ("mount", "dismount"):
            return 1
        return 0  # look / status / skills / equip are near-instant

    def _floor_diameter(self, player: dict) -> float:
        current = self.registry.find(player["location_id"])
        floor = self.registry.find(getattr(current, "floor", "")) if current else None
        return getattr(floor, "diameter_km", 8.0)

    def _tick_and_apply(self, boundaries: list[TickBoundary]) -> list[Delta]:
        """Run every system at every crossed boundary, applying each system's
        deltas before the next runs — the living world sees itself move.
        Caller must hold the turn transaction. Day boundaries also fire an
        hour-0 tick so hourly systems never skip midnight."""
        all_deltas: list[Delta] = []
        regen = self.registry.rule("combat.hp_regen_per_hour", 10)
        for boundary in boundaries:
            granularities = ["hour", "day"] if boundary.granularity == "day" else ["hour"]
            for granularity in granularities:
                for name in TICK_ORDER:
                    deltas = _SYSTEM_MODULES[name].tick(
                        self.ctx, granularity, boundary.day, boundary.hour
                    )
                    if deltas:
                        self.store.apply_deltas(deltas, boundary.day, boundary.hour)
                        all_deltas += deltas
            # passive recovery: hurt players heal outside combat (world rule)
            if regen:
                player = self.store.get_player()
                if player and player["alive"] and 0 < player["hp"] < player["hp_max"]:
                    self.store.update_player(
                        hp=min(player["hp_max"], player["hp"] + regen))
        return all_deltas

    def _reload_clock(self) -> None:
        day, minute = self.store.get_clock()
        self.clock.day, self.clock.minute = day, minute

    def _resolve(self, action: Action, player: dict) -> list[Delta]:
        if action.intent == "travel":
            destination = self.registry.get(action.target)
            vehicle = self._mounted_vehicle()
            how = ""
            if vehicle is not None:
                self._ride(vehicle, action.target, player)
                vehicle_def = self.registry.find(vehicle["def_id"])
                how = f" aboard the {getattr(vehicle_def, 'name', 'vehicle')}"
            return [
                Delta(kind="player_update", payload={"location_id": action.target}),
                Delta(
                    kind="player_history",
                    payload={
                        "kind": "travel",
                        "summary": f"Traveled to {getattr(destination, 'name', action.target)}{how}.",
                        "refs": [action.target],
                    },
                ),
            ]
        if action.intent == "interact":
            return self._resolve_interact(action, player)
        if action.intent == "mount":
            self._resolve_mount(action.parameters["name"], player)
            return []
        if action.intent == "dismount":
            self._resolve_dismount()
            return []
        if action.intent == "hunt":
            return self._resolve_hunt(action, player)
        if action.intent == "equip":
            self._resolve_equip(action.parameters["name"])
            return []
        if action.intent == "skills":
            self._notes += self._skills_report()
            return []
        return []  # wait / look / status change no state

    def _resolve_hunt(self, action: Action, player: dict) -> list[Delta]:
        species = action.target
        monster = self.registry.get(species)
        current = self.registry.find(player["location_id"])
        floor = self.registry.find(getattr(current, "floor", "")) if current else None
        zone_id = None
        if floor is not None:
            for zone in floor.zones:
                if any(pop.species == species for pop in zone.monster_populations):
                    runtime = self.store.get_entity(zone.id)
                    population = ((runtime["state"].get("populations", {}) if runtime else {})
                                  .get(species))
                    if population is None:
                        population = next(pop.current for pop in zone.monster_populations
                                          if pop.species == species)
                    if population >= 1:
                        zone_id = zone.id
                        break
        if zone_id is None:
            self._notes.append(f"You search the wilds, but find no {monster.name} to hunt.")
            return []
        start, end = monster.behavior.active_hours
        hour = self.clock.hour
        active = (start <= hour < end) if start <= end else (hour >= start or hour < end)
        if not active:
            self._notes.append(f"No {monster.name} stirs at this hour. The wilds are quiet.")
            return []
        _, events = combat.start_encounter(self.ctx, species, zone_id, player["location_id"])
        self._notes += events
        return []

    def _resolve_equip(self, name: str) -> None:
        needle = name.lower()
        for item in self.store.get_inventory("player"):
            definition = self.registry.find(item["def_id"])
            if definition is None:
                continue
            if needle in definition.name.lower() or needle in item["def_id"]:
                prefix = definition.category.split(".")[0]
                for other in self.store.get_equipped("player"):
                    other_def = self.registry.find(other["def_id"])
                    if other_def is not None and other_def.category.startswith(prefix):
                        self.store.set_equipped(other["id"], False)
                self.store.set_equipped(item["id"], True)
                self._notes.append(f"You equip the {definition.name}.")
                return
        self._notes.append(f"You carry nothing called {name!r}.")

    # ------------------------------------------------------------ interactions

    def _resolve_interact(self, action: Action, player: dict) -> list[Delta]:
        device = interact.find_device(self.ctx, action.parameters["name"],
                                      player["location_id"])
        if device is None:
            self._notes.append(f"There's no {action.parameters['name']!r} here.")
            return []
        interaction = interact.find_interaction(device, action.parameters["verb"])
        if interaction is None:
            self._notes.append(f"The {device.name} doesn't answer to "
                               f"{action.parameters['verb']!r}.")
            return []
        deltas, messages = interact.resolve(self.ctx, device, interaction)
        self._notes += messages
        return deltas

    # ------------------------------------------------------------ vehicles

    def _mounted_vehicle(self) -> dict | None:
        for row in self.store.conn.execute(
                "SELECT * FROM entities WHERE kind='vehicle' ORDER BY id"):
            record = dict(row)
            record["state"] = json.loads(record.pop("state_json"))
            if record["state"].get("owner") == "player" and record["state"].get("mounted"):
                return record
        return None

    def _vehicle_speed(self, instance: dict, terrain: str) -> float:
        definition = self.registry.find(instance["def_id"])
        if definition is None:
            return 0.0
        return definition.speed_kmh.get(terrain, 0.0)

    def _ride(self, instance: dict, destination: str, player: dict) -> None:
        """Move the vehicle with the player, burning fuel for machines."""
        definition = self.registry.find(instance["def_id"])
        state = instance["state"]
        if definition is not None and definition.fuel is not None:
            distance = self._floor_diameter(player) / 4
            state["fuel"] = round(max(0.0, state.get(
                "fuel", definition.fuel.tank_capacity) - definition.fuel.per_km * distance), 2)
        self.store.upsert_entity(instance["id"], "vehicle", instance["def_id"],
                                 state, destination, self.clock.day)

    def _resolve_mount(self, name: str, player: dict) -> None:
        needle = name.lower()
        for row in self.store.conn.execute(
                "SELECT * FROM entities WHERE kind='vehicle' ORDER BY id"):
            record = dict(row)
            record["state"] = json.loads(record.pop("state_json"))
            definition = self.registry.find(record["def_id"])
            if definition is None or record["state"].get("owner") != "player":
                continue
            if needle not in definition.name.lower() and needle not in record["def_id"]:
                continue
            if record["location_id"] != player["location_id"]:
                self._notes.append(f"Your {definition.name} isn't here.")
                return
            required = (definition.requirements or {}).get("skill")
            if required and self.store.get_player_skill(required) is None:
                skill = self.registry.find(required)
                self._notes.append(
                    f"You don't know how to operate it ({getattr(skill, 'name', required)}).")
                return
            if definition.fuel is not None and record["state"].get(
                    "fuel", definition.fuel.tank_capacity) <= 0:
                self._notes.append(f"The {definition.name} is out of fuel.")
                return
            record["state"]["mounted"] = True
            self.store.upsert_entity(record["id"], "vehicle", record["def_id"],
                                     record["state"], record["location_id"], self.clock.day)
            self._notes.append(f"You mount the {definition.name}.")
            return
        self._notes.append(f"You own no vehicle called {name!r}.")

    def _resolve_dismount(self) -> None:
        vehicle = self._mounted_vehicle()
        if vehicle is None:
            self._notes.append("You aren't riding anything.")
            return
        vehicle["state"]["mounted"] = False
        self.store.upsert_entity(vehicle["id"], "vehicle", vehicle["def_id"],
                                 vehicle["state"], vehicle["location_id"], self.clock.day)
        definition = self.registry.find(vehicle["def_id"])
        self._notes.append(f"You dismount the {getattr(definition, 'name', 'vehicle')}.")

    def _skills_report(self) -> list[str]:
        lines = []
        for skill_id, proficiency in self.store.get_player_skills().items():
            skill = self.registry.find(skill_id)
            name = getattr(skill, "name", skill_id)
            maximum = getattr(skill, "proficiency_max", 1000)
            lines.append(f"{name}: {proficiency:g}/{maximum}")
        known = skills_system.known_techniques(self.store, self.registry)
        if known:
            lines.append("Techniques: " + ", ".join(sorted(t.name for t in known.values())))
        return lines or ["You have no trained skills."]
