"""Save lifecycle: create/open a save directory.

A save is a directory: state.db (SQLite world state) + meta.yaml pinning
the world package, its version, and the seed. Definitions are never copied
into the DB.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import yaml

import json as _json

from engine.core.registry import Registry, load_world, strip_qty_suffix
from engine.schemas import CATEGORY_MODELS


def _hydrate_dynamic_entities(registry: Registry, store) -> None:
    """§24.5: runtime-minted definitions live in the save DB; rebuild the
    registry overlay from them on every open."""
    for row in store.get_dynamic_entities():
        model = CATEGORY_MODELS.get(row["kind"])
        if model is None:
            continue
        registry.overlay[row["id"]] = model.model_validate(
            _json.loads(row["def_json"]))
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

    from engine.persistence.migrations import CURRENT_SAVE_FORMAT

    meta = {
        "world_package": str(Path(world_path).resolve()),
        "world_id": registry.manifest.id,
        "world_version": registry.manifest.version,
        "seed": seed,
        "player_name": player_name,
        "save_format": CURRENT_SAVE_FORMAT,
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
        first_weapon_equipped = False
        for index, ref in enumerate(starting.items):
            def_id, qty = strip_qty_suffix(ref)
            item_def = registry.get(def_id)
            durability = item_def.stats.get("durability_max") if hasattr(item_def, "stats") else None
            instance_id = f"iteminst.start_{index}"
            store.add_item_instance(instance_id, def_id, "player",
                                    durability=durability, qty=qty)
            category = getattr(item_def, "category", "")
            if category.startswith("armor"):
                store.set_equipped(instance_id, True)
            elif category.startswith("weapon") and not first_weapon_equipped:
                store.set_equipped(instance_id, True)
                first_weapon_equipped = True
        for skill_ref in starting.skills:
            store.upsert_player_skill(skill_ref, 0.0)
        for index, vehicle_ref in enumerate(starting.vehicles):
            vehicle_def = registry.get(vehicle_ref)
            state = {"owner": "player", "mounted": False,
                     "hp": (vehicle_def.stats or {}).get("hp", 100)}
            if vehicle_def.fuel is not None:
                state["fuel"] = vehicle_def.fuel.tank_capacity
            store.upsert_entity(f"vehicleinst.start_{index}", "vehicle", vehicle_ref,
                                state, entry.location, entry.time.day)

        ctx = SystemContext(registry=registry, store=store, rng=rng, bus=None)
        seed_deltas = npc_system.seed_runtime_state(ctx, entry.time.day)
        seed_deltas += world_memory.seed_from_world(registry)
        store.apply_deltas(seed_deltas, entry.time.day, entry.time.hour)

        # authored NPC goals become live goal rows the agent loop advances
        for npc in sorted(registry.by_kind("npc"), key=lambda n: n.id):
            for goal in getattr(npc, "goals", []):
                store.upsert_goal(npc.id, goal.id, {}, goal.status)

        store.save_rng(rng.dump_states())

    _hydrate_dynamic_entities(registry, store)
    return Save(save_path, store, registry, rng, meta)


def open_save(save_dir: str | Path) -> Save:
    from engine.persistence.migrations import CURRENT_SAVE_FORMAT, migrate

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
            f"{meta['world_version']} — content changed under the save; "
            f"bump deliberately by editing meta.yaml if the change is compatible"
        )
    store = Store(save_path / "state.db")

    save_format = meta.get("save_format", 0)
    if save_format != CURRENT_SAVE_FORMAT:
        try:
            meta["save_format"] = migrate(store, save_format)
        except RuntimeError as e:
            store.close()
            raise SaveError(str(e))
        with open(meta_file, "w", encoding="utf-8") as f:
            yaml.safe_dump(meta, f, sort_keys=False)

    rng = RngManager(meta["seed"])
    rng.load_states(store.load_rng())
    _hydrate_dynamic_entities(registry, store)
    return Save(save_path, store, registry, rng, meta)
