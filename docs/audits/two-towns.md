# Two Towns: Settlements That Run on Their Own Nodes

*Plan, 2026-10-10.* The user's direction: "get the world to a point where 2 cities can both
operate independently with their own nodes for resources as well as agent built nodes like
farms. This is setup for economy."

## 1. What "operate independently" asks

Two settlements, each fed and supplied by its own people working its own nodes, for weeks and
seasons, with nothing filling itself (A-47), and neither touching the other's nodes. They should
differ in what they have to spare, so that trade between them, the next step, has a reason.

**Settlements, not the city tier.** The city tier's residents (A-23) are records on day plans.
They eat from larders their shifts restock, and they never touch a node. Agents that sow, mine
and mend are organisms with minds, at the scene tier. So the two settlements are scene-tier
towns of a few dozen people:
- Ashford;
- and a new town, Fenwick, on Southfen, five kilometres west.

Bringing the city tier onto nodes (residents whose work yields what a node gives, at day-plan
grain) is its own project. It comes after this one, when trade gives a city something to
produce for.

## 2. What stands in the way

1. **Labour does not keep time (Law 14).** A go of work is one blow a tick, and a mind waits
   three ticks between goes. Putting things down is one thing a go. At ten-minute ticks a lump of
   ore takes hours, and unloading a cart takes an afternoon. Farms grow over months, and towns
   must run for seasons, which means coarse ticks. This comes first.
2. **Nobody owns a node as a town.** A mind leaves alone what it believes is someone else's
   (A-19). There is no "someone" that is a town, and no membership.
3. **Nobody makes a node.** Deposits are declared by the world, or dug as pits (A-42). A farm is
   a node people make: sown, grown, harvested, sown again.
4. **There is one town.**

## 3. Slices

### T1. Work that keeps time (A-48)

- **A go of work lasts the tick.** Taking from a deposit, digging, and mending are each done for
  the tick's length: as many goes as fit, at `rules.physical.go_seconds` each. One intent at
  one-minute ticks is a dozen blows; at ten-minute ticks, a hundred and twenty. A deposit yields
  every unit that work completes, not one at most. A unit takes at least one go, so fruit is a
  handful a go, not a tree a tick. Mending restores what that many goes restore. Tools wear by
  the go (A-46).
- **No more than the hand can carry.** Physical reports how many more units the hand has room for,
  and the deposit yields no more.
- **Many things at once.** Taking and putting down accept several things in one go: a load of
  sheaves set down together, a basket of loaves taken together.
- **Minds ask for a tick's work at a time.** A mind takes its work up again once it has seen
  the last go's yield, and puts down its whole load at once. (Re-asking every tick was tried and
  over-fetched: results still in flight are not yet seen.)

Decision still takes a few ticks (act, see, decide). At coarse ticks each decision now commands
a tick's work and a whole load, so a day's output depends on the hours worked, not the tick.

### T2. Settlements (A-49)

- **A settlement is an entity** (Society): its name, its members, its places.
- **Members** (`society.member_of`): each person belongs to one.
- **A settlement owns nodes** (Economy's ownership, A-19): its deposits, workplaces, and stores.
- **Minds count their settlement's things as theirs**, as they do their household's (A-26), and
  another settlement's as someone else's. Neither town's people take from the other's nodes.
  That is the ground trade will stand on.

### T3. Farms (A-50)

- **Sowing** (`physical.act.sow`, Ruling 13): with seed in hand, on open ground that is soil, not
  stone, paved, flooded, or under another field, a body sows a field there. A field is a new
  deposit, made by the act as a pit is (A-42), of the crop the world declares for that seed
  (`[crops]`): its size, how much it ripens to, how fast, and in what warmth and wet (A-41's
  `grows`). It grows from nothing.
- **A field is harvested once.** It ripens up to its yield. Taken bare after ripening, it is
  spent, and sown again to grow again. The seed is spent in the sowing.
- **Farming** (minds; a job kind, `sow`): keep so many fields of a crop growing at a place,
  sowing with seed from the store when fewer are. Harvesting is carrying from the fields to the
  store, as now.
- Ashford's farmer sows wheat on the farm. Fenwick's sows barley on the fen's edge.

### T4. Fenwick (A-51)

A fen town on Southfen. It is unlike Ashford in what it has, and short of what Ashford has:
- **Food:** eels from the meres (a living stock); cranberries; barley sown and harvested; barley
  ground on a hand quern and baked.
- **Its own:**
  - peat, cut by hand (fuel, burnt in the salt house);
  - reeds;
  - clay;
  - brine from a salt spring, a flow, boiled with peat into salt.
- **No iron.** Its tools are few and wear out. That is the first reason to trade.
- About a dozen people in three households, each with a home. Every store is kept by a job, and
  nothing fills itself.

### T5. Two towns for a month

At ten-minute ticks:
- both towns' people eat, and nobody starves;
- every trade in each is worked;
- fields are sown and harvested by agents;
- no one takes from the other town's nodes.

Each settlement's production and stocks by material are counted. That is the ledger an economy
will start from.

## 4. Not in this plan

- Trade, markets, money, prices.
- Construction: agents building workplaces, houses, roads. Farms are the first node agents make;
  building is the next.
- The city tier on nodes.
- Labour's remaining tick cost: a decision takes a few ticks at any tick length.

## 5. Status

- **T1, built:** A-48, `tests/reference/tests/work_time.rs`.
- T2 to T5: not begun. The user asked to talk before going on.
