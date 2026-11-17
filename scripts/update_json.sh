#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <zip-file-path>" >&2
  exit 1
fi

exec python3 - "$1" <<'PY'
import datetime
import json
import os
import re
from pathlib import Path
import stat
import sys
import tempfile
import unicodedata
from urllib.parse import urlsplit, parse_qs
import zipfile

JSON_BYTES = 2 * 1024 * 1024
ARCHIVE_BYTES = 2 * JSON_BYTES + 64 * 1024


def video_id(value):
    if not isinstance(value, str) or len(value.encode("utf-8")) > 2048:
        raise ValueError("Every video needs a bounded URL string")
    url = urlsplit(value)
    if (url.scheme != "https" or url.username is not None or url.password is not None
            or url.port not in (None, 443) or url.fragment):
        raise ValueError("Use a supported HTTPS YouTube URL")
    parts = url.path.lstrip("/").split("/")
    host = url.hostname
    youtube = host in ("youtube.com", "www.youtube.com", "m.youtube.com")
    if host == "youtu.be" and len(parts) == 1:
        identifier = parts[0]
    elif youtube and parts == ["watch"]:
        values = parse_qs(url.query, keep_blank_values=True).get("v", [])
        identifier = values[0] if len(values) == 1 else ""
    elif (len(parts) == 2 and ((youtube and parts[0] in ("embed", "shorts", "live"))
            or (host in ("youtube-nocookie.com", "www.youtube-nocookie.com")
                and parts[0] == "embed"))):
        identifier = parts[1]
    else:
        raise ValueError("Unsupported YouTube URL")
    if not re.fullmatch(r"[A-Za-z0-9_-]{11}", identifier):
        raise ValueError("Invalid YouTube video ID")
    return identifier


def reject_constant(value):
    raise ValueError(f"Invalid JSON constant: {value}")


def unique_fields(pairs):
    record = {}
    for key, value in pairs:
        if key in record:
            raise ValueError(f"Duplicate JSON field: {key}")
        record[key] = value
    return record


def validate(name, payload):
    records = json.loads(payload.decode("utf-8"), parse_constant=reject_constant,
                         object_pairs_hook=unique_fields)
    if not isinstance(records, list):
        raise ValueError(f"{name} must contain an array")
    if len(records) > (10000 if name == "shows.json" else 1000):
        raise ValueError("Content record limit exceeded")
    seen_videos = set()
    for record in records:
        if not isinstance(record, dict):
            raise ValueError(f"{name} must contain objects")
        if name == "shows.json":
            for key in ("date", "name", "club", "location"):
                if record.get(key) is not None and not isinstance(record[key], str):
                    raise ValueError(f"{name}: {key} must be a string or null")
            if not isinstance(record.get("date"), str):
                raise ValueError("Every show needs a date")
            datetime.datetime.strptime(record["date"], "%d.%m.%Y")
            for key in ("name", "club", "location"):
                value = record.get(key) or ""
                if len(value.encode("utf-8")) > 500 or any(
                        unicodedata.category(char) == "Cc" for char in value):
                    raise ValueError("Show fields exceed the text limits")
        else:
            identifier = video_id(record.get("url"))
            if identifier in seen_videos:
                raise ValueError("Duplicate video")
            seen_videos.add(identifier)


try:
    destination = Path("seed/data_json")
    if Path(sys.argv[1]).stat().st_size > ARCHIVE_BYTES:
        raise ValueError("ZIP exceeds the archive byte limit")
    with zipfile.ZipFile(sys.argv[1]) as archive, tempfile.TemporaryDirectory(
        dir=destination, prefix=".import-"
    ) as staging_dir:
        staged = []
        for entry in archive.infolist():
            name = entry.filename
            file_type = stat.S_IFMT(entry.external_attr >> 16)
            if (name not in ("shows.json", "yt_videos.json") or name in staged
                    or file_type not in (0, stat.S_IFREG) or entry.external_attr & 0x10):
                raise ValueError("ZIP must contain only unique, regular shows.json or yt_videos.json files")
            if entry.file_size > JSON_BYTES:
                raise ValueError("JSON exceeds the 2 MiB limit")
            # Bound the actual expansion as well as the ZIP's declared size.
            with archive.open(entry) as file:
                payload = file.read(JSON_BYTES + 1)
            if len(payload) > JSON_BYTES:
                raise ValueError("JSON exceeds the 2 MiB limit")
            validate(name, payload)
            (Path(staging_dir) / name).write_bytes(payload)
            staged.append(name)
        if not staged:
            raise ValueError("ZIP contains no content files")
        # Validate the entire archive before replacing any live file; never extract ZIP paths.
        for name in staged:
            os.replace(Path(staging_dir) / name, destination / name)
except (OSError, ValueError, RuntimeError, RecursionError, zipfile.BadZipFile) as error:
    sys.exit(f"Import failed: {error}")

print("Validated content copied to seed/data_json. Use it for a new offline import; the live catalog is unchanged.")
PY
