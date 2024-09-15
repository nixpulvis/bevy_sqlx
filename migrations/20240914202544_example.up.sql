CREATE TABLE foos (
    id    INTEGER   PRIMARY KEY,
    text  TEXT      NOT NULL,
    flag  BOOLEAN   NOT NULL DEFAULT 0
);

CREATE TABLE bars (
    id        INTEGER   PRIMARY KEY,
    foo_id    INTEGER   NOT NULL,
    optional  VARCHAR,

    FOREIGN KEY(foo_id) REFERENCES foos(id)
);
