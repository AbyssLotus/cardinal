# The Environment, Finished: Sky, Seasons, Water, and Air Between Places

*Plan, 2026-10-07.* The user's direction: before minds and perception go further, go back to the
roots of the world and finish the environment, and the interactions between environments.

## 1. What the spec asks for

Vol. III Ch. 1 §1.10 lists the environmental state Physical Reality owns: temperature, pressure,
humidity, wind, illumination, water level, soil moisture, air quality (and radiation and magic,
for worlds that want them). It names predictable change (sunrise, sunset, seasons, tides) and
emergent change (wildfires, storms, floods). §1.11 adds that water flows downhill and thick fog
limits visibility. Vol. III Ch. 10 expects habitat inputs from here: terrain, water, climate,
seasonal cycles, fire and flood. The system sweep's M12 asked for the same: seasons and a
calendar, real precipitation, cloud, and fog, water, and heat flow between indoors and outdoors.

## 2. What is built

- Climates: regions carrying a temperature. Each runs its own weather, an independent
  mean-reverting anomaly for temperature, humidity, and pressure. Everything nested inside a
  climate inherits it.
- A day/night temperature swing, damped by exposure and thermal mass.
- The sun: light follows its height over a day fixed at the equinox on the equator. Night is
  dark, with no moon.
- Humidity about one world-wide baseline. "Rain" is humidity past a line, read only by fire.
- Pressure falls with elevation. Wind blows down the gradient to the lowest neighbour. Storms
  and gales.
- Walled rooms: sheltered, lit through their openings, their air following the outside.
- Fire's heat, warmth, and smoke in walled rooms (A-28, A-32).

## 3. What is missing

- **No calendar.** No year, no seasons, no change in day length, no moon.
- **No sky.** No cloud, no measured rain or snow, no fog.
- **Climates do not touch.** Neighbouring climates' weather is independent: no front ever crosses
  from the moor into the Vale.
- **Air does not pass through openings.** A room follows only the outside air through its walls.
  An open door to the next room, or to the yard, carries no warmth and no smoke. Smoke exists
  only indoors, so a burning hay wagon fills nothing.
- **Nothing gets wet.** Rain falls on nothing. The ground holds no water and no snow. Wet timber
  burns as readily as dry. Wind and wet do not chill anyone.
- **No water.** No pond, river, or sea, no flood, no ice, no tide.

## 4. Slices

1. **Calendar, seasons, and the sun (A-34).** A year and a latitude. The sun's height follows
   the season, so days lengthen and shorten and noon is lower in winter. Temperature follows
   the seasons with a lag. A moon with phases lights the night, by the world's measure.
2. **The sky (A-35).** Humidity normals per climate. Cloud from humidity: it dims the day and
   holds the night's warmth. Rain and snow as rates, snow below freezing. Fog in still, damp
   air. Fog, like smoke, shortens sight.
3. **Fronts (A-36).** Neighbouring climates' weather mixes, and the wind carries the upwind
   climate's weather downwind: a cold spell on the moor reaches the Vale.
4. **Air between places (A-37).** Air passes through open openings, by their size, faster in a
   draught. Warmth and smoke cross between rooms and to the outside. Fires in the open smoke
   their place, and that smoke drifts in through open doors.
5. **Wet and snow (A-38).** Ground water and snow cover per climate: rain soaks in, snow lies
   and melts, the ground dries. Things in the open get wet in rain and dry after. Wet things
   burn less readily. Wind and wet chill a body (Living's judgment of Physical's report).
6. **Water (A-39).** Ponds, rivers, and the sea as places with water levels. Rain raises them;
   they drain downstream; past their banks they flood the low ground beside them. They freeze
   below zero, and the sea has tides.

Each slice is tested in Ashford, keeps Law 14 (rates in simulated time, exact at any tick
length), and costs nothing where nothing happens: no rain, no wet; no fire, no smoke.

## 5. Not in this plan

- Sound propagating as a field: it is what a body could hear, and belongs with perception.
- Temperature falling with height within a climate: a climate declares its own normal, as
  Highmoor does.
- Weather at the city tier beyond what climates already give.

## 6. Status

See the end of this file once slices land.
