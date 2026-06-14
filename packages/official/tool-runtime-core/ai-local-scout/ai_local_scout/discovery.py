from __future__ import annotations

from collections.abc import Iterable
from pathlib import Path

BOOTSTRAP_FILENAMES = {
    "CLAUDE.md",
    "AGENTS.md",
    "config.toml",
    "settings.json",
    "obsidian.json",
    "storage.json",
    "state.vscdb",
    "ConsoleHost_history.txt",
    "apps.json",
    "mcp.json",
    "mcp_servers.json",
    "auth.json",
    "default.rules",
    "libraryfolders.vdf",
    "Bookmarks",
    "History",
    "metaData",
    "manifest.json",
    "extensions.json",
    "sessionstore.jsonlz4",
    "recovery.jsonlz4",
    "previous.jsonlz4",
    "galaxy-installed.json",
    "config",
    "metadata.mfst",
    "product.db",
    "GameInstallInfo.sqlite",
    "MicrosoftGame.config",
    "butler.db",
}

OFFICE_DOCUMENT_EXTENSIONS = {".docx", ".pptx", ".xlsx"}

SKIP_DIR_NAMES = {
    ".cargo",
    ".hg",
    ".svn",
    ".venv",
    "venv",
    "__pycache__",
    "node_modules",
    ".next",
    ".nuxt",
    ".cache",
    ".npm",
    ".pnpm-store",
    ".yarn",
    "cache",
    "caches",
    "temp",
    "tmp",
    "logs",
}


def iter_bootstrap_candidates(roots: Iterable[Path], max_depth: int) -> list[Path]:
    candidates: list[Path] = []
    seen: set[str] = set()

    for root in roots:
        if not root.exists():
            continue
        root = root.resolve()

        if root.is_file():
            if _is_bootstrap_candidate(root):
                key = str(root).lower()
                if key not in seen:
                    seen.add(key)
                    candidates.append(root)
            continue

        for path in _walk(root, max_depth=max_depth):
            if not _is_bootstrap_candidate(path):
                continue
            key = str(path).lower()
            if key in seen:
                continue
            seen.add(key)
            candidates.append(path)

    candidates.sort(key=lambda item: str(item).lower())
    return candidates


def _walk(root: Path, max_depth: int) -> Iterable[Path]:
    queue: list[tuple[Path, int]] = [(root, 0)]

    while queue:
        current, depth = queue.pop(0)
        if depth > max_depth:
            continue
        if _should_skip_dir(current):
            continue

        try:
            entries = sorted(current.iterdir(), key=lambda item: item.name.lower())
        except OSError:
            continue

        for entry in entries:
            if entry.is_dir():
                if _should_skip_dir(entry):
                    continue
                queue.append((entry, depth + 1))
                continue
            yield entry


def _is_bootstrap_candidate(path: Path) -> bool:
    if path.suffix.lower() in OFFICE_DOCUMENT_EXTENSIONS and _is_desktop_office_document(path):
        return True
    if path.name in BOOTSTRAP_FILENAMES:
        if path.name == "config" and path.parent.name != ".git":
            return False
        if path.name == "manifest.json":
            parent = path.parent
            grandparent = parent.parent
            if not parent.name or not grandparent.name:
                return False
            return grandparent.parent.name == "Extensions"
        if path.name == "extensions.json":
            return "\\profiles\\" in str(path).lower().replace("/", "\\")
        if path.name == "galaxy-installed.json":
            normalized = str(path).lower().replace("/", "\\")
            return "\\gog.com\\galaxy\\storage\\" in normalized
        if path.name == "metaData":
            normalized = str(path).lower().replace("/", "\\")
            return "\\profiles\\" in normalized and "\\downloads\\metadata" in normalized
        if path.name == "obsidian.json":
            normalized = str(path).lower().replace("/", "\\")
            return normalized.endswith("\\appdata\\roaming\\obsidian\\obsidian.json")
        if path.name in {"sessionstore.jsonlz4", "recovery.jsonlz4", "previous.jsonlz4"}:
            return "\\profiles\\" in str(path).lower().replace("/", "\\")
        if path.name == "product.db":
            return "\\battle.net\\agent\\" in str(path).lower().replace("/", "\\")
        if path.name == "GameInstallInfo.sqlite":
            return "\\amazon games\\data\\games\\sql\\" in str(path).lower().replace("/", "\\")
        if path.name == "butler.db":
            return "\\itch\\db\\" in str(path).lower().replace("/", "\\")
        return True
    if path.parent.name == "games" and path.parent.parent.name == "library" and path.suffix.lower() == ".json":
        normalized = str(path).lower().replace("/", "\\")
        return "\\playnite\\library\\games\\" in normalized
    if path.suffix.lower() == ".mfst":
        normalized = str(path).lower().replace("/", "\\")
        return "\\programdata\\origin\\localcontent\\" in normalized or "\\programdata\\ea desktop\\localcontent\\" in normalized
    if path.parent.name == "Sessions" and path.name.startswith("tabs_") and path.suffix.lower() in {".json", ".bin"}:
        return True
    if path.name.startswith("appmanifest_") and path.suffix.lower() == ".acf":
        return True
    if path.suffix.lower() == ".code-workspace":
        return True
    if path.suffix.lower() in {".sqlite", ".sqlite3", ".db"}:
        return True
    return path.suffix.lower() == ".item"


def _is_desktop_office_document(path: Path) -> bool:
    if path.name.startswith("~$"):
        return False
    normalized_parts = {part.lower() for part in path.parts}
    return "desktop" in normalized_parts


def _should_skip_dir(path: Path) -> bool:
    name = path.name.lower()
    if name in SKIP_DIR_NAMES:
        return True

    normalized = str(path).lower().replace("/", "\\")
    return normalized.endswith("\\appdata\\local\\pip\\cache")
