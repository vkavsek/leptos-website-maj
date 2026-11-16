-- Do not reuse deleted IDs: an old confirmation form must never delete a new row.
ALTER TABLE shows RENAME TO old_shows;
CREATE TABLE shows (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    name TEXT,
    club TEXT,
    location TEXT
);
INSERT INTO shows SELECT * FROM old_shows;
DROP TABLE old_shows;

ALTER TABLE videos RENAME TO old_videos;
CREATE TABLE videos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    video_id TEXT NOT NULL UNIQUE
        CHECK (length(video_id) = 11 AND video_id NOT GLOB '*[^a-zA-Z0-9_-]*')
);
INSERT INTO videos SELECT * FROM old_videos;
DROP TABLE old_videos;

ALTER TABLE gallery_images RENAME TO old_gallery_images;
CREATE TABLE gallery_images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    public_name TEXT NOT NULL UNIQUE,
    original_file TEXT NOT NULL UNIQUE,
    thumbnail_file TEXT NOT NULL UNIQUE
);
INSERT INTO gallery_images SELECT * FROM old_gallery_images;
DROP TABLE old_gallery_images;
