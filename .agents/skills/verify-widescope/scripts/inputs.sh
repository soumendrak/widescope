#!/usr/bin/env bash
{ git ls-files -co --exclude-standard; echo Cargo.lock; } | grep -vE '^\.(agents|github)/|\.md$|^ui/e2e/|\.test\.ts$' | while IFS= read -r file; do [ -e "$file" ] && echo "$file"; done
