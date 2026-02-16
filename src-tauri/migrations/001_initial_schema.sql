-- Cameras
CREATE TABLE IF NOT EXISTS cameras (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    brand           TEXT NOT NULL,
    model           TEXT NOT NULL,
    prefix          TEXT,
    format          TEXT NOT NULL,
    camera_type     TEXT,
    serial_number   TEXT,
    date_purchased  TEXT,
    purchased_from  TEXT,
    date_sold       TEXT,
    notes           TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Camera maintenance records
CREATE TABLE IF NOT EXISTS camera_maintenance (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    camera_id         INTEGER NOT NULL REFERENCES cameras(id) ON DELETE CASCADE,
    maintenance_type  TEXT NOT NULL,
    done_by           TEXT,
    date_done         TEXT,
    cost              REAL,
    notes             TEXT,
    created_at        TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_camera_maintenance_camera ON camera_maintenance(camera_id);

-- Lenses
CREATE TABLE IF NOT EXISTS lenses (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    brand                   TEXT NOT NULL,
    lens_system             TEXT,
    name_on_lens            TEXT,
    focal_length            TEXT,
    max_aperture            TEXT,
    min_aperture            TEXT,
    filter_thread_front_mm  INTEGER,
    filter_thread_rear_mm   INTEGER,
    serial_number           TEXT,
    date_purchased          TEXT,
    purchased_from          TEXT,
    date_sold               TEXT,
    notes                   TEXT,
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Camera-lens associations
CREATE TABLE IF NOT EXISTS camera_lenses (
    camera_id   INTEGER NOT NULL REFERENCES cameras(id) ON DELETE CASCADE,
    lens_id     INTEGER NOT NULL REFERENCES lenses(id) ON DELETE CASCADE,
    PRIMARY KEY (camera_id, lens_id)
);

-- Film stocks
CREATE TABLE IF NOT EXISTS film_stocks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    brand           TEXT NOT NULL,
    name            TEXT NOT NULL,
    format          TEXT NOT NULL,
    exposure_count  INTEGER,
    stock_type      TEXT NOT NULL,
    iso             INTEGER,
    notes           TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(brand, name, format)
);

-- Labs
CREATE TABLE IF NOT EXISTS labs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    location    TEXT,
    website     TEXT,
    notes       TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Rolls (central entity)
CREATE TABLE IF NOT EXISTS rolls (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    roll_id         TEXT NOT NULL UNIQUE,
    camera_id       INTEGER REFERENCES cameras(id) ON DELETE SET NULL,
    film_stock_id   INTEGER REFERENCES film_stocks(id) ON DELETE SET NULL,
    status          TEXT NOT NULL DEFAULT 'loaded'
                    CHECK (status IN ('loaded', 'shooting', 'shot', 'at-lab', 'developing', 'developed', 'scanned', 'archived')),
    frame_count     INTEGER,
    date_loaded     TEXT,
    date_finished   TEXT,
    date_fuzzy      TEXT,
    push_pull       TEXT,
    notes           TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_rolls_camera ON rolls(camera_id);
CREATE INDEX IF NOT EXISTS idx_rolls_film_stock ON rolls(film_stock_id);
CREATE INDEX IF NOT EXISTS idx_rolls_status ON rolls(status);

-- Shots
CREATE TABLE IF NOT EXISTS shots (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    roll_id         INTEGER NOT NULL REFERENCES rolls(id) ON DELETE CASCADE,
    frame_number    TEXT NOT NULL,
    aperture        TEXT,
    shutter_speed   TEXT,
    date            TEXT,
    date_fuzzy      TEXT,
    location        TEXT,
    gps_lat         REAL,
    gps_lon         REAL,
    notes           TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(roll_id, frame_number)
);
CREATE INDEX IF NOT EXISTS idx_shots_roll ON shots(roll_id);

-- Shot-lens associations
CREATE TABLE IF NOT EXISTS shot_lenses (
    shot_id     INTEGER NOT NULL REFERENCES shots(id) ON DELETE CASCADE,
    lens_id     INTEGER NOT NULL REFERENCES lenses(id) ON DELETE CASCADE,
    PRIMARY KEY (shot_id, lens_id)
);

-- Lab development
CREATE TABLE IF NOT EXISTS development_lab (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    roll_id             INTEGER NOT NULL REFERENCES rolls(id) ON DELETE CASCADE,
    lab_id              INTEGER REFERENCES labs(id) ON DELETE SET NULL,
    date_dropped_off    TEXT,
    date_received       TEXT,
    cost                REAL,
    notes               TEXT,
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_dev_lab_roll ON development_lab(roll_id);

-- Self development
CREATE TABLE IF NOT EXISTS development_self (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    roll_id             INTEGER NOT NULL REFERENCES rolls(id) ON DELETE CASCADE,
    date_processed      TEXT,
    developer           TEXT,
    developer_dilution  TEXT,
    fixer               TEXT,
    fixer_dilution      TEXT,
    stop_bath           TEXT,
    wetting_agent       TEXT,
    clearing_agent      TEXT,
    temperature         TEXT,
    agitation_notes     TEXT,
    notes               TEXT,
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_dev_self_roll ON development_self(roll_id);

-- Development stages (timing for each step)
CREATE TABLE IF NOT EXISTS dev_stages (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    development_self_id INTEGER NOT NULL REFERENCES development_self(id) ON DELETE CASCADE,
    stage_name          TEXT NOT NULL,
    duration_seconds    INTEGER,
    notes               TEXT,
    sort_order          INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_dev_stages_dev ON dev_stages(development_self_id);
