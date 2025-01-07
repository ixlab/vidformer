CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- -- users table
-- CREATE TABLE "user" (
--     id UUID PRIMARY KEY DEFAULT uuid_generate_v4()
-- );

-- -- api_key table
-- CREATE TABLE api_key (
--     user_id UUID REFERENCES "user"(id) ON DELETE CASCADE,
--     name TEXT NOT NULL,
--     key TEXT NOT NULL,
--     PRIMARY KEY (user_id, name)
-- );

-- source table
CREATE TABLE source (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    -- user_id UUID REFERENCES "user"(id) ON DELETE CASCADE,
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

-- spec table
CREATE TABLE spec (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    -- user_id UUID REFERENCES "user"(id) ON DELETE CASCADE,
    width INT NOT NULL,
    height INT NOT NULL,
    pix_fmt TEXT NOT NULL,
    vod_segment_length_num INT NOT NULL,
    vod_segment_length_denom INT NOT NULL,
    ready_hook TEXT,
    steer_hook TEXT,
    applied_parts INT NOT NULL,
    terminated BOOLEAN NOT NULL,
    closed BOOLEAN NOT NULL
);

-- spec_t table
CREATE TABLE spec_t (
    spec_id UUID REFERENCES spec(id) ON DELETE CASCADE,
    pos INT NOT NULL,
    t_numer BIGINT NOT NULL,
    t_denom BIGINT NOT NULL,
    frame TEXT,
    PRIMARY KEY (spec_id, pos)
);

-- vod_segment table
CREATE TABLE vod_segment (
    spec_id UUID REFERENCES spec(id) ON DELETE CASCADE,
    segment_number INT NOT NULL,
    first_t INT NOT NULL,
    last_t INT NOT NULL,
    PRIMARY KEY (spec_id, segment_number),
    FOREIGN KEY (spec_id) REFERENCES spec(id),
    FOREIGN KEY (spec_id, first_t) REFERENCES spec_t(spec_id, pos),
    FOREIGN KEY (spec_id, last_t) REFERENCES spec_t(spec_id, pos),
    CHECK (first_t <= last_t)
);

-- vod_segment_source_dep table
CREATE TABLE vod_segment_source_dep (
    spec_id UUID REFERENCES spec(id) ON DELETE CASCADE,
    segment_number INT,
    source_id UUID REFERENCES source(id) ON DELETE CASCADE,
    PRIMARY KEY (spec_id, segment_number, source_id)
);

-- spec_part_staged table
CREATE TABLE spec_part_staged (
    spec_id UUID REFERENCES spec(id) ON DELETE CASCADE,
    pos INT NOT NULL,
    terminal BOOLEAN,
    PRIMARY KEY (spec_id, pos)
);

-- spec_part_staged_t table
CREATE TABLE spec_part_staged_t (
    spec_id UUID REFERENCES spec(id) ON DELETE CASCADE,
    pos INT NOT NULL,
    in_part_pos INT NOT NULL,
    t_numer BIGINT,
    t_denom BIGINT,
    frame TEXT,
    PRIMARY KEY (spec_id, pos, in_part_pos),
    FOREIGN KEY (spec_id, pos) REFERENCES spec_part_staged(spec_id, pos)
);
