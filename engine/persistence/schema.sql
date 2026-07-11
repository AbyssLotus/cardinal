-- Project Cardinal save schema (§14). One SQLite DB per save.
-- Definitions (YAML) are never copied here; rows reference def ids and
-- meta.yaml pins the world package version.

PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS world_clock (
    id      INTEGER PRIMARY KEY CHECK (id = 1),
    day     INTEGER NOT NULL DEFAULT 0,
    minute  INTEGER NOT NULL DEFAULT 0
);

-- runtime state of every live entity (NPCs, monsters, locations with mutable state)
CREATE TABLE IF NOT EXISTS entities (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,
    def_id      TEXT NOT NULL,
    state_json  TEXT NOT NULL DEFAULT '{}',
    location_id TEXT,
    updated_day INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS item_instances (
    id           TEXT PRIMARY KEY,
    def_id       TEXT NOT NULL,
    owner_id     TEXT,
    durability   INTEGER,
    plus         INTEGER NOT NULL DEFAULT 0,
    qty          INTEGER NOT NULL DEFAULT 1,
    equipped     INTEGER NOT NULL DEFAULT 0,
    history_json TEXT NOT NULL DEFAULT '[]'
);

-- §5.1 player memory
CREATE TABLE IF NOT EXISTS player (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    name        TEXT NOT NULL DEFAULT 'Player',
    level       INTEGER NOT NULL DEFAULT 1,
    xp          INTEGER NOT NULL DEFAULT 0,
    hp          INTEGER NOT NULL DEFAULT 100,
    hp_max      INTEGER NOT NULL DEFAULT 100,
    col         INTEGER NOT NULL DEFAULT 0,
    location_id TEXT NOT NULL,
    alive       INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS player_skills (
    skill_id      TEXT PRIMARY KEY,
    proficiency   REAL NOT NULL DEFAULT 0,
    equipped_slot INTEGER
);

CREATE TABLE IF NOT EXISTS player_reputation (
    scope_id TEXT PRIMARY KEY,          -- npc, faction, or settlement id
    value    REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS player_quest_log (
    quest_instance_id TEXT PRIMARY KEY,
    def_id            TEXT NOT NULL,
    state             TEXT NOT NULL,
    accepted_day      INTEGER
);

CREATE TABLE IF NOT EXISTS player_history (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    day       INTEGER NOT NULL,
    hour      INTEGER NOT NULL,
    kind      TEXT NOT NULL,             -- combat|quest|social|travel|craft|milestone
    summary   TEXT NOT NULL,
    refs_json TEXT NOT NULL DEFAULT '[]'
);

-- §5.2 NPC memory
CREATE TABLE IF NOT EXISTS npc_memory (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    npc_id         TEXT NOT NULL,
    day            INTEGER NOT NULL,
    hour           INTEGER NOT NULL,
    kind           TEXT NOT NULL,        -- conversation|promise|betrayal|assistance|combat|observation|rumor
    subject_id     TEXT,
    valence        REAL NOT NULL DEFAULT 0,
    salience       REAL NOT NULL DEFAULT 1.0,
    rumor_certainty REAL,
    summary        TEXT NOT NULL,
    decays         INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_npc_memory_npc ON npc_memory(npc_id);

-- §5.3 world memory (the chronicle) — append-only
CREATE TABLE IF NOT EXISTS chronicle (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    day         INTEGER NOT NULL,
    hour        INTEGER NOT NULL,
    category    TEXT NOT NULL,           -- boss_defeat|war|disaster|economy|politics|discovery|death
    headline    TEXT NOT NULL,
    detail      TEXT NOT NULL DEFAULT '',
    actors_json TEXT NOT NULL DEFAULT '[]',
    location_id TEXT,
    visibility  TEXT NOT NULL DEFAULT 'public'   -- public|regional|secret
);

CREATE TABLE IF NOT EXISTS markets (
    market_id  TEXT NOT NULL,
    item_def   TEXT NOT NULL,
    supply_idx REAL NOT NULL DEFAULT 1.0,
    demand_idx REAL NOT NULL DEFAULT 1.0,
    price      REAL NOT NULL DEFAULT 0,
    PRIMARY KEY (market_id, item_def)
);

CREATE TABLE IF NOT EXISTS quests (
    instance_id   TEXT PRIMARY KEY,
    def_id        TEXT NOT NULL,
    state         TEXT NOT NULL DEFAULT 'dormant',
    available_day INTEGER,
    expires_day   INTEGER
);

CREATE TABLE IF NOT EXISTS goals (
    npc_id        TEXT NOT NULL,
    goal_id       TEXT NOT NULL,
    progress_json TEXT NOT NULL DEFAULT '{}',
    status        TEXT NOT NULL DEFAULT 'active',
    PRIMARY KEY (npc_id, goal_id)
);

CREATE TABLE IF NOT EXISTS promises (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    npc_id             TEXT NOT NULL,
    subject_id         TEXT NOT NULL,
    due_condition_json TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'open'
);

-- active modifiers (curses, blessings, cyberware, tattoos…) on any actor.
-- owner_id: 'player' or an npc id. Vehicles live in `entities` (kind='vehicle').
CREATE TABLE IF NOT EXISTS modifiers (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_id     TEXT NOT NULL,
    def_id       TEXT NOT NULL,
    acquired_day INTEGER NOT NULL DEFAULT 0,
    expires_day  REAL,               -- NULL = permanent (fractional days for venoms)
    active       INTEGER NOT NULL DEFAULT 1,
    state_json   TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_modifiers_owner ON modifiers(owner_id);

CREATE TABLE IF NOT EXISTS rng_streams (
    name  TEXT PRIMARY KEY,
    state TEXT NOT NULL
);

-- catch-up bookkeeping (§6)
CREATE TABLE IF NOT EXISTS zone_ticks (
    zone_id         TEXT PRIMARY KEY,
    last_ticked_day INTEGER NOT NULL DEFAULT 0
);
