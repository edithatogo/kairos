from __future__ import annotations

import json
from pathlib import Path


def main() -> int:
    thresholds = Path("conductor/performance-thresholds.md").read_text(encoding="utf-8")
    current = {
        "schedule_1m_events_preview": 1.0,
        "hybrid_des_abm_smoke_preview": 1.0,
    }
    for name in current:
        if name not in thresholds:
            raise SystemExit(f"missing threshold for {name}")
    print(json.dumps(current, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
