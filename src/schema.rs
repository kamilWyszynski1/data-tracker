table! {
    location (key) {
        key -> Text,
        value -> Integer,
    }
}

table! {
    reports (id) {
        id -> Integer,
        task_id -> Text,
        phases -> Text,
        failed -> Bool,
        start -> Timestamp,
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
        process -> Text,
        input -> Nullable<Text>,
        status -> Text,
        kind -> Text,
    }
}

joinable!(reports -> tasks (task_id));

allow_tables_to_appear_in_same_query!(location, reports, tasks,);
