#!/usr/bin/env bash
set -euo pipefail

# Reset+seed the dev database. Destructive: the existing DB file is backed up, then replaced.

case $# in
  0)
    exec cargo run --bin reset_and_seed_full_scenario_db --
    ;;
  1)
    exec cargo run --bin reset_and_seed_full_scenario_db -- "$1"
    ;;
  2)
    exec cargo run --bin reset_and_seed_full_scenario_db -- "$1" "$2"
    ;;
  *)
    echo "Usage: $0 [db_path] [material_count]" >&2
    exit 2
    ;;
esac
