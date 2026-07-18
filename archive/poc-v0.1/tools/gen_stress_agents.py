"""Generate N procedural agents into a world package for stress testing.

Usage: python tools/gen_stress_agents.py <world_path> <count> [seed]

Writes npcs/gen_agents.yaml with a corpo/grunt/edgerunner mix wired to the
world's existing locations, zones, factions, and goods (currently tuned to
the probe_cyberpunk fixture's ids). Delete the file to de-stress the world.
"""
import random
import sys
from pathlib import Path

FACTIONS = ["fac.cp_arasaka", "fac.cp_maelstrom", "fac.cp_mox",
            "fac.cp_tyger_claws", None]
LOCS = ["loc.cp_afterlife", "loc.cp_lizzies_bar", "loc.cp_kabuki_market",
        "loc.cp_totentanz", "loc.cp_arasaka_waterfront"]
ZONES = ["zone.cp_alleys", "zone.cp_northside_ruins"]
GOODS = ["item.cp_bullets"]


def main(world: str, count: int, seed: int = 42) -> None:
    rng = random.Random(seed)
    lines = [f"# {count} generated agents: stress-test roster (tools/gen_stress_agents.py)"]

    def emit(i, name, policy, faction, col, level, params, home):
        lines.append(f"- id: npc.gen_{i:03d}")
        lines.append(f'  name: "{name}"')
        lines.append(f"  location: {home}")
        lines.append("  actor_class: agent")
        lines.append(f"  policy: {policy}")
        if faction:
            lines.append(f"  faction: {faction}")
        lines.append(f"  col: {col}")
        lines.append(f"  combat: {{ level: {level}, skills: [] }}")
        lines.append("  policy_params:")
        for key, value in params.items():
            lines.append(f"    {key}: {value}")

    n_merchant = round(count * 0.35)
    n_hunter = round(count * 0.40)
    i = 0
    for _ in range(n_merchant):
        route = rng.sample(LOCS, 2)
        emit(i, f"Corpo {i:03d}", "merchant",
             rng.choice(["fac.cp_arasaka", None, None]),
             rng.randint(1500, 6000), rng.randint(3, 6),
             {"good": rng.choice(GOODS), "route": f"[{route[0]}, {route[1]}]"},
             route[0])
        i += 1
    for _ in range(n_hunter):
        home = rng.choice(LOCS)
        # most working people are unaffiliated; gangs are a minority
        emit(i, f"Grunt {i:03d}", "hunter_gatherer",
             rng.choice(FACTIONS[1:4] + [None] * 5), rng.randint(50, 300),
             rng.randint(3, 7),
             {"species": "mon.cp_scav", "zone": rng.choice(ZONES),
              "home": home, "sell_at": rng.choice(LOCS)}, home)
        i += 1
    while i < count:
        turf = rng.sample(LOCS, rng.randint(2, 3))
        faction = rng.choice(FACTIONS[:4] + [None, None])
        # muscle without a cause is a bandit, not an inert aggressor
        policy = "aggressor" if faction else "bandit"
        emit(i, f"Edgerunner {i:03d}", policy, faction,
             rng.randint(100, 500), rng.randint(4, 9),
             {"turf": f"[{', '.join(turf)}]"}, turf[0])
        i += 1

    out = Path(world) / "npcs" / "gen_agents.yaml"
    out.write_text("\n".join(lines) + "\n")
    print(f"wrote {count} agents to {out}")


if __name__ == "__main__":
    main(sys.argv[1], int(sys.argv[2]), int(sys.argv[3]) if len(sys.argv) > 3 else 42)
