#!/bin/sh
set -eu

# Run from the repository root; keep originals unchanged for the photo viewer.
mkdir -p seed/img/about_thumbnails
for original in seed/img/about_pics/*; do
  [ -f "$original" ] || continue
  convert "$original" -auto-orient -thumbnail '480x270^' -gravity center \
    -extent 480x270 -strip -quality 78 \
    "seed/img/about_thumbnails/$(basename "$original").webp"
done
