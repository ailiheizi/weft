from __future__ import annotations

import json
import os
import sqlite3
import tempfile
import re
import tomllib
import configparser
import zipfile
import xml.etree.ElementTree as ET
from collections import Counter
from shutil import copy2
from pathlib import Path
from urllib.parse import unquote
from urllib.parse import parse_qs
from urllib.parse import urlparse
from urllib.request import urlopen


BROWSER_SQLITE_TIMEOUT_SECONDS = 0.1

try:
    import lz4.frame
except ImportError:  # pragma: no cover - optional dependency
    lz4 = None
else:  # pragma: no cover - module import branch
    lz4 = lz4.frame


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def parse_codex_config(path: Path) -> dict:
    data = tomllib.loads(read_text(path))
    trusted_projects: list[str] = []
    projects = data.get("projects", {})
    if isinstance(projects, dict):
        for project_path, config in projects.items():
            if isinstance(config, dict) and config.get("trust_level"):
                trusted_projects.append(str(project_path))

    mcp_servers = data.get("mcp_servers", {})
    server_names = sorted(str(name) for name in mcp_servers.keys()) if isinstance(mcp_servers, dict) else []

    return {
        "model": data.get("model"),
        "provider": data.get("model_provider"),
        "trusted_projects": sorted(trusted_projects),
        "mcp_servers": server_names,
    }


def parse_claude_entrypoint(path: Path) -> dict:
    content = read_text(path)
    referenced_paths = []
    for candidate in re.findall(r"`([^`]+)`", content):
        normalized = candidate.strip()
        if not normalized:
            continue
        if "/" not in normalized and "\\" not in normalized and "." not in normalized:
            continue
        if normalized.startswith("[") and normalized.endswith("]"):
            continue
        referenced_paths.append(normalized)
    return {
        "title": content.splitlines()[0] if content else "",
        "referenced_paths": referenced_paths,
    }


def parse_markdown_summary(path: Path) -> dict:
    content = read_text(path)
    headings = [line.strip("# ").strip() for line in content.splitlines() if line.startswith("#")]
    bullets = [line[2:].strip() for line in content.splitlines() if line.startswith("- ")]
    return {
        "headings": headings[:10],
        "bullets": bullets[:20],
    }


def parse_office_document(path: Path, *, preview_char_limit: int = 2000) -> dict:
    document_type = path.suffix.lower().lstrip(".")
    preview_text = ""
    structural_summary: dict[str, object] = {}
    parse_error = None

    try:
        if document_type == "docx":
            preview_text, structural_summary = _parse_docx_preview(path)
        elif document_type == "pptx":
            preview_text, structural_summary = _parse_pptx_preview(path)
        elif document_type == "xlsx":
            preview_text, structural_summary = _parse_xlsx_preview(path)
    except Exception as error:  # pragma: no cover - defensive for malformed Office archives
        parse_error = f"{type(error).__name__}: {error}"

    preview_text = _redact_sensitive_text(_compact_lines(preview_text))[:preview_char_limit]
    stat = path.stat()
    recommended_skills = ["office-document-specialist-suite" if document_type == "xlsx" else document_type]
    skill_routes = [{"skill": skill, "arguments": {"path": str(path.resolve())}} for skill in recommended_skills]

    return {
        "filename": path.name,
        "document_type": document_type,
        "size_bytes": stat.st_size,
        "modified_time": stat.st_mtime,
        "preview_text": preview_text,
        "preview_char_count": len(preview_text),
        "structural_summary": structural_summary,
        "recommended_skills": recommended_skills,
        "skill_routes": skill_routes,
        "parse_error": parse_error,
        "evidence_basis": "desktop_office_document_light_preview",
    }


def _parse_docx_preview(path: Path) -> tuple[str, dict[str, object]]:
    with zipfile.ZipFile(path) as archive:
        document_xml = archive.read("word/document.xml")
    root = ET.fromstring(document_xml)
    texts = [node.text or "" for node in root.iter() if node.tag.endswith("}t") and node.text]
    return "\n".join(texts), {"paragraph_text_count": len(texts)}


def _parse_pptx_preview(path: Path) -> tuple[str, dict[str, object]]:
    slide_texts: list[str] = []
    slide_names: list[str] = []
    with zipfile.ZipFile(path) as archive:
        for name in sorted(item for item in archive.namelist() if item.startswith("ppt/slides/slide") and item.endswith(".xml")):
            slide_names.append(name)
            root = ET.fromstring(archive.read(name))
            texts = [node.text or "" for node in root.iter() if node.tag.endswith("}t") and node.text]
            if texts:
                slide_texts.append("\n".join(texts))
    return "\n\n".join(slide_texts), {"slide_count": len(slide_names), "slides_with_text_count": len(slide_texts)}


def _parse_xlsx_preview(path: Path) -> tuple[str, dict[str, object]]:
    with zipfile.ZipFile(path) as archive:
        sheet_names = _xlsx_sheet_names(archive)
        shared_strings = _xlsx_shared_strings(archive)
        previews = []
        sheet_dimensions = []
        for index, sheet_name in enumerate(sheet_names, start=1):
            worksheet_path = f"xl/worksheets/sheet{index}.xml"
            if worksheet_path not in archive.namelist():
                continue
            rows = _xlsx_rows(archive.read(worksheet_path), shared_strings)
            non_empty_rows = [row for row in rows if any(cell.strip() for cell in row)]
            if non_empty_rows:
                previews.append(f"[{sheet_name}]\n" + "\n".join(" | ".join(row) for row in non_empty_rows[:10]))
            sheet_dimensions.append(
                {
                    "name": sheet_name,
                    "row_count": len(non_empty_rows),
                    "column_count": max((len(row) for row in non_empty_rows), default=0),
                }
            )
    return "\n\n".join(previews), {"sheet_names": sheet_names, "sheets": sheet_dimensions}


def _xlsx_sheet_names(archive: zipfile.ZipFile) -> list[str]:
    if "xl/workbook.xml" not in archive.namelist():
        return []
    root = ET.fromstring(archive.read("xl/workbook.xml"))
    names = [str(node.attrib.get("name") or "Sheet") for node in root.iter() if node.tag.endswith("}sheet")]
    return names or ["Sheet1"]


def _xlsx_shared_strings(archive: zipfile.ZipFile) -> list[str]:
    if "xl/sharedStrings.xml" not in archive.namelist():
        return []
    root = ET.fromstring(archive.read("xl/sharedStrings.xml"))
    return ["".join(node.itertext()) for node in root.iter() if node.tag.endswith("}si")]


def _xlsx_rows(worksheet_xml: bytes, shared_strings: list[str]) -> list[list[str]]:
    root = ET.fromstring(worksheet_xml)
    rows = []
    for row_node in root.iter():
        if not row_node.tag.endswith("}row"):
            continue
        row = []
        for cell_node in row_node:
            if not cell_node.tag.endswith("}c"):
                continue
            cell_type = cell_node.attrib.get("t")
            value = ""
            if cell_type == "inlineStr":
                value = "".join(cell_node.itertext())
            else:
                value_node = next((child for child in cell_node if child.tag.endswith("}v")), None)
                if value_node is not None and value_node.text is not None:
                    value = value_node.text
                    if cell_type == "s" and value.isdigit():
                        value = shared_strings[int(value)] if int(value) < len(shared_strings) else value
            row.append(value)
        rows.append(row)
    return rows


def _compact_lines(text: str) -> str:
    return "\n".join(line.strip() for line in text.splitlines() if line.strip())


def _redact_sensitive_text(text: str) -> str:
    text = re.sub(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}", "[EMAIL_REDACTED]", text)
    text = re.sub(r"(?i)(api[_-]?key|token|secret|password)\s*[:=]\s*\S+", r"\1=[REDACTED]", text)
    return text


def parse_json_file(path: Path) -> dict:
    return json.loads(read_text(path))


def parse_claude_settings(path: Path) -> dict:
    data = parse_json_file(path)
    mcp_servers = data.get("mcpServers", {})
    if not isinstance(mcp_servers, dict):
        mcp_servers = {}
    env = data.get("env", {})
    if not isinstance(env, dict):
        env = {}

    enabled_packages: set[str] = set()
    package_names: set[str] = set()

    explicit_plugins = data.get("enabledPlugins", [])
    if isinstance(explicit_plugins, list):
        for item in explicit_packages:
            if isinstance(item, str) and item.strip():
                package_names.add(item)
                enabled_plugins.add(item)
    elif isinstance(explicit_plugins, dict):
        for name, enabled in explicit_plugins.items():
            package_name = str(name)
            package_names.add(package_name)
            if enabled is True:
                enabled_plugins.add(package_name)

    plugins = data.get("plugins", {})
    if isinstance(plugins, dict):
        for name, config in plugins.items():
            package_name = str(name)
            package_names.add(package_name)
            if isinstance(config, dict):
                if config.get("enabled") is True:
                    enabled_plugins.add(package_name)
            elif config is True:
                enabled_plugins.add(package_name)

    return {
        "defaultModel": data.get("defaultModel"),
        "apiKey": data.get("apiKey"),
        "mcp_servers": sorted(str(name) for name in mcp_servers.keys()),
        "env": {str(key): value for key, value in env.items()},
        "env_keys": sorted(str(key) for key in env.keys()),
        "package_names": sorted(package_names),
        "enabled_plugins": sorted(enabled_plugins),
    }


def parse_claude_mcp_config(path: Path) -> dict:
    data = parse_json_file(path)
    servers = data.get("servers", {})
    if not isinstance(servers, dict):
        servers = {}
    return {
        "server_names": sorted(str(name) for name in servers.keys()),
    }


def parse_obsidian_global_config(path: Path) -> dict:
    data = parse_json_file(path)
    raw_vaults = data.get("vaults", {})
    vaults: list[dict] = []
    if isinstance(raw_vaults, dict):
        for vault_id, config in raw_vaults.items():
            if not isinstance(config, dict):
                continue
            vault_path = str(config.get("path") or "").strip()
            if not vault_path:
                continue
            vaults.append(
                {
                    "vault_id": str(vault_id),
                    "path": vault_path,
                    "name": Path(vault_path).name or str(vault_id),
                    "last_open_ts": config.get("ts"),
                    "is_open": bool(config.get("open")) if "open" in config else None,
                }
            )
    vaults.sort(key=lambda item: (str(item["name"]).lower(), str(item["path"]).lower()))
    return {"vaults": vaults}


def parse_obsidian_vault(path: Path) -> dict:
    config_dir = path / ".obsidian"
    note_paths: list[str] = []
    for note_path in sorted(path.rglob("*.md")):
        if ".obsidian" in note_path.parts:
            continue
        try:
            note_paths.append(str(note_path.relative_to(path)))
        except ValueError:
            note_paths.append(note_path.name)

    core_plugins = _read_json_string_list(config_dir / "core-plugins.json")
    community_plugins = _read_json_string_list(config_dir / "community-plugins.json")
    app_config = _read_optional_json_file(config_dir / "app.json")

    attachment_folder_path = None
    always_update_links = None
    if isinstance(app_config, dict):
        attachment_folder_path = app_config.get("attachmentFolderPath")
        always_update_links = app_config.get("alwaysUpdateLinks")

    return {
        "vault_name": path.name,
        "vault_path": str(path),
        "note_count": len(note_paths),
        "note_paths": note_paths[:50],
        "core_plugins": core_plugins,
        "community_plugins": community_plugins,
        "attachment_folder_path": str(attachment_folder_path or "").strip() or None,
        "always_update_links": always_update_links,
    }


def parse_obs_studio_profile(path: Path) -> dict:
    parser = configparser.ConfigParser()
    parser.read_string(read_text(path))

    output_mode = parser.get("Output", "Mode", fallback="").strip()
    advanced_recording_format = parser.get("AdvOut", "RecFormat2", fallback="").strip()
    simple_recording_format = parser.get("SimpleOutput", "RecFormat", fallback="").strip()
    if output_mode.lower() == "advanced" and advanced_recording_format:
        recording_format = advanced_recording_format
    else:
        recording_format = advanced_recording_format or simple_recording_format or None

    streaming_services = sorted(
        {
            parser.get(section, "Service", fallback="").strip()
            for section in parser.sections()
            if section.lower().startswith("stream") and parser.get(section, "Service", fallback="").strip()
        }
    )

    return {
        "profile_name": parser.get("General", "Name", fallback=path.parent.name).strip() or path.parent.name,
        "base_resolution": _format_resolution(
            parser.get("Video", "BaseCX", fallback=""),
            parser.get("Video", "BaseCY", fallback=""),
        ),
        "output_resolution": _format_resolution(
            parser.get("Video", "OutputCX", fallback=""),
            parser.get("Video", "OutputCY", fallback=""),
        ),
        "output_mode": output_mode or None,
        "recording_format": recording_format,
        "encoder": parser.get("AdvOut", "Encoder", fallback="").strip() or None,
        "streaming_services": streaming_services,
    }


def parse_obs_studio_scene_collection(path: Path) -> dict:
    data = parse_json_file(path)
    collection_name = str(data.get("name") or path.stem).strip() or path.stem

    scene_names: list[str] = []
    for item in data.get("scene_order", []):
        if not isinstance(item, dict):
            continue
        scene_name = str(item.get("name") or "").strip()
        if scene_name:
            scene_names.append(scene_name)

    if not scene_names:
        for source in data.get("sources", []):
            if not isinstance(source, dict):
                continue
            if str(source.get("id") or "").strip().lower() != "scene":
                continue
            scene_name = str(source.get("name") or "").strip()
            if scene_name:
                scene_names.append(scene_name)

    source_types = sorted(
        {
            str(source.get("id") or "").strip()
            for source in data.get("sources", [])
            if isinstance(source, dict)
            and str(source.get("id") or "").strip()
            and str(source.get("id") or "").strip().lower() != "scene"
        }
    )

    return {
        "collection_name": collection_name,
        "current_scene": str(
            data.get("current_program_scene") or data.get("current_scene") or ""
        ).strip()
        or None,
        "scene_names": sorted(set(scene_names)),
        "scene_count": len(set(scene_names)),
        "source_types": source_types,
    }


def parse_docker_desktop_settings(path: Path) -> dict:
    data = parse_json_file(path)
    if not isinstance(data, dict):
        return {}

    return {
        "uses_wsl_engine": bool(data.get("wslEngineEnabled")),
        "kubernetes_enabled": bool(data.get("kubernetes")),
        "extensions_enabled": bool(data.get("extensionsEnabled")),
        "marketplace_only_extensions": bool(data.get("onlyMarketplaceExtensions")),
        "model_runner_enabled": bool(data.get("enableInference")),
        "model_runner_tcp_enabled": bool(data.get("enableInferenceTCP")),
        "model_runner_tcp_port": _optional_int(data.get("enableInferenceTCPPort")),
        "desktop_terminal_enabled": bool(data.get("desktopTerminalEnabled")),
        "exposes_docker_api_tcp_2375": bool(data.get("exposeDockerAPIOnTCP2375")),
        "enhanced_container_isolation": bool(data.get("enhancedContainerIsolation")),
    }


def parse_wsl_global_config(path: Path) -> dict:
    parser = configparser.ConfigParser()
    parser.read_string(read_text(path))

    return {
        "memory_limit": _config_value(parser, "wsl2", "memory"),
        "processor_count": _config_int(parser, "wsl2", "processors"),
        "localhost_forwarding": _config_bool(parser, "wsl2", "localhostForwarding"),
        "networking_mode": _config_value(parser, "wsl2", "networkingMode"),
        "nested_virtualization": _config_bool(parser, "wsl2", "nestedVirtualization"),
        "swap_size": _config_value(parser, "wsl2", "swap"),
        "auto_memory_reclaim": _config_value(parser, "experimental", "autoMemoryReclaim"),
        "sparse_vhd": _config_bool(parser, "experimental", "sparseVhd"),
    }


def parse_wsl_distribution_list(stdout: str) -> dict:
    distros: list[dict] = []
    default_distro = None

    for raw_line in stdout.splitlines():
        line = raw_line.rstrip()
        if not line.strip():
            continue
        normalized = line.lstrip()
        if normalized.upper().startswith("NAME"):
            continue

        is_default = normalized.startswith("*")
        if is_default:
            normalized = normalized[1:].lstrip()

        parts = re.split(r"\s{2,}", normalized)
        if len(parts) < 3:
            continue
        name = parts[0].strip()
        state = parts[1].strip()
        version = _optional_int(parts[2].strip())
        if not name or version is None:
            continue

        distros.append(
            {
                "name": name,
                "state": state,
                "version": version,
                "is_default": is_default,
            }
        )
        if is_default and default_distro is None:
            default_distro = name

    return {
        "default_distro": default_distro,
        "distros": distros,
    }


def parse_discord_settings(path: Path) -> dict:
    data = parse_json_file(path)
    if not isinstance(data, dict):
        return {}
    return {
        "open_on_startup": data.get("openOnStartup"),
        "theme": data.get("theme"),
        "status": data.get("status"),
        "hardware_acceleration": data.get("enableHardwareAcceleration"),
        "locale": data.get("locale"),
    }


def parse_teams_config(path: Path) -> dict:
    data = parse_json_file(path)
    if not isinstance(data, dict):
        return {
            "client_variant": _infer_teams_client_variant(path),
        }

    preference_settings = data.get("appPreferenceSettings", {})
    preference_settings = preference_settings if isinstance(preference_settings, dict) else {}
    return {
        "client_variant": _infer_teams_client_variant(path),
        "open_at_login": data.get("openAtLogin"),
        "open_as_hidden": data.get("openAsHidden"),
        "running_on_close": data.get("runningOnClose"),
        "disable_gpu": _first_present_value(
            preference_settings.get("disableGpu"),
            data.get("disableGpu"),
        ),
        "theme": _first_non_empty_string(
            data.get("theme"),
            preference_settings.get("theme"),
            data.get("appTheme"),
        ),
        "locale": _first_non_empty_string(
            data.get("currentWebLanguage"),
            data.get("appLanguage"),
            data.get("language"),
        ),
    }


def parse_dropbox_info(path: Path) -> dict:
    data = parse_json_file(path)
    if not isinstance(data, dict):
        return {"accounts": []}

    accounts: list[dict] = []
    for account_type, fields in data.items():
        if not isinstance(fields, dict):
            continue
        normalized_account_type = str(account_type or "").strip().lower()
        sync_path = _normalize_output_path(fields.get("path"))
        host = _first_non_empty_string(fields.get("host"))
        team = _first_non_empty_string(fields.get("team"))
        if not any([normalized_account_type, sync_path, host, team]):
            continue
        accounts.append(
            {
                "account_type": normalized_account_type or None,
                "path": sync_path,
                "host": host,
                "team": team,
            }
        )
    accounts.sort(key=lambda item: (str(item.get("account_type") or "").lower(), str(item.get("path") or "").lower()))
    return {"accounts": accounts[:50]}


def parse_onedrive_global_config(path: Path) -> dict:
    values = _parse_key_value_lines(path)
    account_slot = path.parent.name
    account_type = _infer_onedrive_account_type(account_slot, values.get("LibraryType"))
    return {
        "account_slot": account_slot,
        "account_type": account_type,
        "mount_point": _normalize_output_path(values.get("MountPoint")),
        "cid": _first_non_empty_string(values.get("CID")),
        "library_type": _first_non_empty_string(values.get("LibraryType")),
        "tenant_name": _first_non_empty_string(values.get("TenantName")),
        "site_title": _first_non_empty_string(values.get("SiteTitle")),
        "files_on_demand_enabled": _optional_bool(values.get("FilesOnDemandEnabled")),
        "coauth_enabled": _optional_bool(
            _first_non_empty_string(
                values.get("CoAuthEnabledUserSetting"),
                values.get("CoauthEnabledUserSetting"),
            )
        ),
    }


def parse_joplin_profile(path: Path) -> dict:
    settings_path = path / "settings.json"
    settings = _read_optional_json_file(settings_path)
    settings = settings if isinstance(settings, dict) else {}
    sync_target = _optional_int(settings.get("sync.target"))
    sync_path = None
    if sync_target is not None:
        sync_path = _normalize_output_path(settings.get(f"sync.{sync_target}.path"))

    plugin_ids: list[str] = []
    plugin_dir = path / "plugins"
    if plugin_dir.exists() and plugin_dir.is_dir():
        for plugin_path in sorted(plugin_dir.iterdir()):
            if not plugin_path.is_file():
                continue
            plugin_id = plugin_path.stem.strip()
            if plugin_id:
                plugin_ids.append(plugin_id)

    custom_css_files = sorted(
        [
            css_path.name
            for css_path in [path / "userchrome.css", path / "userstyle.css"]
            if css_path.exists() and css_path.is_file()
        ]
    )
    database_path = path / "database.sqlite"

    return {
        "profile_path": str(path),
        "locale": _first_non_empty_string(settings.get("locale")),
        "theme": _optional_int(settings.get("theme")),
        "theme_auto_detect": _optional_bool(settings.get("themeAutoDetect")),
        "sync_target": sync_target,
        "sync_path": sync_path,
        "resource_download_mode": _first_non_empty_string(settings.get("sync.resourceDownloadMode")),
        "ocr_enabled": _optional_bool(settings.get("ocr.enabled")),
        "plugin_ids": sorted(dict.fromkeys(plugin_ids)),
        "plugin_count": len(set(plugin_ids)),
        "custom_css_files": custom_css_files,
        "database_present": database_path.exists() and database_path.is_file(),
    }


def parse_nextcloud_config(path: Path) -> dict:
    values = _parse_key_value_lines(path)
    account_indexes = sorted(
        {
            match.group(1)
            for key in values
            for match in [re.match(r"^(\d+)\\", key)]
            if match
        },
        key=lambda item: int(item),
    )

    accounts: list[dict] = []
    paused_folder_count = 0
    for account_index in account_indexes:
        url = _first_non_empty_string(values.get(f"{account_index}\\url"))
        display_name = _first_non_empty_string(values.get(f"{account_index}\\displayName"))
        dav_user = _first_non_empty_string(values.get(f"{account_index}\\dav_user"))
        local_paths: list[str] = []
        target_paths: list[str] = []

        folder_indexes = sorted(
            {
                match.group(1)
                for key in values
                for match in [re.match(rf"^{re.escape(account_index)}\\Folders\\(\d+)\\", key)]
                if match
            },
            key=lambda item: int(item),
        )
        for folder_index in folder_indexes:
            local_path = _normalize_output_path(values.get(f"{account_index}\\Folders\\{folder_index}\\localPath"))
            target_path = _first_non_empty_string(values.get(f"{account_index}\\Folders\\{folder_index}\\targetPath"))
            paused = _optional_bool(values.get(f"{account_index}\\Folders\\{folder_index}\\paused"))
            if local_path:
                local_paths.append(local_path)
            if target_path:
                target_paths.append(target_path)
            if paused is True:
                paused_folder_count += 1

        if not any([url, display_name, dav_user, local_paths, target_paths]):
            continue
        accounts.append(
            {
                "url": url,
                "display_name": display_name,
                "dav_user": dav_user,
                "local_paths": sorted(dict.fromkeys(local_paths)),
                "target_paths": sorted(dict.fromkeys(target_paths)),
            }
        )

    accounts.sort(key=lambda item: (str(item.get("display_name") or "").lower(), str(item.get("url") or "").lower()))
    return {
        "launch_on_system_startup": _optional_bool(values.get("launchOnSystemStartup")),
        "move_to_trash": _optional_bool(values.get("moveToTrash")),
        "show_main_dialog_as_normal_window": _optional_bool(values.get("showMainDialogAsNormalWindow")),
        "accounts": accounts[:50],
        "paused_folder_count": paused_folder_count,
    }


def parse_syncthing_config(path: Path) -> dict:
    try:
        root = ET.fromstring(_read_text_with_fallbacks(path))
    except (ET.ParseError, OSError, UnicodeDecodeError):
        return {
            "folders": [],
            "devices": [],
            "gui_enabled": None,
            "gui_tls": None,
            "gui_theme": None,
            "global_announce_enabled": None,
            "local_announce_enabled": None,
            "relays_enabled": None,
        }

    folders: list[dict] = []
    for folder in root.findall("./folder"):
        folder_id = str(folder.attrib.get("id") or "").strip() or None
        label = str(folder.attrib.get("label") or "").strip() or None
        folder_path = _normalize_output_path(folder.attrib.get("path"))
        folder_type = str(folder.attrib.get("type") or "").strip() or None
        paused = _optional_bool(_element_text(folder.find("paused")))
        if not any([folder_id, label, folder_path, folder_type, paused is not None]):
            continue
        folders.append(
            {
                "id": folder_id,
                "label": label,
                "path": folder_path,
                "type": folder_type,
                "paused": paused,
            }
        )

    devices: list[dict] = []
    for device in root.findall("./device"):
        device_id = str(device.attrib.get("id") or "").strip() or None
        name = str(device.attrib.get("name") or "").strip() or None
        paused = _optional_bool(_element_text(device.find("paused")))
        if not any([device_id, name, paused is not None]):
            continue
        devices.append(
            {
                "id": device_id,
                "name": name,
                "paused": paused,
            }
        )

    gui = root.find("./gui")
    options = root.find("./options")
    return {
        "folders": folders[:100],
        "devices": devices[:100],
        "gui_enabled": _optional_bool(gui.attrib.get("enabled")) if gui is not None else None,
        "gui_tls": _optional_bool(gui.attrib.get("tls")) if gui is not None else None,
        "gui_theme": _element_text(gui.find("theme")) if gui is not None else None,
        "global_announce_enabled": _optional_bool(_element_text(options.find("globalAnnounceEnabled"))) if options is not None else None,
        "local_announce_enabled": _optional_bool(_element_text(options.find("localAnnounceEnabled"))) if options is not None else None,
        "relays_enabled": _optional_bool(_element_text(options.find("relaysEnabled"))) if options is not None else None,
    }


def parse_jetbrains_recent_projects(path: Path) -> dict:
    try:
        root = ET.fromstring(read_text(path))
    except (ET.ParseError, OSError, UnicodeDecodeError):
        return {
            "product": path.parent.parent.name,
            "recent_project_paths": [],
        }

    project_paths: list[str] = []
    for option in root.findall(".//option[@name='recentPaths']/list/option"):
        value = str(option.attrib.get("value") or "").strip()
        if value:
            project_paths.append(value)

    if not project_paths:
        for entry in root.findall(".//option[@name='additionalInfo']/map/entry"):
            value = str(entry.attrib.get("key") or "").strip()
            if value:
                project_paths.append(value)

    deduped_paths = sorted(dict.fromkeys(project_paths))
    return {
        "product": path.parent.parent.name,
        "recent_project_paths": deduped_paths[:100],
    }


def parse_windows_terminal_settings(path: Path) -> dict:
    data = parse_json_file(path)
    if not isinstance(data, dict):
        return {
            "default_profile": None,
            "profiles": [],
        }

    profiles_raw = data.get("profiles", {})
    profile_list = profiles_raw.get("list", []) if isinstance(profiles_raw, dict) else []
    profiles: list[dict] = []
    default_profile_guid = str(data.get("defaultProfile") or "").strip()
    default_profile_name = None

    if isinstance(profile_list, list):
        for item in profile_list:
            if not isinstance(item, dict):
                continue
            guid = str(item.get("guid") or "").strip()
            name = str(item.get("name") or "").strip()
            source = str(item.get("source") or "").strip() or None
            commandline = str(item.get("commandline") or "").strip() or None
            if not name:
                continue
            profiles.append(
                {
                    "guid": guid or None,
                    "name": name,
                    "source": source,
                    "commandline": commandline,
                }
            )
            if guid and guid == default_profile_guid:
                default_profile_name = name

    profiles.sort(key=lambda item: item["name"].lower())
    return {
        "default_profile": default_profile_name,
        "profiles": profiles[:100],
    }


def parse_ssh_config(path: Path) -> dict:
    hosts: list[dict] = []
    current_aliases: list[str] = []
    current_hostname = None
    current_identity_files: list[str] = []

    def _flush() -> None:
        nonlocal current_aliases, current_hostname, current_identity_files
        for alias in current_aliases:
            if not alias or "*" in alias or "?" in alias:
                continue
            hosts.append(
                {
                    "alias": alias,
                    "hostname": current_hostname,
                    "identity_files": sorted(dict.fromkeys(current_identity_files)),
                }
            )
        current_aliases = []
        current_hostname = None
        current_identity_files = []

    for raw_line in read_text(path).splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        lower_line = line.lower()
        if lower_line.startswith("host "):
            _flush()
            current_aliases = [part for part in line[5:].split() if part.strip()]
            continue
        if not current_aliases:
            continue
        if lower_line.startswith("hostname "):
            current_hostname = line.split(None, 1)[1].strip() if len(line.split(None, 1)) == 2 else None
            continue
        if lower_line.startswith("identityfile "):
            value = line.split(None, 1)[1].strip() if len(line.split(None, 1)) == 2 else ""
            if value:
                current_identity_files.append(value)

    _flush()
    hosts.sort(key=lambda item: item["alias"].lower())
    return {"hosts": hosts[:200]}


def parse_kubeconfig(path: Path) -> dict:
    lines = read_text(path).splitlines()

    current_context = None
    context_names: list[str] = []
    cluster_names: list[str] = []
    user_names: list[str] = []
    namespace_names: list[str] = []

    current_section = None
    pending_context = False
    for raw_line in lines:
        line = raw_line.rstrip()
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        if stripped.startswith("current-context:"):
            current_context = stripped.split(":", 1)[1].strip() or None
            continue
        if stripped == "contexts:":
            current_section = "contexts"
            pending_context = False
            continue
        if stripped == "clusters:":
            current_section = "clusters"
            pending_context = False
            continue
        if stripped == "users:":
            current_section = "users"
            pending_context = False
            continue

        if current_section == "contexts":
            if stripped.startswith("- name:"):
                context_names.append(stripped.split(":", 1)[1].strip())
                pending_context = False
                continue
            if stripped.startswith("context:"):
                pending_context = True
                continue
            if pending_context and stripped.startswith("namespace:"):
                namespace = stripped.split(":", 1)[1].strip()
                if namespace:
                    namespace_names.append(namespace)
                continue
        elif current_section == "clusters" and stripped.startswith("- name:"):
            cluster_names.append(stripped.split(":", 1)[1].strip())
            continue
        elif current_section == "users" and stripped.startswith("- name:"):
            user_names.append(stripped.split(":", 1)[1].strip())
            continue

    return {
        "current_context": current_context,
        "context_names": sorted(dict.fromkeys(context_names)),
        "cluster_names": sorted(dict.fromkeys(cluster_names)),
        "user_names": sorted(dict.fromkeys(user_names)),
        "namespace_names": sorted(dict.fromkeys(namespace_names)),
    }


def parse_docker_cli_config(path: Path) -> dict:
    data = parse_json_file(path)
    if not isinstance(data, dict):
        return {"current_context": None}
    return {
        "current_context": str(data.get("currentContext") or "").strip() or None,
    }


def parse_docker_context_meta(path: Path) -> dict:
    data = parse_json_file(path)
    if not isinstance(data, dict):
        return {
            "name": path.parent.name,
            "docker_host": None,
            "description": None,
        }

    endpoints = data.get("Endpoints", {})
    docker_endpoint = endpoints.get("docker", {}) if isinstance(endpoints, dict) else {}
    metadata = data.get("Metadata", {})
    return {
        "name": str(data.get("Name") or path.parent.name).strip() or path.parent.name,
        "docker_host": str(docker_endpoint.get("Host") or "").strip() or None,
        "description": str(metadata.get("Description") or "").strip() or None,
    }


def parse_aws_cli_config(path: Path) -> dict:
    parser = configparser.ConfigParser()
    parser.read_string(read_text(path))

    profiles: list[dict] = []
    sso_sessions: list[str] = []
    for section in parser.sections():
        normalized = section.strip()
        lowered = normalized.lower()
        if lowered.startswith("profile "):
            profile_name = normalized[8:].strip()
        elif lowered == "default":
            profile_name = "default"
        else:
            profile_name = None

        if profile_name:
            profiles.append(
                {
                    "name": profile_name,
                    "region": _config_value(parser, section, "region"),
                    "output": _config_value(parser, section, "output"),
                    "sso_session": _config_value(parser, section, "sso_session"),
                }
            )
            continue

        if lowered.startswith("sso-session "):
            session_name = normalized[len("sso-session ") :].strip()
            if session_name:
                sso_sessions.append(session_name)

    profiles.sort(key=lambda item: str(item["name"]).lower())
    return {
        "profiles": profiles[:100],
        "sso_sessions": sorted(dict.fromkeys(sso_sessions)),
    }


def parse_azure_cli_profile(path: Path) -> dict:
    data = parse_json_file(path)
    if not isinstance(data, dict):
        return {
            "subscriptions": [],
            "default_subscription": None,
        }

    subscriptions: list[dict] = []
    default_subscription = None
    raw_subscriptions = data.get("subscriptions", [])
    if isinstance(raw_subscriptions, list):
        for item in raw_subscriptions:
            if not isinstance(item, dict):
                continue
            name = str(item.get("name") or "").strip()
            subscription_id = str(item.get("id") or "").strip() or None
            tenant_id = str(item.get("tenantId") or "").strip() or None
            state = str(item.get("state") or "").strip() or None
            cloud_name = str(item.get("cloudName") or "").strip() or None
            if not (name or subscription_id):
                continue
            subscription = {
                "name": name or subscription_id,
                "id": subscription_id,
                "tenant_id": tenant_id,
                "state": state,
                "cloud_name": cloud_name,
                "is_default": bool(item.get("isDefault")),
            }
            subscriptions.append(subscription)
            if subscription["is_default"] and default_subscription is None:
                default_subscription = subscription["name"]

    subscriptions.sort(key=lambda item: str(item["name"]).lower())
    return {
        "subscriptions": subscriptions[:200],
        "default_subscription": default_subscription,
    }


def parse_gcloud_active_config(path: Path) -> dict:
    active_configuration = read_text(path).strip()
    return {
        "active_configuration": active_configuration or None,
    }


def parse_gcloud_cli_config(path: Path) -> dict:
    parser = configparser.ConfigParser()
    parser.read_string(read_text(path))

    configuration_name = path.name.removeprefix("config_").strip() or path.name
    return {
        "configuration_name": configuration_name,
        "account": _config_value(parser, "core", "account"),
        "project": _config_value(parser, "core", "project"),
        "disable_usage_reporting": _config_bool(parser, "core", "disable_usage_reporting"),
        "region": _config_value(parser, "compute", "region"),
        "zone": _config_value(parser, "compute", "zone"),
    }


def parse_github_cli_config(path: Path) -> dict:
    values = _parse_simple_yaml_mapping(path)
    return {
        "git_protocol": values.get("git_protocol"),
        "editor": values.get("editor"),
        "prompt": values.get("prompt"),
    }


def parse_github_cli_hosts(path: Path) -> dict:
    lines = read_text(path).splitlines()
    hosts: list[dict] = []
    current_host = None
    current_values: dict[str, str] = {}

    def _flush() -> None:
        nonlocal current_host, current_values
        if current_host:
            hosts.append(
                {
                    "host": current_host,
                    "user": current_values.get("user"),
                    "git_protocol": current_values.get("git_protocol"),
                    "oauth_token_present": "oauth_token" in current_values and bool(current_values.get("oauth_token")),
                }
            )
        current_host = None
        current_values = {}

    for raw_line in lines:
        if not raw_line.strip():
            continue
        if not raw_line.startswith(" ") and raw_line.endswith(":"):
            _flush()
            current_host = raw_line[:-1].strip()
            continue
        if current_host is None:
            continue
        stripped = raw_line.strip()
        if ":" not in stripped:
            continue
        key, value = stripped.split(":", 1)
        key_name = key.strip()
        value_text = value.strip().strip("'\"")
        current_values[key_name] = value_text or ""

    _flush()
    hosts.sort(key=lambda item: str(item["host"]).lower())
    return {
        "hosts": hosts[:100],
    }


def parse_git_global_config(path: Path) -> dict:
    parser = configparser.ConfigParser()
    parser.read_string(read_text(path))
    return {
        "user_name": _config_value(parser, "user", "name"),
        "user_email": _config_value(parser, "user", "email"),
        "editor": _config_value(parser, "core", "editor"),
        "default_branch": _config_value(parser, "init", "defaultBranch"),
        "pull_rebase": _config_bool(parser, "pull", "rebase"),
        "github_user": _config_value(parser, "github", "user"),
    }


def parse_npm_user_config(path: Path) -> dict:
    values = _parse_key_value_lines(path)
    scope_registries = sorted(
        {
            key.split(":", 1)[0]
            for key in values
            if key.startswith("@") and key.endswith(":registry")
        }
    )
    return {
        "registry": values.get("registry"),
        "save_exact": _optional_bool(values.get("save-exact")),
        "prefix": values.get("prefix"),
        "scope_registries": scope_registries,
    }


def parse_cargo_user_config(path: Path) -> dict:
    data = tomllib.loads(read_text(path))
    if not isinstance(data, dict):
        return {}

    build = data.get("build", {})
    build = build if isinstance(build, dict) else {}
    term = data.get("term", {})
    term = term if isinstance(term, dict) else {}
    registry = data.get("registry", {})
    registry = registry if isinstance(registry, dict) else {}
    net = data.get("net", {})
    net = net if isinstance(net, dict) else {}
    registries = data.get("registries", {})
    registries = registries if isinstance(registries, dict) else {}
    crates_io = registries.get("crates-io", {})
    crates_io = crates_io if isinstance(crates_io, dict) else {}

    return {
        "target_dir": _normalize_backslash_path(build.get("target-dir")),
        "term_verbose": _optional_bool(term.get("verbose")),
        "default_registry": str(registry.get("default") or "").strip() or None,
        "crates_io_protocol": str(crates_io.get("protocol") or "").strip() or None,
        "git_fetch_with_cli": _optional_bool(net.get("git-fetch-with-cli")),
        "net_retry": _optional_int(net.get("retry")),
    }


def parse_cargo_credentials_store(path: Path) -> dict:
    data = tomllib.loads(read_text(path))
    if not isinstance(data, dict):
        return {"credential_count": 0, "registry_names": [], "token_registries": []}

    registries = data.get("registries", {})
    registries = registries if isinstance(registries, dict) else {}
    registry_names = sorted(str(name).strip() for name in registries if str(name).strip())
    token_registries = sorted(
        {
            str(name).strip()
            for name, config in registries.items()
            if str(name).strip()
            and isinstance(config, dict)
            and _has_meaningful_value(config.get("token"))
        }
    )
    return {
        "credential_count": len(registry_names),
        "registry_names": registry_names,
        "token_registries": token_registries,
    }


def parse_maven_user_settings(path: Path) -> dict:
    try:
        root = ET.fromstring(read_text(path))
    except (ET.ParseError, OSError, UnicodeDecodeError):
        return {}

    namespace = ""
    if root.tag.startswith("{") and "}" in root.tag:
        namespace = root.tag[1:].split("}", 1)[0]

    def _tag(name: str) -> str:
        return f"{{{namespace}}}{name}" if namespace else name

    def _text(element: ET.Element | None, child_name: str) -> str | None:
        if element is None:
            return None
        child = element.find(_tag(child_name))
        if child is None or child.text is None:
            return None
        text = child.text.strip()
        return text or None

    local_repository = _text(root, "localRepository")
    offline = _optional_bool(_text(root, "offline"))
    plugin_groups = sorted(
        {
            str(item.text).strip()
            for item in root.findall(f"./{_tag('pluginGroups')}/{_tag('pluginGroup')}")
            if item.text and str(item.text).strip()
        }
    )

    mirror_ids: list[str] = []
    mirror_urls: list[str] = []
    for mirror in root.findall(f"./{_tag('mirrors')}/{_tag('mirror')}"):
        mirror_id = _text(mirror, "id")
        mirror_url = _text(mirror, "url")
        if mirror_id:
            mirror_ids.append(mirror_id)
        if mirror_url:
            mirror_urls.append(mirror_url)

    server_ids: list[str] = []
    credential_server_ids: list[str] = []
    for server in root.findall(f"./{_tag('servers')}/{_tag('server')}"):
        server_id = _text(server, "id")
        if not server_id:
            continue
        server_ids.append(server_id)
        if any(
            _text(server, child_name)
            for child_name in ("username", "password", "privateKey", "passphrase")
        ):
            credential_server_ids.append(server_id)

    proxy_hosts = sorted(
        {
            str(host).strip()
            for proxy in root.findall(f"./{_tag('proxies')}/{_tag('proxy')}")
            for host in [_text(proxy, "host")]
            if str(host or "").strip()
        }
    )
    active_profiles = sorted(
        {
            str(item.text).strip()
            for item in root.findall(f"./{_tag('activeProfiles')}/{_tag('activeProfile')}")
            if item.text and str(item.text).strip()
        }
    )

    return {
        "local_repository": _normalize_backslash_path(local_repository),
        "offline": offline,
        "plugin_groups": plugin_groups,
        "mirror_ids": sorted(dict.fromkeys(mirror_ids)),
        "mirror_urls": sorted(dict.fromkeys(mirror_urls)),
        "server_ids": sorted(dict.fromkeys(server_ids)),
        "credential_server_ids": sorted(dict.fromkeys(credential_server_ids)),
        "active_profiles": active_profiles,
        "proxy_hosts": proxy_hosts,
    }


def parse_nuget_user_config(path: Path) -> dict:
    try:
        root = ET.fromstring(read_text(path))
    except (ET.ParseError, OSError, UnicodeDecodeError):
        return {}

    def _iter_adds(section_name: str) -> list[tuple[str, str]]:
        section = root.find(section_name)
        if section is None:
            return []
        items: list[tuple[str, str]] = []
        for add in section.findall("add"):
            key = str(add.attrib.get("key") or "").strip()
            value = str(add.attrib.get("value") or "").strip()
            if key and value:
                items.append((key, value))
        return items

    config_values = {key: value for key, value in _iter_adds("config")}
    package_sources = _iter_adds("packageSources")
    disabled_sources = sorted(
        {
            key
            for key, value in _iter_adds("disabledPackageSources")
            if value.strip().lower() in {"true", "1", "yes"}
        }
    )

    credential_sources: list[str] = []
    credentials_root = root.find("packageSourceCredentials")
    if credentials_root is not None:
        for source in list(credentials_root):
            source_name = str(source.tag or "").strip()
            if not source_name:
                continue
            has_credential = any(
                str(add.attrib.get("key") or "").strip().lower() in {"username", "password", "cleartextpassword", "validauthenticationtypes"}
                and str(add.attrib.get("value") or "").strip()
                for add in source.findall("add")
            )
            if has_credential:
                credential_sources.append(source_name)

    return {
        "global_packages_folder": _normalize_backslash_path(config_values.get("globalPackagesFolder")),
        "default_push_source": config_values.get("defaultPushSource"),
        "signature_validation_mode": config_values.get("signatureValidationMode"),
        "package_source_names": sorted(key for key, _value in package_sources),
        "package_source_urls": sorted(value for _key, value in package_sources),
        "disabled_sources": disabled_sources,
        "credential_sources": sorted(dict.fromkeys(credential_sources)),
    }


def parse_pnpm_user_config(path: Path) -> dict:
    values = _parse_key_value_lines(path)
    scope_registries = sorted(
        {
            key.split(":", 1)[0]
            for key in values
            if key.startswith("@") and key.endswith(":registry")
        }
    )
    return {
        "registry": values.get("registry"),
        "global_dir": _normalize_backslash_path(values.get("global-dir")),
        "store_dir": _normalize_backslash_path(values.get("store-dir")),
        "package_import_method": values.get("package-import-method"),
        "node_linker": values.get("node-linker"),
        "shamefully_hoist": _optional_bool(values.get("shamefully-hoist")),
        "scope_registries": scope_registries,
    }


def parse_pip_user_config(path: Path) -> dict:
    parser = configparser.ConfigParser()
    parser.read_string(read_text(path))

    trusted_hosts = _config_multiline_values(parser, "global", "trusted-host")
    extra_index_urls = _config_multiline_values(parser, "install", "extra-index-url")
    return {
        "index_url": _config_value(parser, "global", "index-url"),
        "trusted_hosts": trusted_hosts,
        "extra_index_urls": extra_index_urls,
        "timeout": _config_int(parser, "global", "timeout"),
        "disable_version_check": _config_bool(parser, "global", "disable-pip-version-check"),
    }


def parse_gradle_user_properties(path: Path) -> dict:
    values = _parse_key_value_lines(path)
    return {
        "caching": _optional_bool(values.get("org.gradle.caching")),
        "parallel": _optional_bool(values.get("org.gradle.parallel")),
        "configuration_cache": _optional_bool(values.get("org.gradle.configuration-cache")),
        "daemon": _optional_bool(values.get("org.gradle.daemon")),
        "jvmargs": values.get("org.gradle.jvmargs"),
        "java_home": _normalize_backslash_path(values.get("org.gradle.java.home")),
    }


def parse_dotnet_global_tools(path: Path) -> dict:
    commands: list[str] = []
    if not path.exists() or not path.is_dir():
        return {"commands": [], "command_count": 0}

    ignored_names = {".store"}
    for child in sorted(path.iterdir(), key=lambda item: item.name.lower()):
        if child.name in ignored_names or child.is_dir():
            continue
        stem = child.stem if child.suffix.lower() in {".exe", ".cmd", ".bat", ".ps1"} else child.name
        command = stem.strip()
        if not command:
            continue
        commands.append(command)

    deduped_commands = sorted(dict.fromkeys(commands))
    return {
        "commands": deduped_commands,
        "command_count": len(deduped_commands),
    }


def parse_conda_user_config(path: Path) -> dict:
    data = _parse_simple_yaml_document(path)
    channels = data.get("channels", []) if isinstance(data.get("channels"), list) else []
    envs_dirs = data.get("envs_dirs", []) if isinstance(data.get("envs_dirs"), list) else []
    return {
        "channels": [str(item).strip() for item in channels if str(item).strip()],
        "envs_dirs": [str(item).strip() for item in envs_dirs if str(item).strip()],
        "auto_activate_base": _optional_bool(data.get("auto_activate_base")),
        "changeps1": _optional_bool(data.get("changeps1")),
        "show_channel_urls": _optional_bool(data.get("show_channel_urls")),
    }


def parse_poetry_user_config(path: Path) -> dict:
    data = tomllib.loads(read_text(path))
    if not isinstance(data, dict):
        return {}

    virtualenvs = data.get("virtualenvs", {})
    virtualenvs = virtualenvs if isinstance(virtualenvs, dict) else {}
    installer = data.get("installer", {})
    installer = installer if isinstance(installer, dict) else {}
    return {
        "system_git_client": _optional_bool(data.get("system-git-client")),
        "virtualenvs_create": _optional_bool(virtualenvs.get("create")),
        "virtualenvs_in_project": _optional_bool(virtualenvs.get("in-project")),
        "virtualenvs_path": str(virtualenvs.get("path") or "").strip() or None,
        "installer_parallel": _optional_bool(installer.get("parallel")),
        "installer_max_workers": _optional_int(installer.get("max-workers")),
    }


def parse_uv_user_config(path: Path) -> dict:
    data = tomllib.loads(read_text(path))
    if not isinstance(data, dict):
        return {}

    pip = data.get("pip", {})
    pip = pip if isinstance(pip, dict) else {}
    return {
        "index_url": str(data.get("index-url") or "").strip() or None,
        "extra_index_urls": _string_list_value(data.get("extra-index-url")),
        "cache_dir": _normalize_backslash_path(data.get("cache-dir")),
        "python_preference": str(data.get("python-preference") or "").strip() or None,
        "native_tls": _optional_bool(data.get("native-tls")),
        "offline": _optional_bool(data.get("offline")),
        "preview": _optional_bool(data.get("preview")),
        "pip_index_url": str(pip.get("index-url") or "").strip() or None,
    }


def parse_uv_credentials_store(path: Path) -> dict:
    data = tomllib.loads(read_text(path))
    if not isinstance(data, dict):
        return {"credentials": [], "credential_count": 0}

    credentials: list[dict] = []

    def _walk(value: object, breadcrumb: list[str]) -> None:
        if isinstance(value, dict):
            normalized = {str(key).strip(): nested for key, nested in value.items()}
            service_name = ".".join(part for part in breadcrumb if part) or None
            url = _first_non_empty_string(
                normalized.get("url"),
                normalized.get("index-url"),
                normalized.get("publish-url"),
                normalized.get("endpoint"),
                normalized.get("registry"),
            )
            username = _first_non_empty_string(
                normalized.get("username"),
                normalized.get("user"),
                normalized.get("login"),
            )
            password_present = _has_meaningful_value(normalized.get("password")) or _has_meaningful_value(
                normalized.get("secret")
            )
            token_present = _has_meaningful_value(normalized.get("token"))
            is_credential_leaf = bool(url or username or password_present or token_present)
            if is_credential_leaf:
                credentials.append(
                    {
                        "service_name": service_name,
                        "url": url,
                        "username": username,
                        "password_present": password_present,
                        "token_present": token_present,
                    }
                )
            for key, nested in normalized.items():
                if isinstance(nested, (dict, list)):
                    _walk(nested, [*breadcrumb, key])
            return
        if isinstance(value, list):
            for index, item in enumerate(value):
                _walk(item, [*breadcrumb, str(index)])

    _walk(data, [])

    deduped_credentials: list[dict] = []
    seen: set[tuple[str | None, str | None, str | None, bool, bool]] = set()
    for credential in credentials:
        key = (
            str(credential.get("service_name") or "").strip() or None,
            str(credential.get("url") or "").strip() or None,
            str(credential.get("username") or "").strip() or None,
            bool(credential.get("password_present")),
            bool(credential.get("token_present")),
        )
        if key in seen:
            continue
        seen.add(key)
        deduped_credentials.append(credential)

    deduped_credentials.sort(
        key=lambda item: (
            str(item.get("service_name") or "").lower(),
            str(item.get("url") or "").lower(),
            str(item.get("username") or "").lower(),
        )
    )
    return {
        "credentials": deduped_credentials[:100],
        "credential_count": len(deduped_credentials),
    }


def parse_rustup_settings(path: Path) -> dict:
    data = tomllib.loads(read_text(path))
    if not isinstance(data, dict):
        return {}

    overrides = data.get("overrides", {})
    overrides = overrides if isinstance(overrides, dict) else {}
    override_paths = sorted(_normalize_backslash_path(path) or str(path).strip() for path in overrides if str(path).strip())
    return {
        "version": str(data.get("version") or "").strip() or None,
        "default_toolchain": str(data.get("default_toolchain") or "").strip() or None,
        "profile": str(data.get("profile") or "").strip() or None,
        "override_count": len(override_paths),
        "override_paths": override_paths,
    }


def parse_yarn_user_config(path: Path) -> dict:
    data = _parse_simple_yaml_document(path)
    npm_scopes = data.get("npmScopes", {})
    npm_scopes = npm_scopes if isinstance(npm_scopes, dict) else {}

    scope_names = sorted(str(name).strip() for name in npm_scopes if str(name).strip())
    scope_registries = sorted(
        {
            str(values.get("npmRegistryServer") or "").strip()
            for values in npm_scopes.values()
            if isinstance(values, dict) and str(values.get("npmRegistryServer") or "").strip()
        }
    )
    return {
        "node_linker": str(data.get("nodeLinker") or "").strip() or None,
        "npm_registry_server": str(data.get("npmRegistryServer") or "").strip() or None,
        "enable_global_cache": _optional_bool(data.get("enableGlobalCache")),
        "enable_telemetry": _optional_bool(data.get("enableTelemetry")),
        "global_folder": _normalize_backslash_path(data.get("globalFolder")),
        "yarn_path": str(data.get("yarnPath") or "").strip() or None,
        "npm_scope_names": scope_names,
        "npm_scope_registries": scope_registries,
    }


def parse_codex_auth(path: Path) -> dict:
    data = parse_json_file(path)
    user = data.get("user", {})
    if not isinstance(user, dict):
        user = {}
    return {
        "access_token": data.get("access_token"),
        "email": user.get("email"),
    }


def parse_codex_rules(path: Path) -> dict:
    lines = [line.strip() for line in read_text(path).splitlines() if line.strip()]
    allowed = [line for line in lines if "decision=\"allow\"" in line]
    return {
        "rule_count": len(lines),
        "allowed_rule_count": len(allowed),
        "rules_preview": lines[:10],
    }


def parse_browser_bookmarks(path: Path) -> dict:
    data = parse_json_file(path)
    titles: list[str] = []
    urls: list[str] = []

    def _walk(node: object) -> None:
        if isinstance(node, dict):
            node_type = node.get("type")
            if node_type == "url":
                if node.get("name"):
                    titles.append(str(node["name"]))
                if node.get("url"):
                    urls.append(str(node["url"]))
            for value in node.values():
                _walk(value)
        elif isinstance(node, list):
            for item in node:
                _walk(item)

    _walk(data)
    return {
        "browser": _infer_chromium_browser(path),
        "bookmark_titles": titles[:50],
        "bookmark_urls": urls[:50],
        "bookmark_domains": _normalize_domains(urls),
    }


def parse_browser_history(path: Path) -> dict:
    rows = _read_browser_history_rows(path)

    urls = [url for url, _title, _visit_count in rows]
    history_domain_counts: Counter[str] = Counter()
    search_queries = _extract_search_queries(rows)
    for url, _title, visit_count in rows:
        for domain in _normalize_domains([url]):
            history_domain_counts[domain] += int(visit_count or 0)

    return {
        "browser": _infer_chromium_browser(path),
        "recent_urls": urls,
        "recent_titles": [title for _url, title, _visit_count in rows if title],
        "recent_domains": _normalize_domains(urls),
        "top_history_domains": [
            {"domain": domain, "visit_count": count}
            for domain, count in history_domain_counts.most_common(10)
        ],
        "top_search_queries": search_queries,
    }


def parse_browser_downloads(path: Path) -> dict:
    rows = _read_browser_download_rows(path)
    downloads: list[dict] = []
    domains: list[str] = []
    file_extensions: set[str] = set()

    for row in rows:
        urls = [value for value in (row["tab_url"], row["site_url"], row["chain_url"]) if value]
        domains.extend(_normalize_domains(urls))
        target_name = Path(str(row["target_path"] or "")).name
        suffix = Path(target_name).suffix.lower()
        if suffix:
            file_extensions.add(suffix)
        downloads.append(
            {
                "target_name": target_name or None,
                "file_extension": suffix or None,
                "mime_type": row["mime_type"] or None,
                "source_domains": _normalize_domains(urls),
                "received_bytes": row["received_bytes"],
                "total_bytes": row["total_bytes"],
                "state": row["state"],
            }
        )

    return {
        "browser": _infer_chromium_browser(path),
        "downloads": downloads[:50],
        "download_domains": sorted(set(domains)),
        "download_file_extensions": sorted(file_extensions),
    }


def parse_firefox_downloads(path: Path) -> dict:
    rows = _query_firefox_downloads(path)
    downloads: list[dict] = []
    domains: list[str] = []
    file_extensions: set[str] = set()

    for row in rows:
        urls = [value for value in (row["source"],) if value]
        domains.extend(_normalize_domains(urls))
        target_name = Path(str(row["target"] or "")).name
        suffix = Path(target_name).suffix.lower()
        if suffix:
            file_extensions.add(suffix)
        downloads.append(
            {
                "target_name": target_name or None,
                "file_extension": suffix or None,
                "mime_type": None,
                "source_domains": _normalize_domains(urls),
                "received_bytes": 0,
                "total_bytes": 0,
                "state": None,
            }
        )

    return {
        "browser": _infer_firefox_like_browser(path),
        "downloads": downloads[:50],
        "download_domains": sorted(set(domains)),
        "download_file_extensions": sorted(file_extensions),
    }


def _query_firefox_downloads(path: Path) -> list[dict]:
    with sqlite3.connect(path, timeout=BROWSER_SQLITE_TIMEOUT_SECONDS) as connection:
        table_names = {
            str(row[0])
            for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
        }
        if "moz_downloads" not in table_names:
            return []

        columns = {
            str(row[1])
            for row in connection.execute("PRAGMA table_info(moz_downloads)").fetchall()
        }
        source_expr = "COALESCE(source, '')" if "source" in columns else "''"
        target_expr = "COALESCE(target, '')" if "target" in columns else "''"
        cursor = connection.execute(
            f"""
            SELECT {source_expr}, {target_expr}
            FROM moz_downloads
            ORDER BY id DESC
            LIMIT 100
            """
        )
        return [{"source": str(source or ""), "target": str(target or "")} for source, target in cursor.fetchall()]


def parse_browser_session(path: Path) -> dict:
    urls: list[str] = []
    titles: list[str] = []
    payload = _read_session_payload(path)

    def _collect(node: object) -> None:
        if isinstance(node, dict):
            url = node.get("url")
            title = node.get("title")
            if isinstance(url, str) and url.strip():
                urls.append(url.strip())
            if isinstance(title, str) and title.strip():
                titles.append(title.strip())
            for value in node.values():
                _collect(value)
        elif isinstance(node, list):
            for item in node:
                _collect(item)

    _collect(payload)
    browser = _infer_firefox_like_browser(path) if _is_firefox_like_session(path) else _infer_chromium_browser(path)
    return {
        "browser": browser,
        "session_titles": sorted(set(titles))[:50],
        "session_domains": _normalize_domains(urls),
        "tab_count": len(set(urls)),
    }


def _read_session_payload(path: Path) -> object:
    try:
        return json.loads(read_text(path))
    except (UnicodeDecodeError, json.JSONDecodeError):
        try:
            raw = path.read_bytes()
        except OSError:
            return {}
        if not raw.startswith(b"mozLz40\0") or lz4 is None:
            return {}
        try:
            decompressed = lz4.decompress(raw[8:])
            return json.loads(decompressed.decode("utf-8"))
        except (RuntimeError, ValueError, UnicodeDecodeError, json.JSONDecodeError):
            return {}


def _is_firefox_like_session(path: Path) -> bool:
    normalized = str(path).lower()
    return "\\mozilla\\firefox\\profiles\\" in normalized or "\\zen\\profiles\\" in normalized


def _read_browser_download_rows(path: Path) -> list[dict]:
    try:
        return _query_browser_downloads(path)
    except sqlite3.OperationalError as error:
        if "locked" not in str(error).lower():
            raise

    fd, temp_name = tempfile.mkstemp(prefix="ai-local-scout-downloads-", suffix=path.suffix or ".sqlite")
    os.close(fd)
    copy_path = Path(temp_name)
    try:
        copy2(path, copy_path)
        return _query_browser_downloads(copy_path)
    finally:
        try:
            copy_path.unlink(missing_ok=True)
        except OSError:
            pass


def _query_browser_downloads(path: Path) -> list[dict]:
    with sqlite3.connect(path, timeout=BROWSER_SQLITE_TIMEOUT_SECONDS) as connection:
        table_names = {
            str(row[0])
            for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
        }
        if "downloads" not in table_names:
            return []

        download_columns = {
            str(row[1])
            for row in connection.execute("PRAGMA table_info(downloads)").fetchall()
        }
        has_url_chains = "downloads_url_chains" in table_names
        chain_select = "duc.url" if has_url_chains else "NULL"
        chain_join = "LEFT JOIN downloads_url_chains duc ON duc.id = d.id AND duc.chain_index = 0" if has_url_chains else ""
        if "target_path" in download_columns:
            target_path_expr = "COALESCE(d.target_path, '')"
        elif "current_path" in download_columns:
            target_path_expr = "COALESCE(d.current_path, '')"
        else:
            target_path_expr = "''"
        cursor = connection.execute(
            f"""
            SELECT
              {target_path_expr},
              COALESCE(d.tab_url, ''),
              COALESCE(d.site_url, ''),
              COALESCE(d.mime_type, ''),
              COALESCE(d.received_bytes, 0),
              COALESCE(d.total_bytes, 0),
              COALESCE(d.state, 0),
              COALESCE({chain_select}, '')
            FROM downloads d
            {chain_join}
            ORDER BY d.id DESC
            LIMIT 100
            """
        )
        return [
            {
                "target_path": str(target_path or ""),
                "tab_url": str(tab_url or ""),
                "site_url": str(site_url or ""),
                "mime_type": str(mime_type or ""),
                "received_bytes": int(received_bytes or 0),
                "total_bytes": int(total_bytes or 0),
                "state": int(state or 0),
                "chain_url": str(chain_url or ""),
            }
            for target_path, tab_url, site_url, mime_type, received_bytes, total_bytes, state, chain_url in cursor.fetchall()
        ]


def parse_browser_extension_manifest(path: Path) -> dict:
    data = parse_json_file(path)
    extension_id = path.parent.parent.name
    name = _extension_locale_value(data.get("name"))
    description = _extension_locale_value(data.get("description"))
    permissions = _sorted_string_list(data.get("permissions"))
    host_permissions = _sorted_string_list(data.get("host_permissions"))
    return {
        "browser": _infer_chromium_browser(path),
        "extensions": [
            {
                "id": extension_id,
                "name": name or extension_id,
                "version": str(data.get("version") or ""),
                "description": description,
                "enabled": True,
                "permissions": permissions,
                "host_permissions": host_permissions,
            }
        ],
    }


def parse_firefox_extensions(path: Path) -> dict:
    data = parse_json_file(path)
    addons = data.get("addons", [])
    extensions: list[dict] = []
    if isinstance(addons, list):
        for item in addons:
            if not isinstance(item, dict):
                continue
            if str(item.get("type") or "").lower() != "extension":
                continue
            locale = item.get("defaultLocale")
            locale = locale if isinstance(locale, dict) else {}
            extension_id = str(item.get("id") or "")
            if not extension_id:
                continue
            extensions.append(
                {
                    "id": extension_id,
                    "name": str(locale.get("name") or item.get("name") or extension_id),
                    "version": str(item.get("version") or ""),
                    "enabled": item.get("active") is not False,
                    "permissions": [],
                    "host_permissions": [],
                }
            )
    extensions.sort(key=lambda item: str(item["name"]).lower())
    return {
        "browser": _infer_firefox_like_browser(path),
        "extensions": extensions[:100],
    }


def _read_browser_history_rows(path: Path) -> list[tuple[str, str, int]]:
    try:
        return _query_browser_history(path)
    except sqlite3.OperationalError as error:
        if "locked" not in str(error).lower():
            raise

    fd, temp_name = tempfile.mkstemp(prefix="ai-local-scout-history-", suffix=path.suffix or ".sqlite")
    os.close(fd)
    copy_path = Path(temp_name)
    try:
        copy2(path, copy_path)
        return _query_browser_history(copy_path)
    finally:
        try:
            copy_path.unlink(missing_ok=True)
        except OSError:
            pass


def _query_browser_history(path: Path) -> list[tuple[str, str, int]]:
    with sqlite3.connect(path, timeout=BROWSER_SQLITE_TIMEOUT_SECONDS) as connection:
        try:
            cursor = connection.execute(
                "SELECT url, title, COALESCE(visit_count, 0) FROM urls ORDER BY COALESCE(visit_count, 0) DESC, id DESC LIMIT 100"
            )
            return [(str(url), str(title or ""), int(visit_count or 0)) for url, title, visit_count in cursor.fetchall()]
        except sqlite3.OperationalError as error:
            if "visit_count" not in str(error).lower():
                raise
            cursor = connection.execute("SELECT url, title FROM urls ORDER BY id DESC LIMIT 100")
            return [(str(url), str(title or ""), 0) for url, title in cursor.fetchall()]


def parse_git_config(path: Path) -> dict:
    content = read_text(path)
    remote_urls = re.findall(r"^\s*url\s*=\s*(.+)$", content, flags=re.MULTILINE)
    branches = re.findall(r'^\[branch "([^"]+)"\]', content, flags=re.MULTILINE)
    return {
        "remote_urls": remote_urls,
        "branches": branches,
    }


def parse_workspace_file(path: Path) -> dict:
    data = parse_json_file(path)
    folders = data.get("folders", [])
    paths: list[str] = []
    if isinstance(folders, list):
        for item in folders:
            if isinstance(item, dict) and item.get("path"):
                paths.append(str(item["path"]))
    return {
        "folder_paths": paths,
    }


def parse_editor_recent_workspaces(path: Path) -> dict:
    data = parse_json_file(path)
    recent_paths: list[str] = []

    def _collect(node: object) -> None:
        if isinstance(node, dict):
            for key, value in node.items():
                if key in {"folderUri", "configPath"} and isinstance(value, str):
                    resolved = _workspace_uri_to_path(value)
                    if resolved:
                        recent_paths.append(resolved)
                elif isinstance(value, str) and _looks_like_workspace_path(value):
                    recent_paths.append(value)
                else:
                    _collect(value)
        elif isinstance(node, list):
            for item in node:
                _collect(item)

    _collect(data)
    editor = "cursor" if "cursor" in str(path).lower() else "vscode"
    deduped = sorted(
        {
            item
            for item in recent_paths
            if item and "\\backups\\" not in item.lower() and "\\workspacestorage\\" not in item.lower()
        }
    )
    return {
        "editor": editor,
        "recent_workspaces": deduped[:100],
    }


def parse_shell_history(path: Path) -> dict:
    commands = [line.strip() for line in read_text(path).splitlines() if line.strip()]
    command_names = [
        normalized
        for line in commands
        for normalized in [_normalize_shell_command(line)]
        if normalized
    ]
    top_commands = [
        {"command": command, "count": count}
        for command, count in sorted(Counter(command_names).items(), key=lambda item: (-item[1], item[0]))[:10]
    ]
    shell = "powershell" if path.name.lower() == "consolehost_history.txt" else "unknown"
    return {
        "shell": shell,
        "recent_commands": commands[-50:],
        "top_commands": top_commands,
    }


def _normalize_shell_command(line: str) -> str | None:
    stripped = line.strip()
    if not stripped:
        return None

    lowered = stripped.lower()
    if lowered in {"s", "ss", "sss"}:
        return None

    if lowered.startswith("& "):
        stripped = stripped[2:].strip()
        lowered = stripped.lower()

    if lowered.startswith("$") and ";" in stripped:
        stripped = stripped.split(";", 1)[1].strip()
        lowered = stripped.lower()

    if (stripped.startswith("'") or stripped.startswith('"')) and len(stripped) > 1:
        quote = stripped[0]
        end = stripped.find(quote, 1)
        token = stripped[1:end] if end > 1 else stripped.strip("'\"").split()[0]
    else:
        token = stripped.split()[0]

    token = token.strip("'\"").replace("/", "\\")
    command = Path(token).name.lower()
    if command.endswith(".exe"):
        command = command[:-4]
    if not command or command in {"&", "while", "where"}:
        return None
    if any(char in command for char in "[]{}()"):
        return None
    if not any(char.isalpha() for char in command):
        return None
    return command


def parse_installed_apps(path: Path) -> dict:
    data = parse_json_file(path)
    items = data.get("items", [])
    apps: list[dict] = []
    if isinstance(items, list):
        for item in items:
            if not isinstance(item, dict):
                continue
            name = item.get("DisplayName")
            if not name:
                continue
            apps.append(
                {
                    "name": str(name),
                    "version": item.get("DisplayVersion"),
                    "publisher": item.get("Publisher"),
                    "install_location": item.get("InstallLocation"),
                }
            )
    apps.sort(key=lambda app: app["name"].lower())
    return {
        "apps": apps[:500],
    }


def parse_cursor_state_db(path: Path) -> dict:
    composer_count = 0
    agentic_count = 0
    modes: set[str] = set()
    recent_workspaces: list[str] = []

    with sqlite3.connect(path) as connection:
        item_rows = connection.execute(
            "SELECT key, value FROM ItemTable WHERE key = 'history.recentlyOpenedPathsList' LIMIT 5"
        ).fetchall()
        for _key, value in item_rows:
            try:
                payload = json.loads(_decode_sqlite_value(value))
            except json.JSONDecodeError:
                continue
            entries = payload.get("entries", []) if isinstance(payload, dict) else []
            if isinstance(entries, list):
                for entry in entries:
                    if not isinstance(entry, dict):
                        continue
                    for key_name in ("folderUri", "configPath"):
                        raw = entry.get(key_name)
                        if not isinstance(raw, str):
                            continue
                        resolved = _workspace_uri_to_path(raw)
                        if resolved and "\\backups\\" not in resolved.lower():
                            recent_workspaces.append(resolved)

        composer_rows = connection.execute(
            "SELECT key, value FROM cursorDiskKV WHERE key LIKE 'composerData:%' LIMIT 200"
        ).fetchall()
        for _key, value in composer_rows:
            try:
                payload = json.loads(_decode_sqlite_value(value))
            except json.JSONDecodeError:
                continue
            if not isinstance(payload, dict):
                continue
            composer_count += 1
            mode = payload.get("unifiedMode")
            if isinstance(mode, str) and mode.strip():
                modes.add(mode.strip())
            if payload.get("isAgentic") is True:
                agentic_count += 1

    return {
        "recent_workspaces": sorted(set(recent_workspaces))[:50],
        "composer_count": composer_count,
        "agentic_count": agentic_count,
        "modes": sorted(modes),
    }


def parse_legendary_installed(path: Path) -> dict:
    data = parse_json_file(path)
    games: list[dict] = []
    if isinstance(data, dict):
        for key, item in data.items():
            if not isinstance(item, dict):
                continue
            title = item.get("title") or item.get("app_title") or key
            install_path = item.get("install_path") or item.get("install_location")
            app_name = item.get("app_name") or key
            games.append(
                {
                    "name": str(title),
                    "app_name": str(app_name),
                    "install_location": str(install_path) if install_path else None,
                    "platform": "epic",
                }
            )
    games.sort(key=lambda game: game["name"].lower())
    return {"games": games[:500]}


def parse_playnite_game(path: Path) -> dict:
    data = parse_json_file(path)
    name = data.get("Name") or data.get("name")
    plugin_id = str(data.get("PluginId") or data.get("pluginId") or "")
    executable_paths = []
    for rom in data.get("Roms") or data.get("roms") or []:
        if not isinstance(rom, dict):
            continue
        rom_path = rom.get("Path") or rom.get("path")
        if rom_path:
            executable_paths.append(str(rom_path))
    return {
        "games": [
            {
                "name": str(name),
                "platform": _playnite_platform(plugin_id),
                "platform_game_id": data.get("GameId") or data.get("gameId"),
                "install_location": data.get("InstallDirectory") or data.get("installDirectory"),
                "executable_candidates": executable_paths,
                "source": "playnite",
                "plugin_id": plugin_id or None,
            }
        ]
        if name
        else []
    }


def parse_gog_installed(path: Path) -> dict:
    data = parse_json_file(path)
    raw_items: list[object] = []
    if isinstance(data, dict):
        installed = data.get("installed")
        games = data.get("games")
        if isinstance(installed, list):
            raw_items = installed
        elif isinstance(games, list):
            raw_items = games
        elif all(isinstance(value, dict) for value in data.values()):
            raw_items = list(data.values())
    elif isinstance(data, list):
        raw_items = data

    parsed_games: list[dict] = []
    for item in raw_items:
        if not isinstance(item, dict):
            continue
        name = item.get("title") or item.get("name") or item.get("gameName")
        if not name:
            continue
        parsed_games.append(
            {
                "name": str(name),
                "platform": "gog",
                "platform_game_id": item.get("gameId") or item.get("id") or item.get("productId"),
                "install_location": item.get("installPath") or item.get("install_path") or item.get("path"),
                "source": "gog_galaxy",
            }
        )
    parsed_games.sort(key=lambda game: str(game["name"]).lower())
    return {"games": parsed_games[:500]}


def parse_origin_localcontent_manifest(path: Path) -> dict:
    content = read_text(path).strip()
    if content.startswith("?"):
        content = content[1:]
    params = parse_qs(content, keep_blank_values=True)

    def _first(*names: str) -> str | None:
        for name in names:
            values = params.get(name)
            if values and str(values[0]).strip():
                return str(values[0]).strip()
        return None

    product_id = _first("id", "gameid", "contentid")
    install_location = _first("dipinstallpath", "installpath", "installlocation")
    name = _first("title", "gametitle", "displayname", "name") or path.parent.name
    if not name:
        return {"games": []}

    return {
        "games": [
            {
                "name": name,
                "platform": "ea",
                "platform_game_id": product_id,
                "install_location": install_location,
                "source": "origin_localcontent",
            }
        ]
    }


def parse_battle_net_product_db(path: Path) -> dict:
    try:
        payload = path.read_bytes()
    except OSError:
        return {"games": []}

    product_names = {
        "wow": "World of Warcraft",
        "d3": "Diablo III",
        "s2": "StarCraft II",
        "s1": "StarCraft",
        "wtcg": "Hearthstone",
        "hero": "Heroes of the Storm",
        "pro": "Overwatch 2",
        "fen": "Diablo IV",
        "osi": "Diablo II: Resurrected",
        "w3": "Warcraft III: Reforged",
        "auks": "Call of Duty: Modern Warfare II",
        "pnta": "Call of Duty: Modern Warfare III",
    }

    parsed_games: list[dict] = []
    for install in _protobuf_messages(payload, field_number=1):
        fields = _protobuf_fields(install)
        uid = _first_string(fields, 1)
        product_code = _first_string(fields, 2)
        settings = _first_message(fields, 3)
        install_location = _first_string(settings, 1) if settings else None
        code = (product_code or uid or "").strip()
        if not code or not install_location:
            continue
        normalized_code = code.lower()
        name = product_names.get(normalized_code) or Path(str(install_location)).name
        if not name:
            continue
        parsed_games.append(
            {
                "name": name,
                "platform": "battle_net",
                "platform_game_id": uid or product_code,
                "install_location": install_location,
                "source": "battle_net_product_db",
                "product_code": product_code,
            }
        )

    deduped: list[dict] = []
    seen: set[tuple[str, str]] = set()
    for game in parsed_games:
        key = (
            str(game.get("platform_game_id") or "").lower(),
            str(game.get("install_location") or "").lower(),
        )
        if key in seen:
            continue
        seen.add(key)
        deduped.append(game)
    deduped.sort(key=lambda game: str(game["name"]).lower())
    return {"games": deduped[:500]}


def parse_amazon_games_install_info(path: Path) -> dict:
    try:
        with sqlite3.connect(path) as connection:
            table_names = {
                str(row[0])
                for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
            }
            table_name = "DbSet" if "DbSet" in table_names else None
            if table_name is None:
                return {"games": []}
            columns = {
                str(row[1])
                for row in connection.execute(f'PRAGMA table_info("{table_name}")').fetchall()
            }
            id_expr = _first_existing_column(columns, ["Id", "ProductId", "GameId"])
            name_expr = _first_existing_column(columns, ["ProductTitle", "Title", "Name"])
            install_expr = _first_existing_column(columns, ["InstallDirectory", "InstallPath", "InstallLocation"])
            installed_expr = _first_existing_column(columns, ["Installed", "IsInstalled"])
            if name_expr is None:
                return {"games": []}

            select_columns = [
                f'COALESCE("{id_expr}", "")' if id_expr else "''",
                f'COALESCE("{name_expr}", "")',
                f'COALESCE("{install_expr}", "")' if install_expr else "''",
                f'COALESCE("{installed_expr}", 1)' if installed_expr else "1",
            ]
            cursor = connection.execute(
                f"""
                SELECT {", ".join(select_columns)}
                FROM "{table_name}"
                LIMIT 500
                """
            )
            rows = cursor.fetchall()
    except sqlite3.DatabaseError:
        return {"games": []}

    games: list[dict] = []
    for product_id, name, install_location, installed in rows:
        if not _sqlite_truthy(installed):
            continue
        game_name = str(name or "").strip()
        if not game_name:
            continue
        games.append(
            {
                "name": game_name,
                "platform": "amazon_games",
                "platform_game_id": str(product_id or "").strip() or None,
                "install_location": str(install_location or "").strip() or None,
                "source": "amazon_games_install_info",
            }
        )
    games.sort(key=lambda game: str(game["name"]).lower())
    return {"games": games[:500]}


def parse_xbox_game_config(path: Path) -> dict:
    try:
        root = ET.fromstring(read_text(path))
    except (ET.ParseError, OSError, UnicodeDecodeError):
        return {"games": []}

    identity = root.find("Identity")
    shell_visuals = root.find("ShellVisuals")
    title = None
    if shell_visuals is not None:
        title = (
            shell_visuals.attrib.get("DefaultDisplayName")
            or shell_visuals.attrib.get("DisplayName")
        )
    package_name = identity.attrib.get("Name") if identity is not None else None
    publisher = identity.attrib.get("Publisher") if identity is not None else None
    if not title:
        title = path.parent.parent.name if path.parent.parent.name else path.parent.name
    if not title:
        return {"games": []}

    install_location = str(path.parent)
    return {
        "games": [
            {
                "name": title,
                "platform": "xbox",
                "platform_game_id": package_name,
                "install_location": install_location,
                "publisher": publisher,
                "source": "xbox_game_config",
            }
        ]
    }


def parse_itch_butler_db(path: Path) -> dict:
    try:
        with sqlite3.connect(path) as connection:
            table_names = {
                str(row[0])
                for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
            }
            table_name = next((name for name in table_names if name.lower() == "caves"), None)
            if table_name is None:
                return {"games": []}
            columns = {
                str(row[1])
                for row in connection.execute(f'PRAGMA table_info("{table_name}")').fetchall()
            }
            id_expr = _first_existing_column(columns, ["game_id", "GameID", "gameId", "id", "ID"])
            name_expr = _first_existing_column(columns, ["game_title", "GameTitle", "title", "Title", "name", "Name"])
            install_expr = _first_existing_column(columns, ["install_folder", "InstallFolder", "path", "Path", "install_path", "InstallPath"])
            if name_expr is None:
                return {"games": []}

            select_columns = [
                f'COALESCE("{id_expr}", "")' if id_expr else "''",
                f'COALESCE("{name_expr}", "")',
                f'COALESCE("{install_expr}", "")' if install_expr else "''",
            ]
            rows = connection.execute(
                f"""
                SELECT {", ".join(select_columns)}
                FROM "{table_name}"
                LIMIT 500
                """
            ).fetchall()
    except sqlite3.DatabaseError:
        return {"games": []}

    games: list[dict] = []
    for game_id, name, install_location in rows:
        game_name = str(name or "").strip()
        if not game_name:
            continue
        games.append(
            {
                "name": game_name,
                "platform": "itch",
                "platform_game_id": str(game_id or "").strip() or None,
                "install_location": str(install_location or "").strip() or None,
                "source": "itch_butler_db",
            }
        )
    games.sort(key=lambda game: str(game["name"]).lower())
    return {"games": games[:500]}


def _playnite_platform(plugin_id: str) -> str:
    normalized = plugin_id.strip().lower()
    if "battle" in normalized:
        return "battle_net"
    if "gog" in normalized:
        return "gog"
    if "steam" in normalized:
        return "steam"
    if "epic" in normalized:
        return "epic"
    if "ubisoft" in normalized or "uplay" in normalized:
        return "ubisoft"
    if normalized in {"ea", "ealibrary", "origin", "originlibrary"} or "origin" in normalized:
        return "ea"
    if "xbox" in normalized or "microsoft" in normalized:
        return "xbox"
    if "itch" in normalized:
        return "itch"
    return "playnite"


def _first_existing_column(columns: set[str], candidates: list[str]) -> str | None:
    normalized = {column.lower(): column for column in columns}
    for candidate in candidates:
        existing = normalized.get(candidate.lower())
        if existing:
            return existing
    return None


def _sqlite_truthy(value: object) -> bool:
    if value is None:
        return False
    if isinstance(value, (int, float)):
        return value != 0
    normalized = str(value).strip().lower()
    return normalized not in {"", "0", "false", "no", "none", "null"}


def _protobuf_messages(payload: bytes, field_number: int) -> list[bytes]:
    values = _protobuf_fields(payload).get(field_number, [])
    return [value for value in values if isinstance(value, bytes)]


def _first_message(fields: dict[int, list[bytes]], field_number: int) -> dict[int, list[bytes]] | None:
    values = fields.get(field_number, [])
    for value in values:
        if isinstance(value, bytes):
            return _protobuf_fields(value)
    return None


def _first_string(fields: dict[int, list[bytes]], field_number: int) -> str | None:
    values = fields.get(field_number, [])
    for value in values:
        if not isinstance(value, bytes):
            continue
        try:
            decoded = value.decode("utf-8").strip()
        except UnicodeDecodeError:
            continue
        if decoded:
            return decoded
    return None


def _protobuf_fields(payload: bytes) -> dict[int, list[bytes]]:
    index = 0
    fields: dict[int, list[bytes]] = {}
    while index < len(payload):
        key, index = _protobuf_varint(payload, index)
        field_number = key >> 3
        wire_type = key & 0x07
        if field_number <= 0:
            break
        if wire_type == 2:
            length, index = _protobuf_varint(payload, index)
            end = index + length
            if end > len(payload):
                break
            value = payload[index:end]
            fields.setdefault(field_number, []).append(value)
            index = end
            continue
        if wire_type == 0:
            _value, index = _protobuf_varint(payload, index)
            continue
        if wire_type == 1:
            index += 8
            continue
        if wire_type == 5:
            index += 4
            continue
        break
    return fields


def _protobuf_varint(payload: bytes, index: int) -> tuple[int, int]:
    shift = 0
    value = 0
    while index < len(payload):
        byte = payload[index]
        index += 1
        value |= (byte & 0x7F) << shift
        if not (byte & 0x80):
            return value, index
        shift += 7
        if shift > 63:
            break
    return value, index


def parse_activitywatch_buckets(base_url: str) -> dict:
    normalized = base_url.rstrip("/")
    with urlopen(f"{normalized}/api/0/buckets/", timeout=2) as response:
        payload = json.loads(response.read().decode("utf-8"))
    if not isinstance(payload, dict):
        return {"bucket_count": 0, "watchers": []}

    watchers: set[str] = set()
    for bucket_id, bucket in payload.items():
        if not isinstance(bucket_id, str):
            continue
        bucket_type = bucket.get("type") if isinstance(bucket, dict) else None
        text = f"{bucket_id} {bucket_type or ''}".lower()
        if "watcher-web" in text or "web.tab" in text:
            watchers.add("web")
        elif "watcher-window" in text or "currentwindow" in text:
            watchers.add("window")
        elif "watcher-afk" in text or "afkstatus" in text:
            watchers.add("afk")
    return {
        "bucket_count": len(payload),
        "watchers": sorted(watchers),
    }


def parse_firefox_places(path: Path) -> dict:
    bookmark_titles: list[str] = []
    bookmark_urls: list[str] = []
    history_urls: list[str] = []
    history_titles: list[str] = []
    history_domain_counts: Counter[str] = Counter()
    history_rows_for_search: list[tuple[str, str, int]] = []

    with sqlite3.connect(path) as connection:
        bookmark_rows = connection.execute(
            """
            SELECT p.url, COALESCE(b.title, p.title, '')
            FROM moz_bookmarks b
            JOIN moz_places p ON p.id = b.fk
            WHERE b.fk IS NOT NULL
            LIMIT 100
            """
        ).fetchall()
        for url, title in bookmark_rows:
            if url:
                bookmark_urls.append(str(url))
            if title:
                bookmark_titles.append(str(title))

        history_rows = connection.execute(
            """
            SELECT url, COALESCE(title, ''), COALESCE(visit_count, 0)
            FROM moz_places
            WHERE url IS NOT NULL
            ORDER BY COALESCE(visit_count, 0) DESC, id DESC
            LIMIT 150
            """
        ).fetchall()
        for url, title, visit_count in history_rows:
            if url:
                history_urls.append(str(url))
                history_rows_for_search.append((str(url), str(title or ""), int(visit_count or 0)))
                for domain in _normalize_domains([str(url)]):
                    history_domain_counts[domain] += int(visit_count or 0)
            if title:
                history_titles.append(str(title))

    return {
        "browser": _infer_firefox_like_browser(path),
        "bookmark_titles": bookmark_titles[:50],
        "bookmark_urls": bookmark_urls[:50],
        "bookmark_domains": _normalize_domains(bookmark_urls),
        "recent_urls": history_urls[:50],
        "recent_titles": history_titles[:50],
        "recent_domains": _normalize_domains(history_urls),
        "top_history_domains": [
            {"domain": domain, "visit_count": count}
            for domain, count in history_domain_counts.most_common(10)
        ],
        "top_search_queries": _extract_search_queries(history_rows_for_search),
    }


def parse_sqlite_database(path: Path) -> dict:
    try:
        with sqlite3.connect(path) as connection:
            tables = [
                row[0]
                for row in connection.execute(
                    "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
                ).fetchall()
            ]

            row_counts: dict[str, int] = {}
            for table in tables[:20]:
                try:
                    count = connection.execute(f'SELECT COUNT(*) FROM "{table}"').fetchone()
                    row_counts[table] = int(count[0]) if count else 0
                except sqlite3.DatabaseError:
                    row_counts[table] = -1

        return {
            "valid": True,
            "tables": tables,
            "row_counts": row_counts,
            "error": None,
        }
    except sqlite3.DatabaseError as error:
        return {
            "valid": False,
            "tables": [],
            "row_counts": {},
            "error": str(error),
        }


def parse_vdf_paths(path: Path) -> list[str]:
    content = read_text(path)
    return re.findall(r'"path"\s+"([^"]+)"', content)


def parse_acf_manifest(path: Path) -> dict:
    content = read_text(path)

    def _match(name: str) -> str | None:
        matched = re.search(rf'"{re.escape(name)}"\s+"([^"]+)"', content)
        return matched.group(1) if matched else None

    return {
        "app_id": _match("appid"),
        "name": _match("name"),
        "install_dir_name": _match("installdir"),
        "size_on_disk": _match("SizeOnDisk"),
        "state_flags": _match("StateFlags"),
    }


def parse_epic_manifest(path: Path) -> dict:
    data = parse_json_file(path)
    return {
        "catalog_item_id": data.get("CatalogItemId"),
        "name": data.get("DisplayName") or data.get("AppName"),
        "app_name": data.get("AppName"),
        "install_location": data.get("InstallLocation"),
        "launch_executable": data.get("LaunchExecutable"),
        "install_size": data.get("InstallSize"),
        "technical_type": data.get("TechnicalType"),
    }


def _normalize_domains(urls: list[str]) -> list[str]:
    ignored_domains = {"error", "settings", "url", "open"}
    domains: list[str] = []
    for url in urls:
        parsed = urlparse(url)
        domain = parsed.netloc.strip().lower()
        if not domain and parsed.scheme in {"chrome", "edge", "about"}:
            domain = parsed.path.strip().lower()
        if not domain:
            continue
        if domain.startswith("www."):
            domain = domain[4:]
        if domain in ignored_domains:
            continue
        domains.append(domain)
    return sorted(set(domains))


def _sorted_string_list(value: object) -> list[str]:
    if not isinstance(value, list):
        return []
    return sorted({str(item).strip() for item in value if str(item).strip()})


def _extension_locale_value(value: object) -> str:
    text = str(value or "").strip()
    if not text or text.startswith("__MSG_"):
        return ""
    return text


def _workspace_uri_to_path(value: str) -> str | None:
    if value.startswith("file:///"):
        normalized = unquote(value.removeprefix("file:///")).replace("/", "\\")
        if len(normalized) >= 2 and normalized[1] == ":":
            normalized = normalized[0].upper() + normalized[1:]
        return normalized
    if _looks_like_workspace_path(value):
        return value
    return None


def _looks_like_workspace_path(value: str) -> bool:
    normalized = value.lower()
    return ":\\" in normalized or normalized.startswith("\\\\")


def _decode_sqlite_value(value: object) -> str:
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return str(value)


def _optional_int(value: object) -> int | None:
    if value in {None, ""}:
        return None
    try:
        return int(str(value).strip())
    except (TypeError, ValueError):
        return None


def _read_optional_json_file(path: Path) -> object | None:
    if not path.exists() or not path.is_file():
        return None
    return parse_json_file(path)


def _read_json_string_list(path: Path) -> list[str]:
    data = _read_optional_json_file(path)
    if not isinstance(data, list):
        return []
    return sorted({str(item).strip() for item in data if str(item).strip()})


def _parse_simple_yaml_mapping(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for raw_line in read_text(path).splitlines():
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#") or ":" not in stripped:
            continue
        key, value = stripped.split(":", 1)
        key_name = key.strip()
        if not key_name or key_name.startswith("-"):
            continue
        values[key_name] = value.strip().strip("'\"")
    return values


def _parse_key_value_lines(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for raw_line in _read_text_with_fallbacks(path).splitlines():
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#") or "=" not in stripped:
            continue
        key, value = stripped.split("=", 1)
        key_name = key.strip()
        if not key_name:
            continue
        values[key_name] = value.strip().strip("'\"")
    return values


def _normalize_backslash_path(value: object) -> str | None:
    text = str(value or "").strip()
    if not text:
        return None
    return text.replace("\\\\", "\\")


def _normalize_output_path(value: object) -> str | None:
    normalized = _normalize_backslash_path(value)
    if normalized is None:
        return None
    return normalized.replace("\\", "/")


def _parse_simple_yaml_document(path: Path) -> dict[str, object]:
    data: dict[str, object] = {}
    current_collection_key: str | None = None
    current_collection_type: str | None = None
    current_list: list[str] = []
    current_map: dict[str, dict[str, str]] = {}
    current_nested_key: str | None = None

    def _flush_collection() -> None:
        nonlocal current_collection_key, current_collection_type, current_list, current_map, current_nested_key
        if current_collection_key is not None:
            if current_collection_type == "list":
                data[current_collection_key] = list(current_list)
            elif current_collection_type == "map":
                data[current_collection_key] = {key: dict(value) for key, value in current_map.items()}
            else:
                data[current_collection_key] = []
        current_collection_key = None
        current_collection_type = None
        current_list = []
        current_map = {}
        current_nested_key = None

    for raw_line in read_text(path).splitlines():
        indent = len(raw_line) - len(raw_line.lstrip(" "))
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("- "):
            if current_collection_key is not None:
                if current_collection_type is None:
                    current_collection_type = "list"
                if current_collection_type == "list":
                    current_list.append(stripped[2:].strip().strip("'\""))
            continue
        if current_collection_key is not None and indent >= 4 and ":" in stripped and current_nested_key is not None:
            if current_collection_type is None:
                current_collection_type = "map"
            if current_collection_type == "map":
                key, value = stripped.split(":", 1)
                key_name = key.strip()
                value_text = value.strip().strip("'\"")
                if key_name:
                    current_map[current_nested_key][key_name] = value_text
            continue
        if current_collection_key is not None and indent >= 2 and stripped.endswith(":") and ":" in stripped:
            if current_collection_type is None:
                current_collection_type = "map"
            if current_collection_type == "map":
                nested_key = stripped[:-1].strip()
                if nested_key:
                    current_nested_key = nested_key
                    current_map.setdefault(current_nested_key, {})
            continue
        if indent == 0:
            _flush_collection()
        elif current_collection_key is not None:
            continue
        if ":" not in stripped:
            continue
        key, value = stripped.split(":", 1)
        key_name = key.strip()
        value_text = value.strip().strip("'\"")
        if not key_name:
            continue
        if value_text == "":
            current_collection_key = key_name
            current_collection_type = None
            current_list = []
            current_map = {}
            current_nested_key = None
            continue
        data[key_name] = value_text

    _flush_collection()
    return data


def _config_value(parser: configparser.ConfigParser, section: str, option: str) -> str | None:
    value = parser.get(section, option, fallback="").strip()
    return value or None


def _config_bool(parser: configparser.ConfigParser, section: str, option: str) -> bool | None:
    value = _config_value(parser, section, option)
    if value is None:
        return None
    normalized = value.lower()
    if normalized in {"true", "1", "yes", "on"}:
        return True
    if normalized in {"false", "0", "no", "off"}:
        return False
    return None


def _config_int(parser: configparser.ConfigParser, section: str, option: str) -> int | None:
    value = _config_value(parser, section, option)
    return _optional_int(value)


def _config_multiline_values(parser: configparser.ConfigParser, section: str, option: str) -> list[str]:
    value = _config_value(parser, section, option)
    if value is None:
        return []
    return [line.strip() for line in value.splitlines() if line.strip()]


def _optional_bool(value: object) -> bool | None:
    if value in {None, ""}:
        return None
    normalized = str(value).strip().lower()
    if normalized in {"true", "1", "yes", "on"}:
        return True
    if normalized in {"false", "0", "no", "off"}:
        return False
    return None


def _format_resolution(width: object, height: object) -> str | None:
    width_text = str(width or "").strip()
    height_text = str(height or "").strip()
    if not width_text or not height_text:
        return None
    return f"{width_text}x{height_text}"


def _string_list_value(value: object) -> list[str]:
    if isinstance(value, list):
        return [str(item).strip() for item in value if str(item).strip()]
    text = str(value or "").strip()
    return [text] if text else []


def _first_non_empty_string(*values: object) -> str | None:
    for value in values:
        text = str(value or "").strip()
        if text:
            return text
    return None


def _first_present_value(*values: object) -> object | None:
    for value in values:
        if value is not None:
            return value
    return None


def _has_meaningful_value(value: object) -> bool:
    if value is None:
        return False
    if isinstance(value, str):
        return bool(value.strip())
    if isinstance(value, (list, dict)):
        return bool(value)
    return True


def _infer_chromium_browser(path: Path) -> str:
    normalized = str(path).lower()
    if "\\thebrowsercompany.arc_" in normalized or "\\arc\\user data\\" in normalized:
        return "arc"
    if "\\microsoft\\edge\\" in normalized:
        return "edge"
    if "\\opera software\\opera gx stable\\" in normalized:
        return "opera_gx"
    if "\\opera software\\opera stable\\" in normalized:
        return "opera"
    if "\\vivaldi\\" in normalized and "\\user data\\" in normalized:
        return "vivaldi"
    if "\\bravesoftware\\brave-browser\\" in normalized:
        return "brave"
    return "chrome"


def _infer_firefox_like_browser(path: Path) -> str:
    normalized = str(path).lower()
    if "\\mozilla\\firefox\\profiles\\" in normalized:
        return "firefox"
    if "\\zen\\profiles\\" in normalized:
        return "zen"
    return "firefox"


def _infer_teams_client_variant(path: Path) -> str:
    normalized = str(path).lower()
    if "\\packages\\msteams_8wekyb3d8bbwe\\" in normalized:
        return "new"
    return "classic"


def _infer_onedrive_account_type(account_slot: object, library_type: object) -> str | None:
    normalized_slot = str(account_slot or "").strip().lower()
    normalized_library_type = str(library_type or "").strip().lower()
    if normalized_slot.startswith("personal") or normalized_library_type == "personal":
        return "personal"
    if normalized_slot.startswith("business") or normalized_library_type == "business":
        return "business"
    return None


def _read_text_with_fallbacks(
    path: Path,
    encodings: tuple[str, ...] = ("utf-8", "utf-8-sig", "utf-16", "utf-16-le", "utf-16-be"),
) -> str:
    last_error: Exception | None = None
    for encoding in encodings:
        try:
            return path.read_text(encoding=encoding)
        except (OSError, UnicodeDecodeError) as exc:
            last_error = exc
            continue
    if last_error is not None:
        raise last_error
    return path.read_text(encoding="utf-8")


def _element_text(element: ET.Element | None) -> str | None:
    if element is None or element.text is None:
        return None
    text = element.text.strip()
    return text or None


def _extract_search_queries(rows: list[tuple[str, str, int]]) -> list[dict]:
    query_counts: dict[str, int] = {}
    query_engines: dict[str, set[str]] = {}

    for url, _title, visit_count in rows:
        parsed = _parse_search_query(url)
        if parsed is None:
            continue
        engine, query = parsed
        normalized_query = query.strip().lower()
        if not normalized_query:
            continue
        query_counts[normalized_query] = query_counts.get(normalized_query, 0) + int(visit_count or 0)
        engines = query_engines.get(normalized_query)
        if engines is None:
            engines = set()
            query_engines[normalized_query] = engines
        engines.add(engine)

    return [
        {
            "query": query,
            "visit_count": visit_count,
            "engines": sorted(query_engines.get(query, set())),
        }
        for query, visit_count in sorted(query_counts.items(), key=lambda item: (-item[1], item[0]))[:10]
    ]


def _parse_search_query(url: str) -> tuple[str, str] | None:
    parsed = urlparse(url)
    host = parsed.netloc.strip().lower()
    if host.startswith("www."):
        host = host[4:]

    engine: str | None = None
    query_key: str | None = None

    if host.endswith("google.com") and parsed.path.startswith("/search"):
        engine = "google"
        query_key = "q"
    elif host.endswith("bing.com") and parsed.path.startswith("/search"):
        engine = "bing"
        query_key = "q"
    elif host == "duckduckgo.com":
        engine = "duckduckgo"
        query_key = "q"
    elif host == "search.brave.com" and parsed.path.startswith("/search"):
        engine = "brave_search"
        query_key = "q"
    elif host.endswith("yahoo.com") and parsed.path.startswith("/search"):
        engine = "yahoo"
        query_key = "p"
    elif host.endswith("baidu.com") and parsed.path.startswith("/s"):
        engine = "baidu"
        query_key = "wd"

    if not engine or not query_key:
        return None

    params = parse_qs(parsed.query)
    raw_query = params.get(query_key, [])
    if not raw_query:
        return None
    query = str(raw_query[0]).replace("+", " ").strip()
    if not query:
        return None
    return engine, query
