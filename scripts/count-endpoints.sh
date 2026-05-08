#!/usr/bin/env bash
# Count total HTTP endpoints from the canonical list in docs/api-endpoints.md.
# Each "METHOD /path" line is one endpoint. Feature-gated routes are included.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
grep -cE '^(GET|POST|PUT|DELETE|PATCH) ' "$REPO_ROOT/docs/api-endpoints.md"
