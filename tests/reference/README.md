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
| `love.rs` | fondness growing in each other's sight, capped by compatibility; Finn and Gwen becoming lovers while the ill-matched courier stays a stranger; Gwen, apart, longing for Finn and going back to him; lovers kept apart cooling, parting, and the longing ending with the bond |
| `items.rs` | taking what is within reach and light enough, carrying it upstairs, putting it down; eating the apple, after which it is nowhere but not forgotten; a tonne of wardrobe, an apple out of reach, and an uneaten lamp all refused |
| `addiction.rs` | Ned craving poppy, fetching a vial from the undercroft and drinking it; deeper dependence bringing each craving sooner; withdrawal once the shelf is bare; abstinence ending the craving; Erin never wanting any |
| `needs.rs` | tiring and resting at any tick length, and no rest on the move; the frost that harms and then kills, after which the body stops; falls hurt beyond a safe drop; wounds heal slowly; minds sleep the night and get up at six, lie down when tired where there is no bedtime, and get up rested, when the cold calls, or when a routine does; the dead neither see nor think |
| `agency.rs` | a directed person does as told while their own mind stands aside, and decides for themself again once released; a day in Ashford with nobody directed, every choice with its reason; same seed, same day |
| `things.rs` | the apple tree ripening at its rate up to what it can hold; picking bringing a new apple, with a new id, into the hand; a bare tree yielding nothing; the same world making the same things |
| `hunger.rs` | hunger at the same pace at any tick length; hungry Erin picking an apple from the tree she knows and eating it; the courier, who has never seen the tree, not going for it in the dark; starvation harming |
| `making.rs` | two apples and a scoop of flour becoming a pie at the hearth; nothing made without the flour, or away from the hearth; a pie left baking set down in front of the hearth, where it can be reached and taken |
| `jobs.rs` | nobody working before their hours; Carol bringing the harvest in to the kitchen; Bob baking pies for the kitchen from what he can find; the hungry eating the pies; same seed, same working day |
| `wants.rs` | the curious courier going to see the house, and stopping once he has; Erin claiming the glass bottle no one owns and keeping it in her bedroom, past Bob's lamp, which she leaves alone; what is someone else's left where it lies; a claim needing the thing in hand and no owner |
| `week.rs` | a week with nobody directed: everyone eats and nobody starves; Carol and Bob work every day and the pies get eaten; the household sleeps every night; every choice explained; same seed, same week |
| `living.rs` | body heat against the air of whatever encloses a body; tick-length invariance; Living never perturbing Physical |
| `engine.rs` | replay determinism, seed divergence, the spatial index invisible and exact, declared reads, every fact a system names owned by some domain, tick abort on a broken invariant, the loader's refusals and validation problems |

`src/lib.rs` is the harness: Ashford's ids by name, and `City`, a running Ashford with a
front door through which a test asks people to go somewhere, open or shut something, turn,
take, drop, eat, pick, rest, make, or claim — the same intents a player or a mind would propose
(Appendix A, Rulings 13, 15–18). Commanding someone directs them first, so their own mind stands
aside (Amendment A-14). `City::quiet` and `without_minds` give an Ashford where nobody acts unless
told; `well_fed` one where nobody grows hungry, for tests of other drives.

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
