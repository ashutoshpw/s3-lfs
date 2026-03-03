#!/bin/bash

MAX_LINES=500
HAS_ERROR=0

for file in "$@"; do
  if [ -f "$file" ]; then
    lines=$(wc -l < "$file" | tr -d ' ')
    if [ "$lines" -gt "$MAX_LINES" ]; then
      echo "ERROR: $file has $lines lines (max: $MAX_LINES)"
      HAS_ERROR=1
    fi
  fi
done

if [ "$HAS_ERROR" -eq 1 ]; then
  echo ""
  echo "Please refactor the code into smaller components under 500 LOC for long term maintainability"
  exit 1
fi

echo "All .rs files are under $MAX_LINES lines ✓"
exit 0
