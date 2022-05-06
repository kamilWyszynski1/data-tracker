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
        with_timestamp -> Bool,
        timestamp_position -> Text,
        eval_forest -> Text,
        input -> Nullable<Text>,
        status -> Text,
        kind -> Text,
    }
}

allow_tables_to_appear_in_same_query!(
    location,
    tasks,
);
