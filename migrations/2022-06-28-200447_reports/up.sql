CREATE TABLE reports(
    id          INTEGER     NOT NULL PRIMARY KEY AUTOINCREMENT,
    task_id     TEXT        NOT NULL,
    phases      TEXT        NOT NULL,
    failed      BOOLEAN     NOT NULL,
    start       DATETIME    NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(uuid)
)