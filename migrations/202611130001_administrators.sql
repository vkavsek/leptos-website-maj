-- Approval is always explicit: migrations must never insert an administrator.
CREATE TABLE administrators (
    id INTEGER PRIMARY KEY,
    issuer TEXT NOT NULL CHECK (length(issuer) > 0),
    subject TEXT NOT NULL CHECK (length(subject) > 0),
    label TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    session_generation INTEGER NOT NULL DEFAULT 0 CHECK (session_generation >= 0),
    UNIQUE (issuer, subject)
);
