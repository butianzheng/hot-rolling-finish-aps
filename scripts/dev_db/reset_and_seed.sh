#!/usr/bin/env bash
set -euo pipefail

# Reset+seed the dev database. Destructive: the existing DB file is backed up, then replaced.

DB_PATH="${1:-hot_rolling_aps.db}"

exec cargo run --bin reset_and_seed_full_scenario_db -- "$DB_PATH"

