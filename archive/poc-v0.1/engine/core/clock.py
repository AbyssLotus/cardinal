"""World time (§6).

Resolution: 1 minute, stored as (day, minute_of_day).
`advance()` returns the tick boundaries crossed so the loop can fire
hour-level and day-level system updates hierarchically.
"""

from __future__ import annotations

from dataclasses import dataclass

MINUTES_PER_HOUR = 60


@dataclass(frozen=True)
class TickBoundary:
    granularity: str  # "hour" | "day"
    day: int
    hour: int  # for "day" boundaries this is 0


@dataclass
class WorldClock:
    day: int = 0
    minute: int = 0  # minute of day
    minutes_per_day: int = 1440

    @property
    def hour(self) -> int:
        return self.minute // MINUTES_PER_HOUR

    @property
    def minute_of_hour(self) -> int:
        return self.minute % MINUTES_PER_HOUR

    def stamp(self) -> tuple[int, int]:
        return (self.day, self.minute)

    def label(self) -> str:
        return f"Day {self.day}, {self.hour:02d}:{self.minute_of_hour:02d}"

    def advance(self, minutes: int) -> list[TickBoundary]:
        """Advance the clock, returning every hour/day boundary crossed in order."""
        if minutes < 0:
            raise ValueError("time only moves forward")
        boundaries: list[TickBoundary] = []
        absolute = self.day * self.minutes_per_day + self.minute
        target = absolute + minutes
        current = absolute
        while current < target:
            # next hour boundary after `current`
            next_hour = (current // MINUTES_PER_HOUR + 1) * MINUTES_PER_HOUR
            if next_hour > target:
                break
            current = next_hour
            day, minute_of_day = divmod(current, self.minutes_per_day)
            if minute_of_day == 0:
                boundaries.append(TickBoundary("day", day, 0))
            else:
                boundaries.append(
                    TickBoundary("hour", day, minute_of_day // MINUTES_PER_HOUR)
                )
        self.day, self.minute = divmod(target, self.minutes_per_day)
        return boundaries
