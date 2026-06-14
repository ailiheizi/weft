from __future__ import annotations

import os
import subprocess
from pathlib import Path


RUNTIME_ROOT = Path(r"D:\weft-workspace\weft-plugins\generic-agent-runtime\runtime-root")
CORE_BINARY = Path(r"D:\weft-workspace\weft\WEFT-core\target\release\weft-core.exe")


def main() -> int:
    env = os.environ.copy()
    env.setdefault("WEFT_CORE_MANAGED", "1")
    os.chdir(RUNTIME_ROOT)
    completed = subprocess.run([str(CORE_BINARY)], env=env)
    return int(completed.returncode)


if __name__ == "__main__":
    raise SystemExit(main())
