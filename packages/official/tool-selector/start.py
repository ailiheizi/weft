"""Auto-start wrapper for tool-selector HTTP service.

This is the entry point used by Core's service lifecycle (`runtime = "service"`).
It starts the selector HTTP server with correct paths derived from WEFT_PACKAGE_DIR.
"""
import os
import sys
from pathlib import Path

# Derive paths from WEFT_PACKAGE_DIR (set by Core) or script location
package_dir = Path(os.environ.get("WEFT_PACKAGE_DIR", str(Path(__file__).parent)))
service_dir = package_dir / "service"
libraries_dir = service_dir / "libraries"

# Build argv as if called: python server.py --http --port 17860 --libraries-dir <path>
sys.argv = [
    str(service_dir / "server.py"),
    "--http",
    "--port", "17860",
    "--libraries-dir", str(libraries_dir),
]

# Import and run server with correct __file__ so SCRIPT_DIR resolves to service/
sys.path.insert(0, str(service_dir))
server_path = service_dir / "server.py"
server_globals = {
    "__file__": str(server_path),
    "__name__": "__main__",
}
exec(compile(open(server_path, encoding="utf-8").read(), str(server_path), "exec"), server_globals)
