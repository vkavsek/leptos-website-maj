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
from pathlib import Path
import stat
import sys
import tempfile
import zipfile


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
        elif not isinstance(record.get("url"), str):
            raise ValueError("Every video needs a URL string")


try:
    destination = Path("public/data_json")
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
            payload = archive.read(entry)
            validate(name, payload)
            (Path(staging_dir) / name).write_bytes(payload)
            staged.append(name)
        if not staged:
            raise ValueError("ZIP contains no content files")
        # Validate the entire archive before replacing any live file; never extract ZIP paths.
        for name in staged:
            os.replace(Path(staging_dir) / name, destination / name)
except (OSError, ValueError, RuntimeError, zipfile.BadZipFile) as error:
    sys.exit(f"Import failed: {error}")

print("Validated content copied to public/data_json. Rebuild and restart the server.")
PY
