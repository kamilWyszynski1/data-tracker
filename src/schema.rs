table! {
    location (key) {
        key -> Text,
        value -> Integer,
    }
}

table! {
    tasks (uuid) {
        uuid -> Nullable<Text>,
        name -> Nullable<Text>,
        spreadsheet_id -> Nullable<Text>,
        sheet -> Nullable<Text>,
        position -> Nullable<Text>,
        direction -> Nullable<Text>,
        interval_secs -> Nullable<Integer>,
        input_type -> Nullable<Text>,
        url -> Nullable<Text>,
        description -> Nullable<Text>,
        status -> Nullable<Text>,
        interval -> Nullable<Text>,
        with_timestamp -> Nullable<Bool>,
        timestamp_position -> Nullable<Text>,
        eval_forest -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(
    location,
    tasks,
);
