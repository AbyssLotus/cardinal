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
| `climate.rs` | weather per climate inherited down the nesting; tick-length invariance; exposure; shelter, daylight through openings, indoor lag by thermal mass; pressure falling with height but moving no air; humidity; the year turning warm and back, exact at any tick length; days lengthening toward midsummer; a full moon letting Erin see the tree at midnight |
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
| `residents.rs` | the Lanes at the city tier: asleep at home by night, at the workshop in working hours, home by evening; eating from larders their shifts keep stocked; costing nothing while nothing ends; same seed, same Lanes; a resident never also an organism (Ruling 19) |
| `crossing.rs` | attention on the Lanes at night: its residents becoming organisms in one tick, asleep in their beds, as hungry and tired as their records to the bit; then up at their own hours, out of their doors and into the workshop, and home by evening; a worker crossing on arriving at the workshop; a home kept at the city tier keeping its household; same seed, same crossing; no resident beginning in the scene (Ruling 19); and back: the Lanes brought into the scene and let go again leaving the whole world identical, to the bit, to one where no one looked (Law 14); larders set out as pantries their households know, eat from, and restock with work, and counted back exactly |
| `commute.rs` | a day in a generated town of four districts, attention on two, most people working in the next district along: crossings into the scene and out of it all morning and evening; every resident exactly one thing at every tick; hunger the same to the unit across every crossing; every worker at work at eleven, at whichever tier; those living in the scene eating from their pantries, and nobody near starving; everyone home at day's end |
| `breaking.rs` | Erin's fists not marking the packing crate, Carol's breaking it in a few blows, anyone's with the hammer in two; the wreckage flat timber, its apple rolled out; a glass bottle swung at an iron hammer refused; damage and repair from anyone adding up; the kitchen's walls brought down to a ruin, its door broken open, and Bob in sight of the yard; a broken window that will not shut; a bottle falling as the table under it is wrecked |
| `fire.rs` | the hay wagon alight against Alice's house taking the kitchen and the bedroom over it, but not the granite cellar, and burning Bob where he stands; a kept hearth lighting the kitchen at midnight and burning nothing; a torch lit at the hearth setting the crate alight, which burns down to wreckage while its apple rolls out; rain putting out a fire in the open but not under a roof; the same fire burning the same way |
| `wind.rs` | a gale over the Vale bringing down the timber shed and Alice's house and breaking the crate, while the granite manor, the hammer, and the bottle stand; the kitchen table sheltered until the walls fall; a lesser wind scattering the hay and sparing the shed; the same storm doing the same damage; the torch carried across the yard by a gale while the solid crate stays put; a shove from anyone moving the bottle just so far; a deep low brewing a gale with no storm declared, and Ashford's own weather not |
| `open_ground.rs` | Erin walking into a garden plot across its edge and out again; walking to a place with no size; a walled kitchen with its door shut having no way in; the kitchen's ruin walked into over the rubble, not through its doorway |
| `mending.rs` | the kitchen's walls falling on Bob and hurting him, and not on Dave upstairs; the smashed crate unmendable by hand, mended whole with the hammer, as tall and as solid as it was; the kitchen's ruin rebuilt, walled again, with a front door that shuts |
| `air.rs` | an open door letting the night into Alice's kitchen faster than its walls; a kept hearth's warmth climbing the stair to the bedroom and down to the cellar; the burning table's smoke drifting upstairs; a fire in the still yard smoking the air over it and drifting in at the open door, kept out when the house is shut, carried off by a wind |
| `fronts.rs` | a low over the moor drawing the Vale's air toward it, a high blowing off the moor instead; a cold spell on the moor riding the wind into the Vale and on to the fens, and felt only faintly upwind; no crossing with fronts turned off; the same weather crossing the same way |
| `sky.rs` | humid air clouding the Vale and dimming its noon; an overcast holding back the day's swing; cloud coming and going for ten days and leaving nothing behind, to the hundredth of a degree; saturated air raining, and snowing in frost; dawn fog over the fens, lifted by three; Erin losing the apple tree in the fog at six and seeing it again at noon |
| `fire_air.rs` | a kept hearth warming the kitchen with no smoke; the burning table filling the kitchen with smoke, thicker shut up than open; Bob seeing the lamp by firelight, then losing it in the smoke and hurt breathing it; the wind fanning the burning hay to full |
| `week.rs` | a week with nobody directed: everyone eats and nobody starves; Carol and Bob work every day and the pies get eaten; the household sleeps every night; every choice explained; same seed, same week |
| `living.rs` | body heat against the air of whatever encloses a body; tick-length invariance; Living never perturbing Physical |
| `engine.rs` | replay determinism, seed divergence, the spatial index invisible and exact, declared reads, every fact a system names owned by some domain, tick abort on a broken invariant, the loader's refusals and validation problems |

`src/generate.rs` builds a city on top of Ashford for the commuting test and the city bench:
districts that are streets with a gate onto the city's ground, homes and workshops with doors onto
them, and residents who live in one district and work in the next.

`src/lib.rs` is the harness: Ashford's ids by name, and `City`, a running Ashford with a
front door through which a test asks people to go somewhere, open or shut something, turn,
take, drop, eat, pick, rest, make, or claim — the same intents a player or a mind would propose
(Appendix A, Rulings 13, 15–18) — and through which it can turn attention on a place
(`City::attend`, Amendment A-24), strike something (A-27), or set it alight (A-28). Commanding someone directs them first, so their own mind stands
aside (Amendment A-14). `City::quiet` and `without_minds` give an Ashford where nobody acts unless
told; `well_fed` one where nobody grows hungry, for tests of other drives.

Kernel contract tests stay in `kernel/tests/` — among them tick continuity, change memory (A-21),
and the agenda and who holds a value (A-22, A-24) — and each crate keeps its unit tests beside its
code.

`examples/scale.rs` and `examples/city.rs` are the benches: the scene (N villagers with minds and
sight; `METERS=1` breaks a tick down by system) and the city tier (N residents in generated
districts, a day with attention on the first `SCENE` of them and people crossing all day;
`METERS=1` too). Run them in release.

## Destined

Pyramid layers 4–6 (Vol. V Ch. 8 §8.3) against the five worlds (Vol. IV Ch. 8):
conservation soaks (currency, matter, population reconcile to accounted causes —
Kepler forgives nothing), liveness envelopes (the 365-day pattern that caught the
income bleed), and phenomenological signatures (trade corridors, settlement cycles,
at least one war and one peace per Thornwall century — statistical envelopes, not
scripts). All five green, or the change explains itself (Vol. IV Ch. 8 §8.6.1;
Vol. V Ch. 10 §10.3). A surprising result with a correct causal chain updates
envelopes, never the world (Vol. IV Ch. 7 §7.5.10).
