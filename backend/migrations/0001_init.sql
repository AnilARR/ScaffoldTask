-- Core schema for the N+1 comprehensible-input learning platform.

CREATE TABLE IF NOT EXISTS profiles (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    level       TEXT NOT NULL,
    interests   JSONB NOT NULL DEFAULT '[]',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS courses (
    id          UUID PRIMARY KEY,
    slug        TEXT NOT NULL UNIQUE,
    title       TEXT NOT NULL,
    kind        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS concepts (
    id            UUID PRIMARY KEY,
    course_id     UUID NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    detail        TEXT,
    sequence      INTEGER NOT NULL DEFAULT 0,
    prerequisites JSONB NOT NULL DEFAULT '[]'
);
CREATE INDEX IF NOT EXISTS idx_concepts_course ON concepts(course_id);

CREATE TABLE IF NOT EXISTS content_items (
    id          UUID PRIMARY KEY,
    course_id   UUID NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    title       TEXT NOT NULL,
    front       TEXT NOT NULL DEFAULT '',
    back        TEXT NOT NULL DEFAULT '',
    concept_ids JSONB NOT NULL DEFAULT '[]',
    tags        JSONB NOT NULL DEFAULT '[]',
    difficulty  DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    source_url  TEXT
);
CREATE INDEX IF NOT EXISTS idx_items_course ON content_items(course_id);

CREATE TABLE IF NOT EXISTS freshness (
    profile_id  UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    concept_id  UUID NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    stability   DOUBLE PRECISION NOT NULL DEFAULT 0,
    difficulty  DOUBLE PRECISION NOT NULL DEFAULT 0,
    reps        INTEGER NOT NULL DEFAULT 0,
    lapses      INTEGER NOT NULL DEFAULT 0,
    last_review TIMESTAMPTZ,
    due         TIMESTAMPTZ,
    freshness   DOUBLE PRECISION NOT NULL DEFAULT 0,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (profile_id, concept_id)
);
CREATE INDEX IF NOT EXISTS idx_freshness_profile ON freshness(profile_id);

CREATE TABLE IF NOT EXISTS learning_events (
    id             UUID PRIMARY KEY,
    profile_id     UUID NOT NULL,
    concept_id     UUID NOT NULL,
    item_id        UUID NOT NULL,
    rating         SMALLINT NOT NULL,
    success_weight DOUBLE PRECISION NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_events_profile ON learning_events(profile_id);
CREATE INDEX IF NOT EXISTS idx_events_created ON learning_events(created_at);

-- Interest / tag tracking captured as the user interacts with URLs & media.
CREATE TABLE IF NOT EXISTS interaction_log (
    id          UUID PRIMARY KEY,
    profile_id  UUID NOT NULL,
    source_url  TEXT,
    kind        TEXT NOT NULL,
    detail      JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
