"""World-package loading: schema validation, id indexing, reference checks."""

import pytest
import yaml

from engine.core.registry import WorldLoadError, load_world, strip_qty_suffix


def test_testworld_loads(testworld_path):
    registry = load_world(testworld_path)
    assert registry.manifest.id == "world.testworld"
    assert len(list(registry.by_kind("npc"))) == 5
    assert registry.get("mon.tw_slime").stats.hp == 30
    assert registry.rule("time_costs.travel_per_km_road") == 10
    assert registry.rule("missing.path", "fallback") == "fallback"


def test_aincrad_loads(aincrad_path):
    registry = load_world(aincrad_path)
    assert registry.get("npc.argo").name == "Argo the Rat"
    assert registry.get("item.anneal_blade").upgrade.max_plus == 8
    # nested ids (zones, districts, inline shops) register for ref checking
    assert "zone.f1_west_fields" in registry.known_ids()
    assert "shop.tob_general" in registry.known_ids()


def test_dangling_reference_fails_loudly(tmp_path):
    (tmp_path / "locations").mkdir()
    (tmp_path / "world.yaml").write_text(yaml.safe_dump({
        "id": "world.broken",
        "name": "Broken",
        "entry_point": {"location": "loc.somewhere"},
    }))
    (tmp_path / "rules.yaml").write_text("{}")
    (tmp_path / "locations" / "somewhere.yaml").write_text(yaml.safe_dump({
        "id": "loc.somewhere",
        "name": "Somewhere",
        "floor": "floor.does_not_exist",   # dangling
    }))
    with pytest.raises(WorldLoadError, match="dangling reference 'floor.does_not_exist'"):
        load_world(tmp_path)


def test_duplicate_id_fails(tmp_path):
    (tmp_path / "items").mkdir()
    (tmp_path / "world.yaml").write_text(yaml.safe_dump({
        "id": "world.dup",
        "name": "Dup",
        "entry_point": {"location": "loc.x"},
    }))
    (tmp_path / "rules.yaml").write_text("{}")
    (tmp_path / "locations").mkdir()
    (tmp_path / "locations" / "x.yaml").write_text(
        yaml.safe_dump({"id": "loc.x", "name": "X", "floor": "floor.f"})
    )
    (tmp_path / "floors").mkdir()
    (tmp_path / "floors" / "f.yaml").write_text(
        yaml.safe_dump({"id": "floor.f", "name": "F"})
    )
    doc = yaml.safe_dump({"id": "item.same", "name": "Same"})
    (tmp_path / "items" / "a.yaml").write_text(doc)
    (tmp_path / "items" / "b.yaml").write_text(doc)
    with pytest.raises(WorldLoadError, match="duplicate id 'item.same'"):
        load_world(tmp_path)


def test_ranged_weapons_and_techniques(testworld_path):
    """Genre uplift: any world can define projectile/thrown/beam combat in data."""
    registry = load_world(testworld_path)
    sling = registry.get("item.tw_sling")
    assert sling.ranged.ammo == "item.tw_pebble"
    assert sling.ranged.max_range_m == 30
    shot = registry.get("tech.tw_sling_shot")
    assert shot.delivery == "projectile"
    assert shot.parent_skill == "skill.tw_slinging"


def test_aincrad_ranged_content(aincrad_path):
    registry = load_world(aincrad_path)
    pick = registry.get("item.throwing_pick")
    assert pick.ranged.thrown is True
    bow = registry.get("item.short_bow")
    assert bow.canon_tier == "inference"       # no archery skill in SAO canon
    assert bow.ranged.ammo == "item.wooden_arrow"
    # legacy swordskill.* files still load as (melee) techniques
    assert registry.get("swordskill.horizontal").delivery == "melee"
    assert registry.get("tech.aimed_shot").delivery == "projectile"
    # currency/region vocabulary defaults hold for Aincrad
    assert registry.manifest.currency.name == "Col"
    assert registry.manifest.region_label == "Floor"


def test_modifiers_and_vehicles(testworld_path):
    registry = load_world(testworld_path)
    charm = registry.get("mod.tw_lucky_charm")
    assert charm.mod_type == "blessing"
    assert charm.duration_days is None            # permanent
    rot = registry.get("mod.tw_slime_rot")
    assert rot.duration_days == 5
    assert rot.side_effects[0].target == "npc.tw_alice"
    chrome = registry.get("mod.tw_chrome_arm")
    assert chrome.humanity_cost == 0.1
    assert chrome.acquisition.source == "surgery"

    mule = registry.get("vehicle.tw_mule")
    assert mule.living is True and mule.fuel is None
    cart = registry.get("vehicle.tw_autocart")
    assert cart.fuel.item == "item.tw_oil_flask"
    assert "forest" not in cart.speed_kmh          # absent terrain = impassable


def test_aincrad_modifiers_and_vehicles(aincrad_path):
    registry = load_world(aincrad_path)
    venom = registry.get("mod.paralysis_venom")
    assert venom.canon_tier == "confirmed"
    assert venom.duration_minutes == 10
    emblem = registry.get("mod.alf_emblem")
    assert emblem.effects[0].target == "fac.alf"   # ref-checked against the faction
    wagon = registry.get("vehicle.trade_wagon")
    assert wagon.living is True and wagon.cargo_kg == 400


def test_qty_suffix():
    assert strip_qty_suffix("item.bread_x2") == ("item.bread", 2)
    assert strip_qty_suffix("item.bread") == ("item.bread", 1)
