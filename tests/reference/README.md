# tests/reference/

The integration suite, run in one fictional world: **Ashford**
([worlds/ashford.world](../../worlds/ashford.world)). Every test loads the same city through
the real loader and asks it something — can the cat get in when the door is shut, does Dave
at the window see Erin in the yard, how far would you fall from the gallery window, does the
granite hall lag the afternoon more than the timber kitchen. Features are tested in a working
context, against each other, rather than one small world per feature; a feature that needs
something Ashford lacks grows Ashford.

| File | What it holds Ashford to |
| --- | --- |
| `space.rs` | positions through the nesting, facing and bearings, a ship's turning deck, bodies filling their size, proximity, contents, regions (nested and overlapping), materials, motion written only at its ends |
| `movement.rs` | travel by intent only: doors, windows, the bulkhead, stairs, fit, solids, falls, slopes and the pass, reach, connectivity, and the refusal of anything else that moves a body |
| `sight.rs` | walls, glass, open and shut doors, curtains, a wagon in the way, the ridge |
| `ground.rs` | terrain between its samples, slope, height above the ground, the drop through each opening |
| `climate.rs` | weather per climate inherited down the nesting; tick-length invariance; exposure; shelter, daylight through openings, indoor lag by thermal mass; pressure, wind, humidity |
| `perception.rs` | seeing by daylight and not in the dark, believing what was seen, remembering it after it is gone (rightly or not), knowing where one stands and how warm it is, knowing nothing of what one never saw |
| `minds.rs` | Erin comes in from the cold to the kitchen she remembers warm, opening the shut door in her way; the courier, who never saw the kitchen, stays out; the guard keeps his hours; same seed, same choices |
| `living.rs` | body heat against the air of whatever encloses a body; tick-length invariance; Living never perturbing Physical |
| `engine.rs` | replay determinism, seed divergence, the spatial index invisible and exact, declared reads, tick abort on a broken invariant, the loader's refusals and validation problems |

`src/lib.rs` is the harness: Ashford's ids by name, and `City`, a running Ashford with a
front door through which a test asks people to go somewhere, open or shut something, or
turn — the same intents a player or a mind would propose (Appendix A, Ruling 13).

Kernel contract tests stay in `kernel/tests/`, and each crate keeps its unit tests beside
its code.

## Destined

Pyramid layers 4–6 (Vol. V Ch. 8 §8.3) against the five worlds (Vol. IV Ch. 8):
conservation soaks (currency, matter, population reconcile to accounted causes —
Kepler forgives nothing), liveness envelopes (the 365-day pattern that caught the
income bleed), and phenomenological signatures (trade corridors, settlement cycles,
at least one war and one peace per Thornwall century — statistical envelopes, not
scripts). All five green, or the change explains itself (Vol. IV Ch. 8 §8.6.1;
Vol. V Ch. 10 §10.3). A surprising result with a correct causal chain updates
envelopes, never the world (Vol. IV Ch. 7 §7.5.10).
