table! {
    location (key) {
        key -> Text,
        value -> Integer,
    }
}

table! {
    tasks (uuid) {
        uuid -> Text,
        name -> Text,
        description -> Text,
        spreadsheet_id -> Text,
        position -> Text,
        sheet -> Text,
        direction -> Text,
        interval_secs -> Integer,
        with_timestamp -> Bool,
        timestamp_position -> Text,
        eval_forest -> Text,
        input -> Text,
        status -> Text,
    }
}

allow_tables_to_appear_in_same_query!(
    location,
    tasks,
);
