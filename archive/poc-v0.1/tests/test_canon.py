import pytest

from engine.canon import guard
from engine.core.registry import load_world
from engine.schemas import Item


def test_generated_content_may_not_claim_confirmed_tier(testworld_path):
    registry = load_world(testworld_path)
    fake = Item(id="item.tw_fake", name="Fake", canon_tier="confirmed")
    with pytest.raises(guard.CanonViolation, match="may not claim 'confirmed'"):
        guard.check(fake, registry)


def test_confirmed_canon_cannot_be_replaced(aincrad_path):
    registry = load_world(aincrad_path)
    impostor = Item(id="item.anneal_blade", name="Anneal Blade But Worse")
    with pytest.raises(guard.CanonViolation, match="collides with confirmed canon"):
        guard.check(impostor, registry)


def test_generated_new_content_passes(testworld_path):
    registry = load_world(testworld_path)
    new = Item(id="item.tw_new_thing", name="New Thing", canon_tier="generated")
    guard.check(new, registry)  # no raise
