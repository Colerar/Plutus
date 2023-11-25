CREATE TABLE logs (
   id               BIGSERIAL    PRIMARY KEY,
   room_id          BIGINT       NOT NULL,
   command          VARCHAR(128) NOT NULL,
   raw_json         JSONB        NOT NULL,
   "time"           timestamptz  NOT NULL,
   related_uid      BIGINT
);

CREATE INDEX logs_command_idx ON logs USING HASH (command);
CREATE INDEX logs_related_uid_idx ON logs USING HASH (related_uid);
CREATE INDEX logs_room_id_idx ON logs USING HASH (room_id);
CREATE INDEX logs_time_idx ON logs USING BTREE (room_id);
