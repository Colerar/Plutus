// @generated automatically by Diesel CLI.

diesel::table! {
    logs (id) {
        id -> Int8,
        room_id -> Int8,
        #[max_length = 128]
        command -> Varchar,
        raw_json -> Jsonb,
        time -> Timestamptz,
        related_uid -> Nullable<Int8>,
    }
}
