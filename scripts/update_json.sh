#!/usr/bin/env bash
set -eo pipefail

# Check if the file path is provided
if [ -z "$1" ]; then
  echo "Usage: $0 <zip-file-path>"
  exit 1
fi

# Get the file path from the first argument
ZIP_FILE_PATH="$1"

# Check if the provided file exists
if [ ! -f "$ZIP_FILE_PATH" ]; then
  echo "File not found: $ZIP_FILE_PATH"
  exit 1
fi

# Create a temporary directory to unzip the file
TEMP_DIR=$(mktemp -d)

# Unzip the file into the temporary directory
unzip "$ZIP_FILE_PATH" -d "$TEMP_DIR"

# Copy the unzipped files to the new folder
cp -r "$TEMP_DIR"/* "public/data_json/"

# Clean up the temporary directory
rm -rf "$TEMP_DIR"

echo "Files have been unzipped and copied to the 'data_json' directory."
