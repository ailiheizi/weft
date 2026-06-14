from __future__ import annotations

import contextlib
import json
import socketserver
import sqlite3
import threading
import time
import zipfile
import pytest
from pathlib import Path
from http.server import BaseHTTPRequestHandler, HTTPServer


def _write(path: Path, content: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return path


def _write_bytes(path: Path, content: bytes) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)
    return path


def _write_minimal_docx(path: Path, paragraphs: list[str]) -> Path:
    body = "".join(f"<w:p><w:r><w:t>{text}</w:t></w:r></w:p>" for text in paragraphs)
    document_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        f"<w:body>{body}</w:body>"
        "</w:document>"
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("word/document.xml", document_xml)
    return path


def _write_minimal_pptx(path: Path, slides: list[list[str]]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w") as archive:
        for index, texts in enumerate(slides, start=1):
            runs = "".join(f"<a:p><a:r><a:t>{text}</a:t></a:r></a:p>" for text in texts)
            slide_xml = (
                '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                '<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
                'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">'
                f"<p:cSld><p:spTree><p:sp><p:txBody>{runs}</p:txBody></p:sp></p:spTree></p:cSld>"
                "</p:sld>"
            )
            archive.writestr(f"ppt/slides/slide{index}.xml", slide_xml)
    return path


def _write_minimal_xlsx(path: Path, sheets: dict[str, list[list[str]]]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    workbook_sheets = []
    workbook_rels = []
    with zipfile.ZipFile(path, "w") as archive:
        for index, (sheet_name, rows) in enumerate(sheets.items(), start=1):
            workbook_sheets.append(f'<sheet name="{sheet_name}" sheetId="{index}" r:id="rId{index}"/>')
            workbook_rels.append(
                f'<Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{index}.xml"/>'
            )
            row_xml = []
            for row_index, row in enumerate(rows, start=1):
                cells = []
                for col_index, value in enumerate(row, start=1):
                    column = chr(ord("A") + col_index - 1)
                    cells.append(f'<c r="{column}{row_index}" t="inlineStr"><is><t>{value}</t></is></c>')
                row_xml.append(f'<row r="{row_index}">{"".join(cells)}</row>')
            archive.writestr(
                f"xl/worksheets/sheet{index}.xml",
                '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
                f'<sheetData>{"".join(row_xml)}</sheetData>'
                "</worksheet>",
            )
        archive.writestr(
            "xl/workbook.xml",
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
            'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
            f'<sheets>{"".join(workbook_sheets)}</sheets>'
            "</workbook>",
        )
        archive.writestr(
            "xl/_rels/workbook.xml.rels",
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            f'{"".join(workbook_rels)}'
            "</Relationships>",
        )
    return path


def _assert_contains_fields(items: list[dict], expected: dict) -> dict:
    for item in items:
        if all(item.get(key) == value for key, value in expected.items()):
            return item
    raise AssertionError(f"expected item with fields {expected!r} in {items!r}")


def _pb_varint(value: int) -> bytes:
    chunks: list[int] = []
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            chunks.append(byte | 0x80)
        else:
            chunks.append(byte)
            break
    return bytes(chunks)


def _pb_field(field_number: int, value: bytes) -> bytes:
    return _pb_varint((field_number << 3) | 2) + _pb_varint(len(value)) + value


def _pb_string(field_number: int, value: str) -> bytes:
    return _pb_field(field_number, value.encode("utf-8"))


def _battle_net_product_install(uid: str, product_code: str, install_path: str) -> bytes:
    user_settings = _pb_string(1, install_path)
    product_install = (
        _pb_string(1, uid)
        + _pb_string(2, product_code)
        + _pb_field(3, user_settings)
    )
    return _pb_field(1, product_install)


def _battle_net_product_db(*installs: bytes) -> bytes:
    return b"".join(installs)


def _install_runtime_probe_stub(monkeypatch: pytest.MonkeyPatch, *, wsl_stdout: str = "") -> None:
    class _Completed:
        def __init__(self, stdout: str, returncode: int = 0):
            self.stdout = stdout
            self.stderr = ""
            self.returncode = returncode

    def _fake_run(command, *args, **kwargs):  # noqa: ANN001
        if isinstance(command, list) and command[:2] in (["wsl.exe", "--list"], ["wsl", "--list"]):
            return _Completed(wsl_stdout)
        if isinstance(command, list) and command[:1] == ["wsl"]:
            return _Completed(wsl_stdout)
        if not (isinstance(command, list) and len(command) >= 4 and command[:3] == ["powershell", "-NoProfile", "-Command"]):
            raise AssertionError(f"unexpected subprocess call: {command!r}")
        script = command[3]
        if "CurrentVersion\\Uninstall" in script:
            return _Completed("[]")
        if "SOFTWARE\\ubisoft\\Launcher\\Installs" in script and "$battleGames" in script:
            return _Completed('{"ubisoft":[],"battleNet":[]}')
        if "Get-AppxPackage" in script:
            return _Completed("")
        if "WScript.Shell" in script and "Recent" in script:
            return _Completed("[]")
        raise AssertionError(f"unexpected subprocess call: {command!r}")

    monkeypatch.setattr("ai_local_scout.runtime.subprocess.run", _fake_run)


def _isolate_machine_probes(monkeypatch: pytest.MonkeyPatch, *, wsl_stdout: str = "") -> None:
    _install_runtime_probe_stub(monkeypatch, wsl_stdout=wsl_stdout)
    monkeypatch.setattr("ai_local_scout.runtime._probe_steam_library_indexes", lambda config: [])


def test_run_scout_reuses_machine_probe_cache_with_same_subprocess_runner(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    home.mkdir()
    calls: list[str] = []

    class _Completed:
        def __init__(self, stdout: str, returncode: int = 0):
            self.stdout = stdout
            self.stderr = ""
            self.returncode = returncode

    def _fake_run(command, *args, **kwargs):  # noqa: ANN001
        key = " ".join(command) if isinstance(command, list) else str(command)
        calls.append(key)
        if isinstance(command, list) and command[:2] in (["wsl.exe", "--list"], ["wsl", "--list"]):
            return _Completed("  NAME      STATE           VERSION\n* Ubuntu    Running         2\n")
        if isinstance(command, list) and len(command) >= 4 and command[:3] == ["powershell", "-NoProfile", "-Command"]:
            script = command[3]
            if "CurrentVersion\\Uninstall" in script:
                return _Completed("[]")
            if "SOFTWARE\\ubisoft\\Launcher\\Installs" in script and "$battleGames" in script:
                return _Completed('{"ubisoft":[],"battleNet":[]}')
            if "Get-AppxPackage" in script:
                return _Completed("")
        raise AssertionError(f"unexpected subprocess call: {command!r}")

    monkeypatch.setattr("ai_local_scout.runtime.subprocess.run", _fake_run)
    monkeypatch.setattr("ai_local_scout.runtime._probe_steam_library_indexes", lambda config: [])

    config = ScoutConfig(roots=[home], home=home, max_depth=1, max_ai_expansions=0)
    first = run_scout(config)
    call_count_after_first = len(calls)
    second = run_scout(config)

    assert first["derived_profile"]["linux_runtime_profile"]["default_distro"] == "Ubuntu"
    assert second["derived_profile"]["linux_runtime_profile"]["default_distro"] == "Ubuntu"
    assert call_count_after_first > 0
    assert len(calls) == call_count_after_first


def test_windows_registry_probes_share_single_powershell_inventory(monkeypatch: pytest.MonkeyPatch) -> None:
    from ai_local_scout import runtime

    calls: list[str] = []

    class _Completed:
        def __init__(self, stdout: str, returncode: int = 0):
            self.stdout = stdout
            self.stderr = ""
            self.returncode = returncode

    def _fake_run(command, *args, **kwargs):  # noqa: ANN001
        if isinstance(command, list) and len(command) >= 4 and command[:3] == ["powershell", "-NoProfile", "-Command"]:
            calls.append(command[3])
            return _Completed(
                json.dumps(
                    {
                        "installedApps": [
                            {
                                "DisplayName": "Visual Studio Code",
                                "DisplayVersion": "1.99.0",
                                "Publisher": "Microsoft Corporation",
                                "InstallLocation": "C:\\VSCode",
                            }
                        ],
                        "ubisoft": [],
                        "battleNet": [],
                    }
                )
            )
        raise AssertionError(f"unexpected subprocess call: {command!r}")

    monkeypatch.setattr("ai_local_scout.runtime.subprocess.run", _fake_run)

    installed_apps = runtime._probe_windows_installed_apps()
    launcher_installs = runtime._probe_windows_launcher_installs()

    assert installed_apps[0]["name"] == "Visual Studio Code"
    assert launcher_installs == []
    assert len(calls) == 1


def test_run_scout_skips_xbox_appx_probe_when_no_xbox_roots(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    home.mkdir()

    def _unexpected_appx_probe():
        raise AssertionError("xbox Appx scan should not run when no Xbox roots or explicit config paths exist")

    class _Completed:
        def __init__(self, stdout: str, returncode: int = 0):
            self.stdout = stdout
            self.stderr = ""
            self.returncode = returncode

    def _fake_run(command, *args, **kwargs):  # noqa: ANN001
        if isinstance(command, list) and command[:2] in (["wsl.exe", "--list"], ["wsl", "--list"]):
            return _Completed("")
        if isinstance(command, list) and len(command) >= 4 and command[:3] == ["powershell", "-NoProfile", "-Command"]:
            script = command[3]
            if "Get-AppxPackage" in script:
                raise AssertionError("xbox Appx scan should not run when no Xbox roots or explicit config paths exist")
            if "CurrentVersion\\Uninstall" in script:
                return _Completed(json.dumps({"installedApps": [], "ubisoft": [], "battleNet": []}))
        raise AssertionError(f"unexpected subprocess call: {command!r}")

    monkeypatch.setattr("ai_local_scout.runtime.subprocess.run", _fake_run)
    monkeypatch.setattr("ai_local_scout.runtime._probe_steam_library_indexes", lambda config: [])
    monkeypatch.setattr("ai_local_scout.runtime._probe_xbox_appx_install_locations", _unexpected_appx_probe)

    run_scout(ScoutConfig(roots=[home], home=home, max_depth=1, max_ai_expansions=0))


def _run_editor_fixture_scout(fixture: dict[str, Path], *, max_depth: int = 7) -> dict:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    return run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
                fixture["uninstall_root"],
            ],
            home=fixture["home"],
            max_depth=max_depth,
        )
    )


def _add_breadth_fixture_artifacts(home: Path, *, obs_profile_name: str = "Streaming") -> None:
    _write(
        home / "AppData" / "Roaming" / "Obsidian" / "obsidian.json",
        json.dumps(
            {
                "vaults": {
                    "main": {
                        "path": str(home / "notes"),
                        "open": True,
                    }
                }
            }
        ),
    )
    _write(home / "notes" / ".obsidian" / "core-plugins.json", json.dumps(["file-explorer", "search"]))
    _write(home / "notes" / "games.md", "# Games\n")
    _write(
        home / "AppData" / "Roaming" / "obs-studio" / "basic" / "profiles" / obs_profile_name / "basic.ini",
        f"[General]\nName={obs_profile_name}\n[Video]\nBaseCX=1920\nBaseCY=1080\nOutputCX=1280\nOutputCY=720\n",
    )
    _write(
        home / "AppData" / "Local" / "Syncthing" / "config.xml",
        '<configuration><folder id="notes" path="D:\\notes" type="sendreceive" /></configuration>',
    )
    _write(
        home / "AppData" / "Local" / "Packages" / "Microsoft.WindowsTerminal_8wekyb3d8bbwe" / "LocalState" / "settings.json",
        json.dumps({"profiles": {"defaults": {}, "list": [{"name": "PowerShell", "source": "Windows.Terminal.PowershellCore"}]}}),
    )
    _write(home / ".wslconfig", "[wsl2]\nmemory=8GB\n")
    _write(home / ".docker" / "config.json", json.dumps({"currentContext": "desktop-linux"}))
    _write(home / ".aws" / "config", "[profile personal]\nregion = us-west-2\n")


def _build_fixture(tmp_path: Path) -> dict[str, Path]:
    home = tmp_path / "home"
    project = tmp_path / "workspace" / "demo-project"
    steam_root = tmp_path / "Steam"
    steam_library = tmp_path / "SteamLibrary"
    steam_root_windows = steam_root.as_posix().replace("/", "\\")
    steam_library_windows = steam_library.as_posix().replace("/", "\\")

    _write(
        home / ".claude" / "CLAUDE.md",
        "\n".join(
            [
                "# Claude Context",
                "",
                "Read `AI-Knowledge/as-me.md` before answering.",
            ]
        ),
    )
    _write(
        home / ".claude" / "AI-Knowledge" / "as-me.md",
        "\n".join(
            [
                "# As Me",
                "",
                "- core_identity: builder",
                "- likes: games",
            ]
        ),
    )
    _write(
        home / ".codex" / "config.toml",
        "\n".join(
            [
                'model = "gpt-5.4"',
                "",
                f"[projects.'{project.as_posix()}']",
                'trust_level = "trusted"',
                "",
                "[mcp_servers.context-sync]",
                'command = "node"',
            ]
        ),
    )
    _write(project / "AGENTS.md", "# Project Rules\n\n- stay concise\n")
    _write(
        steam_root / "steamapps" / "libraryfolders.vdf",
        "\n".join(
            [
                '"libraryfolders"',
                "{",
                '  "0"',
                "  {",
                f'    "path" "{steam_root_windows}"',
                "  }",
                '  "1"',
                "  {",
                f'    "path" "{steam_library_windows}"',
                '    "apps"',
                "    {",
                '      "570" "1"',
                "    }",
                "  }",
                "}",
            ]
        ),
    )
    _write(
        steam_library / "steamapps" / "appmanifest_570.acf",
        "\n".join(
            [
                '"AppState"',
                "{",
                '  "appid" "570"',
                '  "name" "Dota 2"',
                '  "installdir" "dota 2 beta"',
                '  "SizeOnDisk" "123"',
                '  "StateFlags" "4"',
                "}",
            ]
        ),
    )
    _write(
        steam_library / "steamapps" / "common" / "dota 2 beta" / "dota2.exe",
        "",
    )

    return {
        "home": home,
        "project": project,
        "steam_root": steam_root,
        "steam_library": steam_library,
    }


def _build_expanded_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_fixture(tmp_path)
    home = fixture["home"]
    epic_manifest_dir = tmp_path / "ProgramData" / "Epic" / "EpicGamesLauncher" / "Data" / "Manifests"

    _write(
        home / ".claude" / "settings.json",
        json.dumps(
            {
                "defaultModel": "claude-sonnet",
                "apiKey": "claude-secret-token",
                "mcpServers": {
                    "memory": {"command": "node"},
                    "files": {"command": "python"},
                },
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": "anthropic-secret",
                    "OPENAI_API_KEY": "openai-secret",
                    "SAFE_MODE": "on",
                },
                "enabledPlugins": ["memory", "browser"],
                "plugins": {
                    "filesystem": {"enabled": True},
                    "search": {"enabled": False},
                },
            }
        ),
    )
    _write(
        home / ".claude" / "mcp.json",
        json.dumps(
            {
                "servers": {
                    "memory": {"command": "node"},
                    "calendar": {"command": "python"},
                }
            }
        ),
    )
    _write(
        home / ".codex" / "auth.json",
        json.dumps(
            {
                "access_token": "codex-secret-token",
                "user": {"email": "leo@example.com"},
            }
        ),
    )
    _write(
        home / ".codex" / "rules" / "default.rules",
        "\n".join(
            [
                'prefix_rule(pattern=["python"], decision="allow")',
                'prefix_rule(pattern=["git", "status"], decision="allow")',
            ]
        ),
    )
    _write(
        epic_manifest_dir / "fortnite.item",
        json.dumps(
            {
                "DisplayName": "Fortnite",
                "AppName": "Fortnite",
                "CatalogItemId": "fortnite-id",
                "InstallLocation": "D:\\Epic Games\\Fortnite",
                "LaunchExecutable": "FortniteGame\\Binaries\\Win64\\FortniteLauncher.exe",
                "InstallSize": "456",
                "TechnicalType": "game",
            }
        ),
    )

    fixture["epic_manifest_dir"] = epic_manifest_dir
    return fixture


def _build_browser_and_workspace_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_expanded_fixture(tmp_path)
    home = fixture["home"]
    browser_profile = home / "AppData" / "Local" / "Google" / "Chrome" / "User Data" / "Default"
    repo_root = fixture["project"]
    workspace_file = fixture["project"].parent / "demo.code-workspace"
    sqlite_path = tmp_path / "data" / "notes.sqlite3"

    _write(
        browser_profile / "Bookmarks",
        json.dumps(
            {
                "roots": {
                    "bookmark_bar": {
                        "children": [
                            {
                                "type": "url",
                                "name": "Weft Docs",
                                "url": "https://example.com/weft-docs",
                            },
                            {
                                "type": "url",
                                "name": "OpenAI",
                                "url": "https://openai.com/",
                            },
                        ]
                    }
                }
            }
        ),
    )

    history_path = browser_profile / "History"
    history_path.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(history_path) as connection:
        connection.execute("CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER DEFAULT 0)")
        connection.execute(
            "INSERT INTO urls(url, title, visit_count) VALUES (?, ?, ?)",
            ("https://example.com/weft", "Weft Home", 9),
        )
        connection.execute(
            "INSERT INTO urls(url, title, visit_count) VALUES (?, ?, ?)",
            ("https://docs.example.com/scout", "Scout Docs", 14),
        )
        connection.execute(
            "INSERT INTO urls(url, title, visit_count) VALUES (?, ?, ?)",
            ("chrome://settings/", "Settings", 3),
        )
        connection.execute(
            "INSERT INTO urls(url, title, visit_count) VALUES (?, ?, ?)",
            ("https://error/", "error", 2),
        )
        connection.commit()

    git_dir = repo_root / ".git"
    git_dir.mkdir(parents=True, exist_ok=True)
    _write(
        git_dir / "config",
        "\n".join(
            [
                "[core]",
                "\trepositoryformatversion = 0",
                "\tbare = false",
                '[remote "origin"]',
                "\turl = https://github.com/example/demo-project.git",
                '[branch "main"]',
                "\tremote = origin",
                "\tmerge = refs/heads/main",
            ]
        ),
    )
    _write(
        workspace_file,
        json.dumps(
            {
                "folders": [
                    {"path": str(repo_root)},
                    {"path": str(fixture["steam_root"])},
                ]
            }
        ),
    )

    sqlite_path.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(sqlite_path) as connection:
        connection.execute("CREATE TABLE notes (id INTEGER PRIMARY KEY, title TEXT)")
        connection.execute("INSERT INTO notes(title) VALUES (?)", ("alpha",))
        connection.execute("INSERT INTO notes(title) VALUES (?)", ("beta",))
        connection.commit()

    fixture["browser_profile"] = browser_profile
    fixture["workspace_file"] = workspace_file
    fixture["sqlite_path"] = sqlite_path
    return fixture


def _build_editor_shell_and_apps_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_browser_and_workspace_fixture(tmp_path)
    home = fixture["home"]
    appdata_roaming = home / "AppData" / "Roaming"
    vscode_storage = appdata_roaming / "Code" / "User" / "globalStorage"
    cursor_storage = appdata_roaming / "Cursor" / "User" / "globalStorage"
    powershell_history = appdata_roaming / "Microsoft" / "Windows" / "PowerShell" / "PSReadLine" / "ConsoleHost_history.txt"
    uninstall_root = tmp_path / "registry-export"

    repo_root = fixture["project"]
    another_workspace = tmp_path / "workspace" / "secondary-project"
    another_workspace.mkdir(parents=True, exist_ok=True)

    _write(
        vscode_storage / "storage.json",
        json.dumps(
            {
                "lastKnownMenubarData": {
                    "workspaces3": [
                        {"folderUri": repo_root.as_uri()},
                        {"configPath": (fixture["workspace_file"]).as_uri()},
                    ]
                }
            }
        ),
    )
    _write(
        cursor_storage / "storage.json",
        json.dumps(
            {
                "profileAssociations": {
                    another_workspace.as_posix(): "default"
                },
                "lastOpenFiles": [
                    {"folderUri": another_workspace.as_uri()},
                ],
            }
        ),
    )
    _write(
        powershell_history,
        "\n".join(
            [
                "git status",
                "cd D:\\weft",
                "python -m pytest",
                "claude doctor",
                "git status",
            ]
        ),
    )
    _write(
        uninstall_root / "apps.json",
        json.dumps(
            {
                "items": [
                    {
                        "DisplayName": "Visual Studio Code",
                        "DisplayVersion": "1.99.0",
                        "Publisher": "Microsoft",
                        "InstallLocation": "C:\\Program Files\\Microsoft VS Code",
                    },
                    {
                        "DisplayName": "Cursor",
                        "DisplayVersion": "0.50.0",
                        "Publisher": "Cursor",
                        "InstallLocation": "C:\\Users\\Admin\\AppData\\Local\\Programs\\Cursor",
                    },
                    {
                        "DisplayName": "Claude Code",
                        "DisplayVersion": "1.2.3",
                        "Publisher": "Anthropic",
                        "InstallLocation": "C:\\Users\\Admin\\AppData\\Roaming\\npm\\node_modules\\@anthropic-ai\\claude-code",
                    },
                    {
                        "DisplayName": "Steam",
                        "DisplayVersion": "2.10.91.91",
                        "Publisher": "Valve",
                        "InstallLocation": "C:\\Program Files (x86)\\Steam",
                    },
                    {
                        "DisplayName": "NVIDIA Graphics Driver",
                        "DisplayVersion": "555.12",
                        "Publisher": "NVIDIA Corporation",
                        "InstallLocation": "C:\\Program Files\\NVIDIA Corporation",
                    },
                    {
                        "DisplayName": "Logitech G HUB",
                        "DisplayVersion": "2026.1",
                        "Publisher": "Logitech",
                        "InstallLocation": "C:\\Program Files\\LGHUB",
                    },
                    {
                        "DisplayName": "Blender",
                        "DisplayVersion": "4.3",
                        "Publisher": "Blender Foundation",
                        "InstallLocation": "C:\\Program Files\\Blender Foundation\\Blender",
                    },
                    {
                        "DisplayName": "Bitwarden",
                        "DisplayVersion": "2026.1",
                        "Publisher": "Bitwarden Inc.",
                        "InstallLocation": "C:\\Program Files\\Bitwarden",
                    },
                    {
                        "DisplayName": "Tailscale",
                        "DisplayVersion": "1.80.0",
                        "Publisher": "Tailscale Inc.",
                        "InstallLocation": "C:\\Program Files\\Tailscale",
                    },
                ]
            }
        ),
    )

    fixture["vscode_storage"] = vscode_storage
    fixture["cursor_storage"] = cursor_storage
    fixture["powershell_history"] = powershell_history
    fixture["uninstall_root"] = uninstall_root
    fixture["another_workspace"] = another_workspace
    return fixture


def _build_multibrowser_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_browser_and_workspace_fixture(tmp_path)
    home = fixture["home"]
    edge_profile = home / "AppData" / "Local" / "Microsoft" / "Edge" / "User Data" / "Default"
    brave_profile = home / "AppData" / "Local" / "BraveSoftware" / "Brave-Browser" / "User Data" / "Default"
    firefox_profile = home / "AppData" / "Roaming" / "Mozilla" / "Firefox" / "Profiles" / "default-release"

    _write(
        edge_profile / "Bookmarks",
        json.dumps(
            {
                "roots": {
                    "bookmark_bar": {
                        "children": [
                            {
                                "type": "url",
                                "name": "Edge Docs",
                                "url": "https://learn.microsoft.com/edge",
                            }
                        ]
                    }
                }
            }
        ),
    )
    edge_history = edge_profile / "History"
    edge_history.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(edge_history) as connection:
        connection.execute("CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER DEFAULT 0)")
        connection.execute(
            "INSERT INTO urls(url, title, visit_count) VALUES (?, ?, ?)",
            ("https://github.com/microsoft/edge", "Edge GitHub", 6),
        )
        connection.commit()

    _write(
        brave_profile / "Bookmarks",
        json.dumps(
            {
                "roots": {
                    "bookmark_bar": {
                        "children": [
                            {
                                "type": "url",
                                "name": "OpenAI Platform",
                                "url": "https://platform.openai.com/docs",
                            }
                        ]
                    }
                }
            }
        ),
    )
    brave_history = brave_profile / "History"
    brave_history.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(brave_history) as connection:
        connection.execute("CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER DEFAULT 0)")
        connection.execute(
            "INSERT INTO urls(url, title, visit_count) VALUES (?, ?, ?)",
            ("https://platform.openai.com/docs", "OpenAI Docs", 20),
        )
        connection.commit()

    firefox_places = firefox_profile / "places.sqlite"
    firefox_places.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(firefox_places) as connection:
        connection.execute(
            "CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER DEFAULT 0)"
        )
        connection.execute(
            "CREATE TABLE moz_bookmarks (id INTEGER PRIMARY KEY, fk INTEGER, type INTEGER, parent INTEGER, title TEXT)"
        )
        connection.execute(
            "INSERT INTO moz_places(id, url, title, visit_count) VALUES (1, ?, ?, ?)",
            ("https://developer.mozilla.org/", "MDN", 12),
        )
        connection.execute(
            "INSERT INTO moz_places(id, url, title, visit_count) VALUES (2, ?, ?, ?)",
            ("https://firefox-source-docs.mozilla.org/", "Firefox Source Docs", 7),
        )
        connection.execute(
            "INSERT INTO moz_bookmarks(id, fk, type, parent, title) VALUES (1, 1, 1, 0, 'MDN')"
        )
        connection.commit()

    fixture["edge_profile"] = edge_profile
    fixture["brave_profile"] = brave_profile
    fixture["firefox_profile"] = firefox_profile
    fixture["firefox_places"] = firefox_places
    return fixture


def _write_chromium_browser_profile(
    profile: Path,
    *,
    bookmark_title: str,
    bookmark_url: str,
    history_entries: list[tuple[str, str, int]],
) -> None:
    _write(
        profile / "Bookmarks",
        json.dumps(
            {
                "roots": {
                    "bookmark_bar": {
                        "children": [
                            {
                                "type": "url",
                                "name": bookmark_title,
                                "url": bookmark_url,
                            }
                        ]
                    }
                }
            }
        ),
    )
    history_path = profile / "History"
    history_path.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(history_path) as connection:
        connection.execute("CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER DEFAULT 0)")
        for url, title, visit_count in history_entries:
            connection.execute(
                "INSERT INTO urls(url, title, visit_count) VALUES (?, ?, ?)",
                (url, title, visit_count),
            )
        connection.commit()


def _build_extended_windows_browser_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_multibrowser_fixture(tmp_path)
    home = fixture["home"]

    arc_profile = (
        home
        / "AppData"
        / "Local"
        / "Packages"
        / "TheBrowserCompany.Arc_ttt1ap7aakyb4"
        / "LocalCache"
        / "Local"
        / "Arc"
        / "User Data"
        / "Default"
    )
    opera_profile = home / "AppData" / "Roaming" / "Opera Software" / "Opera Stable"
    opera_gx_profile = home / "AppData" / "Roaming" / "Opera Software" / "Opera GX Stable"
    vivaldi_profile = home / "AppData" / "Local" / "Vivaldi" / "User Data" / "Profile 1"
    zen_profile = home / "AppData" / "Roaming" / "zen" / "Profiles" / "default-release"

    _write_chromium_browser_profile(
        arc_profile,
        bookmark_title="Arc Notes",
        bookmark_url="https://arc.net/",
        history_entries=[
            ("https://arc.net/", "Arc", 11),
        ],
    )
    _write_chromium_browser_profile(
        opera_profile,
        bookmark_title="Opera Start",
        bookmark_url="https://www.opera.com/",
        history_entries=[
            ("https://www.opera.com/features", "Opera Features", 8),
        ],
    )
    _write_chromium_browser_profile(
        opera_gx_profile,
        bookmark_title="GX Games",
        bookmark_url="https://gx.games/",
        history_entries=[
            ("https://gx.games/", "GX Games", 9),
        ],
    )
    _write_chromium_browser_profile(
        vivaldi_profile,
        bookmark_title="Vivaldi Blog",
        bookmark_url="https://vivaldi.com/blog/",
        history_entries=[
            ("https://vivaldi.com/", "Vivaldi", 10),
        ],
    )

    zen_places = zen_profile / "places.sqlite"
    zen_places.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(zen_places) as connection:
        connection.execute(
            "CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER DEFAULT 0)"
        )
        connection.execute(
            "CREATE TABLE moz_bookmarks (id INTEGER PRIMARY KEY, fk INTEGER, type INTEGER, parent INTEGER, title TEXT)"
        )
        connection.execute(
            "INSERT INTO moz_places(id, url, title, visit_count) VALUES (1, ?, ?, ?)",
            ("https://zen-browser.app/", "Zen Browser", 0),
        )
        connection.execute(
            "INSERT INTO moz_places(id, url, title, visit_count) VALUES (2, ?, ?, ?)",
            ("https://docs.zen-browser.app/", "Zen Docs", 13),
        )
        connection.execute(
            "INSERT INTO moz_bookmarks(id, fk, type, parent, title) VALUES (1, 1, 1, 0, 'Zen Browser')"
        )
        connection.commit()

    fixture["arc_profile"] = arc_profile
    fixture["opera_profile"] = opera_profile
    fixture["opera_gx_profile"] = opera_gx_profile
    fixture["vivaldi_profile"] = vivaldi_profile
    fixture["zen_profile"] = zen_profile
    fixture["zen_places"] = zen_places
    return fixture


def _append_chromium_history_rows(path: Path, rows: list[tuple[str, str, int]]) -> None:
    with sqlite3.connect(path) as connection:
        for url, title, visit_count in rows:
            connection.execute(
                "INSERT INTO urls(url, title, visit_count) VALUES (?, ?, ?)",
                (url, title, visit_count),
            )
        connection.commit()


def _append_firefox_places_rows(path: Path, rows: list[tuple[str, str, int]]) -> None:
    with sqlite3.connect(path) as connection:
        next_id_row = connection.execute("SELECT COALESCE(MAX(id), 0) FROM moz_places").fetchone()
        next_id = int(next_id_row[0] or 0) + 1
        for url, title, visit_count in rows:
            connection.execute(
                "INSERT INTO moz_places(id, url, title, visit_count) VALUES (?, ?, ?, ?)",
                (next_id, url, title, visit_count),
            )
            next_id += 1
        connection.commit()


def _build_search_activity_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_multibrowser_fixture(tmp_path)
    chrome_history = fixture["browser_profile"] / "History"
    edge_history = fixture["edge_profile"] / "History"
    firefox_places = fixture["firefox_places"]

    _append_chromium_history_rows(
        chrome_history,
        [
            ("https://www.google.com/search?q=local+ai+context&sourceid=chrome", "Google Search", 5),
            ("https://www.google.com/search?q=playwright+fixtures", "Google Search", 2),
        ],
    )
    _append_chromium_history_rows(
        edge_history,
        [
            ("https://www.bing.com/search?q=local+ai+context&FORM=QSRE1", "Bing Search", 4),
        ],
    )
    _append_firefox_places_rows(
        firefox_places,
        [
            ("https://duckduckgo.com/?q=playwright+fixtures&ia=web", "DuckDuckGo Search", 6),
            ("https://search.brave.com/search?q=codex+config+windows", "Brave Search", 3),
        ],
    )

    return fixture


def _build_consumer_intent_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_search_activity_fixture(tmp_path)
    chrome_history = fixture["browser_profile"] / "History"
    edge_history = fixture["edge_profile"] / "History"
    firefox_places = fixture["firefox_places"]

    _append_chromium_history_rows(
        chrome_history,
        [
            ("https://www.google.com/search?q=bilibili+anime", "Google Search", 7),
            ("https://www.google.com/search?q=steam+deck+dock", "Google Search", 5),
        ],
    )
    _append_chromium_history_rows(
        edge_history,
        [
            ("https://www.bing.com/search?q=discord+servers", "Bing Search", 6),
        ],
    )
    _append_firefox_places_rows(
        firefox_places,
        [
            ("https://duckduckgo.com/?q=epic+games+sale&ia=web", "DuckDuckGo Search", 4),
        ],
    )

    return fixture


def _build_query_normalization_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_search_activity_fixture(tmp_path)
    chrome_history = fixture["browser_profile"] / "History"
    edge_history = fixture["edge_profile"] / "History"
    firefox_places = fixture["firefox_places"]

    _append_chromium_history_rows(
        chrome_history,
        [
            ("https://www.google.com/search?q=bilibili", "Google Search", 2),
            ("https://www.google.com/search?q=B%E7%AB%99", "Google Search", 3),
            ("https://www.google.com/search?q=discord", "Google Search", 1),
        ],
    )
    _append_chromium_history_rows(
        edge_history,
        [
            ("https://www.bing.com/search?q=b+%E7%AB%99", "Bing Search", 4),
            ("https://www.bing.com/search?q=discord+servers", "Bing Search", 5),
        ],
    )
    _append_firefox_places_rows(
        firefox_places,
        [
            ("https://duckduckgo.com/?q=steam+deck&ia=web", "DuckDuckGo Search", 3),
            ("https://duckduckgo.com/?q=steam&ia=web", "DuckDuckGo Search", 2),
        ],
    )

    return fixture


def _build_browser_artifact_v2_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_multibrowser_fixture(tmp_path)
    chrome_profile = fixture["browser_profile"]
    firefox_profile = fixture["firefox_profile"]
    chrome_history = chrome_profile / "History"

    with sqlite3.connect(chrome_history) as connection:
        connection.execute(
            """
            CREATE TABLE downloads (
                id INTEGER PRIMARY KEY,
                target_path TEXT,
                tab_url TEXT,
                site_url TEXT,
                mime_type TEXT,
                received_bytes INTEGER,
                total_bytes INTEGER,
                state INTEGER
            )
            """
        )
        connection.execute(
            """
            CREATE TABLE downloads_url_chains (
                id INTEGER,
                chain_index INTEGER,
                url TEXT
            )
            """
        )
        connection.execute(
            """
            INSERT INTO downloads(id, target_path, tab_url, site_url, mime_type, received_bytes, total_bytes, state)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                1,
                r"C:\Users\Demo\Downloads\setup.exe",
                "https://download.example.com/app",
                "https://example.com",
                "application/octet-stream",
                42,
                100,
                1,
            ),
        )
        connection.execute(
            "INSERT INTO downloads_url_chains(id, chain_index, url) VALUES (?, ?, ?)",
            (1, 0, "https://cdn.example.com/setup.exe"),
        )
        connection.commit()

    _write(
        chrome_profile / "Extensions" / "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" / "1.2.3" / "manifest.json",
        json.dumps(
            {
                "manifest_version": 3,
                "name": "Tab Manager",
                "version": "1.2.3",
                "description": "Manage tabs",
                "permissions": ["tabs", "storage"],
                "host_permissions": ["https://*.example.com/*"],
            }
        ),
    )
    _write(
        firefox_profile / "extensions.json",
        json.dumps(
            {
                "addons": [
                    {
                        "id": "reader@example",
                        "defaultLocale": {"name": "Reader Helper", "description": "Read pages"},
                        "version": "4.5.6",
                        "type": "extension",
                        "active": True,
                    },
                    {
                        "id": "theme@example",
                        "defaultLocale": {"name": "Theme Pack"},
                        "version": "1.0.0",
                        "type": "theme",
                        "active": True,
                    },
                ]
            }
        ),
    )

    return fixture


def _build_browser_session_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_browser_artifact_v2_fixture(tmp_path)
    chrome_profile = fixture["browser_profile"]
    firefox_profile = fixture["firefox_profile"]

    downloads_meta = firefox_profile / "downloads" / "metaData"
    downloads_meta.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(downloads_meta) as connection:
        connection.execute(
            """
            CREATE TABLE moz_downloads (
                id INTEGER PRIMARY KEY,
                source TEXT,
                target TEXT
            )
            """
        )
        connection.execute(
            "INSERT INTO moz_downloads(id, source, target) VALUES (?, ?, ?)",
            (
                1,
                "https://addons.mozilla.org/firefox/downloads/file.xpi",
                "/home/demo/Downloads/helper.xpi",
            ),
        )
        connection.commit()

    _write(
        chrome_profile / "Sessions" / "tabs_001.json",
        json.dumps(
            {
                "tabs": [
                    {"url": "https://github.com/openai/openai-python", "title": "openai-python"},
                    {"url": "https://platform.openai.com/docs", "title": "OpenAI Docs"},
                ]
            }
        ),
    )
    _write(
        firefox_profile / "sessionstore.jsonlz4",
        json.dumps(
            {
                "windows": [
                    {
                        "tabs": [
                            {
                                "entries": [
                                    {"url": "https://support.mozilla.org/", "title": "Mozilla Support"}
                                ]
                            }
                        ]
                    }
                ]
            }
        ),
    )

    return fixture


def _build_firefox_mozlz4_session_fixture(tmp_path: Path) -> dict[str, Path]:
    fixture = _build_browser_artifact_v2_fixture(tmp_path)
    firefox_profile = fixture["firefox_profile"]

    lz4_frame = pytest.importorskip("lz4.frame")
    payload = json.dumps(
        {
            "windows": [
                {
                    "tabs": [
                        {
                            "entries": [
                                {"url": "https://addons.mozilla.org/", "title": "Firefox Add-ons"}
                            ]
                        },
                        {
                            "entries": [
                                {"url": "https://example.org/docs", "title": "Example Docs"}
                            ]
                        },
                    ]
                }
            ]
        }
    ).encode("utf-8")
    mozlz4_bytes = b"mozLz40\0" + lz4_frame.compress(payload)
    session_path = firefox_profile / "recovery.jsonlz4"
    session_path.parent.mkdir(parents=True, exist_ok=True)
    session_path.write_bytes(mozlz4_bytes)
    fixture["firefox_session_path"] = session_path
    return fixture


def test_run_scout_builds_evidence_and_profile(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[fixture["home"], fixture["project"].parent, fixture["steam_root"]],
            home=fixture["home"],
            max_depth=6,
        )
    )

    assert report["bootstrap_evidence"], "expected bootstrap evidence"
    assert report["search_trace"], "expected AI search trace"
    assert report["raw_evidence"], "expected raw evidence"
    assert any(
        entry["path"].endswith("CLAUDE.md") and entry["discovered_by"] == "bootstrap"
        for entry in report["bootstrap_evidence"]
    )
    assert any(
        entry["path"].endswith("config.toml") and entry["entity_kind"] == "codex_config"
        for entry in report["raw_evidence"]
    )
    assert report["derived_profile"]["tools"]["claude"]["present"] is True
    assert report["derived_profile"]["tools"]["codex"]["present"] is True
    assert fixture["project"].as_posix() in report["derived_profile"]["tools"]["codex"]["trusted_projects"]
    game_names = {item["name"] for item in report["derived_profile"]["game_ecosystem"]["installed_games"]}
    assert "Dota 2" in game_names


def test_cli_writes_json_report(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    fixture = _build_fixture(tmp_path)
    output_path = tmp_path / "report.json"

    exit_code = main(
        [
            "--root",
            str(fixture["home"]),
            "--root",
            str(fixture["project"].parent),
            "--root",
            str(fixture["steam_root"]),
            "--home",
            str(fixture["home"]),
            "--output",
            str(output_path),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    assert report["derived_profile"]["tools"]["claude"]["present"] is True
    assert report["derived_profile"]["game_ecosystem"]["platforms"] == ["steam"]


def test_cli_run_alias_writes_json_report_to_stdout(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    from ai_local_scout.cli import main

    fixture = _build_fixture(tmp_path)

    exit_code = main(
        [
            "run",
            "--root",
            str(fixture["home"]),
            "--root",
            str(fixture["steam_root"]),
            "--home",
            str(fixture["home"]),
            "--max-depth",
            "6",
        ]
    )

    assert exit_code == 0
    report = json.loads(capsys.readouterr().out)
    assert report["derived_profile"]["tools"]["claude"]["present"] is True
    assert report["derived_profile"]["game_ecosystem"]["platforms"] == ["steam"]


def test_ai_search_trace_records_follow_up_reasoning(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[fixture["home"], fixture["project"].parent, fixture["steam_root"]],
            home=fixture["home"],
            max_depth=6,
        )
    )

    actions = [entry["action"] for entry in report["search_trace"]]
    assert "follow_markdown_reference" in actions
    assert "expand_steam_library" in actions
    assert any(entry["discovered_by"] == "bootstrap" for entry in report["raw_evidence"])
    assert any(entry["discovered_by"] == "ai" for entry in report["raw_evidence"])


def test_parse_claude_entrypoint_filters_non_path_backticks(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_claude_entrypoint

    path = _write(
        tmp_path / "CLAUDE.md",
        "\n".join(
            [
                "# Root",
                "Read `AI-Knowledge/as-me.md` first.",
                "Do not emit `[OUTDATED]` or `not a path`.",
            ]
        ),
    )

    parsed = parse_claude_entrypoint(path)

    assert parsed["referenced_paths"] == ["AI-Knowledge/as-me.md"]


def test_run_scout_redacts_sensitive_tooling_and_merges_epic(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_expanded_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
            ],
            home=fixture["home"],
            max_depth=6,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "claude_settings" in kinds
    assert "claude_mcp_config" in kinds
    assert "codex_auth" in kinds
    assert "codex_rules" in kinds
    assert "epic_game_manifest" in kinds

    auth_entry = next(entry for entry in report["raw_evidence"] if entry["entity_kind"] == "codex_auth")
    assert auth_entry["fields"]["access_token"] == "[REDACTED]"

    claude_settings = next(entry for entry in report["raw_evidence"] if entry["entity_kind"] == "claude_settings")
    assert claude_settings["fields"]["apiKey"] == "[REDACTED]"
    assert claude_settings["fields"]["env"]["ANTHROPIC_AUTH_TOKEN"] == "[REDACTED]"
    assert claude_settings["fields"]["env"]["OPENAI_API_KEY"] == "[REDACTED]"
    assert claude_settings["fields"]["env"]["SAFE_MODE"] == "on"
    assert claude_settings["fields"]["env_keys"] == ["ANTHROPIC_AUTH_TOKEN", "OPENAI_API_KEY", "SAFE_MODE"]
    assert claude_settings["fields"]["package_names"] == ["browser", "filesystem", "memory", "search"]
    assert claude_settings["fields"]["enabled_plugins"] == ["browser", "filesystem", "memory"]

    assert report["redactions"], "expected redaction records"
    assert any(item["path"].endswith("auth.json") for item in report["redactions"])
    assert any(item["field"] == "env.ANTHROPIC_AUTH_TOKEN" for item in report["redactions"])
    assert any(item["field"] == "env.OPENAI_API_KEY" for item in report["redactions"])

    assert report["derived_profile"]["tools"]["codex"]["auth_present"] is True
    assert report["derived_profile"]["tools"]["codex"]["approved_rule_count"] == 2
    assert "memory" in report["derived_profile"]["tools"]["claude"]["mcp_servers"]
    assert "calendar" in report["derived_profile"]["tools"]["claude"]["mcp_servers"]

    platforms = report["derived_profile"]["game_ecosystem"]["platforms"]
    assert platforms == ["epic", "steam"]
    game_names = {item["name"] for item in report["derived_profile"]["game_ecosystem"]["installed_games"]}
    assert "Fortnite" in game_names
    assert "Dota 2" in game_names


def test_run_scout_discovers_browser_git_workspace_and_sqlite(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_browser_and_workspace_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=7,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "browser_bookmarks" in kinds
    assert "browser_history" in kinds
    assert "git_repo_config" in kinds
    assert "workspace_file" in kinds
    assert "sqlite_database" in kinds

    browser_profile = report["derived_profile"]["browser_activity"]
    assert "chrome" in browser_profile["browsers"]
    assert "Weft Docs" in browser_profile["bookmark_titles"]
    assert "example.com" in browser_profile["recent_history_domains"]
    assert "docs.example.com" in browser_profile["recent_history_domains"]
    assert "error" not in browser_profile["recent_history_domains"]
    assert "settings" not in browser_profile["recent_history_domains"]

    workspaces = report["derived_profile"]["active_workspaces"]
    assert any(item["path"] == str(fixture["project"]) for item in workspaces["git_repos"])
    assert any(item["path"] == str(fixture["workspace_file"]) for item in workspaces["workspace_files"])

    sqlite_summary = report["derived_profile"]["sqlite_artifacts"]
    assert sqlite_summary["total_count"] >= 1
    assert any(item["path"] == str(fixture["sqlite_path"]) for item in sqlite_summary["top_artifacts"])


def test_run_scout_unifies_edge_and_firefox_browser_activity(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_multibrowser_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=8,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "browser_bookmarks" in kinds
    assert "browser_history" in kinds

    browser_profile = report["derived_profile"]["browser_activity"]
    assert set(browser_profile["browsers"]) >= {"chrome", "edge", "firefox", "brave"}
    assert "Edge Docs" in browser_profile["bookmark_titles"]
    assert "MDN" in browser_profile["bookmark_titles"]
    assert "OpenAI Platform" in browser_profile["bookmark_titles"]
    assert "developer.mozilla.org" in browser_profile["bookmark_domains"]
    assert "learn.microsoft.com" in browser_profile["bookmark_domains"]
    assert "platform.openai.com" in browser_profile["bookmark_domains"]
    assert "github.com" in browser_profile["recent_history_domains"]
    assert "firefox-source-docs.mozilla.org" in browser_profile["recent_history_domains"]
    per_browser = browser_profile["per_browser"]
    assert set(per_browser.keys()) >= {"chrome", "edge", "firefox", "brave"}
    assert "MDN" in per_browser["firefox"]["bookmark_titles"]
    assert "developer.mozilla.org" in per_browser["firefox"]["bookmark_domains"]
    assert per_browser["firefox"]["top_history_domains"][0] == {"domain": "developer.mozilla.org", "visit_count": 12}
    assert {"domain": "firefox-source-docs.mozilla.org", "visit_count": 7} in per_browser["firefox"]["top_history_domains"]
    assert "github.com" in per_browser["edge"]["recent_history_domains"]
    assert per_browser["edge"]["top_history_domains"][0] == {"domain": "github.com", "visit_count": 6}
    assert per_browser["chrome"]["top_history_domains"][0] == {"domain": "docs.example.com", "visit_count": 14}
    assert per_browser["brave"]["top_history_domains"][0] == {"domain": "platform.openai.com", "visit_count": 20}
    assert "Weft Docs" in per_browser["chrome"]["bookmark_titles"]
    web_interest = report["derived_profile"]["web_interest_profile"]
    assert "developer_docs" in web_interest["interest_tags"]
    assert "open_source" in web_interest["interest_tags"]
    assert "ai_tools" in web_interest["interest_tags"]
    assert web_interest["top_domains"][0] == {"domain": "platform.openai.com", "visit_count": 20}
    assert {"domain": "docs.example.com", "visit_count": 14} in web_interest["top_domains"]


def test_run_scout_supports_additional_generic_windows_browsers(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_extended_windows_browser_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=8,
        )
    )

    browser_profile = report["derived_profile"]["browser_activity"]
    assert set(browser_profile["browsers"]) >= {
        "arc",
        "brave",
        "chrome",
        "edge",
        "firefox",
        "opera",
        "opera_gx",
        "vivaldi",
        "zen",
    }
    assert "arc.net" in browser_profile["bookmark_domains"]
    assert "vivaldi.com" in browser_profile["recent_history_domains"]
    assert "docs.zen-browser.app" in browser_profile["recent_history_domains"]

    per_browser = browser_profile["per_browser"]
    assert set(per_browser.keys()) >= {
        "arc",
        "brave",
        "chrome",
        "edge",
        "firefox",
        "opera",
        "opera_gx",
        "vivaldi",
        "zen",
    }
    assert per_browser["arc"]["top_history_domains"][0] == {"domain": "arc.net", "visit_count": 11}
    assert per_browser["opera"]["top_history_domains"][0] == {"domain": "opera.com", "visit_count": 8}
    assert per_browser["opera_gx"]["top_history_domains"][0] == {"domain": "gx.games", "visit_count": 9}
    assert per_browser["vivaldi"]["top_history_domains"][0] == {"domain": "vivaldi.com", "visit_count": 10}
    assert per_browser["zen"]["top_history_domains"][0] == {"domain": "docs.zen-browser.app", "visit_count": 13}
    assert "Zen Browser" in per_browser["zen"]["bookmark_titles"]

    web_interest = report["derived_profile"]["web_interest_profile"]
    assert {"domain": "arc.net", "visit_count": 11} in web_interest["top_domains"]
    assert {"domain": "docs.zen-browser.app", "visit_count": 13} in web_interest["top_domains"]


def test_run_scout_derives_cross_browser_search_activity(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_search_activity_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=8,
        )
    )

    browser_profile = report["derived_profile"]["browser_activity"]
    chrome_query = browser_profile["per_browser"]["chrome"]["top_search_queries"][0]
    assert chrome_query["query"] == "local ai context"
    assert chrome_query["visit_count"] == 5
    assert chrome_query["engines"] == ["google"]
    assert chrome_query["sources"] == ["chrome"]
    edge_query = browser_profile["per_browser"]["edge"]["top_search_queries"][0]
    assert edge_query["query"] == "local ai context"
    assert edge_query["visit_count"] == 4
    assert edge_query["engines"] == ["bing"]
    assert edge_query["sources"] == ["edge"]
    _assert_contains_fields(
        browser_profile["per_browser"]["firefox"]["top_search_queries"],
        {
            "query": "playwright fixtures",
            "visit_count": 6,
            "engines": ["duckduckgo"],
            "sources": ["firefox"],
        },
    )

    search_activity = report["derived_profile"]["search_activity"]
    assert search_activity["present"] is True
    assert search_activity["engines"] == ["bing", "brave_search", "duckduckgo", "google"]
    top_query = search_activity["top_queries"][0]
    assert top_query["query"] == "local ai context"
    assert top_query["visit_count"] == 9
    assert top_query["engines"] == ["bing", "google"]
    assert top_query["sources"] == ["chrome", "edge"]
    _assert_contains_fields(
        search_activity["top_queries"],
        {
            "query": "playwright fixtures",
            "visit_count": 8,
            "engines": ["duckduckgo", "google"],
            "sources": ["chrome", "firefox"],
        },
    )
    _assert_contains_fields(
        search_activity["top_queries"],
        {
            "query": "codex config windows",
            "visit_count": 3,
            "engines": ["brave_search"],
            "sources": ["firefox"],
        },
    )


def test_run_scout_collects_browser_downloads_and_extensions(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_browser_artifact_v2_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=10,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "browser_downloads" in kinds
    assert "browser_extensions" in kinds

    browser_profile = report["derived_profile"]["browser_activity"]
    assert "download.example.com" in browser_profile["download_domains"]
    assert "cdn.example.com" in browser_profile["download_domains"]
    assert browser_profile["download_file_extensions"] == [".exe"]
    assert browser_profile["extension_names"] == ["Reader Helper", "Tab Manager"]

    per_browser = browser_profile["per_browser"]
    assert per_browser["chrome"]["download_file_extensions"] == [".exe"]
    assert per_browser["chrome"]["download_domains"] == ["cdn.example.com", "download.example.com", "example.com"]
    _assert_contains_fields(
        per_browser["chrome"]["extensions"],
        {
            "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "name": "Tab Manager",
            "version": "1.2.3",
            "enabled": True,
            "permissions": ["storage", "tabs"],
            "host_permissions": ["https://*.example.com/*"],
        }
    )
    _assert_contains_fields(
        per_browser["firefox"]["extensions"],
        {
            "id": "reader@example",
            "name": "Reader Helper",
            "version": "4.5.6",
            "enabled": True,
            "permissions": [],
            "host_permissions": [],
        }
    )


def test_run_scout_collects_firefox_downloads_and_session_tabs(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_browser_session_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=10,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "browser_downloads" in kinds
    assert "browser_sessions" in kinds

    browser_profile = report["derived_profile"]["browser_activity"]
    assert "addons.mozilla.org" in browser_profile["download_domains"]
    assert ".xpi" in browser_profile["download_file_extensions"]
    assert "github.com" in browser_profile["session_domains"]
    assert "platform.openai.com" in browser_profile["session_domains"]
    assert "support.mozilla.org" in browser_profile["session_domains"]
    assert "openai-python" in browser_profile["session_titles"]

    per_browser = browser_profile["per_browser"]
    assert "addons.mozilla.org" in per_browser["firefox"]["download_domains"]
    assert ".xpi" in per_browser["firefox"]["download_file_extensions"]
    assert per_browser["chrome"]["session_domains"] == ["github.com", "platform.openai.com"]
    assert per_browser["firefox"]["session_domains"] == ["support.mozilla.org"]
    assert per_browser["chrome"]["session_titles"] == ["OpenAI Docs", "openai-python"]
    assert per_browser["firefox"]["session_titles"] == ["Mozilla Support"]


def test_run_scout_reads_firefox_mozlz4_sessions_when_lz4_is_available(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_firefox_mozlz4_session_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=10,
        )
    )

    browser_profile = report["derived_profile"]["browser_activity"]
    assert "addons.mozilla.org" in browser_profile["session_domains"]
    assert "example.org" in browser_profile["session_domains"]
    assert "Firefox Add-ons" in browser_profile["session_titles"]
    assert "Example Docs" in browser_profile["session_titles"]

    per_browser = browser_profile["per_browser"]
    assert "addons.mozilla.org" in per_browser["firefox"]["session_domains"]
    assert "example.org" in per_browser["firefox"]["session_domains"]
    assert "Example Docs" in per_browser["firefox"]["session_titles"]
    assert "Firefox Add-ons" in per_browser["firefox"]["session_titles"]


def test_run_scout_prioritizes_high_signal_history_rows_beyond_default_sqlite_scan_window(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_multibrowser_fixture(tmp_path)
    chrome_history = fixture["browser_profile"] / "History"
    firefox_places = fixture["firefox_places"]

    _append_chromium_history_rows(
        chrome_history,
        [
            (f"https://example.com/filler-{index}", f"Filler {index}", 1)
            for index in range(60)
        ]
        + [
            ("https://www.google.com/search?q=agentic+memory+systems", "Google Search", 30),
        ],
    )
    _append_firefox_places_rows(
        firefox_places,
        [
            (f"https://mozilla.example/filler-{index}", f"Mozilla Filler {index}", 1)
            for index in range(120)
        ]
        + [
            ("https://duckduckgo.com/?q=context+engineering+patterns&ia=web", "DuckDuckGo Search", 25),
        ],
    )

    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=8,
        )
    )

    chrome_searches = report["derived_profile"]["browser_activity"]["per_browser"]["chrome"]["top_search_queries"]
    firefox_searches = report["derived_profile"]["browser_activity"]["per_browser"]["firefox"]["top_search_queries"]
    _assert_contains_fields(
        chrome_searches,
        {
            "query": "agentic memory systems",
            "visit_count": 30,
            "engines": ["google"],
            "sources": ["chrome"],
        },
    )
    _assert_contains_fields(
        firefox_searches,
        {
            "query": "context engineering patterns",
            "visit_count": 25,
            "engines": ["duckduckgo"],
            "sources": ["firefox"],
        },
    )


def test_run_scout_derives_intent_profile_from_search_and_browser_signals(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_search_activity_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=8,
        )
    )

    intent_profile = report["derived_profile"]["intent_profile"]
    assert intent_profile["present"] is True
    assert set(intent_profile["intent_tags"]) >= {"ai_tools", "developer_research", "system_ops"}
    assert {"intent": "ai_tools", "score": 3} in intent_profile["top_intents"]
    assert {"intent": "developer_research", "score": 20} in intent_profile["top_intents"]
    assert {"intent": "system_ops", "score": 3} in intent_profile["top_intents"]
    _assert_contains_fields(
        intent_profile["query_examples"],
        {
            "query": "playwright fixtures",
            "visit_count": 8,
            "engines": ["duckduckgo", "google"],
            "sources": ["chrome", "firefox"],
            "intent_tags": ["developer_research"],
        },
    )
    _assert_contains_fields(
        intent_profile["query_examples"],
        {
            "query": "local ai context",
            "visit_count": 9,
            "engines": ["bing", "google"],
            "sources": ["chrome", "edge"],
            "intent_tags": ["developer_research"],
        },
    )
    _assert_contains_fields(
        intent_profile["query_examples"],
        {
            "query": "codex config windows",
            "visit_count": 3,
            "engines": ["brave_search"],
            "sources": ["firefox"],
            "intent_tags": ["ai_tools", "developer_research", "system_ops"],
        },
    )


def test_run_scout_splits_consumer_intents_into_specific_buckets(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_consumer_intent_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=8,
        )
    )

    intent_profile = report["derived_profile"]["intent_profile"]
    assert set(intent_profile["intent_tags"]) >= {"video_media", "chat_community", "gaming", "shopping"}
    assert {"intent": "video_media", "score": 7} in intent_profile["top_intents"]
    assert {"intent": "chat_community", "score": 6} in intent_profile["top_intents"]
    assert {"intent": "gaming", "score": 9} in intent_profile["top_intents"]
    assert {"intent": "shopping", "score": 9} in intent_profile["top_intents"]
    _assert_contains_fields(
        intent_profile["query_examples"],
        {
            "query": "bilibili anime",
            "visit_count": 7,
            "engines": ["google"],
            "sources": ["chrome"],
            "intent_tags": ["video_media"],
        },
    )
    _assert_contains_fields(
        intent_profile["query_examples"],
        {
            "query": "discord",
            "visit_count": 6,
            "engines": ["bing"],
            "normalized_from": ["discord servers"],
            "sources": ["edge"],
            "intent_tags": ["chat_community"],
        },
    )
    _assert_contains_fields(
        intent_profile["query_examples"],
        {
            "query": "steam deck dock",
            "visit_count": 5,
            "engines": ["google"],
            "sources": ["chrome"],
            "intent_tags": ["gaming", "shopping"],
        },
    )
    _assert_contains_fields(
        intent_profile["query_examples"],
        {
            "query": "epic games sale",
            "visit_count": 4,
            "engines": ["duckduckgo"],
            "sources": ["firefox"],
            "intent_tags": ["gaming", "shopping"],
        },
    )


def test_run_scout_normalizes_semantically_equivalent_queries_before_aggregation(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_query_normalization_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=8,
        )
    )

    search_activity = report["derived_profile"]["search_activity"]
    assert {
        "query": "bilibili",
        "visit_count": 9,
        "engines": ["bing", "google"],
        "normalized_from": ["b station", "bilibili"],
        "sources": ["chrome", "edge"],
    } in search_activity["top_queries"]
    assert {
        "query": "discord",
        "visit_count": 6,
        "engines": ["bing", "google"],
        "normalized_from": ["discord", "discord servers"],
        "sources": ["chrome", "edge"],
    } in search_activity["top_queries"]
    assert {
        "query": "steam",
        "visit_count": 5,
        "engines": ["duckduckgo"],
        "normalized_from": ["steam", "steam deck"],
        "sources": ["firefox"],
    } in search_activity["top_queries"]

    intent_profile = report["derived_profile"]["intent_profile"]
    assert {
        "query": "bilibili",
        "visit_count": 9,
        "engines": ["bing", "google"],
        "normalized_from": ["b station", "bilibili"],
        "sources": ["chrome", "edge"],
        "intent_tags": ["video_media"],
    } in intent_profile["query_examples"]
    assert {
        "query": "discord",
        "visit_count": 6,
        "engines": ["bing", "google"],
        "normalized_from": ["discord", "discord servers"],
        "sources": ["chrome", "edge"],
        "intent_tags": ["chat_community"],
    } in intent_profile["query_examples"]
    assert {
        "query": "steam",
        "visit_count": 5,
        "engines": ["duckduckgo"],
        "normalized_from": ["steam", "steam deck"],
        "sources": ["firefox"],
        "intent_tags": ["gaming"],
    } in intent_profile["query_examples"]


def test_run_scout_does_not_crash_on_invalid_sqlite_candidates(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_browser_and_workspace_fixture(tmp_path)
    bad_sqlite = tmp_path / "data" / "broken.sqlite3"
    _write(bad_sqlite, "not actually sqlite")

    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["sqlite_path"].parent,
            ],
            home=fixture["home"],
            max_depth=7,
        )
    )

    sqlite_entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "sqlite_database"]
    broken_entry = next(entry for entry in sqlite_entries if entry["path"] == str(bad_sqlite))
    assert broken_entry["fields"]["valid"] is False
    assert broken_entry["fields"]["error"]


def test_home_roots_expand_to_useful_local_search_targets(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, build_effective_roots

    home = tmp_path / "home"
    expected = {
        str(home.resolve()),
        str((home / ".claude").resolve()),
        str((home / ".codex").resolve()),
        str((home / "AppData" / "Local").resolve()),
        str((home / "AppData" / "Roaming").resolve()),
        str((home / "Documents").resolve()),
        str((home / "Desktop").resolve()),
    }

    roots = build_effective_roots(ScoutConfig(roots=[], home=home))

    assert expected.issubset({str(path) for path in roots})


def test_sqlite_derived_profile_prioritizes_interesting_artifacts(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_browser_and_workspace_fixture(tmp_path)
    noisy_dir = tmp_path / "data" / "noise"
    noisy_dir.mkdir(parents=True, exist_ok=True)
    for index in range(12):
        path = noisy_dir / f"noise-{index}.sqlite3"
        with sqlite3.connect(path) as connection:
            connection.execute("CREATE TABLE empty_table (id INTEGER PRIMARY KEY)")
            connection.commit()

    report = run_scout(
        ScoutConfig(
            roots=[fixture["home"], fixture["project"].parent, fixture["sqlite_path"].parent],
            home=fixture["home"],
            max_depth=7,
        )
    )

    sqlite_profile = report["derived_profile"]["sqlite_artifacts"]
    assert sqlite_profile["total_count"] >= 13
    assert len(sqlite_profile["top_artifacts"]) <= 10
    assert sqlite_profile["top_artifacts"][0]["path"] == str(fixture["sqlite_path"])


def test_parse_browser_history_uses_copy_fallback_when_locked(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_browser_history

    path = tmp_path / "History"
    with sqlite3.connect(path) as connection:
        connection.execute("CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT)")
        connection.execute(
            "INSERT INTO urls(url, title) VALUES (?, ?)",
            ("https://example.com/locked", "Locked History"),
        )
        connection.commit()

    locking_connection = sqlite3.connect(path)
    locking_connection.execute("BEGIN EXCLUSIVE")

    try:
        started = time.perf_counter()
        parsed = parse_browser_history(path)
        elapsed = time.perf_counter() - started
    finally:
        locking_connection.rollback()
        locking_connection.close()

    assert parsed["browser"] == "chrome"
    assert "example.com" in parsed["recent_domains"]
    assert "Locked History" in parsed["recent_titles"]
    assert elapsed < 1.0


def test_run_scout_respects_sqlite_parse_budget(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    sqlite_dir = tmp_path / "sqlite"
    sqlite_dir.mkdir(parents=True, exist_ok=True)
    for index in range(8):
        path = sqlite_dir / f"db-{index}.sqlite3"
        with sqlite3.connect(path) as connection:
            connection.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT)")
            connection.execute("INSERT INTO items(value) VALUES (?)", (f"value-{index}",))
            connection.commit()

    report = run_scout(
        ScoutConfig(
            roots=[sqlite_dir],
            home=None,
            max_depth=3,
            max_sqlite_parse=3,
        )
    )

    sqlite_entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "sqlite_database"]
    parsed_count = sum(1 for entry in sqlite_entries if entry["fields"].get("parse_mode") == "parsed")
    budgeted_count = sum(1 for entry in sqlite_entries if entry["fields"].get("parse_mode") == "budget_skipped")

    assert len(sqlite_entries) == 8
    assert parsed_count == 3
    assert budgeted_count == 5
    assert report["derived_profile"]["sqlite_artifacts"]["total_count"] == 8


def test_run_scout_ignores_unrelated_settings_json_files(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    desktop = home / "Desktop"
    desktop.mkdir(parents=True, exist_ok=True)
    _write(desktop / "settings.json", '{"not":"claude-settings"}')
    _write(home / ".claude" / "settings.json", '{"defaultModel":"claude-sonnet"}')

    report = run_scout(
        ScoutConfig(
            roots=[desktop],
            home=home,
            max_depth=3,
        )
    )

    claude_settings_entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "claude_settings"]
    assert len(claude_settings_entries) == 1
    assert claude_settings_entries[0]["path"] == str((home / ".claude" / "settings.json").resolve())


def test_run_scout_keeps_going_when_a_single_candidate_parse_fails(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(home / ".claude" / "settings.json", '{"defaultModel":"claude-sonnet"}')
    _write(home / ".codex" / "config.toml", 'model = "gpt-5.4"')
    _write(home / ".codex" / "auth.json", '{"access_token": "token"}')

    broken_history = home / "AppData" / "Local" / "Google" / "Chrome" / "User Data" / "Default" / "History"
    _write(broken_history, "not-a-sqlite-file")

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=home,
            max_depth=5,
        )
    )

    assert any(entry["entity_kind"] == "claude_settings" for entry in report["raw_evidence"])
    assert any(entry["entity_kind"] == "codex_config" for entry in report["raw_evidence"])
    parse_errors = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "parse_error"]
    assert parse_errors
    assert any(entry["path"] == str(broken_history.resolve()) for entry in parse_errors)


def test_cli_max_sqlite_parse_flag_controls_budget(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    sqlite_dir = tmp_path / "sqlite"
    sqlite_dir.mkdir(parents=True, exist_ok=True)
    for index in range(4):
        path = sqlite_dir / f"db-{index}.sqlite3"
        with sqlite3.connect(path) as connection:
            connection.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT)")
            connection.execute("INSERT INTO items(value) VALUES (?)", (f"value-{index}",))
            connection.commit()

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--root",
            str(sqlite_dir),
            "--output",
            str(output_path),
            "--max-sqlite-parse",
            "2",
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    sqlite_entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "sqlite_database"]
    parsed_count = sum(1 for entry in sqlite_entries if entry["fields"].get("parse_mode") == "parsed")
    budgeted_count = sum(1 for entry in sqlite_entries if entry["fields"].get("parse_mode") == "budget_skipped")
    assert parsed_count == 2
    assert budgeted_count == 2


def test_cli_accepts_explicit_system_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    steam_root = tmp_path / "Steam"
    steam_library = tmp_path / "SteamLibrary"
    epic_manifest_dir = tmp_path / "Epic" / "Manifests"
    legendary_installed = tmp_path / "legendary" / "installed.json"
    amazon_install_info = tmp_path / "Amazon Games" / "Data" / "Games" / "Sql" / "GameInstallInfo.sqlite"
    xbox_game_config = tmp_path / "XboxGames" / "Avowed" / "Content" / "MicrosoftGame.config"
    itch_butler_db = tmp_path / "itch" / "db" / "butler.db"
    battle_net_product_db = tmp_path / "Battle.net" / "Agent" / "product.db"
    origin_local_content = tmp_path / "Origin" / "LocalContent"
    steam_root_windows = steam_root.as_posix().replace("/", "\\")
    steam_library_windows = steam_library.as_posix().replace("/", "\\")

    _write(
        steam_root / "steamapps" / "libraryfolders.vdf",
        "\n".join(
            [
                '"libraryfolders"',
                "{",
                '  "0"',
                "  {",
                f'    "path" "{steam_root_windows}"',
                "  }",
                '  "1"',
                "  {",
                f'    "path" "{steam_library_windows}"',
                "  }",
                "}",
            ]
        ),
    )
    _write(
        steam_library / "steamapps" / "appmanifest_570.acf",
        "\n".join(
            [
                '"AppState"',
                "{",
                '  "appid" "570"',
                '  "name" "Dota 2"',
                '  "installdir" "dota 2 beta"',
                '  "StateFlags" "4"',
                "}",
            ]
        ),
    )
    _write(
        epic_manifest_dir / "fortnite.item",
        json.dumps(
            {
                "DisplayName": "Fortnite",
                "AppName": "Fortnite",
                "CatalogItemId": "fortnite-id",
                "InstallLocation": "D:\\Epic Games\\Fortnite",
                "LaunchExecutable": "FortniteLauncher.exe",
            }
        ),
    )
    _write(
        legendary_installed,
        json.dumps(
            {
                "AlanWake2": {
                    "title": "Alan Wake 2",
                    "install_path": "D:\\Epic\\Alan Wake 2",
                    "app_name": "AlanWake2",
                }
            }
        ),
    )
    amazon_install_info.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(amazon_install_info) as connection:
        connection.execute(
            "CREATE TABLE DbSet (Id TEXT, ProductTitle TEXT, InstallDirectory TEXT, Installed INTEGER)"
        )
        connection.execute(
            "INSERT INTO DbSet(Id, ProductTitle, InstallDirectory, Installed) VALUES (?, ?, ?, ?)",
            (
                "amzn1.adg.product.0d364464-032c-40c9-a6da-c633a53e3374",
                "Blue Fire",
                r"C:\Amazon Games\Library\Blue Fire",
                1,
            ),
        )
        connection.commit()
    _write(
        xbox_game_config,
        """<?xml version="1.0" encoding="utf-8"?>
<Game configVersion="1">
  <Identity Name="Microsoft.Avowed" Version="1.0.0.0" Publisher="CN=Microsoft" />
  <ShellVisuals DefaultDisplayName="Avowed" />
</Game>
""",
    )
    itch_butler_db.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(itch_butler_db) as connection:
        connection.execute(
            "CREATE TABLE caves (id INTEGER PRIMARY KEY, game_title TEXT, install_folder TEXT, game_id INTEGER)"
        )
        connection.execute(
            "INSERT INTO caves(game_title, install_folder, game_id) VALUES (?, ?, ?)",
            ("Celeste Classic", r"D:\itch\games\celeste-classic", 12345),
        )
        connection.commit()
    _write(
        origin_local_content / "Mass Effect Legendary Edition" / "metadata.mfst",
        (
            "?currentstate=kReadyToStart"
            "&dipinstallpath=D%3A%5CEA%20Games%5CMass%20Effect%20Legendary%20Edition"
            "&id=OFB-EAST%3A109552419"
        ),
    )
    _write_bytes(
        battle_net_product_db,
        _battle_net_product_db(
            _battle_net_product_install("s2", "S2", r"D:\Games\Battle.net\StarCraft II"),
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-steam-root",
            str(steam_root),
            "--system-epic-manifest-dir",
            str(epic_manifest_dir),
            "--system-legendary-installed",
            str(legendary_installed),
            "--system-amazon-games-install-info",
            str(amazon_install_info),
            "--system-xbox-game-config",
            str(xbox_game_config),
            "--system-itch-butler-db",
            str(itch_butler_db),
            "--system-battle-net-product-db",
            str(battle_net_product_db),
            "--system-origin-local-content-dir",
            str(origin_local_content),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    platforms = report["derived_profile"]["gaming_profile"]["platforms"]
    game_names = set(report["derived_profile"]["gaming_profile"]["installed_game_names"])
    assert platforms == ["amazon_games", "battle_net", "ea", "epic", "itch", "steam", "xbox"]
    assert {
        "Dota 2",
        "Fortnite",
        "Alan Wake 2",
        "Blue Fire",
        "Avowed",
        "Celeste Classic",
        "Mass Effect Legendary Edition",
        "StarCraft II",
    } <= game_names


def test_cli_accepts_explicit_obsidian_and_obs_studio_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    vault = tmp_path / "knowledge" / "Second Brain"
    obsidian_config = tmp_path / "Obsidian" / "obsidian.json"
    obs_studio_basic_dir = tmp_path / "obs-studio" / "basic"

    _write(
        obsidian_config,
        json.dumps(
            {
                "vaults": {
                    "vault-1": {
                        "path": str(vault),
                        "ts": 1712345678,
                        "open": True,
                    }
                }
            }
        ),
    )
    _write(vault / "Daily" / "2026-04-24.md", "# Note\n")
    _write(vault / "Projects" / "Scout.md", "# Scout\n")
    _write(vault / ".obsidian" / "core-plugins.json", json.dumps(["daily-notes", "templates"]))
    _write(vault / ".obsidian" / "community-plugins.json", json.dumps(["dataview", "tasks"]))
    _write(
        vault / ".obsidian" / "app.json",
        json.dumps(
            {
                "attachmentFolderPath": "Assets",
                "alwaysUpdateLinks": True,
            }
        ),
    )

    _write(
        obs_studio_basic_dir / "profiles" / "Streaming" / "basic.ini",
        "\n".join(
            [
                "[General]",
                "Name=Streaming",
                "[Video]",
                "BaseCX=2560",
                "BaseCY=1440",
                "OutputCX=1920",
                "OutputCY=1080",
                "[Output]",
                "Mode=Advanced",
                "[AdvOut]",
                "RecFormat2=mkv",
                "Encoder=obs_x264",
                "[SimpleOutput]",
                "RecFormat=mp4",
                "[Stream1]",
                "Service=Twitch",
            ]
        ),
    )
    _write(
        obs_studio_basic_dir / "scenes" / "Creator.json",
        json.dumps(
            {
                "name": "Creator",
                "current_scene": "Starting Soon",
                "current_program_scene": "Gameplay",
                "scene_order": [{"name": "Starting Soon"}, {"name": "Gameplay"}],
                "sources": [
                    {"name": "Starting Soon", "id": "scene"},
                    {"name": "Gameplay", "id": "scene"},
                    {"name": "Mic", "id": "wasapi_input_capture"},
                    {"name": "Camera", "id": "dshow_input"},
                ],
            }
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-obsidian-config",
            str(obsidian_config),
            "--system-obs-studio-basic-dir",
            str(obs_studio_basic_dir),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "obsidian_global_config" in kinds
    assert "obsidian_vault" in kinds
    assert "obs_studio_profile" in kinds
    assert "obs_studio_scene_collection" in kinds

    knowledge_profile = report["derived_profile"]["knowledge_tools_profile"]
    assert knowledge_profile["present"] is True
    assert knowledge_profile["tool_families"] == ["obsidian"]
    assert knowledge_profile["vault_count"] == 1
    assert knowledge_profile["vault_names"] == ["Second Brain"]
    assert knowledge_profile["note_count"] == 2
    assert knowledge_profile["community_plugins"] == ["dataview", "tasks"]

    creator_profile = report["derived_profile"]["creator_profile"]
    assert creator_profile["present"] is True
    assert creator_profile["tool_families"] == ["obs_studio"]
    assert creator_profile["profile_names"] == ["Streaming"]
    assert creator_profile["scene_collection_names"] == ["Creator"]
    assert creator_profile["streaming_services"] == ["Twitch"]
    assert creator_profile["recording_formats"] == ["mkv"]

    assert "knowledge_management" in report["derived_profile"]["interest_tags"]
    assert "content_creation" in report["derived_profile"]["interest_tags"]


def test_iter_bootstrap_candidates_skips_noisy_directories(tmp_path: Path) -> None:
    from ai_local_scout.discovery import iter_bootstrap_candidates

    home = tmp_path / "home"
    _write(home / ".claude" / "settings.json", '{"defaultModel":"claude-sonnet"}')
    _write(home / ".cargo" / "registry" / "src" / "demo-package" / "AGENTS.md", "# Noise\n")
    _write(home / "AppData" / "Local" / "Temp" / "cache.sqlite3", "not really sqlite")

    candidates = iter_bootstrap_candidates([home], max_depth=6)
    candidate_paths = {str(path) for path in candidates}

    assert str((home / ".claude" / "settings.json").resolve()) in candidate_paths
    assert str((home / ".cargo" / "registry" / "src" / "demo-package" / "AGENTS.md").resolve()) not in candidate_paths
    assert str((home / "AppData" / "Local" / "Temp" / "cache.sqlite3").resolve()) not in candidate_paths


def test_run_scout_discovers_obsidian_vaults_under_roaming_config(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    vault = home / "Documents" / "Knowledge Base"
    _write(
        home / "AppData" / "Roaming" / "Obsidian" / "obsidian.json",
        json.dumps(
            {
                "vaults": {
                    "vault-1": {
                        "path": str(vault),
                        "ts": 1711111111,
                        "open": True,
                    }
                }
            }
        ),
    )
    _write(vault / "Inbox.md", "# Inbox\n")
    _write(vault / "Areas" / "AI.md", "# AI\n")
    _write(vault / ".obsidian" / "core-plugins.json", json.dumps(["backlink", "graph"]))
    _write(vault / ".obsidian" / "community-plugins.json", json.dumps(["dataview"]))
    _write(vault / ".obsidian" / "app.json", json.dumps({"attachmentFolderPath": "Assets"}))

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=6,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "obsidian_global_config" in kinds
    assert "obsidian_vault" in kinds

    knowledge_profile = report["derived_profile"]["knowledge_tools_profile"]
    assert knowledge_profile["present"] is True
    assert knowledge_profile["vault_count"] == 1
    assert knowledge_profile["vault_names"] == ["Knowledge Base"]
    assert knowledge_profile["core_plugins"] == ["backlink", "graph"]
    assert knowledge_profile["community_plugins"] == ["dataview"]
    assert knowledge_profile["note_count"] == 2


def test_run_scout_discovers_obs_studio_profiles_and_scene_collections_under_roaming(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    obs_studio_basic_dir = home / "AppData" / "Roaming" / "obs-studio" / "basic"
    _write(
        obs_studio_basic_dir / "profiles" / "YouTube" / "basic.ini",
        "\n".join(
            [
                "[General]",
                "Name=YouTube",
                "[Video]",
                "BaseCX=1920",
                "BaseCY=1080",
                "OutputCX=1920",
                "OutputCY=1080",
                "[Output]",
                "Mode=Simple",
                "[SimpleOutput]",
                "RecFormat=mp4",
                "[Stream1]",
                "Service=YouTube - RTMPS",
            ]
        ),
    )
    _write(
        obs_studio_basic_dir / "scenes" / "Main.json",
        json.dumps(
            {
                "name": "Main",
                "current_scene": "Desktop",
                "current_program_scene": "Desktop",
                "scene_order": [{"name": "Desktop"}, {"name": "Be Right Back"}],
                "sources": [
                    {"name": "Desktop", "id": "scene"},
                    {"name": "Be Right Back", "id": "scene"},
                    {"name": "Display Capture", "id": "monitor_capture"},
                    {"name": "Mic", "id": "wasapi_input_capture"},
                ],
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=6,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "obs_studio_profile" in kinds
    assert "obs_studio_scene_collection" in kinds

    creator_profile = report["derived_profile"]["creator_profile"]
    assert creator_profile["present"] is True
    assert creator_profile["profile_names"] == ["YouTube"]
    assert creator_profile["scene_collection_names"] == ["Main"]
    assert creator_profile["streaming_services"] == ["YouTube - RTMPS"]
    assert creator_profile["recording_formats"] == ["mp4"]
    assert creator_profile["scene_count"] == 2


def test_cli_accepts_explicit_docker_desktop_and_wsl_probe_paths(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from ai_local_scout.cli import main

    _install_runtime_probe_stub(
        monkeypatch,
        wsl_stdout=(
            "  NAME              STATE           VERSION\n"
            "* Ubuntu-22.04      Running         2\n"
            "  docker-desktop    Running         2\n"
            "  Debian            Stopped         1\n"
        ),
    )

    docker_settings = tmp_path / "Docker" / "settings-store.json"
    wsl_config = tmp_path / ".wslconfig"
    _write(
        docker_settings,
        json.dumps(
            {
                "wslEngineEnabled": True,
                "kubernetes": True,
                "extensionsEnabled": True,
                "onlyMarketplaceExtensions": False,
                "enableInference": True,
                "enableInferenceTCP": True,
                "enableInferenceTCPPort": 12434,
                "desktopTerminalEnabled": True,
                "exposeDockerAPIOnTCP2375": False,
            }
        ),
    )
    _write(
        wsl_config,
        "\n".join(
            [
                "[wsl2]",
                "memory=6GB",
                "processors=4",
                "localhostForwarding=true",
                "networkingMode=mirrored",
                "nestedVirtualization=true",
                "[experimental]",
                "autoMemoryReclaim=gradual",
                "sparseVhd=true",
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-docker-desktop-settings",
            str(docker_settings),
            "--system-wslconfig",
            str(wsl_config),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "docker_desktop_settings" in kinds
    assert "wsl_global_config" in kinds
    assert "wsl_distribution_list" in kinds

    container_profile = report["derived_profile"]["container_tools_profile"]
    assert container_profile["present"] is True
    assert container_profile["tool_families"] == ["docker_desktop"]
    assert container_profile["uses_wsl_engine"] is True
    assert container_profile["kubernetes_enabled"] is True
    assert container_profile["model_runner_enabled"] is True
    assert container_profile["model_runner_tcp_port"] == 12434

    linux_profile = report["derived_profile"]["linux_runtime_profile"]
    assert linux_profile["present"] is True
    assert linux_profile["tool_families"] == ["wsl"]
    assert linux_profile["default_distro"] == "Ubuntu-22.04"
    assert linux_profile["running_distros"] == ["Ubuntu-22.04", "docker-desktop"]
    assert linux_profile["wsl_versions"] == [1, 2]
    assert linux_profile["memory_limit"] == "6GB"
    assert linux_profile["networking_mode"] == "mirrored"
    assert linux_profile["auto_memory_reclaim"] == "gradual"

    assert "containers" in report["derived_profile"]["interest_tags"]
    assert "linux_runtime" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_docker_desktop_settings_under_roaming(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    _install_runtime_probe_stub(monkeypatch)

    home = tmp_path / "home"
    _write(
        home / "AppData" / "Roaming" / "Docker" / "settings-store.json",
        json.dumps(
            {
                "wslEngineEnabled": True,
                "kubernetes": False,
                "extensionsEnabled": True,
                "onlyMarketplaceExtensions": True,
                "enableInference": False,
                "desktopTerminalEnabled": True,
                "exposeDockerAPIOnTCP2375": True,
                "enhancedContainerIsolation": True,
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=6,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "docker_desktop_settings" in kinds

    container_profile = report["derived_profile"]["container_tools_profile"]
    assert container_profile["present"] is True
    assert container_profile["uses_wsl_engine"] is True
    assert container_profile["kubernetes_enabled"] is False
    assert container_profile["extensions_enabled"] is True
    assert container_profile["marketplace_only_extensions"] is True
    assert container_profile["exposes_docker_api_tcp_2375"] is True
    assert container_profile["enhanced_container_isolation"] is True


def test_run_scout_discovers_wsl_config_and_distros(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    _install_runtime_probe_stub(
        monkeypatch,
        wsl_stdout=(
            "  NAME                   STATE           VERSION\n"
            "* Ubuntu                 Running         2\n"
            "  docker-desktop         Running         2\n"
            "  openSUSE-Leap-15.5     Stopped         2\n"
        ),
    )

    home = tmp_path / "home"
    _write(
        home / ".wslconfig",
        "\n".join(
            [
                "[wsl2]",
                "memory=8GB",
                "processors=6",
                "localhostForwarding=false",
                "nestedVirtualization=false",
                "swap=2GB",
                "[experimental]",
                "autoMemoryReclaim=dropCache",
                "sparseVhd=true",
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "wsl_global_config" in kinds
    assert "wsl_distribution_list" in kinds

    linux_profile = report["derived_profile"]["linux_runtime_profile"]
    assert linux_profile["present"] is True
    assert linux_profile["default_distro"] == "Ubuntu"
    assert linux_profile["distro_names"] == ["Ubuntu", "docker-desktop", "openSUSE-Leap-15.5"]
    assert linux_profile["running_distros"] == ["Ubuntu", "docker-desktop"]
    assert linux_profile["memory_limit"] == "8GB"
    assert linux_profile["processor_count"] == 6
    assert linux_profile["localhost_forwarding"] is False
    assert linux_profile["nested_virtualization"] is False
    assert linux_profile["swap_size"] == "2GB"
    assert linux_profile["sparse_vhd"] is True


def test_cli_accepts_explicit_discord_and_jetbrains_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    discord_settings = tmp_path / "discord" / "settings.json"
    jetbrains_recent_projects = tmp_path / "JetBrains" / "IntelliJIdea2025.1" / "options" / "recentProjects.xml"

    _write(
        discord_settings,
        json.dumps(
            {
                "openOnStartup": True,
                "theme": "dark",
                "status": "online",
                "enableHardwareAcceleration": True,
                "locale": "en-US",
            }
        ),
    )
    _write(
        jetbrains_recent_projects,
        "\n".join(
            [
                '<application>',
                '  <component name="RecentProjectsManager">',
                '    <option name="additionalInfo">',
                '      <map>',
                '        <entry key="$USER_HOME$/src/app-one" />',
                '        <entry key="$USER_HOME$/src/app-two" />',
                '      </map>',
                '    </option>',
                '  </component>',
                '</application>',
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-discord-settings",
            str(discord_settings),
            "--system-jetbrains-recent-projects",
            str(jetbrains_recent_projects),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "discord_settings" in kinds
    assert "jetbrains_recent_projects" in kinds

    social_profile = report["derived_profile"]["social_tools_profile"]
    assert social_profile["present"] is True
    assert social_profile["tool_families"] == ["discord"]
    assert social_profile["discord_open_on_startup"] is True
    assert social_profile["discord_theme"] == "dark"
    assert social_profile["discord_status"] == "online"
    assert social_profile["discord_locale"] == "en-US"

    ide_profile = report["derived_profile"]["ide_profile"]
    assert ide_profile["present"] is True
    assert ide_profile["tool_families"] == ["jetbrains"]
    assert ide_profile["jetbrains_products"] == ["IntelliJIdea2025.1"]
    assert ide_profile["recent_project_count"] == 2
    assert ide_profile["recent_project_paths"] == ["$USER_HOME$/src/app-one", "$USER_HOME$/src/app-two"]

    assert "chat_community_tools" in report["derived_profile"]["interest_tags"]
    assert "ide_workflow" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_discord_settings_under_roaming(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / "AppData" / "Roaming" / "discord" / "settings.json",
        json.dumps(
            {
                "openOnStartup": False,
                "theme": "light",
                "status": "idle",
                "locale": "zh-CN",
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=6,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "discord_settings" in kinds

    social_profile = report["derived_profile"]["social_tools_profile"]
    assert social_profile["present"] is True
    assert social_profile["discord_open_on_startup"] is False
    assert social_profile["discord_theme"] == "light"
    assert social_profile["discord_status"] == "idle"
    assert social_profile["discord_locale"] == "zh-CN"


def test_run_scout_discovers_jetbrains_recent_projects_under_roaming(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / "AppData" / "Roaming" / "JetBrains" / "PyCharm2025.1" / "options" / "recentProjects.xml",
        "\n".join(
            [
                '<application>',
                '  <component name="RecentProjectsManager">',
                '    <option name="recentPaths">',
                '      <list>',
                '        <option value="D:/work/alpha" />',
                '        <option value="D:/work/beta" />',
                '      </list>',
                '    </option>',
                '  </component>',
                '</application>',
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=6,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "jetbrains_recent_projects" in kinds

    ide_profile = report["derived_profile"]["ide_profile"]
    assert ide_profile["present"] is True
    assert ide_profile["jetbrains_products"] == ["PyCharm2025.1"]
    assert ide_profile["recent_project_count"] == 2
    assert ide_profile["recent_project_paths"] == ["D:/work/alpha", "D:/work/beta"]


def test_cli_accepts_explicit_windows_terminal_and_ssh_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    terminal_settings = tmp_path / "Terminal" / "settings.json"
    ssh_config = tmp_path / ".ssh" / "config"

    _write(
        terminal_settings,
        json.dumps(
            {
                "defaultProfile": "{pwsh-guid}",
                "profiles": {
                    "list": [
                        {
                            "guid": "{pwsh-guid}",
                            "name": "PowerShell",
                            "source": "Windows.Terminal.PowershellCore",
                            "commandline": "pwsh.exe",
                        },
                        {
                            "guid": "{ubuntu-guid}",
                            "name": "Ubuntu",
                            "source": "Windows.Terminal.Wsl",
                            "commandline": "wsl.exe -d Ubuntu",
                        },
                    ]
                }
            }
        ),
    )
    _write(
        ssh_config,
        "\n".join(
            [
                "Host github-work",
                "  HostName github.com",
                "  User git",
                "  IdentityFile ~/.ssh/id_ed25519_work",
                "",
                "Host prod-box",
                "  HostName 10.0.0.25",
                "  User admin",
                "  IdentityFile ~/.ssh/prod_rsa",
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-windows-terminal-settings",
            str(terminal_settings),
            "--system-ssh-config",
            str(ssh_config),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "windows_terminal_settings" in kinds
    assert "ssh_config" in kinds

    terminal_profile = report["derived_profile"]["terminal_tools_profile"]
    assert terminal_profile["present"] is True
    assert terminal_profile["tool_families"] == ["windows_terminal"]
    assert terminal_profile["default_profile"] == "PowerShell"
    assert terminal_profile["profile_names"] == ["PowerShell", "Ubuntu"]
    assert terminal_profile["profile_sources"] == ["Windows.Terminal.PowershellCore", "Windows.Terminal.Wsl"]

    ssh_profile = report["derived_profile"]["ssh_profile"]
    assert ssh_profile["present"] is True
    assert ssh_profile["host_aliases"] == ["github-work", "prod-box"]
    assert ssh_profile["identity_files"] == ["~/.ssh/id_ed25519_work", "~/.ssh/prod_rsa"]
    assert ssh_profile["host_count"] == 2

    assert "terminal_workflow" in report["derived_profile"]["interest_tags"]
    assert "remote_access" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_windows_terminal_settings_under_localstate(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home
        / "AppData"
        / "Local"
        / "Packages"
        / "Microsoft.WindowsTerminal_8wekyb3d8bbwe"
        / "LocalState"
        / "settings.json",
        json.dumps(
            {
                "defaultProfile": "{cmd-guid}",
                "profiles": {
                    "list": [
                        {
                            "guid": "{cmd-guid}",
                            "name": "Command Prompt",
                            "source": "Windows.Terminal.Cmd",
                            "commandline": "cmd.exe",
                        },
                        {
                            "guid": "{azure-guid}",
                            "name": "Azure Cloud Shell",
                            "source": "Windows.Terminal.Azure",
                            "commandline": "azshell.exe",
                        },
                    ]
                }
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=6,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "windows_terminal_settings" in kinds

    terminal_profile = report["derived_profile"]["terminal_tools_profile"]
    assert terminal_profile["present"] is True
    assert terminal_profile["default_profile"] == "Command Prompt"
    assert terminal_profile["profile_names"] == ["Azure Cloud Shell", "Command Prompt"]


def test_run_scout_discovers_ssh_config_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / ".ssh" / "config",
        "\n".join(
            [
                "Host codeberg",
                "  HostName codeberg.org",
                "  User git",
                "  IdentityFile ~/.ssh/codeberg_ed25519",
                "",
                "Host internal-jump",
                "  HostName jump.internal",
                "  User ops",
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "ssh_config" in kinds

    ssh_profile = report["derived_profile"]["ssh_profile"]
    assert ssh_profile["present"] is True
    assert ssh_profile["host_aliases"] == ["codeberg", "internal-jump"]
    assert ssh_profile["identity_files"] == ["~/.ssh/codeberg_ed25519"]
    assert ssh_profile["host_count"] == 2


def test_cli_accepts_explicit_kubeconfig_and_docker_context_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    kubeconfig = tmp_path / ".kube" / "config"
    docker_config = tmp_path / ".docker" / "config.json"
    docker_context_meta = tmp_path / ".docker" / "contexts" / "meta" / "ctx1" / "meta.json"

    _write(
        kubeconfig,
        "\n".join(
            [
                "apiVersion: v1",
                "kind: Config",
                "current-context: prod-cluster",
                "contexts:",
                "  - name: dev-cluster",
                "    context:",
                "      cluster: dev",
                "      user: dev-user",
                "      namespace: dev-ns",
                "  - name: prod-cluster",
                "    context:",
                "      cluster: prod",
                "      user: prod-user",
                "clusters:",
                "  - name: dev",
                "    cluster:",
                "      server: https://dev.example.com",
                "  - name: prod",
                "    cluster:",
                "      server: https://prod.example.com",
                "users:",
                "  - name: dev-user",
                "  - name: prod-user",
            ]
        ),
    )
    _write(
        docker_config,
        json.dumps(
            {
                "currentContext": "desktop-linux",
            }
        ),
    )
    _write(
        docker_context_meta,
        json.dumps(
            {
                "Name": "desktop-linux",
                "Metadata": {
                    "Description": "Docker Desktop",
                },
                "Endpoints": {
                    "docker": {
                        "Host": "npipe:////./pipe/dockerDesktopLinuxEngine",
                    }
                },
            }
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-kubeconfig",
            str(kubeconfig),
            "--system-docker-config",
            str(docker_config),
            "--system-docker-context-meta",
            str(docker_context_meta),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "kubeconfig" in kinds
    assert "docker_cli_config" in kinds
    assert "docker_context_meta" in kinds

    kubernetes_profile = report["derived_profile"]["kubernetes_profile"]
    assert kubernetes_profile["present"] is True
    assert kubernetes_profile["current_context"] == "prod-cluster"
    assert kubernetes_profile["context_names"] == ["dev-cluster", "prod-cluster"]
    assert kubernetes_profile["cluster_names"] == ["dev", "prod"]
    assert kubernetes_profile["namespace_names"] == ["dev-ns"]

    container_profile = report["derived_profile"]["container_tools_profile"]
    assert container_profile["present"] is True
    assert container_profile["docker_current_context"] == "desktop-linux"
    assert container_profile["docker_context_names"] == ["desktop-linux"]
    assert container_profile["docker_context_hosts"] == ["npipe:////./pipe/dockerDesktopLinuxEngine"]

    assert "kubernetes" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_kubeconfig_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / ".kube" / "config",
        "\n".join(
            [
                "apiVersion: v1",
                "kind: Config",
                "current-context: dev-cluster",
                "contexts:",
                "  - name: dev-cluster",
                "    context:",
                "      cluster: dev",
                "      user: dev-user",
                "      namespace: dev-ns",
                "clusters:",
                "  - name: dev",
                "    cluster:",
                "      server: https://dev.example.com",
                "users:",
                "  - name: dev-user",
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "kubeconfig" in kinds

    kubernetes_profile = report["derived_profile"]["kubernetes_profile"]
    assert kubernetes_profile["present"] is True
    assert kubernetes_profile["current_context"] == "dev-cluster"
    assert kubernetes_profile["context_names"] == ["dev-cluster"]
    assert kubernetes_profile["cluster_names"] == ["dev"]
    assert kubernetes_profile["user_names"] == ["dev-user"]


def test_run_scout_discovers_docker_contexts_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / ".docker" / "config.json",
        json.dumps(
            {
                "currentContext": "remote-builder",
            }
        ),
    )
    _write(
        home / ".docker" / "contexts" / "meta" / "ctx1" / "meta.json",
        json.dumps(
            {
                "Name": "remote-builder",
                "Metadata": {
                    "Description": "Remote BuildKit",
                },
                "Endpoints": {
                    "docker": {
                        "Host": "ssh://builder@example.com",
                    }
                },
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "docker_cli_config" in kinds
    assert "docker_context_meta" in kinds

    container_profile = report["derived_profile"]["container_tools_profile"]
    assert container_profile["present"] is True
    assert container_profile["docker_current_context"] == "remote-builder"
    assert container_profile["docker_context_names"] == ["remote-builder"]
    assert container_profile["docker_context_hosts"] == ["ssh://builder@example.com"]


def test_cli_accepts_explicit_aws_and_azure_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    aws_config = tmp_path / ".aws" / "config"
    azure_profile = tmp_path / ".azure" / "azureProfile.json"

    _write(
        aws_config,
        "\n".join(
            [
                "[default]",
                "region = us-east-1",
                "output = json",
                "",
                "[profile prod-admin]",
                "region = eu-west-1",
                "output = table",
                "sso_session = corp-sso",
                "",
                "[sso-session corp-sso]",
                "sso_region = us-east-1",
            ]
        ),
    )
    _write(
        azure_profile,
        json.dumps(
            {
                "subscriptions": [
                    {
                        "id": "sub-dev",
                        "name": "Dev Subscription",
                        "tenantId": "tenant-dev",
                        "state": "Enabled",
                        "isDefault": True,
                        "cloudName": "AzureCloud",
                    },
                    {
                        "id": "sub-prod",
                        "name": "Prod Subscription",
                        "tenantId": "tenant-prod",
                        "state": "Enabled",
                        "isDefault": False,
                        "cloudName": "AzureChinaCloud",
                    },
                ]
            }
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-aws-config",
            str(aws_config),
            "--system-azure-profile",
            str(azure_profile),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "aws_cli_config" in kinds
    assert "azure_cli_profile" in kinds

    cloud_profile = report["derived_profile"]["cloud_tools_profile"]
    assert cloud_profile["present"] is True
    assert cloud_profile["tool_families"] == ["aws_cli", "azure_cli"]
    assert cloud_profile["aws_profile_names"] == ["default", "prod-admin"]
    assert cloud_profile["aws_regions"] == ["eu-west-1", "us-east-1"]
    assert cloud_profile["aws_outputs"] == ["json", "table"]
    assert cloud_profile["aws_sso_sessions"] == ["corp-sso"]
    assert cloud_profile["azure_default_subscription"] == "Dev Subscription"
    assert cloud_profile["azure_subscription_names"] == ["Dev Subscription", "Prod Subscription"]
    assert cloud_profile["azure_cloud_names"] == ["AzureChinaCloud", "AzureCloud"]

    assert "cloud_tooling" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_aws_and_azure_profiles_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / ".aws" / "config",
        "\n".join(
            [
                "[default]",
                "region = ap-southeast-1",
                "output = yaml",
                "",
                "[profile sandbox]",
                "region = us-west-2",
                "output = json",
            ]
        ),
    )
    _write(
        home / ".azure" / "azureProfile.json",
        json.dumps(
            {
                "subscriptions": [
                    {
                        "id": "sub-sandbox",
                        "name": "Sandbox Subscription",
                        "tenantId": "tenant-sandbox",
                        "state": "Enabled",
                        "isDefault": True,
                        "cloudName": "AzureCloud",
                    }
                ]
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "aws_cli_config" in kinds
    assert "azure_cli_profile" in kinds

    cloud_profile = report["derived_profile"]["cloud_tools_profile"]
    assert cloud_profile["present"] is True
    assert cloud_profile["aws_profile_names"] == ["default", "sandbox"]
    assert cloud_profile["aws_regions"] == ["ap-southeast-1", "us-west-2"]
    assert cloud_profile["aws_outputs"] == ["json", "yaml"]
    assert cloud_profile["azure_default_subscription"] == "Sandbox Subscription"
    assert cloud_profile["azure_subscription_names"] == ["Sandbox Subscription"]
    assert cloud_profile["azure_cloud_names"] == ["AzureCloud"]


def test_cli_accepts_explicit_gcloud_config_root_probe_path(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    gcloud_root = tmp_path / "gcloud"
    _write(gcloud_root / "active_config", "work\n")
    _write(
        gcloud_root / "configurations" / "config_default",
        "\n".join(
            [
                "[core]",
                "account = builder@example.com",
                "project = proj-dev",
                "",
                "[compute]",
                "region = us-central1",
                "zone = us-central1-a",
            ]
        ),
    )
    _write(
        gcloud_root / "configurations" / "config_work",
        "\n".join(
            [
                "[core]",
                "account = builder@corp.example",
                "project = proj-prod",
                "disable_usage_reporting = true",
                "",
                "[compute]",
                "region = asia-east1",
                "zone = asia-east1-b",
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-gcloud-config-root",
            str(gcloud_root),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "gcloud_active_config" in kinds
    assert "gcloud_cli_config" in kinds

    cloud_profile = report["derived_profile"]["cloud_tools_profile"]
    assert cloud_profile["present"] is True
    assert cloud_profile["tool_families"] == ["gcloud_cli"]
    assert cloud_profile["gcloud_active_configuration"] == "work"
    assert cloud_profile["gcloud_configuration_names"] == ["default", "work"]
    assert cloud_profile["gcloud_projects"] == ["proj-dev", "proj-prod"]
    assert cloud_profile["gcloud_regions"] == ["asia-east1", "us-central1"]
    assert cloud_profile["gcloud_zones"] == ["asia-east1-b", "us-central1-a"]

    assert "cloud_tooling" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_gcloud_config_under_roaming(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    gcloud_root = home / "AppData" / "Roaming" / "gcloud"
    _write(gcloud_root / "active_config", "default\n")
    _write(
        gcloud_root / "configurations" / "config_default",
        "\n".join(
            [
                "[core]",
                "account = admin@example.com",
                "project = demo-sandbox",
                "",
                "[compute]",
                "region = europe-west1",
                "zone = europe-west1-c",
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "gcloud_active_config" in kinds
    assert "gcloud_cli_config" in kinds

    cloud_profile = report["derived_profile"]["cloud_tools_profile"]
    assert cloud_profile["present"] is True
    assert cloud_profile["gcloud_active_configuration"] == "default"
    assert cloud_profile["gcloud_configuration_names"] == ["default"]
    assert cloud_profile["gcloud_projects"] == ["demo-sandbox"]
    assert cloud_profile["gcloud_regions"] == ["europe-west1"]
    assert cloud_profile["gcloud_zones"] == ["europe-west1-c"]


def test_cli_accepts_explicit_github_cli_config_root_probe_path(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    gh_root = tmp_path / "GitHub CLI"
    _write(
        gh_root / "config.yml",
        "\n".join(
            [
                "version: \"1\"",
                "git_protocol: ssh",
                "editor: code --wait",
                "prompt: enabled",
            ]
        ),
    )
    _write(
        gh_root / "hosts.yml",
        "\n".join(
            [
                "github.com:",
                "    user: octocat",
                "    oauth_token: ghp_secret",
                "    git_protocol: ssh",
                "ghe.example.com:",
                "    user: builder",
                "    git_protocol: https",
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-github-cli-config-root",
            str(gh_root),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "github_cli_config" in kinds
    assert "github_cli_hosts" in kinds

    source_control_profile = report["derived_profile"]["source_control_profile"]
    assert source_control_profile["present"] is True
    assert source_control_profile["tool_families"] == ["github_cli"]
    assert source_control_profile["gh_hosts"] == ["ghe.example.com", "github.com"]
    assert source_control_profile["gh_users"] == ["builder", "octocat"]
    assert source_control_profile["gh_git_protocols"] == ["https", "ssh"]
    assert source_control_profile["gh_authenticated_hosts"] == ["github.com"]
    assert source_control_profile["gh_editor"] == "code --wait"
    assert source_control_profile["gh_prompt"] == "enabled"

    assert "source_control" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_github_cli_config_under_roaming(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    gh_root = home / "AppData" / "Roaming" / "GitHub CLI"
    _write(
        gh_root / "config.yml",
        "\n".join(
            [
                "version: \"1\"",
                "git_protocol: https",
                "editor: notepad",
                "prompt: disabled",
            ]
        ),
    )
    _write(
        gh_root / "hosts.yml",
        "\n".join(
            [
                "github.com:",
                "    user: sandbox-user",
                "    oauth_token: ghp_sandbox",
                "    git_protocol: https",
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "github_cli_config" in kinds
    assert "github_cli_hosts" in kinds

    source_control_profile = report["derived_profile"]["source_control_profile"]
    assert source_control_profile["present"] is True
    assert source_control_profile["gh_hosts"] == ["github.com"]
    assert source_control_profile["gh_users"] == ["sandbox-user"]
    assert source_control_profile["gh_git_protocols"] == ["https"]
    assert source_control_profile["gh_authenticated_hosts"] == ["github.com"]
    assert source_control_profile["gh_editor"] == "notepad"
    assert source_control_profile["gh_prompt"] == "disabled"


def test_cli_accepts_explicit_gitconfig_and_npmrc_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    gitconfig = tmp_path / ".gitconfig"
    npmrc = tmp_path / ".npmrc"

    _write(
        gitconfig,
        "\n".join(
            [
                "[user]",
                "    name = Builder Example",
                "    email = builder@example.com",
                "[core]",
                "    editor = code --wait",
                "[init]",
                "    defaultBranch = main",
                "[pull]",
                "    rebase = true",
                "[github]",
                "    user = octocat",
            ]
        ),
    )
    _write(
        npmrc,
        "\n".join(
            [
                "registry=https://registry.npmjs.org/",
                "save-exact=true",
                "prefix=C:\\Users\\Admin\\npm",
                "@acme:registry=https://npm.pkg.github.com",
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-gitconfig",
            str(gitconfig),
            "--system-npmrc",
            str(npmrc),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "git_global_config" in kinds
    assert "npm_user_config" in kinds

    source_control_profile = report["derived_profile"]["source_control_profile"]
    assert source_control_profile["present"] is True
    assert source_control_profile["tool_families"] == ["git", "npm"]
    assert source_control_profile["git_user_name"] == "Builder Example"
    assert source_control_profile["git_user_email"] == "builder@example.com"
    assert source_control_profile["git_default_branch"] == "main"
    assert source_control_profile["git_editor"] == "code --wait"
    assert source_control_profile["git_pull_rebase"] is True
    assert source_control_profile["git_github_user"] == "octocat"
    assert source_control_profile["npm_registry"] == "https://registry.npmjs.org/"
    assert source_control_profile["npm_scope_registries"] == ["@acme"]
    assert source_control_profile["npm_save_exact"] is True
    assert source_control_profile["npm_prefix"] == "C:\\Users\\Admin\\npm"

    developer_profile = report["derived_profile"]["developer_profile"]
    assert "npm" in developer_profile["package_tooling"]

    assert "source_control" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_gitconfig_and_npmrc_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / ".gitconfig",
        "\n".join(
            [
                "[user]",
                "    name = Sandbox User",
                "    email = sandbox@example.com",
                "[init]",
                "    defaultBranch = trunk",
                "[pull]",
                "    rebase = false",
            ]
        ),
    )
    _write(
        home / ".npmrc",
        "\n".join(
            [
                "registry=https://registry.yarnpkg.com/",
                "save-exact=false",
                "@demo:registry=https://npm.example.com",
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "git_global_config" in kinds
    assert "npm_user_config" in kinds

    source_control_profile = report["derived_profile"]["source_control_profile"]
    assert source_control_profile["present"] is True
    assert source_control_profile["git_user_name"] == "Sandbox User"
    assert source_control_profile["git_user_email"] == "sandbox@example.com"
    assert source_control_profile["git_default_branch"] == "trunk"
    assert source_control_profile["git_pull_rebase"] is False
    assert source_control_profile["npm_registry"] == "https://registry.yarnpkg.com/"
    assert source_control_profile["npm_scope_registries"] == ["@demo"]
    assert source_control_profile["npm_save_exact"] is False


def test_cli_accepts_explicit_pip_config_and_condarc_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    pip_config = tmp_path / "pip" / "pip.ini"
    condarc = tmp_path / ".condarc"

    _write(
        pip_config,
        "\n".join(
            [
                "[global]",
                "index-url = https://pypi.org/simple",
                "trusted-host =",
                "    mirror1.example.com",
                "    mirror2.example.com",
                "timeout = 30",
                "disable-pip-version-check = true",
                "",
                "[install]",
                "extra-index-url = https://pkg.example.com/simple",
            ]
        ),
    )
    _write(
        condarc,
        "\n".join(
            [
                "channels:",
                "  - conda-forge",
                "  - defaults",
                "envs_dirs:",
                "  - D:\\conda\\envs",
                "auto_activate_base: false",
                "changeps1: false",
                "show_channel_urls: true",
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-pip-config",
            str(pip_config),
            "--system-condarc",
            str(condarc),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "pip_user_config" in kinds
    assert "conda_user_config" in kinds

    python_profile = report["derived_profile"]["python_tooling_profile"]
    assert python_profile["present"] is True
    assert python_profile["tool_families"] == ["conda", "pip"]
    assert python_profile["pip_index_url"] == "https://pypi.org/simple"
    assert python_profile["pip_trusted_hosts"] == ["mirror1.example.com", "mirror2.example.com"]
    assert python_profile["pip_extra_index_urls"] == ["https://pkg.example.com/simple"]
    assert python_profile["pip_timeout"] == 30
    assert python_profile["pip_disable_version_check"] is True
    assert python_profile["conda_channels"] == ["conda-forge", "defaults"]
    assert python_profile["conda_envs_dirs"] == ["D:\\conda\\envs"]
    assert python_profile["conda_auto_activate_base"] is False
    assert python_profile["conda_changeps1"] is False
    assert python_profile["conda_show_channel_urls"] is True

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["package_tooling"] == ["conda", "pip"]

    assert "python_tooling" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_pip_config_and_condarc_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / "AppData" / "Roaming" / "pip" / "pip.ini",
        "\n".join(
            [
                "[global]",
                "index-url = https://mirror.example.com/simple",
                "timeout = 15",
                "",
                "[install]",
                "extra-index-url = https://packages.example.com/simple",
            ]
        ),
    )
    _write(
        home / ".condarc",
        "\n".join(
            [
                "channels:",
                "  - defaults",
                "auto_activate_base: true",
                "changeps1: true",
                "show_channel_urls: false",
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "pip_user_config" in kinds
    assert "conda_user_config" in kinds

    python_profile = report["derived_profile"]["python_tooling_profile"]
    assert python_profile["present"] is True
    assert python_profile["pip_index_url"] == "https://mirror.example.com/simple"
    assert python_profile["pip_extra_index_urls"] == ["https://packages.example.com/simple"]
    assert python_profile["pip_timeout"] == 15
    assert python_profile["conda_channels"] == ["defaults"]
    assert python_profile["conda_auto_activate_base"] is True
    assert python_profile["conda_changeps1"] is True
    assert python_profile["conda_show_channel_urls"] is False


def test_cli_accepts_explicit_poetry_config_and_yarnrc_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    poetry_config = tmp_path / "pypoetry" / "config.toml"
    yarnrc = tmp_path / ".yarnrc.yml"

    _write(
        poetry_config,
        "\n".join(
            [
                "system-git-client = true",
                "",
                "[virtualenvs]",
                "create = false",
                "in-project = true",
                "path = \"D:\\\\Poetry\\\\venvs\"",
                "",
                "[installer]",
                "parallel = false",
                "max-workers = 6",
            ]
        ),
    )
    _write(
        yarnrc,
        "\n".join(
            [
                "nodeLinker: node-modules",
                "npmRegistryServer: \"https://registry.npmjs.org\"",
                "enableGlobalCache: true",
                "enableTelemetry: false",
                "globalFolder: \"D:\\\\Users\\\\Admin\\\\.yarn\\\\berry\"",
                "yarnPath: \".yarn/releases/yarn-4.1.0.cjs\"",
                "npmScopes:",
                "  acme:",
                "    npmRegistryServer: \"https://npm.pkg.github.com\"",
                "  internal:",
                "    npmRegistryServer: \"https://packages.example.com\"",
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-poetry-config",
            str(poetry_config),
            "--system-yarnrc-yml",
            str(yarnrc),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "poetry_user_config" in kinds
    assert "yarn_user_config" in kinds

    python_profile = report["derived_profile"]["python_tooling_profile"]
    assert python_profile["present"] is True
    assert python_profile["tool_families"] == ["poetry"]
    assert python_profile["poetry_virtualenvs_create"] is False
    assert python_profile["poetry_virtualenvs_in_project"] is True
    assert python_profile["poetry_virtualenvs_path"] == "D:\\Poetry\\venvs"
    assert python_profile["poetry_installer_parallel"] is False
    assert python_profile["poetry_installer_max_workers"] == 6
    assert python_profile["poetry_system_git_client"] is True

    javascript_profile = report["derived_profile"]["javascript_tooling_profile"]
    assert javascript_profile["present"] is True
    assert javascript_profile["tool_families"] == ["yarn"]
    assert javascript_profile["yarn_node_linker"] == "node-modules"
    assert javascript_profile["yarn_npm_registry_server"] == "https://registry.npmjs.org"
    assert javascript_profile["yarn_enable_global_cache"] is True
    assert javascript_profile["yarn_enable_telemetry"] is False
    assert javascript_profile["yarn_global_folder"] == "D:\\Users\\Admin\\.yarn\\berry"
    assert javascript_profile["yarn_path"] == ".yarn/releases/yarn-4.1.0.cjs"
    assert javascript_profile["yarn_npm_scope_names"] == ["acme", "internal"]
    assert javascript_profile["yarn_npm_scope_registries"] == [
        "https://npm.pkg.github.com",
        "https://packages.example.com",
    ]

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["is_developer"] is True
    assert developer_profile["package_tooling"] == ["poetry", "yarn"]

    assert "python_tooling" in report["derived_profile"]["interest_tags"]
    assert "javascript_tooling" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_poetry_config_and_yarnrc_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / "AppData" / "Roaming" / "pypoetry" / "config.toml",
        "\n".join(
            [
                "[virtualenvs]",
                "create = true",
                "in-project = false",
                "path = \"D:\\\\Poetry\\\\cache\\\\venvs\"",
                "",
                "[installer]",
                "parallel = true",
                "max-workers = 10",
            ]
        ),
    )
    _write(
        home / ".yarnrc.yml",
        "\n".join(
            [
                "nodeLinker: pnp",
                "npmRegistryServer: \"https://registry.yarnpkg.com\"",
                "enableGlobalCache: false",
                "enableTelemetry: true",
                "npmScopes:",
                "  demo:",
                "    npmRegistryServer: \"https://npm.example.com\"",
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "poetry_user_config" in kinds
    assert "yarn_user_config" in kinds

    python_profile = report["derived_profile"]["python_tooling_profile"]
    assert python_profile["present"] is True
    assert python_profile["poetry_virtualenvs_create"] is True
    assert python_profile["poetry_virtualenvs_in_project"] is False
    assert python_profile["poetry_virtualenvs_path"] == "D:\\Poetry\\cache\\venvs"
    assert python_profile["poetry_installer_parallel"] is True
    assert python_profile["poetry_installer_max_workers"] == 10

    javascript_profile = report["derived_profile"]["javascript_tooling_profile"]
    assert javascript_profile["present"] is True
    assert javascript_profile["yarn_node_linker"] == "pnp"
    assert javascript_profile["yarn_npm_registry_server"] == "https://registry.yarnpkg.com"
    assert javascript_profile["yarn_enable_global_cache"] is False
    assert javascript_profile["yarn_enable_telemetry"] is True
    assert javascript_profile["yarn_npm_scope_names"] == ["demo"]
    assert javascript_profile["yarn_npm_scope_registries"] == ["https://npm.example.com"]

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["package_tooling"] == ["poetry", "yarn"]


def test_cli_accepts_explicit_pnpm_and_uv_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    pnpm_config = tmp_path / "pnpm" / "config" / "rc"
    uv_config = tmp_path / "uv" / "uv.toml"
    uv_credentials = tmp_path / "uv" / "data" / "credentials" / "credentials.toml"

    _write(
        pnpm_config,
        "\n".join(
            [
                "registry=https://registry.npmjs.org/",
                "global-dir=D:\\pnpm\\global",
                "store-dir=D:\\pnpm\\store",
                "package-import-method=clone",
                "node-linker=isolated",
                "shamefully-hoist=false",
                "@acme:registry=https://npm.pkg.github.com",
            ]
        ),
    )
    _write(
        uv_config,
        "\n".join(
            [
                'index-url = "https://pypi.org/simple"',
                'extra-index-url = ["https://packages.example.com/simple"]',
                'cache-dir = "D:\\\\uv\\\\cache"',
                'python-preference = "only-managed"',
                "native-tls = true",
                "offline = false",
                "preview = true",
                "",
                "[pip]",
                'index-url = "https://mirror.example.com/simple"',
            ]
        ),
    )
    _write(
        uv_credentials,
        "\n".join(
            [
                "[service.primary]",
                'url = "https://pkg.example.com/simple"',
                'username = "builder"',
                'password = "uv-secret"',
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-pnpm-config",
            str(pnpm_config),
            "--system-uv-config",
            str(uv_config),
            "--system-uv-credentials",
            str(uv_credentials),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "pnpm_user_config" in kinds
    assert "uv_user_config" in kinds
    assert "uv_credentials_store" in kinds

    javascript_profile = report["derived_profile"]["javascript_tooling_profile"]
    assert javascript_profile["present"] is True
    assert javascript_profile["tool_families"] == ["pnpm"]
    assert javascript_profile["pnpm_registry"] == "https://registry.npmjs.org/"
    assert javascript_profile["pnpm_scope_registries"] == ["@acme"]
    assert javascript_profile["pnpm_global_dir"] == "D:\\pnpm\\global"
    assert javascript_profile["pnpm_store_dir"] == "D:\\pnpm\\store"
    assert javascript_profile["pnpm_package_import_method"] == "clone"
    assert javascript_profile["pnpm_node_linker"] == "isolated"
    assert javascript_profile["pnpm_shamefully_hoist"] is False

    python_profile = report["derived_profile"]["python_tooling_profile"]
    assert python_profile["present"] is True
    assert python_profile["tool_families"] == ["uv"]
    assert python_profile["uv_index_url"] == "https://pypi.org/simple"
    assert python_profile["uv_extra_index_urls"] == ["https://packages.example.com/simple"]
    assert python_profile["uv_cache_dir"] == "D:\\uv\\cache"
    assert python_profile["uv_python_preference"] == "only-managed"
    assert python_profile["uv_native_tls"] is True
    assert python_profile["uv_offline"] is False
    assert python_profile["uv_preview"] is True
    assert python_profile["uv_pip_index_url"] == "https://mirror.example.com/simple"
    assert python_profile["uv_auth_present"] is True
    assert python_profile["uv_auth_service_urls"] == ["https://pkg.example.com/simple"]
    assert python_profile["uv_auth_usernames"] == ["builder"]

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["package_tooling"] == ["pnpm", "uv"]

    assert "python_tooling" in report["derived_profile"]["interest_tags"]
    assert "javascript_tooling" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_pnpm_and_uv_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / "AppData" / "Local" / "pnpm" / "config" / "rc",
        "\n".join(
            [
                "registry=https://registry.npmjs.org/",
                "store-dir=D:\\pnpm\\store-cache",
                "package-import-method=hardlink",
                "node-linker=hoisted",
            ]
        ),
    )
    _write(
        home / "AppData" / "Roaming" / "uv" / "uv.toml",
        "\n".join(
            [
                'index-url = "https://internal.example.com/simple"',
                'cache-dir = "D:\\\\uv\\\\cache-home"',
                'python-preference = "managed"',
                "native-tls = false",
                "offline = true",
            ]
        ),
    )
    _write(
        home / "AppData" / "Roaming" / "uv" / "data" / "credentials" / "credentials.toml",
        "\n".join(
            [
                "[service.internal]",
                'url = "https://internal.example.com/simple"',
                'username = "sandbox-user"',
                'password = "secret"',
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "pnpm_user_config" in kinds
    assert "uv_user_config" in kinds
    assert "uv_credentials_store" in kinds

    javascript_profile = report["derived_profile"]["javascript_tooling_profile"]
    assert javascript_profile["present"] is True
    assert javascript_profile["tool_families"] == ["pnpm"]
    assert javascript_profile["pnpm_registry"] == "https://registry.npmjs.org/"
    assert javascript_profile["pnpm_store_dir"] == "D:\\pnpm\\store-cache"
    assert javascript_profile["pnpm_package_import_method"] == "hardlink"
    assert javascript_profile["pnpm_node_linker"] == "hoisted"

    python_profile = report["derived_profile"]["python_tooling_profile"]
    assert python_profile["present"] is True
    assert python_profile["tool_families"] == ["uv"]
    assert python_profile["uv_index_url"] == "https://internal.example.com/simple"
    assert python_profile["uv_cache_dir"] == "D:\\uv\\cache-home"
    assert python_profile["uv_python_preference"] == "managed"
    assert python_profile["uv_native_tls"] is False
    assert python_profile["uv_offline"] is True
    assert python_profile["uv_auth_present"] is True
    assert python_profile["uv_auth_service_urls"] == ["https://internal.example.com/simple"]
    assert python_profile["uv_auth_usernames"] == ["sandbox-user"]

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["package_tooling"] == ["pnpm", "uv"]


def test_cli_accepts_explicit_cargo_and_rustup_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    cargo_config = tmp_path / ".cargo" / "config.toml"
    cargo_credentials = tmp_path / ".cargo" / "credentials.toml"
    rustup_settings = tmp_path / ".rustup" / "settings.toml"

    _write(
        cargo_config,
        "\n".join(
            [
                "[build]",
                'target-dir = "D:\\\\cargo\\\\target"',
                "",
                "[term]",
                "verbose = true",
                "",
                "[registries.crates-io]",
                'protocol = "sparse"',
                "",
                "[registry]",
                'default = "internal"',
                "",
                "[net]",
                "git-fetch-with-cli = true",
                "retry = 3",
            ]
        ),
    )
    _write(
        cargo_credentials,
        "\n".join(
            [
                "[registries.crates-io]",
                'token = "cargo-secret"',
                "",
                "[registries.internal]",
                'token = "internal-secret"',
            ]
        ),
    )
    _write(
        rustup_settings,
        "\n".join(
            [
                'version = "12"',
                'default_toolchain = "stable-x86_64-pc-windows-msvc"',
                'profile = "default"',
                "",
                "[overrides]",
                '"D:\\\\work\\\\demo" = "nightly-x86_64-pc-windows-msvc"',
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-cargo-config",
            str(cargo_config),
            "--system-cargo-credentials",
            str(cargo_credentials),
            "--system-rustup-settings",
            str(rustup_settings),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "cargo_user_config" in kinds
    assert "cargo_credentials_store" in kinds
    assert "rustup_settings" in kinds

    rust_profile = report["derived_profile"]["rust_tooling_profile"]
    assert rust_profile["present"] is True
    assert rust_profile["tool_families"] == ["cargo", "rustup"]
    assert rust_profile["cargo_target_dir"] == "D:\\cargo\\target"
    assert rust_profile["cargo_term_verbose"] is True
    assert rust_profile["cargo_default_registry"] == "internal"
    assert rust_profile["cargo_crates_io_protocol"] == "sparse"
    assert rust_profile["cargo_git_fetch_with_cli"] is True
    assert rust_profile["cargo_net_retry"] == 3
    assert rust_profile["cargo_credentials_present"] is True
    assert rust_profile["cargo_registry_names"] == ["crates-io", "internal"]
    assert rust_profile["rustup_default_toolchain"] == "stable-x86_64-pc-windows-msvc"
    assert rust_profile["rustup_profile"] == "default"
    assert rust_profile["rustup_override_count"] == 1
    assert rust_profile["rustup_settings_version"] == "12"

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["package_tooling"] == ["cargo"]
    assert "rust" in developer_profile["language_tooling"]

    assert "rust_tooling" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_cargo_and_rustup_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / ".cargo" / "config",
        "\n".join(
            [
                "[build]",
                'target-dir = "D:\\\\cargo\\\\legacy-target"',
                "",
                "[registries.crates-io]",
                'protocol = "git"',
                "",
                "[net]",
                "git-fetch-with-cli = false",
            ]
        ),
    )
    _write(
        home / ".cargo" / "credentials",
        "\n".join(
            [
                "[registries.crates-io]",
                'token = "legacy-secret"',
            ]
        ),
    )
    _write(
        home / ".rustup" / "settings.toml",
        "\n".join(
            [
                'version = "12"',
                'default_toolchain = "nightly-x86_64-pc-windows-msvc"',
                'profile = "minimal"',
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "cargo_user_config" in kinds
    assert "cargo_credentials_store" in kinds
    assert "rustup_settings" in kinds

    rust_profile = report["derived_profile"]["rust_tooling_profile"]
    assert rust_profile["present"] is True
    assert rust_profile["tool_families"] == ["cargo", "rustup"]
    assert rust_profile["cargo_target_dir"] == "D:\\cargo\\legacy-target"
    assert rust_profile["cargo_crates_io_protocol"] == "git"
    assert rust_profile["cargo_git_fetch_with_cli"] is False
    assert rust_profile["cargo_credentials_present"] is True
    assert rust_profile["cargo_registry_names"] == ["crates-io"]
    assert rust_profile["rustup_default_toolchain"] == "nightly-x86_64-pc-windows-msvc"
    assert rust_profile["rustup_profile"] == "minimal"
    assert rust_profile["rustup_override_count"] == 0

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["package_tooling"] == ["cargo"]
    assert "rust" in developer_profile["language_tooling"]


def test_cli_accepts_explicit_maven_and_gradle_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    maven_settings = tmp_path / ".m2" / "settings.xml"
    gradle_properties = tmp_path / ".gradle" / "gradle.properties"

    _write(
        maven_settings,
        """<settings xmlns="http://maven.apache.org/SETTINGS/1.0.0">
  <localRepository>D:\\m2\\repo</localRepository>
  <offline>true</offline>
  <pluginGroups>
    <pluginGroup>com.example.plugins</pluginGroup>
  </pluginGroups>
  <servers>
    <server>
      <id>repo-1</id>
      <username>builder</username>
      <password>secret</password>
    </server>
    <server>
      <id>snapshots</id>
      <privateKey>${user.home}/.ssh/id_rsa</privateKey>
    </server>
  </servers>
  <mirrors>
    <mirror>
      <id>internal</id>
      <url>https://maven.example.com/repository</url>
    </mirror>
  </mirrors>
  <proxies>
    <proxy>
      <id>corp-proxy</id>
      <active>true</active>
      <host>proxy.example.com</host>
    </proxy>
  </proxies>
  <profiles>
    <profile>
      <id>corp</id>
    </profile>
  </profiles>
  <activeProfiles>
    <activeProfile>corp</activeProfile>
  </activeProfiles>
</settings>
""",
    )
    _write(
        gradle_properties,
        "\n".join(
            [
                "org.gradle.caching=true",
                "org.gradle.parallel=false",
                "org.gradle.configuration-cache=true",
                "org.gradle.daemon=true",
                "org.gradle.jvmargs=-Xmx2g -Dfile.encoding=UTF-8",
                "org.gradle.java.home=C:\\Java\\jdk-21",
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-maven-settings",
            str(maven_settings),
            "--system-gradle-properties",
            str(gradle_properties),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "maven_user_settings" in kinds
    assert "gradle_user_properties" in kinds

    jvm_profile = report["derived_profile"]["jvm_tooling_profile"]
    assert jvm_profile["present"] is True
    assert jvm_profile["tool_families"] == ["gradle", "maven"]
    assert jvm_profile["maven_local_repository"] == "D:\\m2\\repo"
    assert jvm_profile["maven_offline"] is True
    assert jvm_profile["maven_plugin_groups"] == ["com.example.plugins"]
    assert jvm_profile["maven_mirror_ids"] == ["internal"]
    assert jvm_profile["maven_mirror_urls"] == ["https://maven.example.com/repository"]
    assert jvm_profile["maven_server_ids"] == ["repo-1", "snapshots"]
    assert jvm_profile["maven_credentials_present"] is True
    assert jvm_profile["maven_active_profiles"] == ["corp"]
    assert jvm_profile["maven_proxy_hosts"] == ["proxy.example.com"]
    assert jvm_profile["gradle_caching"] is True
    assert jvm_profile["gradle_parallel"] is False
    assert jvm_profile["gradle_configuration_cache"] is True
    assert jvm_profile["gradle_daemon"] is True
    assert jvm_profile["gradle_jvmargs"] == "-Xmx2g -Dfile.encoding=UTF-8"
    assert jvm_profile["gradle_java_home"] == "C:\\Java\\jdk-21"

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["package_tooling"] == ["gradle", "maven"]
    assert "java" in developer_profile["language_tooling"]

    assert "jvm_tooling" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_maven_and_gradle_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / ".m2" / "settings.xml",
        """<settings xmlns="http://maven.apache.org/SETTINGS/1.0.0">
  <offline>false</offline>
  <mirrors>
    <mirror>
      <id>central-proxy</id>
      <url>https://repo.example.com/maven</url>
    </mirror>
  </mirrors>
  <servers>
    <server>
      <id>central-proxy</id>
      <username>sandbox</username>
      <password>secret</password>
    </server>
  </servers>
</settings>
""",
    )
    _write(
        home / ".gradle" / "gradle.properties",
        "\n".join(
            [
                "org.gradle.caching=false",
                "org.gradle.parallel=true",
                "org.gradle.daemon=false",
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "maven_user_settings" in kinds
    assert "gradle_user_properties" in kinds

    jvm_profile = report["derived_profile"]["jvm_tooling_profile"]
    assert jvm_profile["present"] is True
    assert jvm_profile["tool_families"] == ["gradle", "maven"]
    assert jvm_profile["maven_offline"] is False
    assert jvm_profile["maven_mirror_ids"] == ["central-proxy"]
    assert jvm_profile["maven_mirror_urls"] == ["https://repo.example.com/maven"]
    assert jvm_profile["maven_server_ids"] == ["central-proxy"]
    assert jvm_profile["maven_credentials_present"] is True
    assert jvm_profile["gradle_caching"] is False
    assert jvm_profile["gradle_parallel"] is True
    assert jvm_profile["gradle_daemon"] is False

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["package_tooling"] == ["gradle", "maven"]
    assert "java" in developer_profile["language_tooling"]


def test_cli_accepts_explicit_nuget_and_dotnet_tools_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    nuget_config = tmp_path / "NuGet" / "NuGet.Config"
    dotnet_tools_dir = tmp_path / ".dotnet" / "tools"

    _write(
        nuget_config,
        """<configuration>
  <config>
    <add key="globalPackagesFolder" value="D:\\nuget\\packages" />
    <add key="defaultPushSource" value="https://nuget.example.com/v3/index.json" />
    <add key="signatureValidationMode" value="require" />
  </config>
  <packageSources>
    <add key="nuget.org" value="https://api.nuget.org/v3/index.json" />
    <add key="internal" value="https://nuget.example.com/v3/index.json" />
  </packageSources>
  <disabledPackageSources>
    <add key="legacy" value="true" />
  </disabledPackageSources>
  <packageSourceCredentials>
    <internal>
      <add key="Username" value="builder" />
      <add key="ClearTextPassword" value="secret" />
    </internal>
  </packageSourceCredentials>
</configuration>
""",
    )
    _write(dotnet_tools_dir / "dotnetsay.exe", "")
    _write(dotnet_tools_dir / "fantomas-tool.exe", "")
    _write(dotnet_tools_dir / ".store" / "ignore.txt", "")

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-nuget-config",
            str(nuget_config),
            "--system-dotnet-tools-dir",
            str(dotnet_tools_dir),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "nuget_user_config" in kinds
    assert "dotnet_global_tools" in kinds

    dotnet_profile = report["derived_profile"]["dotnet_tooling_profile"]
    assert dotnet_profile["present"] is True
    assert dotnet_profile["tool_families"] == ["dotnet_tools", "nuget"]
    assert dotnet_profile["nuget_global_packages_folder"] == "D:\\nuget\\packages"
    assert dotnet_profile["nuget_default_push_source"] == "https://nuget.example.com/v3/index.json"
    assert dotnet_profile["nuget_signature_validation_mode"] == "require"
    assert dotnet_profile["nuget_package_source_names"] == ["internal", "nuget.org"]
    assert dotnet_profile["nuget_package_source_urls"] == [
        "https://api.nuget.org/v3/index.json",
        "https://nuget.example.com/v3/index.json",
    ]
    assert dotnet_profile["nuget_disabled_sources"] == ["legacy"]
    assert dotnet_profile["nuget_credentials_present"] is True
    assert dotnet_profile["nuget_credential_sources"] == ["internal"]
    assert dotnet_profile["global_tool_commands"] == ["dotnetsay", "fantomas-tool"]
    assert dotnet_profile["global_tool_count"] == 2

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["package_tooling"] == ["nuget"]
    assert "dotnet" in developer_profile["language_tooling"]

    assert "dotnet_tooling" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_nuget_and_dotnet_tools_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / "AppData" / "Roaming" / "NuGet" / "NuGet.Config",
        """<configuration>
  <packageSources>
    <add key="nuget.org" value="https://api.nuget.org/v3/index.json" />
  </packageSources>
  <packageSourceCredentials>
    <nuget.org>
      <add key="Username" value="sandbox" />
      <add key="ClearTextPassword" value="secret" />
    </nuget.org>
  </packageSourceCredentials>
</configuration>
""",
    )
    _write(home / ".dotnet" / "tools" / "csharpier.exe", "")

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=4,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "nuget_user_config" in kinds
    assert "dotnet_global_tools" in kinds

    dotnet_profile = report["derived_profile"]["dotnet_tooling_profile"]
    assert dotnet_profile["present"] is True
    assert dotnet_profile["tool_families"] == ["dotnet_tools", "nuget"]
    assert dotnet_profile["nuget_package_source_names"] == ["nuget.org"]
    assert dotnet_profile["nuget_credentials_present"] is True
    assert dotnet_profile["nuget_credential_sources"] == ["nuget.org"]
    assert dotnet_profile["global_tool_commands"] == ["csharpier"]
    assert dotnet_profile["global_tool_count"] == 1

    developer_profile = report["derived_profile"]["developer_profile"]
    assert developer_profile["package_tooling"] == ["nuget"]
    assert "dotnet" in developer_profile["language_tooling"]


def test_parse_claude_settings_supports_object_style_enabled_plugins(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_claude_settings

    path = _write(
        tmp_path / "settings.json",
        json.dumps(
            {
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": "secret",
                },
                "enabledPlugins": {
                    "compound-engineering@every-marketplace": True,
                    "frontend-design@claude-plugins-official": True,
                    "disabled-plugin@example": False,
                },
            }
        ),
    )

    parsed = parse_claude_settings(path)

    assert parsed["env_keys"] == ["ANTHROPIC_AUTH_TOKEN"]
    assert parsed["package_names"] == [
        "compound-engineering@every-marketplace",
        "disabled-plugin@example",
        "frontend-design@claude-plugins-official",
    ]
    assert parsed["enabled_plugins"] == [
        "compound-engineering@every-marketplace",
        "frontend-design@claude-plugins-official",
    ]


def test_parse_pnpm_user_config_extracts_explicit_fields(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_pnpm_user_config

    path = _write(
        tmp_path / "rc",
        "\n".join(
            [
                "registry=https://registry.npmjs.org/",
                "global-dir=D:\\pnpm\\global",
                "store-dir=D:\\pnpm\\store",
                "package-import-method=clone",
                "node-linker=isolated",
                "shamefully-hoist=true",
                "@scope:registry=https://npm.pkg.github.com",
            ]
        ),
    )

    parsed = parse_pnpm_user_config(path)

    assert parsed["registry"] == "https://registry.npmjs.org/"
    assert parsed["global_dir"] == "D:\\pnpm\\global"
    assert parsed["store_dir"] == "D:\\pnpm\\store"
    assert parsed["package_import_method"] == "clone"
    assert parsed["node_linker"] == "isolated"
    assert parsed["shamefully_hoist"] is True
    assert parsed["scope_registries"] == ["@scope"]


def test_parse_uv_credentials_store_surfaces_presence_without_secret_values(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_uv_credentials_store

    path = _write(
        tmp_path / "credentials.toml",
        "\n".join(
            [
                "[service.primary]",
                'url = "https://pkg.example.com/simple"',
                'username = "builder"',
                'password = "super-secret"',
                "",
                "[service.secondary]",
                'registry = "https://mirror.example.com/simple"',
                'token = "token-secret"',
            ]
        ),
    )

    parsed = parse_uv_credentials_store(path)

    assert parsed["credential_count"] == 2
    credentials = parsed["credentials"]
    primary = _assert_contains_fields(
        credentials,
        {
            "service_name": "service.primary",
            "url": "https://pkg.example.com/simple",
            "username": "builder",
            "password_present": True,
            "token_present": False,
        },
    )
    secondary = _assert_contains_fields(
        credentials,
        {
            "service_name": "service.secondary",
            "url": "https://mirror.example.com/simple",
            "password_present": False,
            "token_present": True,
        },
    )

    assert "password" not in primary
    assert "token" not in secondary


def test_parse_cargo_user_config_extracts_explicit_fields(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_cargo_user_config

    path = _write(
        tmp_path / "config.toml",
        "\n".join(
            [
                "[build]",
                'target-dir = "D:\\\\cargo\\\\target"',
                "",
                "[registries.crates-io]",
                'protocol = "sparse"',
                "",
                "[registry]",
                'default = "internal"',
                "",
                "[term]",
                "verbose = false",
                "",
                "[net]",
                "git-fetch-with-cli = true",
                "retry = 5",
            ]
        ),
    )

    parsed = parse_cargo_user_config(path)

    assert parsed["target_dir"] == "D:\\cargo\\target"
    assert parsed["term_verbose"] is False
    assert parsed["default_registry"] == "internal"
    assert parsed["crates_io_protocol"] == "sparse"
    assert parsed["git_fetch_with_cli"] is True
    assert parsed["net_retry"] == 5


def test_parse_cargo_credentials_store_surfaces_registry_presence_only(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_cargo_credentials_store

    path = _write(
        tmp_path / "credentials.toml",
        "\n".join(
            [
                "[registries.crates-io]",
                'token = "cargo-secret"',
                "",
                "[registries.internal]",
                'token = "internal-secret"',
            ]
        ),
    )

    parsed = parse_cargo_credentials_store(path)

    assert parsed["credential_count"] == 2
    assert parsed["registry_names"] == ["crates-io", "internal"]
    assert parsed["token_registries"] == ["crates-io", "internal"]
    assert "token" not in parsed


def test_parse_rustup_settings_extracts_explicit_fields(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_rustup_settings

    path = _write(
        tmp_path / "settings.toml",
        "\n".join(
            [
                'version = "12"',
                'default_toolchain = "stable-x86_64-pc-windows-msvc"',
                'profile = "complete"',
                "",
                "[overrides]",
                '"D:\\\\demo" = "nightly-x86_64-pc-windows-msvc"',
                '"D:\\\\demo2" = "beta-x86_64-pc-windows-msvc"',
            ]
        ),
    )

    parsed = parse_rustup_settings(path)

    assert parsed["version"] == "12"
    assert parsed["default_toolchain"] == "stable-x86_64-pc-windows-msvc"
    assert parsed["profile"] == "complete"
    assert parsed["override_count"] == 2
    assert parsed["override_paths"] == ["D:\\demo", "D:\\demo2"]


def test_parse_maven_user_settings_extracts_explicit_fields(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_maven_user_settings

    path = _write(
        tmp_path / "settings.xml",
        """<settings xmlns="http://maven.apache.org/SETTINGS/1.0.0">
  <localRepository>D:\\m2\\repo</localRepository>
  <offline>true</offline>
  <pluginGroups>
    <pluginGroup>com.example.plugins</pluginGroup>
  </pluginGroups>
  <servers>
    <server>
      <id>releases</id>
      <username>builder</username>
      <password>secret</password>
    </server>
  </servers>
  <mirrors>
    <mirror>
      <id>corp</id>
      <url>https://maven.example.com/repository</url>
    </mirror>
  </mirrors>
  <proxies>
    <proxy>
      <id>proxy-1</id>
      <active>true</active>
      <host>proxy.example.com</host>
    </proxy>
  </proxies>
  <profiles>
    <profile>
      <id>corp</id>
    </profile>
  </profiles>
  <activeProfiles>
    <activeProfile>corp</activeProfile>
  </activeProfiles>
</settings>
""",
    )

    parsed = parse_maven_user_settings(path)

    assert parsed["local_repository"] == "D:\\m2\\repo"
    assert parsed["offline"] is True
    assert parsed["plugin_groups"] == ["com.example.plugins"]
    assert parsed["mirror_ids"] == ["corp"]
    assert parsed["mirror_urls"] == ["https://maven.example.com/repository"]
    assert parsed["server_ids"] == ["releases"]
    assert parsed["credential_server_ids"] == ["releases"]
    assert parsed["active_profiles"] == ["corp"]
    assert parsed["proxy_hosts"] == ["proxy.example.com"]


def test_parse_gradle_user_properties_extracts_explicit_fields(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_gradle_user_properties

    path = _write(
        tmp_path / "gradle.properties",
        "\n".join(
            [
                "org.gradle.caching=true",
                "org.gradle.parallel=false",
                "org.gradle.configuration-cache=true",
                "org.gradle.daemon=true",
                "org.gradle.jvmargs=-Xmx2g -Dfile.encoding=UTF-8",
                "org.gradle.java.home=C:\\Java\\jdk-21",
            ]
        ),
    )

    parsed = parse_gradle_user_properties(path)

    assert parsed["caching"] is True
    assert parsed["parallel"] is False
    assert parsed["configuration_cache"] is True
    assert parsed["daemon"] is True
    assert parsed["jvmargs"] == "-Xmx2g -Dfile.encoding=UTF-8"
    assert parsed["java_home"] == "C:\\Java\\jdk-21"


def test_parse_nuget_user_config_extracts_explicit_fields(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_nuget_user_config

    path = _write(
        tmp_path / "NuGet.Config",
        """<configuration>
  <config>
    <add key="globalPackagesFolder" value="D:\\nuget\\packages" />
    <add key="defaultPushSource" value="https://nuget.example.com/v3/index.json" />
    <add key="signatureValidationMode" value="require" />
  </config>
  <packageSources>
    <add key="nuget.org" value="https://api.nuget.org/v3/index.json" />
    <add key="internal" value="https://nuget.example.com/v3/index.json" />
  </packageSources>
  <disabledPackageSources>
    <add key="legacy" value="true" />
  </disabledPackageSources>
  <packageSourceCredentials>
    <internal>
      <add key="Username" value="builder" />
      <add key="ClearTextPassword" value="secret" />
    </internal>
  </packageSourceCredentials>
</configuration>
""",
    )

    parsed = parse_nuget_user_config(path)

    assert parsed["global_packages_folder"] == "D:\\nuget\\packages"
    assert parsed["default_push_source"] == "https://nuget.example.com/v3/index.json"
    assert parsed["signature_validation_mode"] == "require"
    assert parsed["package_source_names"] == ["internal", "nuget.org"]
    assert parsed["package_source_urls"] == [
        "https://api.nuget.org/v3/index.json",
        "https://nuget.example.com/v3/index.json",
    ]
    assert parsed["disabled_sources"] == ["legacy"]
    assert parsed["credential_sources"] == ["internal"]


def test_parse_dotnet_global_tools_extracts_commands(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_dotnet_global_tools

    tools_dir = tmp_path / ".dotnet" / "tools"
    _write(tools_dir / "dotnetsay.exe", "")
    _write(tools_dir / "csharpier.cmd", "")
    _write(tools_dir / ".store" / "ignored.txt", "")

    parsed = parse_dotnet_global_tools(tools_dir)

    assert parsed["commands"] == ["csharpier", "dotnetsay"]
    assert parsed["command_count"] == 2


def test_parse_editor_recent_workspaces_decodes_file_uris_and_skips_backup_paths(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_editor_recent_workspaces

    path = _write(
        tmp_path / "storage.json",
        json.dumps(
            {
                "lastKnownMenubarData": {
                    "workspaces3": [
                        {"folderUri": "file:///d%3A/weft"},
                        {"folderUri": "file:///C%3A/Users/Admin/AppData/Roaming/Code/Backups/123"},
                        {"configPath": "file:///D%3A/weft/demo.code-workspace"},
                    ]
                }
            }
        ),
    )

    parsed = parse_editor_recent_workspaces(path)

    assert parsed["recent_workspaces"] == [
        "D:\\weft",
        "D:\\weft\\demo.code-workspace",
    ]


def test_run_scout_collects_installed_apps_from_registry_export_probe(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    export_root = tmp_path / "registry-export"
    export_root.mkdir(parents=True, exist_ok=True)
    _write(
        export_root / "apps.json",
        json.dumps(
            {
                "items": [
                    {
                        "DisplayName": "Steam",
                        "DisplayVersion": "2.10.91.91",
                        "Publisher": "Valve",
                    },
                    {
                        "DisplayName": "Cursor",
                        "DisplayVersion": "0.50.0",
                        "Publisher": "Cursor",
                    },
                ]
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[export_root],
            home=None,
            max_depth=3,
        )
    )

    installed_apps = report["derived_profile"]["installed_apps"]
    app_names = {item["name"] for item in installed_apps}
    assert {"Cursor", "Steam"}.issubset(app_names)


def test_run_scout_discovers_desktop_office_documents_with_skill_routes(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    desktop = home / "Desktop"
    docx_path = _write_minimal_docx(desktop / "Babylon memo.docx", ["Babylon monocle roadmap", "Optics and AI companion notes"])
    pptx_path = _write_minimal_pptx(desktop / "Investor deck.pptx", [["Babylon Vision"], ["Hardware roadmap", "Developer ecosystem"]])
    xlsx_path = _write_minimal_xlsx(desktop / "Budget model.xlsx", {"Budget": [["Category", "Amount"], ["Optics", "1200"]]})
    _write_minimal_pptx(desktop / "~$Investor deck.pptx", [["Office lock file"]])

    report = run_scout(
        ScoutConfig(
            roots=[desktop],
            home=home,
            max_depth=2,
        )
    )

    office_entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "office_document"]
    assert {entry["path"] for entry in office_entries} == {str(docx_path.resolve()), str(pptx_path.resolve()), str(xlsx_path.resolve())}

    docx_entry = next(entry for entry in office_entries if entry["path"] == str(docx_path.resolve()))
    assert docx_entry["sensitivity"] == "high"
    assert docx_entry["fields"]["document_type"] == "docx"
    assert docx_entry["fields"]["preview_text"] == "Babylon monocle roadmap\nOptics and AI companion notes"
    assert docx_entry["fields"]["recommended_skills"] == ["docx"]
    assert docx_entry["fields"]["skill_routes"] == [
        {"skill": "docx", "arguments": {"path": str(docx_path.resolve())}},
    ]

    profile = report["derived_profile"]["office_documents_profile"]
    assert profile["present"] is True
    assert profile["document_count"] == 3
    assert profile["type_counts"] == {"docx": 1, "pptx": 1, "xlsx": 1}
    assert profile["skill_coupling"]["primary_skill"] == "specialized_document_skill"
    assert profile["skill_coupling"]["specialized_skills"] == ["docx", "pptx", "xlsx"]
    candidate_routes = {
        candidate["path"]: candidate["skill_routes"][0]
        for candidate in profile["deep_read_candidates"]
    }
    assert candidate_routes[str(xlsx_path.resolve())] == {"skill": "office-document-specialist-suite", "arguments": {"path": str(xlsx_path.resolve())}}


def test_run_scout_builds_recent_documents_profile_from_windows_recent_shortcuts(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    desktop = home / "Desktop"
    (home / "AppData" / "Roaming" / "Microsoft" / "Windows" / "Recent").mkdir(parents=True, exist_ok=True)
    docx_path = _write_minimal_docx(desktop / "Recent memo.docx", ["Recent memo content"])
    pptx_path = _write_minimal_pptx(desktop / "Recent deck.pptx", [["Recent deck title"]])

    class _Completed:
        stdout = json.dumps(
            [
                {"shortcut_path": str(home / "AppData" / "Roaming" / "Microsoft" / "Windows" / "Recent" / "Recent memo.lnk"), "target_path": str(docx_path)},
                {"shortcut_path": str(home / "AppData" / "Roaming" / "Microsoft" / "Windows" / "Recent" / "Recent deck.lnk"), "target_path": str(pptx_path)},
                {"shortcut_path": str(home / "AppData" / "Roaming" / "Microsoft" / "Windows" / "Recent" / "notes.lnk"), "target_path": str(desktop / "notes.txt")},
            ]
        )
        stderr = ""
        returncode = 0

    def _fake_run(command, *args, **kwargs):  # noqa: ANN001
        if isinstance(command, list) and command[:3] == ["powershell", "-NoProfile", "-Command"]:
            return _Completed()
        if isinstance(command, list) and len(command) >= 2 and command[0] in {"wsl.exe", "wsl"} and command[1] == "--list":
            class _EmptyCompleted:
                stdout = ""
                stderr = ""
                returncode = 0

            return _EmptyCompleted()
        raise AssertionError(f"unexpected subprocess call: {command!r}")

    monkeypatch.setattr("ai_local_scout.runtime.subprocess.run", _fake_run)

    report = run_scout(ScoutConfig(roots=[desktop], home=home, max_depth=2))

    entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "recent_document"]
    assert {entry["path"] for entry in entries} == {str(docx_path.resolve()), str(pptx_path.resolve())}
    assert all(entry["sensitivity"] == "high" for entry in entries)

    profile = report["derived_profile"]["recent_documents_profile"]
    assert profile["present"] is True
    assert profile["document_count"] == 2
    assert profile["type_counts"] == {"docx": 1, "pptx": 1}
    document_routes = [candidate["skill_routes"][0] for candidate in profile["deep_read_candidates"]]
    assert {json.dumps(route, sort_keys=True) for route in document_routes} == {
        json.dumps({"skill": "docx", "arguments": {"path": str(docx_path.resolve())}}, sort_keys=True),
        json.dumps({"skill": "pptx", "arguments": {"path": str(pptx_path.resolve())}}, sort_keys=True),
    }
    assert profile["skill_coupling"]["primary_skill"] == "specialized_document_skill"


def test_run_scout_builds_downloads_profile_from_browser_records_and_downloads_folder(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    downloads = home / "Downloads"
    _write_bytes(downloads / "installer.exe", b"demo")
    _write_bytes(downloads / "research.pdf", b"%PDF-demo")
    _write_bytes(downloads / "archive.zip", b"zip")

    browser_profile = home / "AppData" / "Local" / "Google" / "Chrome" / "User Data" / "Default"
    history = browser_profile / "History"
    history.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(history) as connection:
        connection.execute("CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER DEFAULT 0)")
        connection.execute(
            """
            CREATE TABLE downloads (
              id INTEGER PRIMARY KEY,
              target_path TEXT,
              tab_url TEXT,
              site_url TEXT,
              mime_type TEXT,
              received_bytes INTEGER,
              total_bytes INTEGER,
              state INTEGER
            )
            """
        )
        connection.execute(
            """
            INSERT INTO downloads(id, target_path, tab_url, site_url, mime_type, received_bytes, total_bytes, state)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (1, str(downloads / "installer.exe"), "https://download.example.com/app", "https://example.com", "application/x-msdownload", 4, 4, 1),
        )
        connection.execute(
            """
            INSERT INTO downloads(id, target_path, tab_url, site_url, mime_type, received_bytes, total_bytes, state)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (2, str(downloads / "missing.zip"), "https://cdn.example.com/missing.zip", "https://example.com", "application/zip", 0, 10, 0),
        )
        connection.commit()

    report = run_scout(ScoutConfig(roots=[home], home=home, max_depth=5))

    downloaded_entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "downloaded_file"]
    assert {entry["fields"]["filename"] for entry in downloaded_entries} == {"archive.zip", "installer.exe", "research.pdf"}

    profile = report["derived_profile"]["downloads_profile"]
    assert profile["present"] is True
    assert profile["filesystem_file_count"] == 3
    assert profile["browser_record_count"] == 2
    assert profile["extension_counts"] == {".exe": 1, ".pdf": 1, ".zip": 1}
    assert profile["category_counts"] == {"archive": 1, "document": 1, "installer": 1}
    assert profile["browser_source_domains"] == ["cdn.example.com", "download.example.com", "example.com"]
    assert profile["matched_browser_downloads"] == [{"filename": "installer.exe", "file_extension": ".exe", "source_domains": ["download.example.com", "example.com"]}]
    assert profile["skill_coupling"]["routes_by_extension"][".pdf"] == ["pdf-reader"]


def test_run_scout_discovers_editor_shell_and_installed_apps(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    fixture = _build_editor_shell_and_apps_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
                fixture["uninstall_root"],
            ],
            home=fixture["home"],
            max_depth=7,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "editor_recent_workspaces" in kinds
    assert "shell_history" in kinds
    assert "installed_apps" in kinds

    workspaces = report["derived_profile"]["active_workspaces"]
    recent_paths = {item["path"] for item in workspaces["recent_editor_workspaces"]}
    assert str(fixture["project"]) in recent_paths
    assert str(fixture["workspace_file"]) in recent_paths
    assert str(fixture["another_workspace"]) in recent_paths

    shell_profile = report["derived_profile"]["shell_activity"]
    assert shell_profile["shells"] == ["powershell"]
    assert "git status" in shell_profile["recent_commands"]
    assert shell_profile["top_commands"][0]["command"] == "git"

    installed_apps = report["derived_profile"]["installed_apps"]
    app_names = {item["name"] for item in installed_apps}
    assert {"Visual Studio Code", "Cursor", "Claude Code", "Steam"}.issubset(app_names)


def test_run_scout_builds_second_level_profiles(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    _install_runtime_probe_stub(monkeypatch)
    fixture = _build_editor_shell_and_apps_fixture(tmp_path)
    report = run_scout(
        ScoutConfig(
            roots=[
                fixture["home"],
                fixture["project"].parent,
                fixture["steam_root"],
                fixture["epic_manifest_dir"],
                fixture["sqlite_path"].parent,
                fixture["uninstall_root"],
            ],
            home=fixture["home"],
            max_depth=7,
        )
    )

    profile = report["derived_profile"]

    developer_profile = profile["developer_profile"]
    assert developer_profile["is_developer"] is True
    assert "git" in developer_profile["signals"]
    assert "vscode" in developer_profile["signals"]
    assert "cursor" in developer_profile["signals"]
    assert "python" in developer_profile["shell_tooling"]
    assert "git" in developer_profile["shell_tooling"]

    gaming_profile = profile["gaming_profile"]
    assert gaming_profile["is_gamer"] is True
    assert gaming_profile["platforms"] == ["epic", "steam"]
    assert "Dota 2" in gaming_profile["installed_game_names"]
    assert "Fortnite" in gaming_profile["installed_game_names"]

    ai_tools_profile = profile["ai_tools_profile"]
    assert ai_tools_profile["uses_ai_tools"] is True
    assert "claude" in ai_tools_profile["tool_families"]
    assert "codex" in ai_tools_profile["tool_families"]
    assert "cursor" in ai_tools_profile["tool_families"]
    assert "claude code" in ai_tools_profile["desktop_tools"]

    agent_config_profile = profile["agent_config_profile"]
    assert agent_config_profile["present"] is True
    assert agent_config_profile["agent_families"] == ["claude", "codex", "project_agents"]
    assert "claude_entrypoint" in agent_config_profile["config_surfaces"]
    assert "claude_context_file" in agent_config_profile["config_surfaces"]
    assert "codex_config" in agent_config_profile["config_surfaces"]
    assert "project_rules" in agent_config_profile["config_surfaces"]
    assert agent_config_profile["claude"]["context_file_count"] == 1
    assert "memory" in agent_config_profile["claude"]["mcp_servers"]
    assert agent_config_profile["codex"]["trusted_project_count"] == 1
    assert agent_config_profile["project_rules"]["agents_file_count"] == 1
    assert "uses_project_agents_rules" in agent_config_profile["setup_hints"]

    hardware_profile = profile["hardware_profile"]
    assert hardware_profile["present"] is True
    assert hardware_profile["gpu_tooling"] == ["nvidia"]
    assert hardware_profile["peripheral_tooling"] == ["logitech"]
    assert hardware_profile["setup_hints"] == ["discrete_gpu_tooling", "gaming_peripherals"]

    creative_tools_profile = profile["creative_tools_profile"]
    assert creative_tools_profile["present"] is True
    assert creative_tools_profile["tool_families"] == ["blender"]
    assert creative_tools_profile["domains"] == ["3d_creation"]
    assert creative_tools_profile["app_names"] == ["Blender"]

    privacy_security_profile = profile["privacy_security_profile"]
    assert privacy_security_profile["present"] is True
    assert privacy_security_profile["tool_families"] == ["bitwarden", "tailscale"]
    assert privacy_security_profile["domains"] == ["password_management", "vpn_or_mesh_networking"]
    assert privacy_security_profile["setup_hints"] == ["uses_password_manager", "uses_private_networking"]

    interests = profile["interest_tags"]
    assert "developer_tools" in interests
    assert "ai_tools" in interests
    assert "gaming" in interests


def test_run_scout_builds_meaningful_activity_profile_with_breadth(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    _isolate_machine_probes(monkeypatch)
    fixture = _build_editor_shell_and_apps_fixture(tmp_path)
    home = fixture["home"]
    _add_breadth_fixture_artifacts(home)

    report = _run_editor_fixture_scout(fixture, max_depth=9)

    meaningful = report["derived_profile"]["meaningful_activity_profile"]
    assert meaningful["primary_modes"] == [
        "gaming",
        "building",
        "ai_tool_use",
        "knowledge_work",
        "content_creation",
        "sync_storage",
        "terminal_work",
        "cloud_work",
        "local_linux_runtime",
        "container_work",
    ]
    assert meaningful["pc_setup_hints"] == [
        "multi_launcher_game_library",
        "developer_workstation",
        "ai_augmented_workstation",
        "notes_and_vaults",
        "creator_streaming_setup",
        "cross_device_sync",
        "terminal_centered_setup",
        "cloud_cli_ready",
        "local_linux_runtime",
        "containerized_dev",
    ]
    assert meaningful["evidence_quality"] == {
        "gaming": "strong",
        "development": "strong",
        "ai_tool_use": "strong",
        "knowledge_work": "strong",
        "content_creation": "strong",
        "sync_storage": "strong",
        "recent_activity": "weak",
    }
    assert meaningful["breadth"] == {
        "primary_mode_count": 10,
        "setup_hint_count": 10,
        "game_platform_count": 2,
        "installed_game_count": 2,
    }
    assert any("install/library evidence" in line for line in meaningful["wholesome_summary"])


def test_run_scout_derives_lightweight_gaming_style_hints(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    _install_runtime_probe_stub(monkeypatch)
    steam_root = tmp_path / "Steam"
    steam_root_windows = steam_root.as_posix().replace("/", "\\")
    _write(
        steam_root / "steamapps" / "libraryfolders.vdf",
        '"libraryfolders"\n{\n  "0"\n  {\n    "path" "' + steam_root_windows + '"\n  }\n}',
    )
    for app_id, name, install_dir in [
        ("413150", "Stardew Valley", "Stardew Valley"),
        ("730", "Counter-Strike 2", "Counter-Strike Global Offensive"),
        ("289070", "Sid Meier's Civilization VI", "Sid Meier's Civilization VI"),
        ("292030", "The Witcher 3: Wild Hunt", "The Witcher 3"),
        ("252490", "Rust", "Rust"),
    ]:
        _write(
            steam_root / "steamapps" / f"appmanifest_{app_id}.acf",
            '"AppState"\n{\n  "appid" "'
            + app_id
            + '"\n  "name" "'
            + name
            + '"\n  "installdir" "'
            + install_dir
            + '"\n}',
        )

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
            system_steam_roots=[steam_root],
        )
    )

    meaningful = report["derived_profile"]["meaningful_activity_profile"]
    assert meaningful["gaming_style_hints"] == [
        "cozy_indie",
        "competitive_multiplayer",
        "strategy",
        "rpg_story",
        "sandbox_survival",
    ]
    assert meaningful["evidence_quality"]["gaming"] == "strong"
    assert any("installed games suggest" in line for line in meaningful["wholesome_summary"])




def test_run_scout_builds_local_signal_coverage_without_audio_domains(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    _isolate_machine_probes(monkeypatch)
    fixture = _build_editor_shell_and_apps_fixture(tmp_path)
    home = fixture["home"]
    _add_breadth_fixture_artifacts(home, obs_profile_name="Capture")

    report = _run_editor_fixture_scout(fixture, max_depth=9)

    coverage = report["derived_profile"]["local_signal_coverage"]
    assert coverage["present_categories"] == [
        "games",
        "browser",
        "development",
        "ai_tools",
        "knowledge",
        "creator",
        "sync_storage",
        "terminal_remote",
        "cloud_runtime",
        "local_apps",
        "generic_data",
    ]
    assert coverage["breadth_score"] == 11
    assert coverage["category_count"] == 18
    assert coverage["absent_categories"] == ["downloads", "communication", "language_tooling", "office_documents", "recent_documents", "audio_voice", "translation"]
    assert coverage["categories"]["games"]["entity_kinds"] == ["epic_game_manifest", "steam_game_manifest"]
    assert coverage["categories"]["games"]["quality"] == "strong"
    assert coverage["categories"]["audio_voice"]["present"] is False
    assert coverage["categories"]["translation"]["present"] is False


def test_run_scout_builds_useful_next_questions_from_coverage(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    _isolate_machine_probes(monkeypatch)
    fixture = _build_editor_shell_and_apps_fixture(tmp_path)

    report = _run_editor_fixture_scout(fixture)

    next_questions = report["derived_profile"]["next_questions"]
    assert next_questions["ask_user"] == [
        {
            "topic": "recent_activity",
            "question": "Do you want a one-shot ActivityWatch bucket check for recent app/window context, if ActivityWatch is already running?",
            "why": "Installed libraries are strong, but recent activity is weak without a local activity source.",
        },
        {
            "topic": "knowledge_tools",
            "question": "Do you want to include Obsidian or Joplin config paths in the next scan?",
            "why": "No knowledge-tool signal was found in this scan.",
        },
        {
            "topic": "creator_tools",
            "question": "Do you want to include OBS Studio profile paths in the next scan?",
            "why": "No creator-tool signal was found in this scan.",
        },
    ]
    assert next_questions["do_not_ask"] == [
        {
            "topic": "audio_voice",
            "reason": "Out of scope for AI Local Scout; this project should not drift into voice separation.",
        },
        {
            "topic": "translation",
            "reason": "Out of scope for AI Local Scout; this project should not drift into simultaneous interpretation.",
        },
    ]


def test_run_scout_builds_compact_llm_context(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    _isolate_machine_probes(monkeypatch)
    fixture = _build_editor_shell_and_apps_fixture(tmp_path)

    report = _run_editor_fixture_scout(fixture)

    context = report["derived_profile"]["llm_context"]
    assert context == {
        "summary": "One-shot local scout found gaming, building, and AI tool use signals. Strongest categories: games, browser, development, ai_tools, local_apps.",
        "strong_facts": [
            "Installed game libraries contain 2 unique games across 2 platforms.",
            "Developer workflow signals are present from repositories, editors, workspaces, or shell history.",
            "AI tooling signals are present: claude, codex, cursor.",
            "Broad local coverage is 6/18 categories.",
        ],
        "weak_hints": [
            "Recent activity is weak because this scout does not run as a background monitor.",
        ],
        "uncertainties": [
            "No knowledge-tool signal was found in this scan.",
            "No creator-tool signal was found in this scan.",
        ],
        "boundaries": [
            "Do not infer live playtime from installed game libraries.",
            "Do not infer private message contents from app presence.",
            "Do not treat this scout as audio/voice separation tooling.",
            "Do not treat this scout as simultaneous interpretation tooling.",
        ],
        "good_followups": [
            "Do you want a one-shot ActivityWatch bucket check for recent app/window context, if ActivityWatch is already running?",
            "Do you want to include Obsidian or Joplin config paths in the next scan?",
            "Do you want to include OBS Studio profile paths in the next scan?",
        ],
    }


def test_run_scout_llm_context_uses_singular_platform_word(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    _install_runtime_probe_stub(monkeypatch)
    steam_root = tmp_path / "Steam"
    steam_root_windows = steam_root.as_posix().replace("/", "\\")
    _write(
        steam_root / "steamapps" / "libraryfolders.vdf",
        '"libraryfolders"\n{\n  "0"\n  {\n    "path" "' + steam_root_windows + '"\n  }\n}',
    )
    _write(
        steam_root / "steamapps" / "appmanifest_413150.acf",
        '"AppState"\n{\n  "appid" "413150"\n  "name" "Stardew Valley"\n  "installdir" "Stardew Valley"\n}',
    )

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
            system_steam_roots=[steam_root],
        )
    )

    facts = report["derived_profile"]["llm_context"]["strong_facts"]
    assert "Installed game libraries contain 1 unique game across 1 platform." in facts


def test_parse_shell_history_normalizes_noisy_commands(tmp_path: Path) -> None:
    from ai_local_scout.parsers import parse_shell_history

    history = _write(
        tmp_path / "ConsoleHost_history.txt",
        "\n".join(
            [
                "& 'C:\\Program Files\\Git\\bin\\git.exe' status",
                "$i=0; Get-ChildItem -Recurse",
                "CODEX --version",
                "python -m pytest",
                "s",
                "ss",
                "sss",
                "node scripts/build.js",
                "[o[iclaude --version",
                "while ($true) { Start-Sleep 1 }",
                "where.exe git",
                "winget list Git.Git",
            ]
        ),
    )

    parsed = parse_shell_history(history)

    assert parsed["top_commands"] == [
        {"command": "codex", "count": 1},
        {"command": "get-childitem", "count": 1},
        {"command": "git", "count": 1},
        {"command": "node", "count": 1},
        {"command": "python", "count": 1},
        {"command": "winget", "count": 1},
    ]


def test_system_game_probe_entries_expand_steam_and_epic_games(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    steam_root = tmp_path / "Steam"
    steam_library = tmp_path / "SteamLibrary"
    epic_manifest_dir = tmp_path / "Epic" / "Manifests"
    steam_root_windows = steam_root.as_posix().replace("/", "\\")
    steam_library_windows = steam_library.as_posix().replace("/", "\\")

    _write(
        steam_root / "steamapps" / "libraryfolders.vdf",
        "\n".join(
            [
                '"libraryfolders"',
                "{",
                '  "0"',
                "  {",
                f'    "path" "{steam_root_windows}"',
                "  }",
                '  "1"',
                "  {",
                f'    "path" "{steam_library_windows}"',
                "  }",
                "}",
            ]
        ),
    )
    _write(
        steam_library / "steamapps" / "appmanifest_570.acf",
        "\n".join(
            [
                '"AppState"',
                "{",
                '  "appid" "570"',
                '  "name" "Dota 2"',
                '  "installdir" "dota 2 beta"',
                '  "SizeOnDisk" "987654321"',
                '  "StateFlags" "4"',
                "}",
            ]
        ),
    )
    _write(
        epic_manifest_dir / "fortnite.item",
        json.dumps(
            {
                "DisplayName": "Fortnite",
                "AppName": "Fortnite",
                "CatalogItemId": "fortnite-id",
                "InstallLocation": "D:\\Epic Games\\Fortnite",
                "LaunchExecutable": "FortniteGame\\Binaries\\Win64\\FortniteLauncher.exe",
                "InstallSize": 123456789,
                "TechnicalType": "gamesoftware",
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
            system_steam_roots=[steam_root],
            system_epic_manifest_dirs=[epic_manifest_dir],
        )
    )

    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert gaming_profile["is_gamer"] is True
    assert gaming_profile["platforms"] == ["epic", "steam"]
    assert gaming_profile["installed_game_names"] == ["Dota 2", "Fortnite"]
    assert gaming_profile["steam_game_count"] == 1
    assert gaming_profile["steam_app_ids"] == ["570"]
    assert gaming_profile["steam_install_dir_names"] == ["dota 2 beta"]
    assert gaming_profile["steam_sizes_on_disk"] == ["987654321"]
    assert gaming_profile["steam_state_flags"] == ["4"]
    assert gaming_profile["steam_library_roots"] == [str(steam_root), str(steam_library)]
    assert gaming_profile["steam_manifest_count"] == 1
    assert gaming_profile["epic_game_count"] == 1
    assert gaming_profile["epic_catalog_item_ids"] == ["fortnite-id"]
    assert gaming_profile["epic_app_names"] == ["Fortnite"]
    assert gaming_profile["epic_install_locations"] == ["D:\\Epic Games\\Fortnite"]
    assert gaming_profile["epic_launch_executables"] == ["FortniteGame\\Binaries\\Win64\\FortniteLauncher.exe"]
    assert gaming_profile["epic_install_sizes"] == [123456789]
    assert gaming_profile["epic_technical_types"] == ["gamesoftware"]
    assert gaming_profile["epic_executable_path_count"] == 1


def test_run_scout_builds_steam_public_playtime_profile_from_public_games_page(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    steam_root = tmp_path / "Steam"
    steam_root_windows = steam_root.as_posix().replace("/", "\\")
    _write(
        steam_root / "steamapps" / "libraryfolders.vdf",
        '"libraryfolders"\n{\n  "0"\n  {\n    "path" "' + steam_root_windows + '"\n  }\n}',
    )
    _write(
        steam_root / "config" / "loginusers.vdf",
        '"users"\n{\n  "76561198000000000"\n  {\n    "AccountName" "leo"\n    "MostRecent" "1"\n  }\n}',
    )
    _write(
        steam_root / "steamapps" / "appmanifest_570.acf",
        '"AppState"\n{\n  "appid" "570"\n  "name" "Dota 2"\n  "installdir" "dota 2 beta"\n}',
    )

    def _fake_fetch(url: str) -> str:
        assert "76561198000000000" in url
        return """
        <html><body>
          <a href="https://store.steampowered.com/app/570/Dota_2/">Dota 2</a>
          <div class="hours">1,234.5 hrs on record</div>
          <a href="https://store.steampowered.com/app/294100/RimWorld/">RimWorld</a>
          <div class="hours">98 hrs on record</div>
        </body></html>
        """

    monkeypatch.setattr("ai_local_scout.runtime._fetch_steam_public_games_page", _fake_fetch)
    _install_runtime_probe_stub(monkeypatch)

    report = run_scout(
        ScoutConfig(
            roots=[steam_root],
            home=tmp_path / "home",
            max_depth=4,
            system_steam_roots=[steam_root],
            enable_steam_public_profile=True,
        )
    )

    entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "steam_public_playtime"]
    assert len(entries) == 1
    assert entries[0]["fields"] == {
        "steam_id64": "76561198000000000",
        "source": "steam_public_profile",
        "privacy_limited": False,
        "games": [
            {"app_id": "570", "name": "Dota 2", "playtime_forever_hours": 1234.5},
            {"app_id": "294100", "name": "RimWorld", "playtime_forever_hours": 98.0},
        ],
    }

    profile = report["derived_profile"]["steam_playtime_profile"]
    assert profile["present"] is True
    assert profile["source"] == "steam_public_profile"
    assert profile["privacy_limited"] is False
    assert profile["game_count"] == 2
    assert profile["top_games_by_playtime"][0] == {"app_id": "570", "name": "Dota 2", "playtime_forever_hours": 1234.5}


def test_run_scout_parses_cursor_state_vscdb_signals(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    state_db = home / "AppData" / "Roaming" / "Cursor" / "User" / "globalStorage" / "state.vscdb"
    state_db.parent.mkdir(parents=True, exist_ok=True)

    with sqlite3.connect(state_db) as connection:
        connection.execute("CREATE TABLE ItemTable (key TEXT, value BLOB)")
        connection.execute("CREATE TABLE cursorDiskKV (key TEXT, value BLOB)")
        connection.execute(
            "INSERT INTO ItemTable(key, value) VALUES (?, ?)",
            ("history.recentlyOpenedPathsList", json.dumps({"entries": [{"folderUri": "file:///D%3A/weft"}]})),
        )
        connection.execute(
            "INSERT INTO cursorDiskKV(key, value) VALUES (?, ?)",
            (
                "composerData:one",
                json.dumps(
                    {
                        "unifiedMode": "agent",
                        "isAgentic": True,
                    }
                ),
            ),
        )
        connection.execute(
            "INSERT INTO cursorDiskKV(key, value) VALUES (?, ?)",
            (
                "composerData:two",
                json.dumps(
                    {
                        "unifiedMode": "chat",
                        "isAgentic": False,
                    }
                ),
            ),
        )
        connection.commit()

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=home,
            max_depth=6,
        )
    )

    cursor_entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "cursor_state_db"]
    assert cursor_entries
    fields = cursor_entries[0]["fields"]
    assert fields["composer_count"] == 2
    assert fields["agentic_count"] == 1
    assert fields["modes"] == ["agent", "chat"]

    ai_tools_profile = report["derived_profile"]["ai_tools_profile"]
    assert "cursor" in ai_tools_profile["tool_families"]
    assert ai_tools_profile["cursor_signals"]["composer_count"] == 2
    assert ai_tools_profile["cursor_signals"]["agentic_count"] == 1


def test_system_legendary_installed_probe_adds_epic_games(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    installed_path = tmp_path / "legendary" / "installed.json"
    _write(
        installed_path,
        json.dumps(
            {
                "Fortnite": {
                    "title": "Fortnite",
                    "install_path": "D:\\Epic\\Fortnite",
                    "app_name": "Fortnite",
                    "version": "1.0",
                },
                "AlanWake2": {
                    "title": "Alan Wake 2",
                    "install_path": "D:\\Epic\\Alan Wake 2",
                    "app_name": "AlanWake2",
                },
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
            system_legendary_installed_paths=[installed_path],
        )
    )

    legendary_entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "legendary_installed"]
    assert legendary_entries
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "epic" in gaming_profile["platforms"]
    assert {"Fortnite", "Alan Wake 2"}.issubset(set(gaming_profile["installed_game_names"]))
    assert gaming_profile["epic_game_count"] == 2
    assert gaming_profile["epic_app_names"] == ["AlanWake2", "Fortnite"]
    assert gaming_profile["epic_install_locations"] == ["D:\\Epic\\Alan Wake 2", "D:\\Epic\\Fortnite"]
    assert gaming_profile["epic_catalog_item_ids"] == []
    assert gaming_profile["epic_launch_executables"] == []
    assert gaming_profile["epic_install_sizes"] == []
    assert gaming_profile["epic_technical_types"] == []
    assert gaming_profile["epic_executable_path_count"] == 0


def test_system_windows_launcher_registry_probes_add_ubisoft_and_battlenet_games(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    class _Completed:
        def __init__(self, stdout: str, returncode: int = 0) -> None:
            self.stdout = stdout
            self.returncode = returncode

    def _fake_run(command, capture_output, text, encoding, errors, timeout, check):
        script = command[-1]
        if isinstance(command, list) and (command[:2] == ["wsl.exe", "--list"] or command[:2] == ["wsl", "--list"]):
            return _Completed("")
        if "SOFTWARE\\ubisoft\\Launcher\\Installs" in script and "$battleGames" in script:
            return _Completed(
                json.dumps(
                    {
                        "ubisoft": [
                            {
                                "gameId": "635",
                                "installLocation": r"D:\Games\Ubisoft\Assassins Creed Odyssey",
                            }
                        ],
                        "battleNet": [
                            {
                                "uid": "s2",
                                "name": "StarCraft II",
                                "installLocation": r"D:\Games\Battle.net\StarCraft II",
                            }
                        ],
                    }
                )
            )
        if "Get-AppxPackage" in script and "InstallLocation" in script:
            return _Completed("[]")
        if "CurrentVersion\\Uninstall" in script:
            return _Completed("[]")
        raise AssertionError(f"unexpected subprocess call: {command!r}")

    monkeypatch.setattr("ai_local_scout.runtime.subprocess.run", _fake_run)

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "ubisoft_launcher_installs" in kinds
    assert "battle_net_launcher_installs" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "ubisoft" in gaming_profile["platforms"]
    assert "battle_net" in gaming_profile["platforms"]
    assert {"Assassins Creed Odyssey", "StarCraft II"}.issubset(set(gaming_profile["installed_game_names"]))


def test_system_battle_net_product_db_probe_adds_games(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    product_db = tmp_path / "Battle.net" / "Agent" / "product.db"
    _write_bytes(
        product_db,
        _battle_net_product_db(
            _battle_net_product_install("s2", "S2", r"D:\Games\Battle.net\StarCraft II"),
            _battle_net_product_install("fen", "Fen", r"D:\Games\Battle.net\Diablo IV"),
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
            system_battle_net_product_db_paths=[product_db],
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "battle_net_product_db" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "battle_net" in gaming_profile["platforms"]
    assert {"StarCraft II", "Diablo IV"}.issubset(set(gaming_profile["installed_game_names"]))
    assert gaming_profile["battle_net_game_count"] == 2
    assert gaming_profile["battle_net_game_ids"] == ["fen", "s2"]
    assert gaming_profile["battle_net_product_codes"] == ["Fen", "S2"]
    assert gaming_profile["battle_net_install_locations"] == [
        "D:\\Games\\Battle.net\\Diablo IV",
        "D:\\Games\\Battle.net\\StarCraft II",
    ]


def test_system_amazon_games_install_info_probe_adds_games(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    install_info = tmp_path / "Amazon Games" / "Data" / "Games" / "Sql" / "GameInstallInfo.sqlite"
    install_info.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(install_info) as connection:
        connection.execute(
            """
            CREATE TABLE DbSet (
              Id TEXT,
              ProductTitle TEXT,
              InstallDirectory TEXT,
              Installed INTEGER
            )
            """
        )
        connection.execute(
            "INSERT INTO DbSet(Id, ProductTitle, InstallDirectory, Installed) VALUES (?, ?, ?, ?)",
            (
                "amzn1.adg.product.0d364464-032c-40c9-a6da-c633a53e3374",
                "Blue Fire",
                r"C:\Amazon Games\Library\Blue Fire",
                1,
            ),
        )
        connection.execute(
            "INSERT INTO DbSet(Id, ProductTitle, InstallDirectory, Installed) VALUES (?, ?, ?, ?)",
            (
                "amzn1.adg.product.not-installed",
                "Not Installed",
                r"C:\Amazon Games\Library\Not Installed",
                0,
            ),
        )
        connection.commit()

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
            system_amazon_games_install_info_paths=[install_info],
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "amazon_games_install_info" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "amazon_games" in gaming_profile["platforms"]
    assert "Blue Fire" in gaming_profile["installed_game_names"]
    assert "Not Installed" not in gaming_profile["installed_game_names"]


def test_system_xbox_microsoft_game_config_probe_adds_games(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    game_config = tmp_path / "XboxGames" / "Avowed" / "Content" / "MicrosoftGame.config"
    _write(
        game_config,
        """<?xml version="1.0" encoding="utf-8"?>
<Game configVersion="1">
  <Identity Name="Microsoft.Avowed" Version="1.0.0.0" Publisher="CN=Microsoft" />
  <ShellVisuals DefaultDisplayName="Avowed" />
</Game>
""",
    )

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
            system_xbox_game_config_paths=[game_config],
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "xbox_game_config" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "xbox" in gaming_profile["platforms"]
    assert "Avowed" in gaming_profile["installed_game_names"]


def test_system_itch_butler_db_probe_adds_games(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    butler_db = tmp_path / "itch" / "db" / "butler.db"
    butler_db.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(butler_db) as connection:
        connection.execute(
            """
            CREATE TABLE caves (
              id INTEGER PRIMARY KEY,
              game_title TEXT,
              install_folder TEXT,
              game_id INTEGER
            )
            """
        )
        connection.execute(
            "INSERT INTO caves(game_title, install_folder, game_id) VALUES (?, ?, ?)",
            ("Celeste Classic", r"D:\itch\games\celeste-classic", 12345),
        )
        connection.commit()

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
            system_itch_butler_db_paths=[butler_db],
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "itch_butler_db" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "itch" in gaming_profile["platforms"]
    assert "Celeste Classic" in gaming_profile["installed_game_names"]


def test_run_scout_discovers_itch_butler_db_under_appdata(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    butler_db = home / "AppData" / "Roaming" / "itch" / "db" / "butler.db"
    butler_db.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(butler_db) as connection:
        connection.execute(
            """
            CREATE TABLE Caves (
              ID INTEGER PRIMARY KEY,
              Title TEXT,
              Path TEXT,
              GameID INTEGER
            )
            """
        )
        connection.execute(
            "INSERT INTO Caves(Title, Path, GameID) VALUES (?, ?, ?)",
            ("A Short Hike", r"C:\Users\Admin\AppData\Roaming\itch\apps\a-short-hike", 67890),
        )
        connection.commit()

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=8,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "itch_butler_db" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "itch" in gaming_profile["platforms"]
    assert "A Short Hike" in gaming_profile["installed_game_names"]


def test_system_xbox_appx_probe_adds_games_from_install_locations(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    package_root = tmp_path / "WindowsApps" / "Microsoft.Avowed_1.0.0.0_x64__8wekyb3d8bbwe"
    game_config = package_root / "Content" / "MicrosoftGame.config"
    _write(
        game_config,
        """<?xml version="1.0" encoding="utf-8"?>
<Game configVersion="1">
  <Identity Name="Microsoft.Avowed" Version="1.0.0.0" Publisher="CN=Microsoft" />
  <ShellVisuals DefaultDisplayName="Avowed" />
</Game>
""",
    )

    class _Completed:
        def __init__(self, stdout: str, returncode: int = 0) -> None:
            self.stdout = stdout
            self.returncode = returncode

    def _fake_run(command, capture_output, text, encoding, errors, timeout, check):
        script = command[-1]
        if isinstance(command, list) and (command[:2] == ["wsl.exe", "--list"] or command[:2] == ["wsl", "--list"]):
            return _Completed("")
        if "Get-AppxPackage" in script and "InstallLocation" in script:
            return _Completed(
                json.dumps(
                    [
                        {
                            "Name": "Microsoft.Avowed",
                            "InstallLocation": str(package_root),
                        }
                    ]
                )
            )
        if "CurrentVersion\\Uninstall" in script:
            return _Completed("[]")
        if "SOFTWARE\\ubisoft\\Launcher\\Installs" in script and "$battleGames" in script:
            return _Completed('{"ubisoft":[],"battleNet":[]}')
        raise AssertionError(f"unexpected subprocess call: {command!r}")

    monkeypatch.setattr("ai_local_scout.runtime.subprocess.run", _fake_run)

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "xbox_game_config" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "xbox" in gaming_profile["platforms"]
    assert "Avowed" in gaming_profile["installed_game_names"]


def test_run_scout_discovers_amazon_games_install_info_under_localappdata(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    install_info = home / "AppData" / "Local" / "Amazon Games" / "Data" / "Games" / "Sql" / "GameInstallInfo.sqlite"
    install_info.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(install_info) as connection:
        connection.execute("CREATE TABLE DbSet (Id TEXT, ProductTitle TEXT, Installed INTEGER)")
        connection.execute(
            "INSERT INTO DbSet(Id, ProductTitle, Installed) VALUES (?, ?, ?)",
            (
                "amzn1.adg.product.11111111-1111-1111-1111-111111111111",
                "A Tiny Sticker Tale",
                1,
            ),
        )
        connection.commit()

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=7,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "amazon_games_install_info" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "amazon_games" in gaming_profile["platforms"]
    assert "A Tiny Sticker Tale" in gaming_profile["installed_game_names"]


def test_run_scout_discovers_battle_net_product_db_under_programdata(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    root = tmp_path / "machine"
    product_db = root / "ProgramData" / "Battle.net" / "Agent" / "product.db"
    _write_bytes(
        product_db,
        _battle_net_product_db(
            _battle_net_product_install("prometheus", "Pro", r"D:\Games\Battle.net\Overwatch"),
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[root],
            home=None,
            max_depth=5,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "battle_net_product_db" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "battle_net" in gaming_profile["platforms"]
    assert "Overwatch 2" in gaming_profile["installed_game_names"]
    assert gaming_profile["battle_net_game_count"] == 1
    assert gaming_profile["battle_net_game_ids"] == ["prometheus"]
    assert gaming_profile["battle_net_product_codes"] == ["Pro"]
    assert gaming_profile["battle_net_install_locations"] == ["D:\\Games\\Battle.net\\Overwatch"]


def test_system_origin_localcontent_probe_adds_ea_games(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    origin_local_content = tmp_path / "ProgramData" / "Origin" / "LocalContent"
    _write(
        origin_local_content / "Mass Effect Legendary Edition" / "metadata.mfst",
        (
            "?currentstate=kReadyToStart"
            "&dipinstallpath=D%3A%5CEA%20Games%5CMass%20Effect%20Legendary%20Edition"
            "&id=OFB-EAST%3A109552419"
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[],
            home=None,
            max_depth=2,
            system_origin_local_content_dirs=[origin_local_content],
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "origin_localcontent_manifest" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "ea" in gaming_profile["platforms"]
    assert "Mass Effect Legendary Edition" in gaming_profile["installed_game_names"]
    assert gaming_profile["ea_game_count"] == 1
    assert gaming_profile["ea_game_ids"] == ["OFB-EAST:109552419"]
    assert gaming_profile["ea_install_locations"] == ["D:\\EA Games\\Mass Effect Legendary Edition"]


def test_run_scout_adds_playnite_library_games_to_gaming_profile(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    playnite_library = home / "AppData" / "Roaming" / "Playnite" / "library" / "games"
    _write(
        playnite_library / "game-1.json",
        json.dumps(
            {
                "Name": "Cyberpunk 2077",
                "InstallDirectory": "D:\\Games\\Cyberpunk 2077",
                "PluginId": "GogLibrary",
                "GameId": "gog-cp2077",
            }
        ),
    )
    _write(
        playnite_library / "game-2.json",
        json.dumps(
            {
                "Name": "StarCraft II",
                "InstallDirectory": "D:\\Games\\StarCraft II",
                "PluginId": "BattleNetLibrary",
                "GameId": "bnet-sc2",
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=8,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "playnite_library_game" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "gog" in gaming_profile["platforms"]
    assert "battle_net" in gaming_profile["platforms"]
    assert {"Cyberpunk 2077", "StarCraft II"}.issubset(set(gaming_profile["installed_game_names"]))


def test_run_scout_builds_headless_game_index_for_runtime_matching(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    playnite_library = home / "AppData" / "Roaming" / "Playnite" / "library" / "games"
    _write(
        playnite_library / "game-1.json",
        json.dumps(
            {
                "Name": "Cyberpunk 2077",
                "InstallDirectory": "D:\\Games\\Cyberpunk 2077",
                "PluginId": "GogLibrary",
                "GameId": "gog-cp2077",
                "Roms": [
                    {"Path": "D:\\Games\\Cyberpunk 2077\\bin\\x64\\Cyberpunk2077.exe"}
                ],
            }
        ),
    )
    steam_root = tmp_path / "Steam"
    steam_library = tmp_path / "SteamLibrary"
    _write(
        steam_root / "steamapps" / "libraryfolders.vdf",
        '"libraryfolders"\n{\n  "0"\n  {\n    "path" "' + str(steam_library).replace("/", "\\") + '"\n  }\n}',
    )
    _write(
        steam_library / "steamapps" / "appmanifest_570.acf",
        '"AppState"\n{\n  "appid" "570"\n  "name" "Dota 2"\n  "installdir" "dota 2 beta"\n}',
    )
    _write(steam_library / "steamapps" / "common" / "dota 2 beta" / "dota2.exe", "")

    report = run_scout(
        ScoutConfig(
            roots=[home, steam_root],
            home=home,
            max_depth=8,
        )
    )

    game_index = report["derived_profile"]["game_index"]
    assert game_index["status"] == "ready"
    assert game_index["schema"] == "weft.game_index/v1"
    cyberpunk = next(game for game in game_index["games"] if game["name"] == "Cyberpunk 2077")
    assert cyberpunk["source"] == "playnite"
    assert cyberpunk["platform"] == "gog"
    assert cyberpunk["executable_names"] == ["cyberpunk2077.exe"]
    assert cyberpunk["executable_paths"] == ["D:/Games/Cyberpunk 2077/bin/x64/Cyberpunk2077.exe"]
    dota = next(game for game in game_index["games"] if game["name"] == "Dota 2")
    assert dota["source"] == "steam_appmanifest"
    assert dota["executable_names"] == ["dota2.exe"]
    assert dota["executable_paths"] == [str(steam_library / "steamapps" / "common" / "dota 2 beta" / "dota2.exe").replace("\\", "/")]


def test_run_scout_adds_gog_galaxy_installed_games(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    gog_storage = home / "AppData" / "ProgramData" / "GOG.com" / "Galaxy" / "storage"
    _write(
        gog_storage / "galaxy-installed.json",
        json.dumps(
            {
                "installed": [
                    {
                        "gameId": "1495134320",
                        "title": "The Witcher 3: Wild Hunt GOTY",
                        "installPath": "D:\\GOG Games\\The Witcher 3 Wild Hunt GOTY",
                    }
                ]
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=8,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "gog_installed" in kinds
    gaming_profile = report["derived_profile"]["gaming_profile"]
    assert "gog" in gaming_profile["platforms"]
    assert "The Witcher 3: Wild Hunt GOTY" in gaming_profile["installed_game_names"]
    assert gaming_profile["gog_game_count"] == 1
    assert gaming_profile["gog_game_ids"] == ["1495134320"]
    assert gaming_profile["gog_install_locations"] == ["D:\\GOG Games\\The Witcher 3 Wild Hunt GOTY"]


def test_system_activitywatch_probe_adds_activity_profile(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    class _Handler(BaseHTTPRequestHandler):
        def do_GET(self):  # noqa: N802
            if self.path != "/api/0/buckets/":
                self.send_response(404)
                self.end_headers()
                return
            payload = {
                "aw-watcher-window_test": {"type": "currentwindow"},
                "aw-watcher-web_test": {"type": "web.tab.current"},
                "aw-watcher-afk_test": {"type": "afkstatus"},
            }
            body = json.dumps(payload).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, format, *args):  # noqa: A003
            return

    with socketserver.TCPServer(("127.0.0.1", 0), _Handler) as server:
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            base_url = f"http://127.0.0.1:{server.server_address[1]}"
            report = run_scout(
                ScoutConfig(
                    roots=[],
                    home=None,
                    max_depth=2,
                    system_activitywatch_base_urls=[base_url],
                )
            )
        finally:
            server.shutdown()
            thread.join(timeout=5)

    activity_entries = [entry for entry in report["raw_evidence"] if entry["entity_kind"] == "activitywatch_runtime"]
    assert activity_entries
    activity_profile = report["derived_profile"]["activitywatch_profile"]
    assert activity_profile["present"] is True
    assert activity_profile["bucket_count"] == 3
    assert activity_profile["watchers"] == ["afk", "web", "window"]


def test_cli_accepts_explicit_teams_dropbox_and_onedrive_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    teams_config = tmp_path / "Teams" / "desktop-config.json"
    dropbox_info = tmp_path / "Dropbox" / "info.json"
    onedrive_settings_root = tmp_path / "OneDrive" / "settings"

    _write(
        teams_config,
        json.dumps(
            {
                "openAtLogin": True,
                "openAsHidden": False,
                "runningOnClose": True,
                "appPreferenceSettings": {
                    "disableGpu": True,
                },
                "currentWebLanguage": "en-US",
                "theme": "dark",
            }
        ),
    )
    _write(
        dropbox_info,
        json.dumps(
            {
                "personal": {
                    "path": "C:\\Users\\Demo\\Dropbox",
                    "host": "dbid:personal123",
                },
                "business": {
                    "path": "C:\\Users\\Demo\\Dropbox (Acme)",
                    "host": "dbid:business456",
                    "team": "Acme",
                },
            }
        ),
    )
    _write(
        onedrive_settings_root / "Personal" / "global.ini",
        "\n".join(
            [
                "MountPoint = C:\\Users\\Demo\\OneDrive",
                "CID = personal-cid",
                "LibraryType = Personal",
                "UserEmail = demo@example.com",
                "FilesOnDemandEnabled = true",
            ]
        ),
    )
    _write(
        onedrive_settings_root / "Business1" / "global.ini",
        "\n".join(
            [
                "MountPoint = C:\\Users\\Demo\\OneDrive - Acme",
                "SiteTitle = Acme Shared",
                "TenantName = Acme Corp",
                "LibraryType = Business",
                "CoAuthEnabledUserSetting = true",
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-teams-config",
            str(teams_config),
            "--system-dropbox-info",
            str(dropbox_info),
            "--system-onedrive-settings-root",
            str(onedrive_settings_root),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "teams_config" in kinds
    assert "dropbox_info" in kinds
    assert "onedrive_global_config" in kinds

    collaboration_profile = report["derived_profile"]["collaboration_tools_profile"]
    assert collaboration_profile["present"] is True
    assert collaboration_profile["tool_families"] == ["microsoft_teams"]
    assert collaboration_profile["teams_client_variants"] == ["classic"]
    assert collaboration_profile["teams_open_at_login"] is True
    assert collaboration_profile["teams_open_as_hidden"] is False
    assert collaboration_profile["teams_running_on_close"] is True
    assert collaboration_profile["teams_disable_gpu"] is True
    assert collaboration_profile["teams_themes"] == ["dark"]
    assert collaboration_profile["teams_locales"] == ["en-US"]

    sync_profile = report["derived_profile"]["sync_storage_profile"]
    assert sync_profile["present"] is True
    assert sync_profile["tool_families"] == ["dropbox", "onedrive"]
    assert sync_profile["dropbox_account_types"] == ["business", "personal"]
    assert sync_profile["dropbox_paths"] == ["C:/Users/Demo/Dropbox", "C:/Users/Demo/Dropbox (Acme)"]
    assert sync_profile["dropbox_hosts"] == ["dbid:business456", "dbid:personal123"]
    assert sync_profile["onedrive_account_slots"] == ["Business1", "Personal"]
    assert sync_profile["onedrive_account_types"] == ["business", "personal"]
    assert sync_profile["onedrive_mount_points"] == [
        "C:/Users/Demo/OneDrive",
        "C:/Users/Demo/OneDrive - Acme",
    ]
    assert sync_profile["onedrive_tenants"] == ["Acme Corp"]
    assert sync_profile["onedrive_site_titles"] == ["Acme Shared"]
    assert sync_profile["onedrive_files_on_demand_enabled"] is True
    assert sync_profile["onedrive_coauth_enabled"] is True

    assert "collaboration_tools" in report["derived_profile"]["interest_tags"]
    assert "sync_storage" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_teams_configs_under_appdata(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / "AppData" / "Roaming" / "Microsoft" / "Teams" / "desktop-config.json",
        json.dumps(
            {
                "openAtLogin": False,
                "openAsHidden": True,
                "runningOnClose": False,
                "theme": "default",
                "currentWebLanguage": "zh-CN",
            }
        ),
    )
    _write(
        home
        / "AppData"
        / "Local"
        / "Packages"
        / "MSTeams_8wekyb3d8bbwe"
        / "LocalCache"
        / "Microsoft"
        / "MSTeams"
        / "settings.json",
        json.dumps(
            {
                "theme": "dark",
                "appLanguage": "en-US",
            }
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=6,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "teams_config" in kinds

    collaboration_profile = report["derived_profile"]["collaboration_tools_profile"]
    assert collaboration_profile["present"] is True
    assert collaboration_profile["teams_client_variants"] == ["classic", "new"]
    assert collaboration_profile["teams_open_at_login"] is False
    assert collaboration_profile["teams_open_as_hidden"] is True
    assert collaboration_profile["teams_running_on_close"] is False
    assert collaboration_profile["teams_themes"] == ["dark", "default"]
    assert collaboration_profile["teams_locales"] == ["en-US", "zh-CN"]


def test_run_scout_discovers_dropbox_and_onedrive_under_appdata(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / "AppData" / "Roaming" / "Dropbox" / "info.json",
        json.dumps(
            {
                "personal": {
                    "path": "D:\\Sync\\Dropbox",
                    "host": "dbid:dropbox123",
                }
            }
        ),
    )
    _write(
        home / "AppData" / "Local" / "Microsoft" / "OneDrive" / "settings" / "Personal" / "global.ini",
        "\n".join(
            [
                "MountPoint = D:\\Sync\\OneDrive",
                "CID = cid-personal",
                "LibraryType = Personal",
                "FilesOnDemandEnabled = false",
            ]
        ),
    )
    _write(
        home / "AppData" / "Local" / "Microsoft" / "OneDrive" / "settings" / "Business1" / "global.ini",
        "\n".join(
            [
                "MountPoint = D:\\Sync\\OneDrive - Acme",
                "SiteTitle = Acme Docs",
                "TenantName = Acme Corp",
                "LibraryType = Business",
                "CoAuthEnabledUserSetting = false",
            ]
        ),
        )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=7,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "dropbox_info" in kinds
    assert "onedrive_global_config" in kinds

    sync_profile = report["derived_profile"]["sync_storage_profile"]
    assert sync_profile["present"] is True
    assert sync_profile["dropbox_account_types"] == ["personal"]
    assert sync_profile["dropbox_paths"] == ["D:/Sync/Dropbox"]
    assert sync_profile["onedrive_account_slots"] == ["Business1", "Personal"]
    assert sync_profile["onedrive_mount_points"] == ["D:/Sync/OneDrive", "D:/Sync/OneDrive - Acme"]
    assert sync_profile["onedrive_site_titles"] == ["Acme Docs"]
    assert sync_profile["onedrive_tenants"] == ["Acme Corp"]
    assert sync_profile["onedrive_files_on_demand_enabled"] is False
    assert sync_profile["onedrive_coauth_enabled"] is False


def test_cli_accepts_explicit_joplin_nextcloud_and_syncthing_probe_paths(tmp_path: Path) -> None:
    from ai_local_scout.cli import main

    joplin_profile = tmp_path / "joplin-desktop"
    nextcloud_config = tmp_path / "Nextcloud" / "nextcloud.cfg"
    syncthing_config = tmp_path / "Syncthing" / "config.xml"

    _write(
        joplin_profile / "settings.json",
        json.dumps(
            {
                "locale": "en_US",
                "theme": 2,
                "themeAutoDetect": True,
                "sync.target": 5,
                "sync.5.path": "https://cloud.example.com/remote.php/dav/files/demo/Joplin",
                "sync.resourceDownloadMode": "auto",
                "ocr.enabled": False,
            }
        ),
    )
    _write(joplin_profile / "plugins" / "io.github.jackgruber.backup.jpl", "")
    _write(joplin_profile / "plugins" / "net.rmusin.saywhat.jpl", "")
    _write(joplin_profile / "userchrome.css", "body { font-size: 14px; }")
    _write_bytes(joplin_profile / "database.sqlite", b"SQLite format 3\x00")

    _write(
        nextcloud_config,
        "\n".join(
            [
                "[General]",
                "launchOnSystemStartup=true",
                "moveToTrash=true",
                "showMainDialogAsNormalWindow=false",
                "",
                "[Accounts]",
                "0\\url=https://cloud.example.com",
                "0\\displayName=Demo User",
                "0\\dav_user=demo",
                "0\\Folders\\1\\localPath=D:/Nextcloud/",
                "0\\Folders\\1\\targetPath=/",
                "0\\Folders\\1\\paused=false",
                "1\\url=https://team.example.com",
                "1\\displayName=Work",
                "1\\Folders\\1\\localPath=E:/TeamFiles/",
                "1\\Folders\\1\\targetPath=/Team",
                "1\\Folders\\1\\paused=true",
            ]
        ),
    )
    _write(
        syncthing_config,
        "\n".join(
            [
                '<configuration version="37">',
                '  <folder id="default" label="Default Folder" path="D:/Sync" type="sendreceive">',
                "    <paused>false</paused>",
                "  </folder>",
                '  <folder id="media" label="Media" path="E:/Media" type="receiveonly">',
                "    <paused>true</paused>",
                "  </folder>",
                '  <device id="DEVICE1" name="Laptop">',
                "    <paused>false</paused>",
                "  </device>",
                '  <device id="DEVICE2" name="NAS">',
                "    <paused>false</paused>",
                "  </device>",
                '  <gui enabled="true" tls="false">',
                "    <address>127.0.0.1:8384</address>",
                "    <theme>default</theme>",
                "    <apikey>secret</apikey>",
                "  </gui>",
                "  <options>",
                "    <globalAnnounceEnabled>true</globalAnnounceEnabled>",
                "    <localAnnounceEnabled>false</localAnnounceEnabled>",
                "    <relaysEnabled>true</relaysEnabled>",
                "    <startBrowser>false</startBrowser>",
                "  </options>",
                "</configuration>",
            ]
        ),
    )

    output_path = tmp_path / "report.json"
    exit_code = main(
        [
            "--output",
            str(output_path),
            "--system-joplin-profile",
            str(joplin_profile),
            "--system-nextcloud-config",
            str(nextcloud_config),
            "--system-syncthing-config",
            str(syncthing_config),
        ]
    )

    assert exit_code == 0
    report = json.loads(output_path.read_text(encoding="utf-8"))
    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "joplin_profile" in kinds
    assert "nextcloud_config" in kinds
    assert "syncthing_config" in kinds

    knowledge_profile = report["derived_profile"]["knowledge_tools_profile"]
    assert knowledge_profile["present"] is True
    assert knowledge_profile["tool_families"] == ["joplin"]
    assert knowledge_profile["joplin_profile_count"] == 1
    assert knowledge_profile["joplin_sync_targets"] == [5]
    assert knowledge_profile["joplin_sync_paths"] == ["https://cloud.example.com/remote.php/dav/files/demo/Joplin"]
    assert knowledge_profile["joplin_locales"] == ["en_US"]
    assert knowledge_profile["joplin_theme_ids"] == [2]
    assert knowledge_profile["joplin_theme_auto_detect"] is True
    assert knowledge_profile["joplin_resource_download_modes"] == ["auto"]
    assert knowledge_profile["joplin_ocr_enabled"] is False
    assert knowledge_profile["joplin_plugin_count"] == 2
    assert knowledge_profile["joplin_plugin_ids"] == ["io.github.jackgruber.backup", "net.rmusin.saywhat"]
    assert knowledge_profile["joplin_custom_css_files"] == ["userchrome.css"]
    assert knowledge_profile["joplin_database_present"] is True

    sync_profile = report["derived_profile"]["sync_storage_profile"]
    assert sync_profile["present"] is True
    assert sync_profile["tool_families"] == ["nextcloud", "syncthing"]
    assert sync_profile["nextcloud_urls"] == ["https://cloud.example.com", "https://team.example.com"]
    assert sync_profile["nextcloud_display_names"] == ["Demo User", "Work"]
    assert sync_profile["nextcloud_local_paths"] == ["D:/Nextcloud/", "E:/TeamFiles/"]
    assert sync_profile["nextcloud_launch_on_startup"] is True
    assert sync_profile["nextcloud_move_to_trash"] is True
    assert sync_profile["nextcloud_paused_folder_count"] == 1
    assert sync_profile["syncthing_folder_ids"] == ["default", "media"]
    assert sync_profile["syncthing_folder_paths"] == ["D:/Sync", "E:/Media"]
    assert sync_profile["syncthing_folder_types"] == ["receiveonly", "sendreceive"]
    assert sync_profile["syncthing_device_names"] == ["Laptop", "NAS"]
    assert sync_profile["syncthing_device_count"] == 2
    assert sync_profile["syncthing_gui_enabled"] is True
    assert sync_profile["syncthing_gui_tls"] is False
    assert sync_profile["syncthing_gui_theme"] == "default"
    assert sync_profile["syncthing_global_discovery_enabled"] is True
    assert sync_profile["syncthing_local_discovery_enabled"] is False
    assert sync_profile["syncthing_relays_enabled"] is True

    assert "knowledge_management" in report["derived_profile"]["interest_tags"]
    assert "sync_storage" in report["derived_profile"]["interest_tags"]


def test_run_scout_discovers_joplin_profile_under_home(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    joplin_profile = home / ".config" / "joplin-desktop"
    _write(
        joplin_profile / "settings.json",
        json.dumps(
            {
                "locale": "zh_CN",
                "theme": 1,
                "sync.target": 2,
                "sync.2.path": "D:\\Docs\\JoplinSync",
                "sync.resourceDownloadMode": "manual",
                "ocr.enabled": True,
            }
        ),
    )
    _write(joplin_profile / "plugins" / "com.example.plugin.jpl", "")
    _write(joplin_profile / "userstyle.css", ".note { line-height: 1.5; }")

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=6,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "joplin_profile" in kinds

    knowledge_profile = report["derived_profile"]["knowledge_tools_profile"]
    assert knowledge_profile["present"] is True
    assert knowledge_profile["tool_families"] == ["joplin"]
    assert knowledge_profile["joplin_sync_targets"] == [2]
    assert knowledge_profile["joplin_sync_paths"] == ["D:/Docs/JoplinSync"]
    assert knowledge_profile["joplin_locales"] == ["zh_CN"]
    assert knowledge_profile["joplin_theme_ids"] == [1]
    assert knowledge_profile["joplin_resource_download_modes"] == ["manual"]
    assert knowledge_profile["joplin_ocr_enabled"] is True
    assert knowledge_profile["joplin_plugin_ids"] == ["com.example.plugin"]
    assert knowledge_profile["joplin_custom_css_files"] == ["userstyle.css"]


def test_run_scout_discovers_nextcloud_and_syncthing_under_appdata(tmp_path: Path) -> None:
    from ai_local_scout.runtime import ScoutConfig, run_scout

    home = tmp_path / "home"
    _write(
        home / "AppData" / "Roaming" / "Nextcloud" / "nextcloud.cfg",
        "\n".join(
            [
                "[General]",
                "launchOnSystemStartup=false",
                "moveToTrash=false",
                "",
                "[Accounts]",
                "0\\url=https://files.example.com",
                "0\\displayName=Files",
                "0\\Folders\\1\\localPath=D:/Files/",
                "0\\Folders\\1\\targetPath=/",
                "0\\Folders\\1\\paused=false",
            ]
        ),
    )
    _write(
        home / "AppData" / "Local" / "Syncthing" / "config.xml",
        "\n".join(
            [
                '<configuration version="37">',
                '  <folder id="docs" label="Docs" path="D:/Docs" type="sendonly">',
                "    <paused>false</paused>",
                "  </folder>",
                '  <device id="DEVICE3" name="Office-PC">',
                "    <paused>false</paused>",
                "  </device>",
                '  <gui enabled="true" tls="true">',
                "    <address>127.0.0.1:8384</address>",
                "    <theme>black</theme>",
                "  </gui>",
                "  <options>",
                "    <globalAnnounceEnabled>false</globalAnnounceEnabled>",
                "    <localAnnounceEnabled>true</localAnnounceEnabled>",
                "    <relaysEnabled>false</relaysEnabled>",
                "  </options>",
                "</configuration>",
            ]
        ),
    )

    report = run_scout(
        ScoutConfig(
            roots=[home],
            home=home,
            max_depth=7,
        )
    )

    kinds = {entry["entity_kind"] for entry in report["raw_evidence"]}
    assert "nextcloud_config" in kinds
    assert "syncthing_config" in kinds

    sync_profile = report["derived_profile"]["sync_storage_profile"]
    assert sync_profile["present"] is True
    assert sync_profile["nextcloud_urls"] == ["https://files.example.com"]
    assert sync_profile["nextcloud_display_names"] == ["Files"]
    assert sync_profile["nextcloud_local_paths"] == ["D:/Files/"]
    assert sync_profile["nextcloud_launch_on_startup"] is False
    assert sync_profile["nextcloud_move_to_trash"] is False
    assert sync_profile["nextcloud_paused_folder_count"] == 0
    assert sync_profile["syncthing_folder_ids"] == ["docs"]
    assert sync_profile["syncthing_folder_paths"] == ["D:/Docs"]
    assert sync_profile["syncthing_folder_types"] == ["sendonly"]
    assert sync_profile["syncthing_device_names"] == ["Office-PC"]
    assert sync_profile["syncthing_gui_enabled"] is True
    assert sync_profile["syncthing_gui_tls"] is True
    assert sync_profile["syncthing_gui_theme"] == "black"
    assert sync_profile["syncthing_global_discovery_enabled"] is False
    assert sync_profile["syncthing_local_discovery_enabled"] is True
    assert sync_profile["syncthing_relays_enabled"] is False
