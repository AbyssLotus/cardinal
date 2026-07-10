"""Cardinal CLI (§16).

    cardinal new <save> --world <path|name> [--seed N] [--name PLAYER]
    cardinal play <save> [--narrator llm|plain]
    cardinal tick <save> --days N
    cardinal inspect <save> <query>
    cardinal validate <world_path>
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from engine.core.loop import SimulationLoop
from engine.core.registry import WorldLoadError, load_world
from engine.narrator.llm_narrator import LlmNarrator
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import SaveError, create_save, open_save

REPO_ROOT = Path(__file__).resolve().parent.parent
SAVES_DIR = REPO_ROOT / "saves"


def _save_path(name: str) -> Path:
    path = Path(name)
    return path if path.is_absolute() or path.exists() else SAVES_DIR / name


def _world_path(name: str) -> Path:
    path = Path(name)
    return path if path.exists() else REPO_ROOT / "worlds" / name


def cmd_new(args: argparse.Namespace) -> int:
    try:
        save = create_save(_save_path(args.save), _world_path(args.world),
                           seed=args.seed, player_name=args.name)
    except (SaveError, WorldLoadError) as e:
        print(e, file=sys.stderr)
        return 1
    print(f"Created save {save.path} — world {save.meta['world_id']} "
          f"v{save.meta['world_version']}, seed {save.meta['seed']}.")
    save.store.close()
    return 0


def cmd_play(args: argparse.Namespace) -> int:
    try:
        save = open_save(_save_path(args.save))
    except (SaveError, WorldLoadError) as e:
        print(e, file=sys.stderr)
        return 1
    narrator = LlmNarrator() if args.narrator == "llm" else PlainNarrator()
    loop = SimulationLoop(save.registry, save.store, save.rng, narrator)

    print(f"— {save.registry.manifest.name} — {loop.clock.label()} —")
    print("(commands: look, status, wait <min>, go <place>, quit)")
    result = loop.submit("look")
    print(result.text)

    while True:
        try:
            text = input("> ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            break
        if not text:
            continue
        if text.lower() in ("quit", "exit", "q"):
            break
        result = loop.submit(text)
        print(result.text)
        player = save.store.get_player()
        if player and not player["alive"]:
            print("Your story ends here. The world continues without you.")
            break

    save.store.close()
    return 0


def cmd_tick(args: argparse.Namespace) -> int:
    try:
        save = open_save(_save_path(args.save))
    except (SaveError, WorldLoadError) as e:
        print(e, file=sys.stderr)
        return 1
    loop = SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())
    result = loop.advance_days(args.days)
    print(result.text)
    print(f"({len(result.deltas)} state delta(s) committed)")
    save.store.close()
    return 0


def cmd_inspect(args: argparse.Namespace) -> int:
    try:
        save = open_save(_save_path(args.save))
    except (SaveError, WorldLoadError) as e:
        print(e, file=sys.stderr)
        return 1
    store, query = save.store, args.query

    if query == "clock":
        day, minute = store.get_clock()
        print(f"Day {day}, {minute // 60:02d}:{minute % 60:02d}")
    elif query == "player":
        print(store.get_player())
        for item in store.get_inventory("player"):
            print(f"  {item['def_id']} x{item['qty']}"
                  + (f" (durability {item['durability']})" if item["durability"] is not None else ""))
        for mod in store.get_modifiers("player"):
            expiry = f", expires day {mod['expires_day']}" if mod["expires_day"] is not None else ""
            print(f"  [mod] {mod['def_id']} (since day {mod['acquired_day']}{expiry})")
    elif query == "chronicle":
        for entry in reversed(store.get_chronicle()):
            print(f"[Day {entry['day']} {entry['hour']:02d}:00] "
                  f"({entry['category']}) {entry['headline']}")
    elif query == "quests":
        for row in store.get_quests():
            print(f"{row['instance_id']}: {row['state']} "
                  f"(available day {row['available_day']}, expires day {row['expires_day']})")
    elif query == "npcs":
        for row in store.conn.execute(
                "SELECT * FROM entities WHERE kind='npc' ORDER BY id"):
            print(f"{row['id']} @ {row['location_id']} — {row['state_json']}")
    elif query.startswith("market"):
        for row in store.conn.execute("SELECT * FROM markets ORDER BY market_id, item_def"):
            print(dict(row))
    else:
        entity = store.get_entity(query)
        definition = save.registry.find(query)
        if entity is None and definition is None:
            print(f"nothing known about {query!r}", file=sys.stderr)
            store.close()
            return 1
        if definition is not None:
            print("definition:", definition.model_dump(exclude_none=True))
        if entity is not None:
            print("runtime:", entity)
    store.close()
    return 0


def cmd_validate(args: argparse.Namespace) -> int:
    try:
        registry = load_world(args.world)
    except WorldLoadError as e:
        print(e, file=sys.stderr)
        return 1
    counts: dict[str, int] = {}
    for entity_id in registry.entities:
        prefix = entity_id.split(".", 1)[0]
        counts[prefix] = counts.get(prefix, 0) + 1
    summary = ", ".join(f"{count} {prefix}" for prefix, count in sorted(counts.items()))
    print(f"OK: {registry.manifest.name} v{registry.manifest.version} — {summary}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="cardinal",
                                     description="Persistent world simulation engine")
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("new", help="create a new save")
    p.add_argument("save")
    p.add_argument("--world", required=True)
    p.add_argument("--seed", type=int, default=0)
    p.add_argument("--name", default="Player")
    p.set_defaults(func=cmd_new)

    p = sub.add_parser("play", help="play a save (REPL)")
    p.add_argument("save")
    p.add_argument("--narrator", choices=["llm", "plain"], default="plain")
    p.set_defaults(func=cmd_play)

    p = sub.add_parser("tick", help="advance the world with no player")
    p.add_argument("save")
    p.add_argument("--days", type=int, required=True)
    p.set_defaults(func=cmd_tick)

    p = sub.add_parser("inspect", help="debug: entity state, market, chronicle")
    p.add_argument("save")
    p.add_argument("query")
    p.set_defaults(func=cmd_inspect)

    p = sub.add_parser("validate", help="schema + reference + canon checks")
    p.add_argument("world")
    p.set_defaults(func=cmd_validate)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
