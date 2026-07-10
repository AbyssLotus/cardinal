"""VALIDATE step: legality checks against world rules and player state.

Returns nothing on success; raises ValidationFailure with a player-facing
reason otherwise. World rules apply equally to everyone — there is no
"but it would be more fun" bypass.
"""

from __future__ import annotations

from typing import Any

from engine.actions.actions import Action
from engine.core.registry import Registry


class ValidationFailure(Exception):
    pass


def validate(action: Action, player: dict[str, Any], registry: Registry) -> None:
    if not player.get("alive", 1):
        raise ValidationFailure("The dead take no actions.")

    if action.intent == "travel":
        destination = registry.find(action.target)
        if destination is None:
            raise ValidationFailure(f"{action.target!r} does not exist.")
        if action.target == player["location_id"]:
            raise ValidationFailure("You are already there.")
        current = registry.find(player["location_id"])
        # M1: travel allowed between locations on the same floor
        if current is not None and getattr(destination, "floor", None) != getattr(current, "floor", None):
            raise ValidationFailure(
                "You can't reach another floor from here — floors open only when their boss falls."
            )
