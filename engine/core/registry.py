"""Loads & indexes world package data (§3, §4).

The registry is the ONLY doorway between world content and the engine:
- parses every YAML file under a world package
- validates each document against its Pydantic model
- registers every entity id (including nested `id:` mappings such as
  zones, districts, and inline shop definitions)
- fails loudly on any dangling cross-reference

Engine code never reads world YAML directly.
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any, Iterator

import yaml
from pydantic import ValidationError

from engine.schemas import (
    CATEGORY_MODELS,
    Entity,
    Rules,
    Skill,
    SwordSkill,
    WorldManifest,
)

# id prefixes that participate in cross-reference checking
REF_PREFIXES = {
    "floor", "loc", "zone", "dist", "shop", "npc", "arch", "mon", "item",
    "skill", "swordskill", "tech", "quest", "fac", "market", "res", "goal", "world",
    "mod", "vehicle",
}
_REF_RE = re.compile(r"^([a-z][a-z0-9_]*)\.[a-z0-9_.]+$")

# world-package directory -> registry category (schemas.CATEGORY_MODELS key)
_DIR_CATEGORIES = {
    "floors": "floors",
    "locations": "locations",
    "npcs": "npcs",
    "monsters": "monsters",
    "items": "items",
    "skills": "skills",
    "quests": "quests",
    "factions": "factions",
    "resources": "resources",
    "modifiers": "modifiers",
    "vehicles": "vehicles",
}


class WorldLoadError(Exception):
    def __init__(self, errors: list[str]):
        self.errors = errors
        super().__init__("world package failed to load:\n" + "\n".join(f"  - {e}" for e in errors))


class Registry:
    def __init__(self, root: Path, manifest: WorldManifest, rules: Rules):
        self.root = root
        self.manifest = manifest
        self.rules = rules
        self.entities: dict[str, Entity] = {}
        self.canon: dict[str, Any] = {"confirmed": [], "inferences": []}
        self._nested_ids: set[str] = set()

    # --- lookups ----------------------------------------------------------

    def get(self, entity_id: str) -> Entity:
        return self.entities[entity_id]

    def find(self, entity_id: str) -> Entity | None:
        return self.entities.get(entity_id)

    def by_kind(self, prefix: str) -> Iterator[Entity]:
        wanted = prefix + "."
        return (e for e in self.entities.values() if e.id.startswith(wanted))

    def known_ids(self) -> set[str]:
        return set(self.entities) | self._nested_ids | {self.manifest.id}

    def rule(self, path: str, default: Any = None) -> Any:
        """Dotted-path lookup into rules.yaml, e.g. rule('time_costs.meal', 30)."""
        node: Any = self.rules.model_dump()
        for part in path.split("."):
            if not isinstance(node, dict) or part not in node:
                return default
            node = node[part]
        return node


def load_world(package_path: str | Path) -> Registry:
    root = Path(package_path)
    errors: list[str] = []

    manifest = _load_model(root / "world.yaml", WorldManifest, errors)
    rules = _load_model(root / "rules.yaml", Rules, errors)
    if manifest is None or rules is None:
        raise WorldLoadError(errors)

    registry = Registry(root, manifest, rules)

    for dirname, category in _DIR_CATEGORIES.items():
        for path in sorted((root / dirname).rglob("*.yaml")):
            _load_entity_file(path, category, registry, errors)

    markets_file = root / "economy" / "markets.yaml"
    if markets_file.exists():
        _load_entity_file(markets_file, "markets", registry, errors)

    for tier in ("confirmed", "inferences"):
        canon_file = root / "canon" / f"{tier}.yaml"
        if canon_file.exists():
            registry.canon[tier] = _read_yaml(canon_file, errors) or []

    _register_nested_ids(registry)
    _check_references(registry, errors)

    if errors:
        raise WorldLoadError(errors)
    return registry


# --- internals --------------------------------------------------------------


def _read_yaml(path: Path, errors: list[str]) -> Any:
    try:
        with open(path, encoding="utf-8") as f:
            return yaml.safe_load(f)
    except (OSError, yaml.YAMLError) as e:
        errors.append(f"{path}: {e}")
        return None


def _load_model(path: Path, model: type, errors: list[str]):
    if not path.exists():
        errors.append(f"{path}: required file missing")
        return None
    data = _read_yaml(path, errors)
    if data is None:
        return None
    try:
        return model.model_validate(data)
    except ValidationError as e:
        errors.append(f"{path}: {e}")
        return None


def _model_for(category: str, doc: dict) -> type[Entity]:
    doc_id = str(doc.get("id", ""))
    if category == "skills":
        # skills/ holds skill.* plus their techniques: tech.* (and the legacy
        # swordskill.* prefix, which is just a melee technique) (§4.8)
        if doc_id.startswith(("swordskill.", "tech.")):
            return SwordSkill
        return Skill
    if category == "npcs" and doc_id.startswith("arch."):
        return CATEGORY_MODELS["archetypes"]
    return CATEGORY_MODELS[category]


def _load_entity_file(path: Path, category: str, registry: Registry, errors: list[str]) -> None:
    data = _read_yaml(path, errors)
    if data is None:
        return
    docs = data if isinstance(data, list) else [data]
    for doc in docs:
        if not isinstance(doc, dict) or "id" not in doc:
            errors.append(f"{path}: document without an 'id' field")
            continue
        try:
            entity = _model_for(category, doc).model_validate(doc)
        except ValidationError as e:
            errors.append(f"{path}: {e}")
            continue
        if entity.id in registry.entities:
            errors.append(f"{path}: duplicate id {entity.id!r}")
            continue
        registry.entities[entity.id] = entity


def _register_nested_ids(registry: Registry) -> None:
    """Any mapping with an `id:` field anywhere inside an entity defines that id
    (zones, districts, inline shops, goals)."""

    def walk(node: Any) -> None:
        if isinstance(node, dict):
            nested = node.get("id")
            if isinstance(nested, str) and _REF_RE.match(nested):
                registry._nested_ids.add(nested)
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    for entity in registry.entities.values():
        walk(entity.model_dump())
    walk(registry.manifest.model_dump())


_QTY_SUFFIX = re.compile(r"_x\d+$")


def strip_qty_suffix(ref: str) -> tuple[str, int]:
    """`item.bread_x2` -> (`item.bread`, 2). Refs without a suffix pass through."""
    match = _QTY_SUFFIX.search(ref)
    if match:
        return ref[: match.start()], int(match.group()[2:])
    return ref, 1


def _check_references(registry: Registry, errors: list[str]) -> None:
    known = registry.known_ids()

    def walk(node: Any, owner: str) -> None:
        if isinstance(node, str):
            match = _REF_RE.match(node)
            if match and match.group(1) in REF_PREFIXES:
                base, _ = strip_qty_suffix(node)
                if base not in known:
                    errors.append(f"{owner}: dangling reference {node!r}")
        elif isinstance(node, dict):
            for value in node.values():
                walk(value, owner)
        elif isinstance(node, list):
            for value in node:
                walk(value, owner)

    for entity in registry.entities.values():
        walk(entity.model_dump(), entity.id)
    walk(registry.manifest.model_dump(), registry.manifest.id)
