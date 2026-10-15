#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
if [[ $(uname -s) != Linux ]]; then
  echo 'The serialized sync fixture requires Linux; use native core tests elsewhere.' >&2
  exit 1
fi
contention_dir=$(mktemp -d -t twodrive-contention.XXXXXX)
trap 'rm -rf -- "$contention_dir"' EXIT
cargo test -p twodrive-core --locked --no-run --message-format=json > "$contention_dir/build.jsonl"
contention_binary=$(python3 - "$contention_dir/build.jsonl" <<'PY'
import json
import sys
from pathlib import Path
binaries = [
    item['executable']
    for line in Path(sys.argv[1]).read_text().splitlines()
    if (item := json.loads(line)).get('reason') == 'compiler-artifact'
    and item['target']['name'] == 'twodrive_core'
    and item['profile']['test']
    and item.get('executable')
]
assert len(binaries) == 1, 'Expected exactly one core test executable'
print(binaries[0])
PY
)
cc -shared -fPIC -Wall -Wextra -Werror -pthread \
  scripts/test-support/serialized-sqlite-sync.c -ldl -o "$contention_dir/slow-sync.so"
LD_PRELOAD="$contention_dir/slow-sync.so${LD_PRELOAD:+:$LD_PRELOAD}" \
  "$contention_binary" --test-threads=32
