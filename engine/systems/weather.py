"""Weather & seasons (§6).

Day-tick: each floor rolls its weather from seasonal patterns in rules.yaml
(`weather.patterns.<season>`), falling back to engine defaults. Season
turnover enters the chronicle. Storms are chronicled regionally.

Weather state lives per-floor in the entities table (id = the floor id
suffixed view is unnecessary — state rides on a dedicated weather entity).
"""

from __future__ import annotations

from engine.persistence.store import Delta
from engine.systems import SystemContext

_DEFAULT_PATTERN = {"clear": 0.6, "rain": 0.25, "storm": 0.1, "fog": 0.05}


def current_season(ctx: SystemContext, day: int) -> str:
    calendar = ctx.registry.manifest.calendar
    if not calendar.seasons:
        return "none"
    index = (day // calendar.days_per_season) % len(calendar.seasons)
    return calendar.seasons[index]


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    if granularity != "day":
        return []
    rng = ctx.rng.stream("weather")
    season = current_season(ctx, day)
    pattern = ctx.registry.rule(f"weather.patterns.{season}", _DEFAULT_PATTERN)
    deltas: list[Delta] = []

    calendar = ctx.registry.manifest.calendar
    if day > 0 and calendar.seasons and day % calendar.days_per_season == 0:
        deltas.append(Delta(kind="chronicle", payload={
            "category": "discovery",
            "headline": f"{season.capitalize()} settles over {ctx.registry.manifest.name}.",
        }))

    conditions = list(pattern.keys())
    cumulative_weights = list(pattern.values())
    for floor in sorted(ctx.registry.by_kind("floor"), key=lambda f: f.id):
        condition = rng.choices(conditions, weights=cumulative_weights, k=1)[0]
        deltas.append(Delta(
            kind="entity_state",
            payload={"id": f"weather.{floor.id}", "kind": "weather", "def_id": floor.id,
                     "state": {"condition": condition, "season": season}},
            location_id=floor.id,
        ))
        if condition == "storm":
            deltas.append(Delta(
                kind="chronicle",
                payload={
                    "category": "disaster",
                    "headline": f"A storm lashes {floor.name}.",
                    "visibility": "regional",
                },
                location_id=floor.id,
            ))
    return deltas
