"""Save lifecycle: create/open a save directory.

A save is a directory: state.db (SQLite world state) + meta.yaml pinning
the world package, its version, and the seed. Definitions are never copied
into the DB.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import yaml

from engine.core.registry import Registry, load_world, strip_qty_suffix
from engine.core.rng import RngManager
from engine.memory import world_memory
from engine.persistence.store import Store
from engine.systems import SystemContext
from engine.systems import npc as npc_system


@dataclass
class Save:
    path: Path
    store: Store
    registry: Registry
    rng: RngManager
    meta: dict


class SaveError(Exception):
    pass


def create_save(save_dir: str | Path, world_path: str | Path, seed: int,
                player_name: str = "Player") -> Save:
    save_path = Path(save_dir)
    if (save_path / "state.db").exists():
        raise SaveError(f"save already exists at {save_path}")
    registry = load_world(world_path)
    save_path.mkdir(parents=True, exist_ok=True)

    meta = {
        "world_package": str(Path(world_path).resolve()),
        "world_id": registry.manifest.id,
        "world_version": registry.manifest.version,
        "seed": seed,
        "player_name": player_name,
    }
    with open(save_path / "meta.yaml", "w", encoding="utf-8") as f:
        yaml.safe_dump(meta, f, sort_keys=False)

    store = Store.create(save_path / "state.db")
    rng = RngManager(seed)
    manifest = registry.manifest

    with store.transaction():
        entry = manifest.entry_point
        store.set_clock(entry.time.day, entry.time.hour * 60)

        starting = manifest.starting_player
        store.init_player(player_name, starting.level, starting.col, entry.location)
        for index, ref in enumerate(starting.items):
            def_id, qty = strip_qty_suffix(ref)
            item_def = registry.get(def_id)
            durability = item_def.stats.get("durability_max") if hasattr(item_def, "stats") else None
            store.add_item_instance(f"iteminst.start_{index}", def_id, "player",
                                    durability=durability, qty=qty)

        ctx = SystemContext(registry=registry, store=store, rng=rng, bus=None)
        seed_deltas = npc_system.seed_runtime_state(ctx, entry.time.day)
        seed_deltas += world_memory.seed_from_world(registry)
        store.apply_deltas(seed_deltas, entry.time.day, entry.time.hour)
        store.save_rng(rng.dump_states())

    return Save(save_path, store, registry, rng, meta)


def open_save(save_dir: str | Path) -> Save:
    save_path = Path(save_dir)
    meta_file = save_path / "meta.yaml"
    if not meta_file.exists():
        raise SaveError(f"no save at {save_path} (missing meta.yaml)")
    with open(meta_file, encoding="utf-8") as f:
        meta = yaml.safe_load(f)
    registry = load_world(meta["world_package"])
    if registry.manifest.version != meta["world_version"]:
        raise SaveError(
            f"world package version {registry.manifest.version} != save's pinned "
            f"{meta['world_version']} (migrations arrive in M6)"
        )
    store = Store(save_path / "state.db")
    rng = RngManager(meta["seed"])
    rng.load_states(store.load_rng())
    return Save(save_path, store, registry, rng, meta)
