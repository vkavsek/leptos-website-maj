CREATE TABLE login_transactions (
    state_hash TEXT PRIMARY KEY NOT NULL,
    browser_hash TEXT NOT NULL,
    pkce_verifier TEXT NOT NULL,
    nonce TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);
CREATE INDEX login_transactions_expiry ON login_transactions (expires_at);

CREATE TABLE shows (
    id INTEGER PRIMARY KEY,
    date TEXT NOT NULL,
    name TEXT,
    club TEXT,
    location TEXT
);

CREATE TABLE videos (
    id INTEGER PRIMARY KEY,
    video_id TEXT NOT NULL UNIQUE
        CHECK (length(video_id) = 11 AND video_id NOT GLOB '*[^a-zA-Z0-9_-]*')
);

-- A row is published only after both private files have been prepared.
CREATE TABLE gallery_images (
    id INTEGER PRIMARY KEY,
    public_name TEXT NOT NULL UNIQUE,
    original_file TEXT NOT NULL UNIQUE,
    thumbnail_file TEXT NOT NULL UNIQUE
);

CREATE TABLE content_import (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    completed_at INTEGER NOT NULL
);
