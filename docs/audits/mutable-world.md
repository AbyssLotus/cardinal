# A Mutable World: Things That Break, Burn, and Blow Down

*Plan, 2026-10-07.* The user's direction: a world whose physical space can be changed by what
happens in it. A building can burn down or be wrecked by wind. A crate can be smashed by someone
strong enough, or by anyone with a tool.

## 1. What the spec already settles

- **Ruling 9 (damage).** Anything may *propose* structural damage; Physical Reality applies it under
  its own rules. Conflict, fire, wind, and a fist are all just proposers.
- **Composition (Vol. III Ch. 12).** "Fire consuming a wall that soldiers are repairing": damage
  and repair to one wall in one tick both apply, in the owner's declared composition.
- **Properties over names (Vol. III Ch. 1).** A bridge fails because its material cannot bear the
  stress, not because it is "wood". Fire spreads because nearby materials satisfy ignition
  conditions. The materials already carry hardness, flammability, density, and thermal capacity;
  a composite is as strong as its weakest material and burns like its most flammable.
- **Conservation (Vol. I).** Matter is not destroyed by breaking: what is left of a smashed crate is
  still the timber it was made of, still its owner's.
- **Law 14.** Nothing here may depend on tick length: one blow is one action, and burning and
  wind act at rates in simulated time.

## 2. The design in one paragraph

Every thing and every walled place has an **integrity**, from whole to nothing, stored only once it
has been damaged. Damage is a proposal anyone may make. Physical Reality judges every blow by
property: one that strikes no harder than the target's weakest material leaves no mark. When
integrity reaches nothing, the thing **breaks**. A thing becomes wreckage: the same entity, still its
materials and its owner's, flattened, no longer solid or opaque, spilling what it held. A walled
place becomes a **ruin**: the same place, its walls down, open to the sky and to sight, its doors
broken open. Fire and wind are processes that make such proposals at rates; people make them by
striking.

## 3. Slices

1. **Integrity and breaking (A-27).**
   - The integrity fact: additive, so damage and repair compose. Absent means whole.
   - Breaking into wreckage or ruin.
   - The strike intent, judged by the striker's strength (a Living capability) and the best tool
     in hand (its hardness and weight) against the target's weakest material and its mass.
   - Doors broken open.
   - Ashford: a packing crate with an apple in it, a hammer, a strong Carol.
2. **Fire (A-28).**
   - Burning as a level on a thing or a walled place, consuming its integrity at a rate set by
     flammability.
   - Spreading by heat to flammable things within reach and to the walls around.
   - Firelight: a place with something burning in it is lit.
   - Heat exposure reported to Living, which judges burns.
   - Rain and humidity damp it. Ignition from something burning in hand or within reach (the
     hearth, a torch). A building burned down is a ruin.
3. **Wind (A-29).**
   - In a storm, exposed structures take damage where the wind's force exceeds what their weakest
     material bears. Light, loose things are blown about.
   - Storms the world package declares, and those the weather makes.
4. **Minds that notice (A-30).** Fleeing fire, keeping out of burning places, breaking a way
   through when the way is shut.
5. **Open ground in a ruin.** Walking into and out of a place with no walls anywhere along its edge,
   not only through its doorways. This also lifts the "no travel between open sibling places
   without an opening" limit.

## 4. Scale

Integrity is sparse: only damaged things carry it. Breaking looks only at things whose integrity
changed. Fire looks only at what burns and what lies within reach of it. Wind looks only at
regions where the wind exceeds the weakest structure's resistance. A still, unburning, calm
world costs nothing more than it does today. The city tier's buildings are records, not
geometry: fire and storm there are a later, aggregate slice.

## 5. Status (2026-10-07)

- **Built:** slice 1, integrity and breaking (A-27): `tests/reference/tests/breaking.rs`.
- **Built:** slice 2, fire (A-28): `tests/reference/tests/fire.rs`.
- **Next:** wind (A-29). Then minds that notice (A-30), and open ground in a ruin.

Two things the building found:
- **A thing on a broken table must fall.** Gravity only drops bodies that move by themselves.
  Breaking now drops what stood on what broke, found by looking over its place, so the answer is
  the same with or without the spatial index. The first attempt (having gravity look above
  whatever changed) failed the engine's index-equivalence test: it found floating things one way
  and not the other.
- **Heat is drawn from a fire's middle, not its top.** A hay pile exactly as tall as a bedroom's
  floor had its heat to the kitchen shadowed by the walled bedroom above, until the line was drawn
  from where the fire is rather than where a gaze would start.
