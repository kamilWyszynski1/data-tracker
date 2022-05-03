CREATE TABLE tasks (
    uuid                TEXT PRIMARY KEY NOT NULL, 
    name                TEXT NOT NULL,
    description         TEXT NOT NULL,
    spreadsheet_id      TEXT NOT NULL,
    position            TEXT NOT NULL,    
    sheet               TEXT NOT NULL,
    direction           TEXT NOT NULL,
    interval_secs       INTEGER NOT NULL,
    with_timestamp      BOOLEAN NOT NULL,
    timestamp_position  TEXT NOT NULL,
    eval_forest         TEXT NOT NULL,
    input               TEXT NOT NULL,
    status              TEXT NOT NULL
);
