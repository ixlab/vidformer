CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- users table
CREATE TABLE "user" (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    api_key VARCHAR(32) NOT NULL
);

CREATE INDEX user_api_key_idx ON "user"(api_key);

-- source table
CREATE TABLE source (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    stream_idx INT NOT NULL,
    storage_service TEXT NOT NULL,
    storage_config JSONB NOT NULL,
    codec TEXT NOT NULL, 
    pix_fmt TEXT NOT NULL,
    width INT NOT NULL,
    height INT NOT NULL,
    file_size BIGINT NOT NULL
);

-- source_t table
CREATE TABLE source_t (
    source_id UUID REFERENCES source(id) ON DELETE CASCADE,
    pos INT NOT NULL,
    key BOOLEAN  NOT NULL,
    t_num BIGINT  NOT NULL,
    t_denom BIGINT  NOT NULL,
    PRIMARY KEY (source_id, pos)
);

CREATE TABLE spec (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    width INT NOT NULL,
    height INT NOT NULL,
    pix_fmt TEXT NOT NULL,
    vod_segment_length_num BIGINT NOT NULL,
    vod_segment_length_denom BIGINT NOT NULL,
    frame_rate_num BIGINT NOT NULL,
    frame_rate_denom BIGINT NOT NULL,
    pos_discontinuity INT NOT NULL,
    pos_terminal INT,
    closed BOOLEAN NOT NULL,
    ready_hook TEXT,
    steer_hook TEXT
);

-- spec_t table
CREATE TABLE spec_t (
    spec_id UUID REFERENCES spec(id) ON DELETE CASCADE,
    pos INT NOT NULL,
    frame TEXT,
    PRIMARY KEY (spec_id, pos)
);

CREATE TABLE spec_source_dependency (
    spec_id UUID REFERENCES spec(id) ON DELETE CASCADE,
    source_id UUID REFERENCES source(id),
    PRIMARY KEY (spec_id, source_id)
);
