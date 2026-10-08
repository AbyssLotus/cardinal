# Resources: Ground, Deposits, Extraction, and What Making and Mending Consume

*Plan, 2026-10-08, for review before building.* The user's direction: keep at the environment;
resource deposits are next. Stone can be quarried anywhere the ground is stone. Copper, iron,
timber, and the like need deposits. Repairing a house should take the materials it needs, and so
should baking bread. Agents need a way to get all of these.

## 1. What the spec settles (Vol. III Ch. 3)

- A **deposit** is an entity: a located, finite stock of a resource, with a grade and a
  difficulty of extraction. It keeps its identity from discovery to exhaustion.
- Every stock renews one of four ways: **nonrenewable** (ore: it only falls), **renewable**
  (timber: it regrows toward a cap), **flow** (river water: it refills, but cannot be stockpiled
  at the source), or **threshold** (a renewable stock that stops regrowing once taken below a
  floor).
- **Extraction conserves.** What is taken leaves the stock and arrives as held matter. Nothing is
  made by taking.
- **Yield** is a function of the deposit's grade, the extractor's capability, and the method.
- Resources owns none of the value (Economy), the ownership (Society), or the knowledge of
  deposits (Information). The labour is the organism's.
- Named anti-patterns: "the infinite node" and "spawning supply."

## 2. What is built

- **Deposits** (A-15): a thing with a stock that regrows at a daily rate up to a cap. Anyone in
  reach picks one unit by hand, instantly, with no tool. Picked items are real entities of the
  deposit's material and size.
- **Recipes** (A-17): made at a workplace from inputs carried, over time. Ashford has one: the
  apple pie (two apples and a scoop of flour, at the hearth).
- **Jobs** (A-18): carry a material from a source to a store, or make a recipe and keep a store
  stocked. Minds get what work needs from the deposits and things they believe in.
- **Mending** (A-31): needs a tool no softer than the thing, and nothing else.

## 3. What is missing

- **Nothing can be dug, mined, or felled.** No ground material, no quarries, no ore, no timber
  trees. Every deposit is fruit picked by hand.
- **Supply appears from nowhere.** The flour bin "is sent three scoops a day" by no one. The
  miller's loaves, the manor pantry, and the farmhouse larder regrow bread. These are the spec's
  named anti-pattern, spawning supply. (Households' larders at the city tier, A-26, are a
  separate abstraction and stay.)
- **Bread has no recipe**, and flour has no source: no grain, no milling.
- **Mending is free**: a hammer rebuilds a fallen house out of nothing.
- **Minds cannot do any of this.** They never fetch a tool, never extract with effort, and never
  mend. They cannot perceive what they would need to know: how hard a material is, or that a
  thing is damaged.

## 4. Slices

### R1. Ground and quarries (A-42)

- **Ground** (Physical; world data). `[ground] place = material` says what a place's ground is
  made of. It is inherited down the nesting, like climate. Ashford's ground:
  - Highmoor: granite bedrock.
  - the Hill: granite under thin soil.
  - the Vale: soil (clay).
- **Digging** (`act.dig`, open to any decider, Ruling 13). A body digs the ground where it stands.
  Physical carries it out where there is ground to dig. The obvious exceptions are refused:
  - inside a walled place;
  - in water, on flooded ground, or on ice;
  - on ground the world marks unbroken (`[flags] place = paved`: the Lanes, the manor's floors).
- **Quarries** (Resources). The first dig at a spot opens a pit there: a nonrenewable deposit of
  the ground's material, with the reserve the world declares for a pit of that material. Later
  digs within reach draw on that pit. An exhausted pit is still there; dig a new one beside it.
  Stone needs a tool (R2's rule); soil does not.

### R2. Extraction: tools, effort, grade, and renewal (A-41)

- **One way to take from a deposit.** Picking (A-15) becomes extracting. Each go by a body puts
  work into the deposit:
  - its strength, multiplied by the tool it holds, as a blow is judged (A-27);
  - a tool is needed where the deposit's material is hard: one no softer than it (as for
    mending, A-31). Fruit needs no tool; stone and ore need a pick or hammer; timber needs an
    axe.
- **Work per unit**: the deposit's difficulty divided by its grade. Each time the work put in
  reaches it, one unit leaves the stock and arrives in the hand. An apple tree needs no work: one
  go, one apple. A poor iron vein needs many blows.
- **Renewal classes** (world data per deposit): nonrenewable, renewable, threshold (a floor
  below which it never regrows), or flow (refills to its cap each step).
- **Living stocks grow with the season and the weather** (ties to A-34 to A-38). A renewable
  deposit may declare itself grown: then it regrows only:
  - in its warmth (none below a declared temperature, at full rate by another);
  - with water in the ground;
  - in daylight.

  So apples ripen through the summer, a field of wheat stands still in winter, and a drought
  stunts it.
- **Ashford gets**:
  - an iron vein and a copper vein in the mine's deep gallery (nonrenewable);
  - a stand of timber in the wood (threshold: fell it below a quarter and it stops regrowing);
  - a wheat field on the farm (renewable, grown);
  - the well at the harbour gate and the Brack as water (flow): see the open question on
    liquids.
- **Retired**: the flour bin's and the bread larders' regrowth. These become stores that only
  what is brought fills.

### R3. The chains: heat, power, and recipes (A-43)

- **Heat.** A recipe may need its workplace burning (A-28): the forge, the oven, the kiln.
  Economy reads Physical's fire by id. No fire, no smelting.
- **Power.** A recipe may need a mill turned:
  - by water, while the river runs at or above its normal level (A-39);
  - or by wind, past a declared wind speed (A-36).
- **A tool to hand.** A recipe may need a tool held: a saw for planks.
- **New materials**: iron ore, copper ore, copper, wheat, charcoal, planks, stone (granite
  blocks). Iron, timber, granite, flour, and bread exist.
- **Ashford's chains:**
  - wheat → flour at the mill, while the Brack turns it;
  - flour → bread at the kitchen oven, burning;
  - timber → planks at the sawpit, with a saw;
  - timber → charcoal at a kiln, burning;
  - iron ore + charcoal → iron at a forge, burning (and copper the same way);
  - granite is quarried into blocks, used as they come.

### R4. Mending takes material (A-44)

- Each go at mending a thing uses up its main material: as much as the go restores, in its share
  of the thing's mass. This comes from items of that material the mender holds or stands beside:
  planks or logs for timber, blocks for granite.
- With none to hand, the go is refused, as it is without a tool. With some, it restores what that
  much allows.
- Rebuilding Alice's fallen kitchen (about 44 tonnes of timber shell) takes that much timber.
  Mending a 10% gash takes a tenth of it.

### R5. Agents get what they need (A-45)

- **Perception** (Information): what things are made of is already seen. Added:
  - how hard a material is (seen in what is made of it);
  - how whole a thing is, whether damaged or broken;
  - what a deposit is (stone, ore, timber) and that it needs a tool.
- **Minds**:
  - Work that extracts first fetches a tool fit for it: one the mind carries, owns, or can
    claim.
  - It then works the deposit go after go until a unit comes, and carries it on (A-18's loop,
    with patience for effort).
  - Jobs in Ashford: a quarryman (granite blocks to the builder's yard), a miner (ore to the
    forge), a woodcutter (logs to the sawpit), a farmer (wheat to the mill), a miller (flour to
    the bin), a baker (bread to the kitchen), and a smith (iron).
- **Repair** (a household want, like keeping one's things at home, A-19). A mind whose home, or
  a thing it owns, is damaged gathers the material and a tool, and mends it, go by go. It fetches
  from a store it knows, or from a source if no store holds enough.

## 5. How it is tested

Each slice in Ashford, as before. The closing test is a week in Ashford:

- the wheat is cut, milled while the Brack runs, baked, and eaten, and nobody goes hungry with
  the flour bin no longer filling itself;
- after a storm tears at the shed, its owner fells timber, saws planks, and mends it;
- the iron vein is a little smaller each day the miner works it, and never regrows.

## 6. Decisions for the user

1. **Items or quantities for heavy materials.** Every unit so far is an entity: an apple, a log.
   A house's shell is tens of tonnes, and a carry limit of 25 kg makes that thousands of trips.
   - *Recommended:* keep items for now (everything that takes, drops, makes, and mends works on
     them), with units sized near the carry limit. Carts, sacks, and stacks that hold a quantity
     come after.
   - *Alternative:* build stackable quantities first. Larger, and it touches taking, dropping,
     making, and minds.
2. **How deep a quarry goes.** *Recommended:* each pit holds a declared, finite reserve (spec:
   nonrenewable means nonrenewable); when it is done, open another beside it. *Alternative:*
   bedrock is effectively endless where the ground is stone.
3. **Who may dig, mine, and fell.** *Recommended:* anyone, anywhere the ground allows, for now.
   Claims and licences are Society's and Institutions', later.
4. **Water and liquids.** Bread really wants water, and wells and rivers are flow deposits. But a
   bucket of water needs liquids in containers. *Recommended:* bread from flour alone for now;
   liquids later.
5. **Minds now.** R5 is minds and perception work, which you set aside last time for the
   environment. Agents cannot get resources without it. *Recommended:* include it, as the last
   slice.

## 7. Not in this plan

- Prospecting and hidden deposits (Information: what is known of a vein).
- Ownership of land and deposits, licences, taxes.
- Markets and prices.
- Carts and beasts of burden.
- Resources at the city tier beyond households' larders.

## 8. Status

Going, on the user's word, with every recommendation in §6 taken. Built in the order R2, R1
(digging is taking from a pit), R3, R4, R5:

- **Built:** R2, extraction (A-41): `tests/reference/tests/extract.rs`.
- **Built:** R1, ground and quarries (A-42): `tests/reference/tests/quarry.rs`.
- **Built:** R3, the chains (A-43): `tests/reference/tests/chains.rs`. The flour bin's and the
  loaves' self-filling are retired with the jobs that replace them (R5), not here: without the
  farmer, the miller, and a baker at work, nothing would make flour.
- **Built:** R4, mending takes material (A-44): `tests/reference/tests/mending.rs`. Found: a ruin's
  mass was counted as a solid block, four times its shell.
- **Built:** R5, agents get what they need (A-45): `tests/reference/tests/labour.rs`. Found and
  fixed along the way:
  - minds counted anything with a material among its parts as a source of it (Alice's front
    door, for timber), and fixtures as pieces to carry off. Perception now shows what is a
    fixture, and a source is a deposit or a piece made of that alone;
  - minds had no way across open ground between places with no opening, so the farmer could not
    reach the mill.

  Millside's farmer, miller, and baker now keep the mill in bread. Of the jobs §4 R5 lists, the
  quarryman, the miner, the woodcutter, and the smith are not given to anyone in Ashford yet:
  each works (the tests set Carol to the seam and Erin to the clay), but nothing in Ashford uses
  iron, planks, or blocks yet, so a standing job would only pile them up. The closing week (§5)
  still runs on the stand-in larders for everyone but Millside. The storm-and-rebuild and the
  vein shrinking daily wait for those jobs to have a reason to exist.

Retired as planned: the flour bin's and the miller's loaves' regrowth, once the chains that feed
them stand (R3). The manor pantry, the Heron's stores, and the fish barrel stay as stand-ins for
trade not yet built, marked so in the world file.
