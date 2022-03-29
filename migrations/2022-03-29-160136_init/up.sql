CREATE TABLE tasks (
    uuid TEXT PRIMARY KEY,
    name TEXT,
    spreadsheet_id TEXT,
    sheet TEXT,
    position TEXT,
    direction TEXT,
    interval_secs INTEGER,
    input_type TEXT,
    url TEXT,
    description TEXT,
    status TEXT,
    interval TEXT,
    with_timestamp BOOLEAN,
    timestamp_position TEXT,
    eval_forest TEXT
);
