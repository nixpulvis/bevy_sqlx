CREATE TABLE foos (
    id    INT   PRIMARY KEY  NOT NULL,
    text  TEXT               NOT NULL,
    flag  BOOLEAN            NOT NULL DEFAULT 0
);

CREATE TABLE bars (
    foo_id    INT      NOT NULL,
    optional  VARCHAR,

    FOREIGN KEY(foo_id) REFERENCES foos(id)
);
