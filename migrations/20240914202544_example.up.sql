CREATE TABLE foos (
    id    INTEGER   PRIMARY KEY,
    text  TEXT      NOT NULL,
    flag  BOOLEAN   NOT NULL DEFAULT false
);

CREATE TABLE bars (
    id        INTEGER   PRIMARY KEY,
    foo_id    INTEGER   NOT NULL,
    optional  VARCHAR,

    FOREIGN KEY(foo_id) REFERENCES foos(id)
);
