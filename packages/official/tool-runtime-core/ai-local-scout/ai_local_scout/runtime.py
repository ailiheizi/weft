from __future__ import annotations

from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
import json
import re
import socket
import subprocess
from urllib.error import URLError
from urllib.request import Request, urlopen

from .discovery import iter_bootstrap_candidates
from .inference import (
    derive_agent_config_profile,
    derive_creative_tools_profile,
    derive_downloads_profile,
    derive_hardware_profile,
    derive_llm_context,
    derive_local_signal_coverage,
    derive_meaningful_activity_profile,
    derive_next_questions,
    derive_office_documents_profile,
    derive_privacy_security_profile,
    derive_recent_documents_profile,
    derive_steam_playtime_profile,
)
from .parsers import (
    parse_acf_manifest,
    parse_activitywatch_buckets,
    parse_amazon_games_install_info,
    parse_aws_cli_config,
    parse_battle_net_product_db,
    parse_browser_bookmarks,
    parse_browser_downloads,
    parse_browser_extension_manifest,
    parse_browser_history,
    parse_browser_session,
    parse_cargo_credentials_store,
    parse_cargo_user_config,
    parse_claude_entrypoint,
    parse_claude_mcp_config,
    parse_claude_settings,
    parse_codex_auth,
    parse_codex_config,
    parse_codex_rules,
    parse_cursor_state_db,
    parse_discord_settings,
    parse_dropbox_info,
    parse_dotnet_global_tools,
    parse_docker_cli_config,
    parse_docker_context_meta,
    parse_docker_desktop_settings,
    parse_editor_recent_workspaces,
    parse_epic_manifest,
    parse_azure_cli_profile,
    parse_firefox_extensions,
    parse_firefox_downloads,
    parse_firefox_places,
    parse_gradle_user_properties,
    parse_git_config,
    parse_git_global_config,
    parse_gcloud_active_config,
    parse_gcloud_cli_config,
    parse_github_cli_config,
    parse_github_cli_hosts,
    parse_gog_installed,
    parse_installed_apps,
    parse_itch_butler_db,
    parse_joplin_profile,
    parse_kubeconfig,
    parse_legendary_installed,
    parse_markdown_summary,
    parse_maven_user_settings,
    parse_nextcloud_config,
    parse_nuget_user_config,
    parse_npm_user_config,
    parse_pnpm_user_config,
    parse_obs_studio_profile,
    parse_obs_studio_scene_collection,
    parse_obsidian_global_config,
    parse_obsidian_vault,
    parse_onedrive_global_config,
    parse_origin_localcontent_manifest,
    parse_office_document,
    parse_pip_user_config,
    parse_playnite_game,
    parse_poetry_user_config,
    parse_jetbrains_recent_projects,
    parse_rustup_settings,
    parse_shell_history,
    parse_sqlite_database,
    parse_ssh_config,
    parse_uv_credentials_store,
    parse_uv_user_config,
    parse_vdf_paths,
    parse_windows_terminal_settings,
    parse_wsl_distribution_list,
    parse_wsl_global_config,
    parse_workspace_file,
    parse_xbox_game_config,
    parse_conda_user_config,
    parse_syncthing_config,
    parse_yarn_user_config,
    parse_teams_config,
)


_MACHINE_PROBE_CACHE: dict[tuple[str, object], object] = {}


def _cached_machine_probe(cache_key: str, loader):
    key = (cache_key, subprocess.run)
    if key not in _MACHINE_PROBE_CACHE:
        _MACHINE_PROBE_CACHE[key] = loader()
    return _MACHINE_PROBE_CACHE[key]


@dataclass(slots=True)
class ScoutConfig:
    roots: list[Path]
    home: Path | None = None
    max_depth: int = 6
    max_ai_expansions: int = 64
    max_sqlite_parse: int = 32
    system_steam_roots: list[Path] = field(default_factory=list)
    system_epic_manifest_dirs: list[Path] = field(default_factory=list)
    system_legendary_installed_paths: list[Path] = field(default_factory=list)
    system_amazon_games_install_info_paths: list[Path] = field(default_factory=list)
    system_xbox_game_config_paths: list[Path] = field(default_factory=list)
    system_itch_butler_db_paths: list[Path] = field(default_factory=list)
    system_battle_net_product_db_paths: list[Path] = field(default_factory=list)
    system_origin_local_content_dirs: list[Path] = field(default_factory=list)
    system_activitywatch_base_urls: list[str] = field(default_factory=list)
    system_obsidian_config_paths: list[Path] = field(default_factory=list)
    system_obs_studio_basic_dirs: list[Path] = field(default_factory=list)
    system_docker_desktop_settings_paths: list[Path] = field(default_factory=list)
    system_wslconfig_paths: list[Path] = field(default_factory=list)
    system_discord_settings_paths: list[Path] = field(default_factory=list)
    system_teams_config_paths: list[Path] = field(default_factory=list)
    system_dropbox_info_paths: list[Path] = field(default_factory=list)
    system_onedrive_settings_roots: list[Path] = field(default_factory=list)
    system_joplin_profile_paths: list[Path] = field(default_factory=list)
    system_nextcloud_config_paths: list[Path] = field(default_factory=list)
    system_syncthing_config_paths: list[Path] = field(default_factory=list)
    system_jetbrains_recent_projects_paths: list[Path] = field(default_factory=list)
    system_windows_terminal_settings_paths: list[Path] = field(default_factory=list)
    system_ssh_config_paths: list[Path] = field(default_factory=list)
    system_kubeconfig_paths: list[Path] = field(default_factory=list)
    system_docker_config_paths: list[Path] = field(default_factory=list)
    system_docker_context_meta_paths: list[Path] = field(default_factory=list)
    system_aws_config_paths: list[Path] = field(default_factory=list)
    system_azure_profile_paths: list[Path] = field(default_factory=list)
    system_gcloud_config_root_paths: list[Path] = field(default_factory=list)
    system_github_cli_config_root_paths: list[Path] = field(default_factory=list)
    system_gitconfig_paths: list[Path] = field(default_factory=list)
    system_cargo_config_paths: list[Path] = field(default_factory=list)
    system_cargo_credentials_paths: list[Path] = field(default_factory=list)
    system_maven_settings_paths: list[Path] = field(default_factory=list)
    system_gradle_properties_paths: list[Path] = field(default_factory=list)
    system_nuget_config_paths: list[Path] = field(default_factory=list)
    system_dotnet_tools_dirs: list[Path] = field(default_factory=list)
    system_npmrc_paths: list[Path] = field(default_factory=list)
    system_pnpm_config_paths: list[Path] = field(default_factory=list)
    system_pip_config_paths: list[Path] = field(default_factory=list)
    system_condarc_paths: list[Path] = field(default_factory=list)
    system_poetry_config_paths: list[Path] = field(default_factory=list)
    system_rustup_settings_paths: list[Path] = field(default_factory=list)
    system_uv_config_paths: list[Path] = field(default_factory=list)
    system_uv_credentials_paths: list[Path] = field(default_factory=list)
    system_yarnrc_yml_paths: list[Path] = field(default_factory=list)
    enable_steam_public_profile: bool = False
    run_id: str | None = None
    output_path: Path | None = None
    timestamp: str = field(default_factory=lambda: datetime.now(UTC).isoformat())


def run_scout(config: ScoutConfig) -> dict:
    effective_roots = build_effective_roots(config)
    bootstrap_evidence = _bootstrap(config)
    bootstrap_evidence.extend(_system_probe_entries(config, start_index=len(bootstrap_evidence) + 1))
    raw_evidence = list(bootstrap_evidence)
    search_trace: list[dict] = []
    ai_entries = _ai_expand(config, bootstrap_evidence, search_trace)
    raw_evidence.extend(ai_entries)
    redactions = _apply_redactions(raw_evidence)

    report = {
        "run_meta": {
            "run_id": config.run_id or f"scout-{config.timestamp}",
            "timestamp": config.timestamp,
            "roots": [str(path) for path in effective_roots],
            "home": str(config.home) if config.home else None,
            "mode": "read_only_ai_first",
        },
        "bootstrap_evidence": bootstrap_evidence,
        "search_trace": search_trace,
        "raw_evidence": raw_evidence,
        "entities": _build_entities(raw_evidence),
        "derived_profile": _derive_profile(raw_evidence),
        "confidence_summary": _build_confidence_summary(raw_evidence),
        "redactions": redactions,
        "open_questions": _build_open_questions(raw_evidence),
    }
    return report


def _system_probe_entries(config: ScoutConfig, start_index: int) -> list[dict]:
    entries: list[dict] = []
    for item in _probe_downloaded_files(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": item["path"],
                "source_kind": "filesystem",
                "entity_kind": "downloaded_file",
                "discovered_by": "bootstrap",
                "confidence": 0.86,
                "sensitivity": "medium",
                "fields": item,
            }
        )
    for item in _probe_windows_recent_documents(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": item["path"],
                "source_kind": "system_probe",
                "entity_kind": "recent_document",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "high",
                "fields": item,
            }
        )
    installed_apps = _probe_windows_installed_apps()
    if installed_apps:
        entries.append(
            {
                "id": f"bootstrap-{start_index}",
                "path": "windows-registry://installed-apps",
                "source_kind": "system_probe",
                "entity_kind": "installed_apps",
                "discovered_by": "bootstrap",
                "confidence": 0.9,
                "sensitivity": "low",
                "fields": {"apps": installed_apps},
            }
        )
    for path in _probe_steam_library_indexes(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "steam_library_index",
                "discovered_by": "bootstrap",
                "confidence": 0.9,
                "sensitivity": "low",
                "fields": {"library_paths": parse_vdf_paths(path)},
            }
        )
    steam_playtime = _probe_steam_public_playtime(config) if config.enable_steam_public_profile else None
    if steam_playtime:
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": f"steam-public-profile://{steam_playtime['steam_id64']}/games",
                "source_kind": "public_web",
                "entity_kind": "steam_public_playtime",
                "discovered_by": "bootstrap",
                "confidence": 0.78,
                "sensitivity": "medium",
                "fields": steam_playtime,
            }
        )
    for path in _probe_epic_manifests(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "epic_game_manifest",
                "discovered_by": "bootstrap",
                "confidence": 0.9,
                "sensitivity": "low",
                "fields": parse_epic_manifest(path),
            }
        )
    for path in _probe_legendary_installed(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "legendary_installed",
                "discovered_by": "bootstrap",
                "confidence": 0.9,
                "sensitivity": "low",
                "fields": parse_legendary_installed(path),
            }
        )
    for entry in _probe_windows_launcher_installs():
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": entry["path"],
                "source_kind": "system_probe",
                "entity_kind": entry["entity_kind"],
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "low",
                "fields": {"games": entry["games"]},
            }
        )
    for path in _probe_battle_net_product_dbs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "battle_net_product_db",
                "discovered_by": "bootstrap",
                "confidence": 0.9,
                "sensitivity": "low",
                "fields": parse_battle_net_product_db(path),
            }
        )
    for path in _probe_amazon_games_install_info(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "amazon_games_install_info",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "low",
                "fields": parse_amazon_games_install_info(path),
            }
        )
    for path in _probe_xbox_game_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "xbox_game_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "low",
                "fields": parse_xbox_game_config(path),
            }
        )
    for path in _probe_itch_butler_dbs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "itch_butler_db",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "low",
                "fields": parse_itch_butler_db(path),
            }
        )
    for path in _probe_origin_localcontent_manifests(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "origin_localcontent_manifest",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "low",
                "fields": parse_origin_localcontent_manifest(path),
            }
        )
    for base_url in _probe_activitywatch_base_urls(config):
        try:
            fields = parse_activitywatch_buckets(base_url)
        except (URLError, TimeoutError, OSError, json.JSONDecodeError):
            continue
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": base_url,
                "source_kind": "system_probe",
                "entity_kind": "activitywatch_runtime",
                "discovered_by": "bootstrap",
                "confidence": 0.85,
                "sensitivity": "low",
                "fields": fields,
            }
        )
    for path in _probe_obsidian_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "obsidian_global_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_obsidian_global_config(path),
            }
        )
    for path in _probe_obs_studio_profile_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "obs_studio_profile",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_obs_studio_profile(path),
            }
        )
    for path in _probe_obs_studio_scene_collections(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "obs_studio_scene_collection",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_obs_studio_scene_collection(path),
            }
        )
    for path in _probe_docker_desktop_settings(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "docker_desktop_settings",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_docker_desktop_settings(path),
            }
        )
    for path in _probe_wslconfigs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "wsl_global_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_wsl_global_config(path),
            }
        )
    wsl_distribution_fields = _probe_wsl_distribution_list()
    if wsl_distribution_fields.get("distros"):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": "wsl.exe --list --verbose",
                "source_kind": "system_probe",
                "entity_kind": "wsl_distribution_list",
                "discovered_by": "bootstrap",
                "confidence": 0.86,
                "sensitivity": "low",
                "fields": wsl_distribution_fields,
            }
        )
    for path in _probe_discord_settings(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "discord_settings",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_discord_settings(path),
            }
        )
    for path in _probe_teams_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "teams_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_teams_config(path),
            }
        )
    for path in _probe_dropbox_info_paths(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "dropbox_info",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_dropbox_info(path),
            }
        )
    for path in _probe_onedrive_global_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "onedrive_global_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_onedrive_global_config(path),
            }
        )
    for path in _probe_joplin_profiles(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "joplin_profile",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_joplin_profile(path),
            }
        )
    for path in _probe_nextcloud_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "nextcloud_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_nextcloud_config(path),
            }
        )
    for path in _probe_syncthing_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "syncthing_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_syncthing_config(path),
            }
        )
    for path in _probe_jetbrains_recent_projects(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "jetbrains_recent_projects",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_jetbrains_recent_projects(path),
            }
        )
    for path in _probe_windows_terminal_settings(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "windows_terminal_settings",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_windows_terminal_settings(path),
            }
        )
    for path in _probe_ssh_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "ssh_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_ssh_config(path),
            }
        )
    for path in _probe_kubeconfigs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "kubeconfig",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_kubeconfig(path),
            }
        )
    for path in _probe_docker_cli_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "docker_cli_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_docker_cli_config(path),
            }
        )
    for path in _probe_docker_context_meta_paths(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "docker_context_meta",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_docker_context_meta(path),
            }
        )
    for path in _probe_aws_cli_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "aws_cli_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_aws_cli_config(path),
            }
        )
    for path in _probe_azure_cli_profiles(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "azure_cli_profile",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_azure_cli_profile(path),
            }
        )
    for path in _probe_gcloud_active_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "gcloud_active_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_gcloud_active_config(path),
            }
        )
    for path in _probe_gcloud_cli_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "gcloud_cli_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_gcloud_cli_config(path),
            }
        )
    for path in _probe_github_cli_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "github_cli_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_github_cli_config(path),
            }
        )
    for path in _probe_github_cli_hosts(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "github_cli_hosts",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_github_cli_hosts(path),
            }
        )
    for path in _probe_git_global_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "git_global_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_git_global_config(path),
            }
        )
    for path in _probe_cargo_user_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "cargo_user_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_cargo_user_config(path),
            }
        )
    for path in _probe_cargo_credentials_stores(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "cargo_credentials_store",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "high",
                "fields": parse_cargo_credentials_store(path),
            }
        )
    for path in _probe_maven_user_settings(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "maven_user_settings",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "high",
                "fields": parse_maven_user_settings(path),
            }
        )
    for path in _probe_gradle_user_properties(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "gradle_user_properties",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_gradle_user_properties(path),
            }
        )
    for path in _probe_nuget_user_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "nuget_user_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "high",
                "fields": parse_nuget_user_config(path),
            }
        )
    for path in _probe_dotnet_global_tools_dirs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "dotnet_global_tools",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_dotnet_global_tools(path),
            }
        )
    for path in _probe_npm_user_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "npm_user_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_npm_user_config(path),
            }
        )
    for path in _probe_pnpm_user_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "pnpm_user_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_pnpm_user_config(path),
            }
        )
    for path in _probe_pip_user_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "pip_user_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_pip_user_config(path),
            }
        )
    for path in _probe_conda_user_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "conda_user_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_conda_user_config(path),
            }
        )
    for path in _probe_poetry_user_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "poetry_user_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_poetry_user_config(path),
            }
        )
    for path in _probe_rustup_settings(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "rustup_settings",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_rustup_settings(path),
            }
        )
    for path in _probe_uv_user_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "uv_user_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_uv_user_config(path),
            }
        )
    for path in _probe_uv_credentials_stores(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "uv_credentials_store",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "high",
                "fields": parse_uv_credentials_store(path),
            }
        )
    for path in _probe_yarn_user_configs(config):
        entries.append(
            {
                "id": f"bootstrap-{start_index + len(entries)}",
                "path": str(path),
                "source_kind": "system_probe",
                "entity_kind": "yarn_user_config",
                "discovered_by": "bootstrap",
                "confidence": 0.88,
                "sensitivity": "medium",
                "fields": parse_yarn_user_config(path),
            }
        )
    return entries


def _probe_steam_library_indexes(config: ScoutConfig) -> list[Path]:
    roots: list[Path] = [*config.system_steam_roots]
    if not config.system_steam_roots:
        roots.extend(_default_steam_roots(config))
    indexes: list[Path] = []
    seen: set[str] = set()
    for root in roots:
        candidates = [root / "steamapps" / "libraryfolders.vdf", root / "libraryfolders.vdf"]
        for candidate in candidates:
            if not candidate.exists() or not candidate.is_file():
                continue
            key = str(candidate.resolve()).lower()
            if key in seen:
                continue
            seen.add(key)
            indexes.append(candidate.resolve())
    return indexes


def _probe_steam_public_playtime(config: ScoutConfig) -> dict | None:
    steam_id64 = _find_recent_steam_id64(config)
    if not steam_id64:
        return None
    try:
        html = _fetch_steam_public_games_page(f"https://steamcommunity.com/profiles/{steam_id64}/games/?tab=all")
    except (OSError, URLError, TimeoutError):
        return {
            "steam_id64": steam_id64,
            "source": "steam_public_profile",
            "privacy_limited": True,
            "games": [],
        }
    games = _parse_steam_public_games_page(html)
    return {
        "steam_id64": steam_id64,
        "source": "steam_public_profile",
        "privacy_limited": not bool(games),
        "games": games,
    }


def _find_recent_steam_id64(config: ScoutConfig) -> str | None:
    login_paths = []
    roots = [*config.system_steam_roots] if config.system_steam_roots else _default_steam_roots(config)
    for root in roots:
        login_paths.append(root / "config" / "loginusers.vdf")
    for path in login_paths:
        if not path.exists() or not path.is_file():
            continue
        content = path.read_text(encoding="utf-8", errors="ignore")
        users = re.findall(r'"(\d{17})"\s*\{(.*?)\n\s*\}', content, flags=re.DOTALL)
        for steam_id64, block in users:
            if re.search(r'"MostRecent"\s+"1"', block):
                return steam_id64
        if users:
            return users[0][0]
    return None


def _fetch_steam_public_games_page(url: str) -> str:
    request = Request(url, headers={"User-Agent": "AI Local Scout/1.0"})
    with urlopen(request, timeout=15) as response:
        return response.read().decode("utf-8", errors="replace")


def _parse_steam_public_games_page(html: str) -> list[dict]:
    games = []
    pattern = re.compile(
        r'store\.steampowered\.com/app/(?P<app_id>\d+)/[^"<]*["\']?[^>]*>\s*(?P<name>[^<]+)</a>.*?(?P<hours>[\d,.]+)\s+hrs? on record',
        flags=re.IGNORECASE | re.DOTALL,
    )
    for match in pattern.finditer(html):
        games.append(
            {
                "app_id": match.group("app_id"),
                "name": re.sub(r"\s+", " ", match.group("name")).strip(),
                "playtime_forever_hours": float(match.group("hours").replace(",", "")),
            }
        )
    games.sort(key=lambda game: float(game["playtime_forever_hours"]), reverse=True)
    return games[:500]


def _default_steam_roots(config: ScoutConfig) -> list[Path]:
    roots: list[Path] = [
        Path(r"C:\Program Files (x86)\Steam"),
        Path(r"C:\Program Files\Steam"),
        Path(r"D:\SteamLibrary"),
        Path(r"E:\SteamLibrary"),
    ]
    for root in config.roots:
        roots.extend([root, root / "Steam", root / "SteamLibrary"])
    return roots


def _probe_epic_manifests(config: ScoutConfig) -> list[Path]:
    dirs: list[Path] = [*config.system_epic_manifest_dirs]
    if not config.system_epic_manifest_dirs:
        dirs.extend(
            [
                Path(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests"),
                Path(r"D:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests"),
            ]
        )
    manifests: list[Path] = []
    seen: set[str] = set()
    for directory in dirs:
        if not directory.exists() or not directory.is_dir():
            continue
        for manifest in sorted(directory.glob("*.item")):
            key = str(manifest.resolve()).lower()
            if key in seen:
                continue
            seen.add(key)
            manifests.append(manifest.resolve())
    return manifests


def _probe_legendary_installed(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_legendary_installed_paths]
    if not config.system_legendary_installed_paths:
        roaming = Path.home() / "AppData" / "Roaming"
        paths.extend(
            [
                roaming / "legendary" / "installed.json",
                roaming / "heroic" / "legendaryConfig" / "legendary" / "installed.json",
            ]
        )
    installed_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        installed_paths.append(path.resolve())
    return installed_paths


def _probe_activitywatch_base_urls(config: ScoutConfig) -> list[str]:
    if config.system_activitywatch_base_urls:
        return list(dict.fromkeys(config.system_activitywatch_base_urls))
    if _is_tcp_port_open("127.0.0.1", 5600):
        return ["http://127.0.0.1:5600"]
    return []


def _is_tcp_port_open(host: str, port: int, timeout: float = 0.05) -> bool:
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except OSError:
        return False


def _probe_obsidian_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_obsidian_config_paths]
    if not config.system_obsidian_config_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Roaming" / "Obsidian" / "obsidian.json")
        for root in config.roots:
            paths.append(root / "AppData" / "Roaming" / "Obsidian" / "obsidian.json")

    configs: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        configs.append(path.resolve())
    return configs


def _probe_obs_studio_basic_dirs(config: ScoutConfig) -> list[Path]:
    directories: list[Path] = [*config.system_obs_studio_basic_dirs]
    if not config.system_obs_studio_basic_dirs:
        if config.home:
            directories.append(config.home / "AppData" / "Roaming" / "obs-studio" / "basic")
        for root in config.roots:
            directories.append(root / "AppData" / "Roaming" / "obs-studio" / "basic")

    basic_dirs: list[Path] = []
    seen: set[str] = set()
    for directory in directories:
        if not directory.exists() or not directory.is_dir():
            continue
        key = str(directory.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        basic_dirs.append(directory.resolve())
    return basic_dirs


def _probe_obs_studio_profile_configs(config: ScoutConfig) -> list[Path]:
    configs: list[Path] = []
    seen: set[str] = set()
    for basic_dir in _probe_obs_studio_basic_dirs(config):
        profiles_dir = basic_dir / "profiles"
        if not profiles_dir.exists() or not profiles_dir.is_dir():
            continue
        for config_path in sorted(profiles_dir.glob("*/basic.ini")):
            key = str(config_path.resolve()).lower()
            if key in seen:
                continue
            seen.add(key)
            configs.append(config_path.resolve())
    return configs


def _probe_obs_studio_scene_collections(config: ScoutConfig) -> list[Path]:
    collections: list[Path] = []
    seen: set[str] = set()
    for basic_dir in _probe_obs_studio_basic_dirs(config):
        scenes_dir = basic_dir / "scenes"
        if not scenes_dir.exists() or not scenes_dir.is_dir():
            continue
        for scene_path in sorted(scenes_dir.glob("*.json")):
            key = str(scene_path.resolve()).lower()
            if key in seen:
                continue
            seen.add(key)
            collections.append(scene_path.resolve())
    return collections


def _probe_docker_desktop_settings(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_docker_desktop_settings_paths]
    if not config.system_docker_desktop_settings_paths:
        if config.home:
            paths.extend(
                [
                    config.home / "AppData" / "Roaming" / "Docker" / "settings-store.json",
                    config.home / "AppData" / "Roaming" / "Docker Desktop" / "settings-store.json",
                ]
            )
        for root in config.roots:
            paths.extend(
                [
                    root / "AppData" / "Roaming" / "Docker" / "settings-store.json",
                    root / "AppData" / "Roaming" / "Docker Desktop" / "settings-store.json",
                ]
            )

    settings_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        settings_paths.append(path.resolve())
    return settings_paths


def _probe_wslconfigs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_wslconfig_paths]
    if not config.system_wslconfig_paths:
        if config.home:
            paths.append(config.home / ".wslconfig")
        for root in config.roots:
            paths.append(root / ".wslconfig")

    configs: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        configs.append(path.resolve())
    return configs


def _probe_wsl_distribution_list() -> dict:
    return dict(_cached_machine_probe("wsl_distribution_list", _probe_wsl_distribution_list_uncached))


def _probe_wsl_distribution_list_uncached() -> dict:
    commands = [
        ["wsl.exe", "--list", "--verbose"],
        ["wsl", "--list", "--verbose"],
    ]
    for command in commands:
        try:
            completed = subprocess.run(
                command,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=10,
                check=False,
            )
        except (OSError, subprocess.SubprocessError):
            continue
        if completed.returncode != 0 or not completed.stdout.strip():
            continue
        fields = parse_wsl_distribution_list(completed.stdout)
        if fields.get("distros"):
            return fields
    return {"default_distro": None, "distros": []}


def _probe_discord_settings(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_discord_settings_paths]
    if not config.system_discord_settings_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Roaming" / "discord" / "settings.json")
        for root in config.roots:
            paths.append(root / "AppData" / "Roaming" / "discord" / "settings.json")

    settings_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        settings_paths.append(path.resolve())
    return settings_paths


def _probe_teams_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_teams_config_paths]
    if not config.system_teams_config_paths:
        default_suffixes = [
            Path("AppData") / "Roaming" / "Microsoft" / "Teams" / "desktop-config.json",
            Path("AppData") / "Local" / "Packages" / "MSTeams_8wekyb3d8bbwe" / "LocalCache" / "Microsoft" / "MSTeams" / "settings.json",
        ]
        if config.home:
            paths.extend([config.home / suffix for suffix in default_suffixes])
        for root in config.roots:
            paths.extend([root / suffix for suffix in default_suffixes])

    teams_config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        teams_config_paths.append(path.resolve())
    return teams_config_paths


def _probe_dropbox_info_paths(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_dropbox_info_paths]
    if not config.system_dropbox_info_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Roaming" / "Dropbox" / "info.json")
        for root in config.roots:
            paths.append(root / "AppData" / "Roaming" / "Dropbox" / "info.json")

    dropbox_info_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        dropbox_info_paths.append(path.resolve())
    return dropbox_info_paths


def _probe_onedrive_settings_roots(config: ScoutConfig) -> list[Path]:
    roots: list[Path] = [*config.system_onedrive_settings_roots]
    if not config.system_onedrive_settings_roots:
        if config.home:
            roots.append(config.home / "AppData" / "Local" / "Microsoft" / "OneDrive" / "settings")
        for root in config.roots:
            roots.append(root / "AppData" / "Local" / "Microsoft" / "OneDrive" / "settings")

    settings_roots: list[Path] = []
    seen: set[str] = set()
    for root in roots:
        if not root.exists() or not root.is_dir():
            continue
        key = str(root.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        settings_roots.append(root.resolve())
    return settings_roots


def _probe_onedrive_global_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = []
    seen: set[str] = set()
    for root in _probe_onedrive_settings_roots(config):
        for path in sorted(root.glob("*/global.ini")):
            if not path.is_file():
                continue
            key = str(path.resolve()).lower()
            if key in seen:
                continue
            seen.add(key)
            paths.append(path.resolve())
    return paths


def _probe_joplin_profiles(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_joplin_profile_paths]
    if not config.system_joplin_profile_paths:
        default_suffixes = [
            Path(".config") / "joplin-desktop",
            Path("AppData") / "Roaming" / "Joplin",
        ]
        if config.home:
            paths.extend([config.home / suffix for suffix in default_suffixes])
        for root in config.roots:
            paths.extend([root / suffix for suffix in default_suffixes])

    profile_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_dir():
            continue
        if not (path / "settings.json").exists():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        profile_paths.append(path.resolve())
    return profile_paths


def _probe_nextcloud_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_nextcloud_config_paths]
    if not config.system_nextcloud_config_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Roaming" / "Nextcloud" / "nextcloud.cfg")
        for root in config.roots:
            paths.append(root / "AppData" / "Roaming" / "Nextcloud" / "nextcloud.cfg")

    config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        config_paths.append(path.resolve())
    return config_paths


def _probe_syncthing_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_syncthing_config_paths]
    if not config.system_syncthing_config_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Local" / "Syncthing" / "config.xml")
        for root in config.roots:
            paths.append(root / "AppData" / "Local" / "Syncthing" / "config.xml")

    config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        config_paths.append(path.resolve())
    return config_paths


def _probe_jetbrains_recent_projects(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_jetbrains_recent_projects_paths]
    if not config.system_jetbrains_recent_projects_paths:
        roots: list[Path] = []
        if config.home:
            roots.append(config.home / "AppData" / "Roaming" / "JetBrains")
        for root in config.roots:
            roots.append(root / "AppData" / "Roaming" / "JetBrains")
        for jetbrains_root in roots:
            if not jetbrains_root.exists() or not jetbrains_root.is_dir():
                continue
            paths.extend(jetbrains_root.glob("*/options/recentProjects.xml"))

    recent_projects_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        recent_projects_paths.append(path.resolve())
    return recent_projects_paths


def _probe_windows_terminal_settings(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_windows_terminal_settings_paths]
    if not config.system_windows_terminal_settings_paths:
        default_suffixes = [
            Path("AppData") / "Local" / "Packages" / "Microsoft.WindowsTerminal_8wekyb3d8bbwe" / "LocalState" / "settings.json",
            Path("AppData") / "Local" / "Packages" / "Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe" / "LocalState" / "settings.json",
        ]
        if config.home:
            paths.extend([config.home / suffix for suffix in default_suffixes])
        for root in config.roots:
            paths.extend([root / suffix for suffix in default_suffixes])

    settings_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        settings_paths.append(path.resolve())
    return settings_paths


def _probe_ssh_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_ssh_config_paths]
    if not config.system_ssh_config_paths:
        if config.home:
            paths.append(config.home / ".ssh" / "config")
        for root in config.roots:
            paths.append(root / ".ssh" / "config")

    ssh_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        ssh_paths.append(path.resolve())
    return ssh_paths


def _probe_kubeconfigs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_kubeconfig_paths]
    if not config.system_kubeconfig_paths:
        if config.home:
            paths.append(config.home / ".kube" / "config")
        for root in config.roots:
            paths.append(root / ".kube" / "config")

    kubeconfig_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        kubeconfig_paths.append(path.resolve())
    return kubeconfig_paths


def _probe_docker_cli_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_docker_config_paths]
    if not config.system_docker_config_paths:
        if config.home:
            paths.append(config.home / ".docker" / "config.json")
        for root in config.roots:
            paths.append(root / ".docker" / "config.json")

    docker_config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        docker_config_paths.append(path.resolve())
    return docker_config_paths


def _probe_docker_context_meta_paths(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_docker_context_meta_paths]
    if not config.system_docker_context_meta_paths:
        docker_roots: list[Path] = []
        if config.home:
            docker_roots.append(config.home / ".docker" / "contexts" / "meta")
        for root in config.roots:
            docker_roots.append(root / ".docker" / "contexts" / "meta")
        for meta_root in docker_roots:
            if not meta_root.exists() or not meta_root.is_dir():
                continue
            paths.extend(meta_root.glob("*/meta.json"))

    meta_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        meta_paths.append(path.resolve())
    return meta_paths


def _probe_aws_cli_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_aws_config_paths]
    if not config.system_aws_config_paths:
        if config.home:
            paths.append(config.home / ".aws" / "config")
        for root in config.roots:
            paths.append(root / ".aws" / "config")

    aws_config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        aws_config_paths.append(path.resolve())
    return aws_config_paths


def _probe_azure_cli_profiles(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_azure_profile_paths]
    if not config.system_azure_profile_paths:
        if config.home:
            paths.append(config.home / ".azure" / "azureProfile.json")
        for root in config.roots:
            paths.append(root / ".azure" / "azureProfile.json")

    azure_profile_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        azure_profile_paths.append(path.resolve())
    return azure_profile_paths


def _probe_gcloud_roots(config: ScoutConfig) -> list[Path]:
    roots: list[Path] = [*config.system_gcloud_config_root_paths]
    if not config.system_gcloud_config_root_paths:
        if config.home:
            roots.append(config.home / "AppData" / "Roaming" / "gcloud")
        for root in config.roots:
            roots.append(root / "AppData" / "Roaming" / "gcloud")

    resolved_roots: list[Path] = []
    seen: set[str] = set()
    for root in roots:
        if not root.exists() or not root.is_dir():
            continue
        key = str(root.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        resolved_roots.append(root.resolve())
    return resolved_roots


def _probe_gcloud_active_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = []
    for root in _probe_gcloud_roots(config):
        active_path = root / "active_config"
        if active_path.exists() and active_path.is_file():
            paths.append(active_path.resolve())
    return paths


def _probe_gcloud_cli_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = []
    seen: set[str] = set()
    for root in _probe_gcloud_roots(config):
        config_dir = root / "configurations"
        if not config_dir.exists() or not config_dir.is_dir():
            continue
        for path in sorted(config_dir.glob("config_*")):
            if not path.is_file():
                continue
            key = str(path.resolve()).lower()
            if key in seen:
                continue
            seen.add(key)
            paths.append(path.resolve())
    return paths


def _probe_github_cli_roots(config: ScoutConfig) -> list[Path]:
    roots: list[Path] = [*config.system_github_cli_config_root_paths]
    if not config.system_github_cli_config_root_paths:
        if config.home:
            roots.append(config.home / "AppData" / "Roaming" / "GitHub CLI")
        for root in config.roots:
            roots.append(root / "AppData" / "Roaming" / "GitHub CLI")

    resolved_roots: list[Path] = []
    seen: set[str] = set()
    for root in roots:
        if not root.exists() or not root.is_dir():
            continue
        key = str(root.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        resolved_roots.append(root.resolve())
    return resolved_roots


def _probe_github_cli_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = []
    for root in _probe_github_cli_roots(config):
        config_path = root / "config.yml"
        if config_path.exists() and config_path.is_file():
            paths.append(config_path.resolve())
    return paths


def _probe_github_cli_hosts(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = []
    for root in _probe_github_cli_roots(config):
        hosts_path = root / "hosts.yml"
        if hosts_path.exists() and hosts_path.is_file():
            paths.append(hosts_path.resolve())
    return paths


def _probe_git_global_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_gitconfig_paths]
    if not config.system_gitconfig_paths:
        if config.home:
            paths.append(config.home / ".gitconfig")
        for root in config.roots:
            paths.append(root / ".gitconfig")

    gitconfig_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        gitconfig_paths.append(path.resolve())
    return gitconfig_paths


def _probe_cargo_user_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_cargo_config_paths]
    if not config.system_cargo_config_paths:
        if config.home:
            paths.extend(
                [
                    config.home / ".cargo" / "config.toml",
                    config.home / ".cargo" / "config",
                ]
            )
        for root in config.roots:
            paths.extend(
                [
                    root / ".cargo" / "config.toml",
                    root / ".cargo" / "config",
                ]
            )

    cargo_config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        cargo_config_paths.append(path.resolve())
    return cargo_config_paths


def _probe_cargo_credentials_stores(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_cargo_credentials_paths]
    if not config.system_cargo_credentials_paths:
        if config.home:
            paths.extend(
                [
                    config.home / ".cargo" / "credentials.toml",
                    config.home / ".cargo" / "credentials",
                ]
            )
        for root in config.roots:
            paths.extend(
                [
                    root / ".cargo" / "credentials.toml",
                    root / ".cargo" / "credentials",
                ]
            )

    cargo_credentials_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        cargo_credentials_paths.append(path.resolve())
    return cargo_credentials_paths


def _probe_maven_user_settings(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_maven_settings_paths]
    if not config.system_maven_settings_paths:
        if config.home:
            paths.append(config.home / ".m2" / "settings.xml")
        for root in config.roots:
            paths.append(root / ".m2" / "settings.xml")

    maven_settings_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        maven_settings_paths.append(path.resolve())
    return maven_settings_paths


def _probe_gradle_user_properties(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_gradle_properties_paths]
    if not config.system_gradle_properties_paths:
        if config.home:
            paths.append(config.home / ".gradle" / "gradle.properties")
        for root in config.roots:
            paths.append(root / ".gradle" / "gradle.properties")

    gradle_properties_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        gradle_properties_paths.append(path.resolve())
    return gradle_properties_paths


def _probe_nuget_user_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_nuget_config_paths]
    if not config.system_nuget_config_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Roaming" / "NuGet" / "NuGet.Config")
        for root in config.roots:
            paths.append(root / "AppData" / "Roaming" / "NuGet" / "NuGet.Config")

    nuget_config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        nuget_config_paths.append(path.resolve())
    return nuget_config_paths


def _probe_dotnet_global_tools_dirs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_dotnet_tools_dirs]
    if not config.system_dotnet_tools_dirs:
        if config.home:
            paths.append(config.home / ".dotnet" / "tools")
        for root in config.roots:
            paths.append(root / ".dotnet" / "tools")

    dotnet_tools_dirs: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_dir():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        dotnet_tools_dirs.append(path.resolve())
    return dotnet_tools_dirs


def _probe_npm_user_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_npmrc_paths]
    if not config.system_npmrc_paths:
        if config.home:
            paths.append(config.home / ".npmrc")
        for root in config.roots:
            paths.append(root / ".npmrc")

    npmrc_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        npmrc_paths.append(path.resolve())
    return npmrc_paths


def _probe_pip_user_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_pip_config_paths]
    if not config.system_pip_config_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Roaming" / "pip" / "pip.ini")
        for root in config.roots:
            paths.append(root / "AppData" / "Roaming" / "pip" / "pip.ini")

    pip_config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        pip_config_paths.append(path.resolve())
    return pip_config_paths


def _probe_pnpm_user_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_pnpm_config_paths]
    if not config.system_pnpm_config_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Local" / "pnpm" / "config" / "rc")
        for root in config.roots:
            paths.append(root / "AppData" / "Local" / "pnpm" / "config" / "rc")

    pnpm_config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        pnpm_config_paths.append(path.resolve())
    return pnpm_config_paths


def _probe_conda_user_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_condarc_paths]
    if not config.system_condarc_paths:
        if config.home:
            paths.append(config.home / ".condarc")
        for root in config.roots:
            paths.append(root / ".condarc")

    condarc_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        condarc_paths.append(path.resolve())
    return condarc_paths


def _probe_poetry_user_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_poetry_config_paths]
    if not config.system_poetry_config_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Roaming" / "pypoetry" / "config.toml")
        for root in config.roots:
            paths.append(root / "AppData" / "Roaming" / "pypoetry" / "config.toml")

    poetry_config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        poetry_config_paths.append(path.resolve())
    return poetry_config_paths


def _probe_rustup_settings(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_rustup_settings_paths]
    if not config.system_rustup_settings_paths:
        if config.home:
            paths.append(config.home / ".rustup" / "settings.toml")
        for root in config.roots:
            paths.append(root / ".rustup" / "settings.toml")

    rustup_settings_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        rustup_settings_paths.append(path.resolve())
    return rustup_settings_paths


def _probe_uv_user_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_uv_config_paths]
    if not config.system_uv_config_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Roaming" / "uv" / "uv.toml")
        for root in config.roots:
            paths.append(root / "AppData" / "Roaming" / "uv" / "uv.toml")

    uv_config_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        uv_config_paths.append(path.resolve())
    return uv_config_paths


def _probe_uv_credentials_stores(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_uv_credentials_paths]
    if not config.system_uv_credentials_paths:
        if config.home:
            paths.append(config.home / "AppData" / "Roaming" / "uv" / "data" / "credentials" / "credentials.toml")
        for root in config.roots:
            paths.append(root / "AppData" / "Roaming" / "uv" / "data" / "credentials" / "credentials.toml")

    uv_credentials_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        uv_credentials_paths.append(path.resolve())
    return uv_credentials_paths


def _probe_yarn_user_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_yarnrc_yml_paths]
    if not config.system_yarnrc_yml_paths:
        if config.home:
            paths.append(config.home / ".yarnrc.yml")
        for root in config.roots:
            paths.append(root / ".yarnrc.yml")

    yarnrc_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        yarnrc_paths.append(path.resolve())
    return yarnrc_paths


def _probe_origin_localcontent_manifests(config: ScoutConfig) -> list[Path]:
    directories: list[Path] = [*config.system_origin_local_content_dirs]
    if not config.system_origin_local_content_dirs:
        directories.extend(
            [
                Path(r"C:\ProgramData\Origin\LocalContent"),
                Path(r"C:\ProgramData\EA Desktop\LocalContent"),
            ]
        )
    manifests: list[Path] = []
    seen: set[str] = set()
    for directory in directories:
        if not directory.exists() or not directory.is_dir():
            continue
        for manifest in sorted(directory.glob("*/*.mfst")):
            key = str(manifest.resolve()).lower()
            if key in seen:
                continue
            seen.add(key)
            manifests.append(manifest.resolve())
    return manifests


def _probe_battle_net_product_dbs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_battle_net_product_db_paths]
    if not config.system_battle_net_product_db_paths:
        paths.extend(
            [
                Path(r"C:\ProgramData\Battle.net\Agent\product.db"),
                Path(r"D:\ProgramData\Battle.net\Agent\product.db"),
            ]
        )
    product_dbs: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        product_dbs.append(path.resolve())
    return product_dbs


def _probe_amazon_games_install_info(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_amazon_games_install_info_paths]
    if not config.system_amazon_games_install_info_paths:
        paths.append(Path.home() / "AppData" / "Local" / "Amazon Games" / "Data" / "Games" / "Sql" / "GameInstallInfo.sqlite")
    install_info_paths: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        install_info_paths.append(path.resolve())
    return install_info_paths


def _probe_xbox_game_configs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_xbox_game_config_paths]
    if not config.system_xbox_game_config_paths:
        for drive in ["C:", "D:", "E:"]:
            xbox_root = Path(f"{drive}\\XboxGames")
            if xbox_root.exists() and xbox_root.is_dir():
                paths.extend(xbox_root.glob("*\\Content\\MicrosoftGame.config"))
        if not paths and not config.roots and config.home is None:
            paths.extend(_probe_xbox_appx_install_locations())

    configs: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        candidate = path if path.name == "MicrosoftGame.config" else path / "MicrosoftGame.config"
        if not candidate.exists() or not candidate.is_file():
            continue
        key = str(candidate.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        configs.append(candidate.resolve())
    return configs


def _probe_xbox_appx_install_locations() -> list[Path]:
    return list(_cached_machine_probe("xbox_appx_install_locations", _probe_xbox_appx_install_locations_uncached))


def _probe_xbox_appx_install_locations_uncached() -> list[Path]:
    powershell = r"""
Get-AppxPackage |
  Where-Object { $_.InstallLocation -and (Test-Path (Join-Path $_.InstallLocation 'MicrosoftGame.config') -or Test-Path (Join-Path $_.InstallLocation 'Content\MicrosoftGame.config')) } |
  Select-Object Name, InstallLocation |
  ConvertTo-Json -Depth 3
"""
    try:
        completed = subprocess.run(
            ["powershell", "-NoProfile", "-Command", powershell],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=20,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return []

    if completed.returncode != 0 or not completed.stdout.strip():
        return []

    try:
        parsed = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return []

    raw_items = parsed if isinstance(parsed, list) else [parsed]
    paths: list[Path] = []
    for item in raw_items:
        if not isinstance(item, dict):
            continue
        install_location = str(item.get("InstallLocation") or "").strip()
        if not install_location:
            continue
        root = Path(install_location)
        paths.extend([root / "MicrosoftGame.config", root / "Content" / "MicrosoftGame.config"])
    return paths


def _probe_itch_butler_dbs(config: ScoutConfig) -> list[Path]:
    paths: list[Path] = [*config.system_itch_butler_db_paths]
    if not config.system_itch_butler_db_paths:
        paths.append(Path.home() / "AppData" / "Roaming" / "itch" / "db" / "butler.db")
    butler_dbs: list[Path] = []
    seen: set[str] = set()
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        key = str(path.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        butler_dbs.append(path.resolve())
    return butler_dbs


def _probe_windows_launcher_installs() -> list[dict]:
    return list(_windows_machine_inventory().get("launcher_installs", []))


def _probe_windows_launcher_installs_uncached() -> list[dict]:
    powershell = r"""
$ubisoftGames = @()
$ubisoftRoot = 'HKLM:\SOFTWARE\ubisoft\Launcher\Installs'
if (Test-Path $ubisoftRoot) {
  Get-ChildItem $ubisoftRoot -ErrorAction SilentlyContinue | ForEach-Object {
    $installDir = (Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue).InstallDir
    if ($installDir) {
      $name = Split-Path -Leaf $installDir
      if ($name) {
        $ubisoftGames += @{
          gameId = $_.PSChildName
          name = $name
          installLocation = $installDir
        }
      }
    }
  }
}

$battleGames = @()
$uninstallRoots = @(
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
foreach ($root in $uninstallRoots) {
  if (Test-Path $root) {
    Get-ItemProperty $root -ErrorAction SilentlyContinue |
      Where-Object {
        $_.DisplayName -and $_.InstallLocation -and (
          $_.Publisher -match 'Blizzard' -or
          $_.InstallSource -match 'Battle\.net' -or
          $_.UninstallString -match 'Battle\.net'
        )
      } |
      ForEach-Object {
        $name = $_.DisplayName
        if ($name -and $name -ne 'Battle.net') {
          $battleGames += @{
            uid = $_.PSChildName
            name = $name
            installLocation = $_.InstallLocation
          }
        }
      }
  }
}

@{
  ubisoft = $ubisoftGames
  battleNet = $battleGames
} | ConvertTo-Json -Depth 4
"""
    try:
        completed = subprocess.run(
            ["powershell", "-NoProfile", "-Command", powershell],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=20,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return []

    if completed.returncode != 0 or not completed.stdout.strip():
        return []

    try:
        parsed = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return []

    if not isinstance(parsed, dict):
        return []

    entries: list[dict] = []
    ubisoft_games: list[dict] = []
    for item in parsed.get("ubisoft", []):
        if not isinstance(item, dict):
            continue
        install_location = str(item.get("installLocation") or "").strip()
        name = str(item.get("name") or "").strip() or Path(install_location).name
        if not name:
            continue
        ubisoft_games.append(
            {
                "name": name,
                "platform": "ubisoft",
                "platform_game_id": item.get("gameId"),
                "install_location": install_location or None,
                "source": "ubisoft_registry",
            }
        )
    if ubisoft_games:
        entries.append(
            {
                "path": "windows-registry://ubisoft-launcher-installs",
                "entity_kind": "ubisoft_launcher_installs",
                "games": sorted(ubisoft_games, key=lambda game: str(game["name"]).lower())[:500],
            }
        )

    battle_games: list[dict] = []
    seen_battle: set[tuple[str, str]] = set()
    for item in parsed.get("battleNet", []):
        if not isinstance(item, dict):
            continue
        name = str(item.get("name") or "").strip()
        install_location = str(item.get("installLocation") or "").strip()
        if not name or name.lower() == "battle.net":
            continue
        key = (name.lower(), install_location.lower())
        if key in seen_battle:
            continue
        seen_battle.add(key)
        battle_games.append(
            {
                "name": name,
                "platform": "battle_net",
                "platform_game_id": item.get("uid"),
                "install_location": install_location or None,
                "source": "battle_net_registry",
            }
        )
    if battle_games:
        entries.append(
            {
                "path": "windows-registry://battle-net-launcher-installs",
                "entity_kind": "battle_net_launcher_installs",
                "games": sorted(battle_games, key=lambda game: str(game["name"]).lower())[:500],
            }
        )

    return entries


def _probe_windows_installed_apps() -> list[dict]:
    return list(_windows_machine_inventory().get("installed_apps", []))


def _windows_machine_inventory() -> dict[str, object]:
    return dict(_cached_machine_probe("windows_machine_inventory", _windows_machine_inventory_uncached))


def _windows_machine_inventory_uncached() -> dict[str, object]:
    powershell = r"""
$uninstallRoots = @(
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
)

$installedApps = foreach ($root in $uninstallRoots) {
  if (Test-Path $root) {
    Get-ItemProperty $root -ErrorAction SilentlyContinue |
      Where-Object { $_.DisplayName } |
      Select-Object DisplayName, DisplayVersion, Publisher, InstallLocation
  }
}

$ubisoftGames = @()
$ubisoftRoot = 'HKLM:\SOFTWARE\ubisoft\Launcher\Installs'
if (Test-Path $ubisoftRoot) {
  Get-ChildItem $ubisoftRoot -ErrorAction SilentlyContinue | ForEach-Object {
    $installDir = (Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue).InstallDir
    if ($installDir) {
      $name = Split-Path -Leaf $installDir
      if ($name) {
        $ubisoftGames += @{
          gameId = $_.PSChildName
          name = $name
          installLocation = $installDir
        }
      }
    }
  }
}

$battleGames = @()
foreach ($root in $uninstallRoots) {
  if (Test-Path $root) {
    Get-ItemProperty $root -ErrorAction SilentlyContinue |
      Where-Object {
        $_.DisplayName -and $_.InstallLocation -and (
          $_.Publisher -match 'Blizzard' -or
          $_.InstallSource -match 'Battle\.net' -or
          $_.UninstallString -match 'Battle\.net'
        )
      } |
      ForEach-Object {
        $name = $_.DisplayName
        if ($name -and $name -ne 'Battle.net') {
          $battleGames += @{
            uid = $_.PSChildName
            name = $name
            installLocation = $_.InstallLocation
          }
        }
      }
  }
}

@{
  installedApps = $installedApps
  ubisoft = $ubisoftGames
  battleNet = $battleGames
} | ConvertTo-Json -Depth 4
"""
    try:
        completed = subprocess.run(
            ["powershell", "-NoProfile", "-Command", powershell],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=20,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return {"installed_apps": [], "launcher_installs": [], "xbox_appx_install_locations": []}

    if completed.returncode != 0 or not completed.stdout.strip():
        return {"installed_apps": [], "launcher_installs": [], "xbox_appx_install_locations": []}

    try:
        parsed = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return {"installed_apps": [], "launcher_installs": [], "xbox_appx_install_locations": []}
    if not isinstance(parsed, dict):
        return {"installed_apps": [], "launcher_installs": [], "xbox_appx_install_locations": []}

    return {
        "installed_apps": _normalize_windows_installed_apps(parsed.get("installedApps")),
        "launcher_installs": _normalize_windows_launcher_installs(parsed),
    }


def _normalize_xbox_appx_install_locations(raw_items: object) -> list[Path]:
    items = raw_items if isinstance(raw_items, list) else [raw_items]
    paths: list[Path] = []
    for item in items:
        install_location = str(item or "").strip()
        if not install_location:
            continue
        root = Path(install_location)
        paths.extend([root / "MicrosoftGame.config", root / "Content" / "MicrosoftGame.config"])
    return paths


def _normalize_windows_installed_apps(raw_items: object) -> list[dict]:
    raw_apps = raw_items if isinstance(raw_items, list) else [raw_items]
    apps: list[dict] = []
    seen: set[tuple[str, str, str]] = set()
    for item in raw_apps:
        if not isinstance(item, dict):
            continue
        name = str(item.get("DisplayName") or "").strip()
        if not name:
            continue
        version = str(item.get("DisplayVersion") or "").strip()
        publisher = str(item.get("Publisher") or "").strip()
        key = (name.lower(), version.lower(), publisher.lower())
        if key in seen:
            continue
        seen.add(key)
        apps.append(
            {
                "name": name,
                "version": version or None,
                "publisher": publisher or None,
                "install_location": (str(item.get("InstallLocation") or "").strip() or None),
            }
        )
    apps.sort(key=lambda app: app["name"].lower())
    return apps[:500]


def _normalize_windows_launcher_installs(parsed: dict) -> list[dict]:
    entries: list[dict] = []
    ubisoft_games: list[dict] = []
    for item in parsed.get("ubisoft", []):
        if not isinstance(item, dict):
            continue
        install_location = str(item.get("installLocation") or "").strip()
        name = str(item.get("name") or "").strip() or Path(install_location).name
        if not name:
            continue
        ubisoft_games.append(
            {
                "name": name,
                "platform": "ubisoft",
                "platform_game_id": item.get("gameId"),
                "install_location": install_location or None,
                "source": "ubisoft_registry",
            }
        )
    if ubisoft_games:
        entries.append(
            {
                "path": "windows-registry://ubisoft-launcher-installs",
                "entity_kind": "ubisoft_launcher_installs",
                "games": sorted(ubisoft_games, key=lambda game: str(game["name"]).lower())[:500],
            }
        )

    battle_games: list[dict] = []
    seen_battle: set[tuple[str, str]] = set()
    for item in parsed.get("battleNet", []):
        if not isinstance(item, dict):
            continue
        name = str(item.get("name") or "").strip()
        install_location = str(item.get("installLocation") or "").strip()
        if not name or name.lower() == "battle.net":
            continue
        key = (name.lower(), install_location.lower())
        if key in seen_battle:
            continue
        seen_battle.add(key)
        battle_games.append(
            {
                "name": name,
                "platform": "battle_net",
                "platform_game_id": item.get("uid"),
                "install_location": install_location or None,
                "source": "battle_net_registry",
            }
        )
    if battle_games:
        entries.append(
            {
                "path": "windows-registry://battle-net-launcher-installs",
                "entity_kind": "battle_net_launcher_installs",
                "games": sorted(battle_games, key=lambda game: str(game["name"]).lower())[:500],
            }
        )

    return entries


def _probe_windows_installed_apps_uncached() -> list[dict]:
    powershell = r"""
$roots = @(
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
$items = foreach ($root in $roots) {
  if (Test-Path $root) {
    Get-ItemProperty $root -ErrorAction SilentlyContinue |
      Where-Object { $_.DisplayName } |
      Select-Object DisplayName, DisplayVersion, Publisher, InstallLocation
  }
}
$items | ConvertTo-Json -Depth 3
"""
    try:
        completed = subprocess.run(
            ["powershell", "-NoProfile", "-Command", powershell],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=20,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return []

    if completed.returncode != 0 or not completed.stdout.strip():
        return []

    try:
        parsed = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return []

    raw_items = parsed if isinstance(parsed, list) else [parsed]
    apps: list[dict] = []
    seen: set[tuple[str, str, str]] = set()
    for item in raw_items:
        if not isinstance(item, dict):
            continue
        name = str(item.get("DisplayName") or "").strip()
        if not name:
            continue
        version = str(item.get("DisplayVersion") or "").strip()
        publisher = str(item.get("Publisher") or "").strip()
        key = (name.lower(), version.lower(), publisher.lower())
        if key in seen:
            continue
        seen.add(key)
        apps.append(
            {
                "name": name,
                "version": version or None,
                "publisher": publisher or None,
                "install_location": (str(item.get("InstallLocation") or "").strip() or None),
            }
        )
    apps.sort(key=lambda app: app["name"].lower())
    return apps[:500]


def _probe_downloaded_files(config: ScoutConfig) -> list[dict]:
    download_dirs = []
    if config.home:
        download_dirs.append(config.home / "Downloads")
    for root in config.roots:
        if root.name.lower() == "downloads":
            download_dirs.append(root)
        else:
            download_dirs.append(root / "Downloads")
    seen_dirs: set[str] = set()
    seen_files: set[str] = set()
    files = []
    for directory in download_dirs:
        if not directory.exists() or not directory.is_dir():
            continue
        dir_key = str(directory.resolve()).lower()
        if dir_key in seen_dirs:
            continue
        seen_dirs.add(dir_key)
        for path in sorted(directory.iterdir(), key=lambda item: item.stat().st_mtime if item.exists() else 0, reverse=True):
            if not path.is_file() or path.name.startswith("~$"):
                continue
            file_key = str(path.resolve()).lower()
            if file_key in seen_files:
                continue
            seen_files.add(file_key)
            suffix = path.suffix.lower()
            files.append(
                {
                    "path": str(path.resolve()),
                    "filename": path.name,
                    "file_extension": suffix or None,
                    "category": _download_category(suffix),
                    "size_bytes": path.stat().st_size,
                    "modified_time": path.stat().st_mtime,
                }
            )
    return files[:200]


def _download_category(extension: str) -> str:
    if extension in {".exe", ".msi", ".dmg", ".pkg"}:
        return "installer"
    if extension in {".zip", ".rar", ".7z", ".tar", ".gz"}:
        return "archive"
    if extension in {".pdf", ".docx", ".pptx", ".xlsx", ".md", ".txt"}:
        return "document"
    if extension in {".png", ".jpg", ".jpeg", ".gif", ".webp"}:
        return "image"
    if extension in {".mp4", ".mov", ".mkv", ".webm"}:
        return "video"
    return "other"


def _probe_windows_recent_documents(config: ScoutConfig) -> list[dict]:
    if not config.home:
        return []
    recent_dir = config.home / "AppData" / "Roaming" / "Microsoft" / "Windows" / "Recent"
    if not recent_dir.exists() or not recent_dir.is_dir():
        return []
    powershell = rf"""
$shell = New-Object -ComObject WScript.Shell
$recent = '{str(recent_dir).replace("'", "''")}'
$items = @()
if (Test-Path $recent) {{
  Get-ChildItem $recent -Filter '*.lnk' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 80 |
    ForEach-Object {{
      try {{
        $shortcut = $shell.CreateShortcut($_.FullName)
        if ($shortcut.TargetPath) {{
          $items += @{{ shortcut_path = $_.FullName; target_path = $shortcut.TargetPath }}
        }}
      }} catch {{}}
    }}
}}
$items | ConvertTo-Json -Depth 3
"""
    try:
        completed = subprocess.run(
            ["powershell", "-NoProfile", "-Command", powershell],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=20,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return []

    if completed.returncode != 0 or not completed.stdout.strip():
        return []
    try:
        parsed = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return []

    raw_items = parsed if isinstance(parsed, list) else [parsed]
    recent_documents: list[dict] = []
    seen: set[str] = set()
    for item in raw_items:
        if not isinstance(item, dict):
            continue
        target = Path(str(item.get("target_path") or "").strip())
        if target.suffix.lower() not in {".docx", ".pptx", ".xlsx"}:
            continue
        if target.name.startswith("~$") or not target.exists() or not target.is_file():
            continue
        key = str(target.resolve()).lower()
        if key in seen:
            continue
        seen.add(key)
        document_type = target.suffix.lower().lstrip(".")
        recommended_skills = ["office-document-specialist-suite" if document_type == "xlsx" else document_type]
        recent_documents.append(
            {
                "path": str(target.resolve()),
                "shortcut_path": str(item.get("shortcut_path") or ""),
                "filename": target.name,
                "document_type": document_type,
                "size_bytes": target.stat().st_size,
                "modified_time": target.stat().st_mtime,
                "recommended_skills": recommended_skills,
                "skill_routes": [{"skill": skill, "arguments": {"path": str(target.resolve())}} for skill in recommended_skills],
                "evidence_basis": "windows_recent_shortcut",
            }
        )
    return recent_documents[:40]


def _bootstrap(config: ScoutConfig) -> list[dict]:
    evidence: list[dict] = []
    candidates = iter_bootstrap_candidates(build_effective_roots(config), config.max_depth)
    sqlite_parse_count = 0

    for index, path in enumerate(candidates, start=1):
        if path.name == "History":
            try:
                history_fields = parse_browser_history(path)
                evidence.append(
                    {
                        "id": f"bootstrap-{index}",
                        "path": str(path),
                        "source_kind": "filesystem",
                        "entity_kind": "browser_history",
                        "discovered_by": "bootstrap",
                        "confidence": 0.95,
                        "sensitivity": "medium",
                        "fields": history_fields,
                    }
                )
                download_fields = parse_browser_downloads(path)
                if download_fields.get("downloads"):
                    evidence.append(
                        {
                            "id": f"bootstrap-{index}-downloads",
                            "path": str(path),
                            "source_kind": "filesystem",
                            "entity_kind": "browser_downloads",
                            "discovered_by": "bootstrap",
                            "confidence": 0.95,
                            "sensitivity": "medium",
                            "fields": download_fields,
                        }
                    )
            except Exception as error:
                evidence.append(
                    {
                        "id": f"bootstrap-{index}",
                        "path": str(path),
                        "source_kind": "filesystem",
                        "entity_kind": "parse_error",
                        "discovered_by": "bootstrap",
                        "confidence": 0.95,
                        "sensitivity": "medium",
                        "fields": {
                            "error": f"{type(error).__name__}: {error}",
                            "candidate_name": path.name,
                        },
                    }
                )
            continue

        sqlite_mode = None
        if path.suffix.lower() in {".sqlite", ".sqlite3", ".db"}:
            if sqlite_parse_count < config.max_sqlite_parse:
                sqlite_mode = "parsed"
                sqlite_parse_count += 1
            else:
                sqlite_mode = "budget_skipped"

        try:
            kind, fields = _classify_bootstrap(path, sqlite_mode=sqlite_mode)
        except Exception as error:
            kind = "parse_error"
            fields = {
                "error": f"{type(error).__name__}: {error}",
                "candidate_name": path.name,
            }
        if not kind:
            continue
        sensitivity = "high" if kind == "office_document" else "medium"
        evidence.append(
            {
                "id": f"bootstrap-{index}",
                "path": str(path),
                "source_kind": "filesystem",
                "entity_kind": kind,
                "discovered_by": "bootstrap",
                "confidence": 0.95,
                "sensitivity": sensitivity,
                "fields": fields,
            }
        )

    return evidence


def _classify_bootstrap(path: Path, sqlite_mode: str | None = None) -> tuple[str | None, dict]:
    lower = str(path).lower()
    if lower.endswith(".claude\\claude.md"):
        return "claude_entrypoint", parse_claude_entrypoint(path)
    if lower.endswith(".claude\\settings.json"):
        return "claude_settings", parse_claude_settings(path)
    if lower.endswith("\\cursor\\user\\globalstorage\\state.vscdb"):
        return "cursor_state_db", parse_cursor_state_db(path)
    if lower.endswith("\\code\\user\\globalstorage\\storage.json") or lower.endswith(
        "\\cursor\\user\\globalstorage\\storage.json"
    ):
        return "editor_recent_workspaces", parse_editor_recent_workspaces(path)
    if lower.endswith(".claude\\mcp.json") or lower.endswith(".claude\\mcp_servers.json"):
        return "claude_mcp_config", parse_claude_mcp_config(path)
    if lower.endswith(".codex\\config.toml"):
        return "codex_config", parse_codex_config(path)
    if lower.endswith(".codex\\auth.json"):
        return "codex_auth", parse_codex_auth(path)
    if lower.endswith(".codex\\rules\\default.rules"):
        return "codex_rules", parse_codex_rules(path)
    if lower.endswith("\\appdata\\roaming\\obsidian\\obsidian.json"):
        return "obsidian_global_config", parse_obsidian_global_config(path)
    if path.name == "Bookmarks":
        return "browser_bookmarks", parse_browser_bookmarks(path)
    if path.name == "History":
        return "browser_history", parse_browser_history(path)
    if path.name == "manifest.json" and path.parent.parent.parent.name == "Extensions":
        return "browser_extensions", parse_browser_extension_manifest(path)
    if path.name == "extensions.json" and "\\profiles\\" in lower:
        return "browser_extensions", parse_firefox_extensions(path)
    if path.name == "metaData" and "\\profiles\\" in lower and "\\downloads\\metadata" in lower:
        return "browser_downloads", parse_firefox_downloads(path)
    if path.name in {"sessionstore.jsonlz4", "recovery.jsonlz4", "previous.jsonlz4"} and "\\profiles\\" in lower:
        return "browser_sessions", parse_browser_session(path)
    if path.parent.name == "Sessions" and path.name.startswith("tabs_"):
        return "browser_sessions", parse_browser_session(path)
    if path.name == "places.sqlite" and (
        "\\mozilla\\firefox\\profiles\\" in lower or "\\zen\\profiles\\" in lower
    ):
        parsed = parse_firefox_places(path)
        return "browser_history", parsed
    if path.name == "ConsoleHost_history.txt":
        return "shell_history", parse_shell_history(path)
    if path.name == "apps.json":
        return "installed_apps", parse_installed_apps(path)
    if path.parent.name == "games" and path.parent.parent.name == "library" and "\\playnite\\library\\games\\" in lower:
        return "playnite_library_game", parse_playnite_game(path)
    if path.name == "galaxy-installed.json" and "\\gog.com\\galaxy\\storage\\" in lower:
        return "gog_installed", parse_gog_installed(path)
    if path.name == "GameInstallInfo.sqlite" and "\\amazon games\\data\\games\\sql\\" in lower:
        return "amazon_games_install_info", parse_amazon_games_install_info(path)
    if path.name == "MicrosoftGame.config":
        return "xbox_game_config", parse_xbox_game_config(path)
    if path.name == "butler.db" and "\\itch\\db\\" in lower:
        return "itch_butler_db", parse_itch_butler_db(path)
    if path.name == "product.db" and "\\battle.net\\agent\\" in lower:
        return "battle_net_product_db", parse_battle_net_product_db(path)
    if path.suffix.lower() == ".mfst" and (
        "\\programdata\\origin\\localcontent\\" in lower or "\\programdata\\ea desktop\\localcontent\\" in lower
    ):
        return "origin_localcontent_manifest", parse_origin_localcontent_manifest(path)
    if path.name == "config" and path.parent.name == ".git":
        return "git_repo_config", parse_git_config(path)
    if path.name == "AGENTS.md":
        return "project_rules", parse_markdown_summary(path)
    if path.suffix.lower() in {".docx", ".pptx", ".xlsx"}:
        return "office_document", parse_office_document(path)
    if path.name == "libraryfolders.vdf":
        return "steam_library_index", {"library_paths": parse_vdf_paths(path)}
    if path.name.startswith("appmanifest_") and path.suffix.lower() == ".acf":
        return "steam_game_manifest", parse_acf_manifest(path)
    if path.suffix.lower() == ".code-workspace":
        return "workspace_file", parse_workspace_file(path)
    if path.suffix.lower() in {".sqlite", ".sqlite3", ".db"}:
        if sqlite_mode == "budget_skipped":
            return "sqlite_database", {
                "valid": None,
                "tables": [],
                "row_counts": {},
                "error": None,
                "parse_mode": "budget_skipped",
            }
        parsed = parse_sqlite_database(path)
        parsed["parse_mode"] = "parsed"
        return "sqlite_database", parsed
    if path.suffix.lower() == ".item":
        return "epic_game_manifest", parse_epic_manifest(path)
    return None, {}


def _ai_expand(config: ScoutConfig, bootstrap_evidence: list[dict], search_trace: list[dict]) -> list[dict]:
    evidence: list[dict] = []
    seen: set[str] = {entry["path"].lower() for entry in bootstrap_evidence}
    count = 0

    for entry in bootstrap_evidence:
        if count >= config.max_ai_expansions:
            break

        if entry["entity_kind"] == "claude_entrypoint":
            base = Path(entry["path"]).parent
            for relative in entry["fields"].get("referenced_paths", []):
                target = (base / relative).resolve()
                if not target.exists() or not target.is_file():
                    continue
                key = str(target).lower()
                if key in seen:
                    continue
                seen.add(key)
                count += 1
                search_trace.append(
                    {
                        "step": len(search_trace) + 1,
                        "planner": "ai",
                        "action": "follow_markdown_reference",
                        "from": entry["path"],
                        "target": str(target),
                        "reason": "CLAUDE entrypoint referenced a likely high-signal context file",
                    }
                )
                evidence.append(
                    {
                        "id": f"ai-{count}",
                        "path": str(target),
                        "source_kind": "filesystem",
                        "entity_kind": "claude_context_file",
                        "discovered_by": "ai",
                        "confidence": 0.88,
                        "sensitivity": "high",
                        "fields": parse_markdown_summary(target),
                    }
                )
        elif entry["entity_kind"] == "steam_library_index":
            for library_path in entry["fields"].get("library_paths", []):
                candidate = Path(library_path) / "steamapps"
                if not candidate.exists():
                    continue
                manifests = sorted(candidate.glob("appmanifest_*.acf"))
                search_trace.append(
                    {
                        "step": len(search_trace) + 1,
                        "planner": "ai",
                        "action": "expand_steam_library",
                        "from": entry["path"],
                        "target": str(candidate),
                        "reason": "Steam library index exposed a library path that may contain installed game manifests",
                    }
                )
                for manifest_path in manifests:
                    key = str(manifest_path).lower()
                    if key in seen:
                        continue
                    seen.add(key)
                    count += 1
                    manifest = parse_acf_manifest(manifest_path)
                    install_dir = manifest.get("install_dir_name")
                    executable_candidates: list[str] = []
                    if install_dir:
                        common_dir = candidate / "common" / install_dir
                        if common_dir.exists():
                            executable_candidates = [str(path) for path in sorted(common_dir.glob("*.exe"))[:10]]
                    evidence.append(
                        {
                            "id": f"ai-{count}",
                            "path": str(manifest_path),
                            "source_kind": "filesystem",
                            "entity_kind": "steam_game_manifest",
                            "discovered_by": "ai",
                            "confidence": 0.9,
                            "sensitivity": "low",
                            "fields": {
                                **manifest,
                                "executable_candidates": executable_candidates,
                            },
                        }
                    )
        elif entry["entity_kind"] == "obsidian_global_config":
            for vault in entry["fields"].get("vaults", []):
                if count >= config.max_ai_expansions:
                    break
                if not isinstance(vault, dict):
                    continue
                raw_vault_path = str(vault.get("path") or "").strip()
                if not raw_vault_path:
                    continue
                vault_path = Path(raw_vault_path).resolve()
                if not vault_path.exists() or not vault_path.is_dir():
                    continue
                key = str(vault_path).lower()
                if key in seen:
                    continue
                seen.add(key)
                count += 1
                search_trace.append(
                    {
                        "step": len(search_trace) + 1,
                        "planner": "ai",
                        "action": "follow_obsidian_vault_registration",
                        "from": entry["path"],
                        "target": str(vault_path),
                        "reason": "Obsidian global config registered a vault directory",
                    }
                )
                evidence.append(
                    {
                        "id": f"ai-{count}",
                        "path": str(vault_path),
                        "source_kind": "filesystem",
                        "entity_kind": "obsidian_vault",
                        "discovered_by": "ai",
                        "confidence": 0.88,
                        "sensitivity": "medium",
                        "fields": parse_obsidian_vault(vault_path),
                    }
                )

    return evidence


def _build_entities(raw_evidence: list[dict]) -> dict:
    return {
        "paths": sorted({entry["path"] for entry in raw_evidence}),
        "entity_kinds": sorted({entry["entity_kind"] for entry in raw_evidence}),
    }


def _normalize_game_path(value: object) -> str:
    text = str(value or "").strip()
    return text.replace("\\", "/") if text else ""


def _build_game_index(all_games: list[dict]) -> dict:
    games = []
    seen: set[tuple[str, str, str]] = set()
    for game in all_games:
        name = str(game.get("name") or "").strip()
        if not name:
            continue
        install_dir = _normalize_game_path(game.get("install_dir_name"))
        executable_paths = sorted(
            {
                _normalize_game_path(path)
                for path in game.get("executable_candidates", [])
                if _normalize_game_path(path).lower().endswith(".exe")
            }
        )
        executable_names = sorted({path.rstrip("/").split("/")[-1].lower() for path in executable_paths})
        key = (str(game.get("platform") or "unknown").lower(), name.lower(), install_dir.lower())
        if key in seen:
            continue
        seen.add(key)
        games.append(
            {
                "name": name,
                "platform": game.get("platform") or "unknown",
                "platform_game_id": game.get("platform_game_id"),
                "install_dir": install_dir,
                "executable_names": executable_names,
                "executable_paths": executable_paths,
                "source": game.get("source") or "scout",
                "confidence": "strong" if executable_paths or install_dir else "medium",
            }
        )
    games.sort(key=lambda item: (str(item["platform"]).lower(), str(item["name"]).lower()))
    return {"schema": "weft.game_index/v1", "status": "ready" if games else "empty", "games": games}


def _derive_profile(raw_evidence: list[dict]) -> dict:
    claude_present = any(entry["entity_kind"] == "claude_entrypoint" for entry in raw_evidence)
    claude_settings_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "claude_settings"]
    claude_mcp_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "claude_mcp_config"]
    codex_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "codex_config"]
    codex_auth_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "codex_auth"]
    codex_rules_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "codex_rules"]
    legendary_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "legendary_installed"]
    playnite_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "playnite_library_game"]
    gog_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "gog_installed"]
    amazon_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "amazon_games_install_info"]
    xbox_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "xbox_game_config"]
    itch_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "itch_butler_db"]
    ubisoft_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "ubisoft_launcher_installs"]
    battle_net_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "battle_net_launcher_installs"]
    battle_net_product_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "battle_net_product_db"]
    origin_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "origin_localcontent_manifest"]
    activitywatch_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "activitywatch_runtime"]
    obsidian_global_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "obsidian_global_config"]
    obsidian_vault_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "obsidian_vault"]
    obs_studio_profile_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "obs_studio_profile"]
    obs_studio_scene_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "obs_studio_scene_collection"]
    docker_desktop_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "docker_desktop_settings"]
    wsl_global_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "wsl_global_config"]
    wsl_distribution_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "wsl_distribution_list"]
    discord_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "discord_settings"]
    teams_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "teams_config"]
    dropbox_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "dropbox_info"]
    onedrive_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "onedrive_global_config"]
    joplin_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "joplin_profile"]
    nextcloud_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "nextcloud_config"]
    syncthing_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "syncthing_config"]
    jetbrains_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "jetbrains_recent_projects"]
    windows_terminal_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "windows_terminal_settings"]
    ssh_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "ssh_config"]
    kubeconfig_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "kubeconfig"]
    docker_cli_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "docker_cli_config"]
    docker_context_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "docker_context_meta"]
    aws_cli_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "aws_cli_config"]
    azure_cli_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "azure_cli_profile"]
    gcloud_active_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "gcloud_active_config"]
    gcloud_cli_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "gcloud_cli_config"]
    github_cli_config_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "github_cli_config"]
    github_cli_hosts_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "github_cli_hosts"]
    git_global_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "git_global_config"]
    cargo_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "cargo_user_config"]
    cargo_credentials_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "cargo_credentials_store"]
    maven_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "maven_user_settings"]
    gradle_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "gradle_user_properties"]
    nuget_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "nuget_user_config"]
    dotnet_global_tool_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "dotnet_global_tools"]
    npm_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "npm_user_config"]
    pnpm_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "pnpm_user_config"]
    pip_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "pip_user_config"]
    conda_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "conda_user_config"]
    poetry_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "poetry_user_config"]
    rustup_settings_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "rustup_settings"]
    uv_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "uv_user_config"]
    uv_credentials_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "uv_credentials_store"]
    yarn_user_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "yarn_user_config"]
    codex_present = bool(codex_entries)
    trusted_projects: list[str] = []
    mcp_servers: list[str] = []
    for entry in codex_entries:
        trusted_projects.extend(entry["fields"].get("trusted_projects", []))
        mcp_servers.extend(entry["fields"].get("mcp_servers", []))
    claude_mcp_servers: list[str] = []
    claude_default_models: list[str] = []
    for entry in claude_settings_entries:
        claude_mcp_servers.extend(entry["fields"].get("mcp_servers", []))
        if entry["fields"].get("defaultModel"):
            claude_default_models.append(str(entry["fields"]["defaultModel"]))
    for entry in claude_mcp_entries:
        claude_mcp_servers.extend(entry["fields"].get("server_names", []))
    approved_rule_count = sum(int(entry["fields"].get("allowed_rule_count", 0)) for entry in codex_rules_entries)

    steam_games = []
    steam_app_ids: set[str] = set()
    steam_install_dir_names: set[str] = set()
    steam_sizes_on_disk: set[str] = set()
    steam_state_flags: set[str] = set()
    for entry in raw_evidence:
        if entry["entity_kind"] != "steam_game_manifest":
            continue
        fields = entry["fields"]
        if not fields.get("name"):
            continue
        app_id = str(fields.get("app_id") or "").strip()
        install_dir_name = str(fields.get("install_dir_name") or "").strip()
        size_on_disk = str(fields.get("size_on_disk") or "").strip()
        state_flags = str(fields.get("state_flags") or "").strip()
        if app_id:
            steam_app_ids.add(app_id)
        if install_dir_name:
            steam_install_dir_names.add(install_dir_name)
        if size_on_disk:
            steam_sizes_on_disk.add(size_on_disk)
        if state_flags:
            steam_state_flags.add(state_flags)
        steam_games.append(
            {
                "platform": "steam",
                "platform_game_id": fields.get("app_id"),
                "name": fields.get("name"),
                "install_dir_name": fields.get("install_dir_name"),
                "executable_candidates": fields.get("executable_candidates", []),
                "source": "steam_appmanifest",
            }
        )
    steam_library_roots = sorted(
        {
            str(library_path).strip()
            for entry in raw_evidence
            if entry["entity_kind"] == "steam_library_index"
            for library_path in entry["fields"].get("library_paths", [])
            if str(library_path).strip()
        }
    )
    steam_manifest_count = sum(1 for entry in raw_evidence if entry["entity_kind"] == "steam_game_manifest")

    epic_games = []
    epic_catalog_item_ids: set[str] = set()
    epic_app_names: set[str] = set()
    epic_install_locations: set[str] = set()
    epic_launch_executables: set[str] = set()
    epic_technical_types: set[str] = set()
    epic_install_sizes: set[int] = set()
    epic_executable_path_count = 0
    for entry in raw_evidence:
        if entry["entity_kind"] != "epic_game_manifest":
            continue
        fields = entry["fields"]
        if not fields.get("name"):
            continue
        executable_candidates = []
        install_location = fields.get("install_location")
        launch_executable = fields.get("launch_executable")
        if install_location and launch_executable:
            executable_candidates.append(f"{install_location}\\{launch_executable}")
            epic_executable_path_count += 1
        catalog_item_id = str(fields.get("catalog_item_id") or "").strip()
        app_name = str(fields.get("app_name") or "").strip()
        install_location_text = str(install_location or "").strip()
        launch_executable_text = str(launch_executable or "").strip()
        technical_type = str(fields.get("technical_type") or "").strip()
        install_size = fields.get("install_size")
        if catalog_item_id:
            epic_catalog_item_ids.add(catalog_item_id)
        if app_name:
            epic_app_names.add(app_name)
        if install_location_text:
            epic_install_locations.add(install_location_text)
        if launch_executable_text:
            epic_launch_executables.add(launch_executable_text)
        if technical_type:
            epic_technical_types.add(technical_type)
        if isinstance(install_size, int) and not isinstance(install_size, bool):
            epic_install_sizes.add(install_size)
        epic_games.append(
            {
                "platform": "epic",
                "platform_game_id": fields.get("catalog_item_id"),
                "name": fields.get("name"),
                "install_dir_name": fields.get("install_location"),
                "executable_candidates": executable_candidates,
                "source": "epic_manifest",
            }
        )
    for entry in legendary_entries:
        for game in entry["fields"].get("games", []):
            if not isinstance(game, dict):
                continue
            name = game.get("name")
            if not name:
                continue
            app_name = str(game.get("app_name") or "").strip()
            install_location = str(game.get("install_location") or "").strip()
            if app_name:
                epic_app_names.add(app_name)
            if install_location:
                epic_install_locations.add(install_location)
            epic_games.append(
                {
                    "platform": "epic",
                    "platform_game_id": game.get("app_name"),
                    "name": name,
                    "install_dir_name": game.get("install_location"),
                    "executable_candidates": [],
                    "source": "legendary_installed",
                }
            )

    gog_game_ids: set[str] = set()
    gog_install_locations: set[str] = set()
    for entry in gog_entries:
        for game in entry["fields"].get("games", []):
            if not isinstance(game, dict):
                continue
            game_id = str(game.get("platform_game_id") or "").strip()
            install_location = str(game.get("install_location") or "").strip()
            if game_id:
                gog_game_ids.add(game_id)
            if install_location:
                gog_install_locations.add(install_location)

    battle_net_game_ids: set[str] = set()
    battle_net_product_codes: set[str] = set()
    battle_net_install_locations: set[str] = set()
    for entry in battle_net_product_entries:
        for game in entry["fields"].get("games", []):
            if not isinstance(game, dict):
                continue
            game_id = str(game.get("platform_game_id") or "").strip()
            product_code = str(game.get("product_code") or "").strip()
            install_location = str(game.get("install_location") or "").strip()
            if game_id:
                battle_net_game_ids.add(game_id)
            if product_code:
                battle_net_product_codes.add(product_code)
            if install_location:
                battle_net_install_locations.add(install_location)

    ea_game_ids: set[str] = set()
    ea_install_locations: set[str] = set()
    for entry in origin_entries:
        for game in entry["fields"].get("games", []):
            if not isinstance(game, dict):
                continue
            game_id = str(game.get("platform_game_id") or "").strip()
            install_location = str(game.get("install_location") or "").strip()
            if game_id:
                ea_game_ids.add(game_id)
            if install_location:
                ea_install_locations.add(install_location)

    launcher_games = []
    for entry in [
        *playnite_entries,
        *gog_entries,
        *amazon_entries,
        *xbox_entries,
        *itch_entries,
        *ubisoft_entries,
        *battle_net_entries,
        *battle_net_product_entries,
        *origin_entries,
    ]:
        for game in entry["fields"].get("games", []):
            if not isinstance(game, dict):
                continue
            name = game.get("name")
            if not name:
                continue
            launcher_games.append(
                {
                    "platform": game.get("platform") or "unknown",
                    "platform_game_id": game.get("platform_game_id"),
                    "name": name,
                    "install_dir_name": game.get("install_location"),
                    "executable_candidates": game.get("executable_candidates", []),
                    "source": game.get("source") or entry["entity_kind"],
                }
            )

    all_games = [*steam_games, *epic_games, *launcher_games]
    all_games.sort(key=lambda item: (str(item["platform"]).lower(), str(item["name"]).lower()))
    game_index = _build_game_index(all_games)
    platforms = sorted({item["platform"] for item in all_games})
    browser_bookmarks_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "browser_bookmarks"]
    browser_history_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "browser_history"]
    browser_download_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "browser_downloads"]
    browser_extension_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "browser_extensions"]
    browser_session_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "browser_sessions"]
    editor_workspace_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "editor_recent_workspaces"]
    cursor_state_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "cursor_state_db"]
    shell_history_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "shell_history"]
    installed_app_entries = [entry for entry in raw_evidence if entry["entity_kind"] == "installed_apps"]
    bookmark_titles: list[str] = []
    recent_history_domains: list[str] = []
    bookmark_domains: list[str] = []
    download_domains: list[str] = []
    extension_names: list[str] = []
    session_domains: list[str] = []
    session_titles: list[str] = []
    browsers: list[str] = []
    per_browser: dict[str, dict[str, list[str]]] = {}

    def _browser_bucket(name: str) -> dict[str, list[object]]:
        bucket = per_browser.get(name)
        if bucket is None:
            bucket = {
                "bookmark_titles": [],
                "bookmark_domains": [],
                "recent_history_domains": [],
                "top_history_domains": [],
                "top_search_queries": [],
                "download_domains": [],
                "download_file_extensions": [],
                "extensions": [],
                "session_domains": [],
                "session_titles": [],
            }
            per_browser[name] = bucket
        return bucket

    for entry in browser_bookmarks_entries:
        bookmark_titles.extend(entry["fields"].get("bookmark_titles", []))
        bookmark_domains.extend(entry["fields"].get("bookmark_domains", []))
        browser_name = entry["fields"].get("browser")
        if browser_name:
            browser_name = str(browser_name)
            browsers.append(browser_name)
            bucket = _browser_bucket(browser_name)
            bucket["bookmark_titles"].extend(entry["fields"].get("bookmark_titles", []))
            bucket["bookmark_domains"].extend(entry["fields"].get("bookmark_domains", []))
    for entry in browser_history_entries:
        bookmark_titles.extend(entry["fields"].get("bookmark_titles", []))
        bookmark_domains.extend(entry["fields"].get("bookmark_domains", []))
        recent_history_domains.extend(entry["fields"].get("recent_domains", []))
        browser_name = entry["fields"].get("browser")
        if browser_name:
            browser_name = str(browser_name)
            browsers.append(browser_name)
            bucket = _browser_bucket(browser_name)
            bucket["bookmark_titles"].extend(entry["fields"].get("bookmark_titles", []))
            bucket["bookmark_domains"].extend(entry["fields"].get("bookmark_domains", []))
            bucket["recent_history_domains"].extend(entry["fields"].get("recent_domains", []))
            bucket["top_history_domains"].extend(entry["fields"].get("top_history_domains", []))
            bucket["top_search_queries"].extend(
                [
                    {
                        **item,
                        "sources": sorted({str(browser_name)}),
                    }
                    for item in entry["fields"].get("top_search_queries", [])
                    if isinstance(item, dict)
                ]
            )
    for entry in browser_download_entries:
        download_domains.extend(entry["fields"].get("download_domains", []))
        browser_name = entry["fields"].get("browser")
        if browser_name:
            browser_name = str(browser_name)
            browsers.append(browser_name)
            bucket = _browser_bucket(browser_name)
            bucket["download_domains"].extend(entry["fields"].get("download_domains", []))
            bucket["download_file_extensions"].extend(entry["fields"].get("download_file_extensions", []))
    for entry in browser_extension_entries:
        browser_name = entry["fields"].get("browser")
        if browser_name:
            browser_name = str(browser_name)
            browsers.append(browser_name)
            bucket = _browser_bucket(browser_name)
            extensions = [item for item in entry["fields"].get("extensions", []) if isinstance(item, dict)]
            bucket["extensions"].extend(extensions)
            extension_names.extend(
                [str(item.get("name")) for item in extensions if str(item.get("name") or "").strip()]
            )
    for entry in browser_session_entries:
        session_domains.extend(entry["fields"].get("session_domains", []))
        session_titles.extend(entry["fields"].get("session_titles", []))
        browser_name = entry["fields"].get("browser")
        if browser_name:
            browser_name = str(browser_name)
            browsers.append(browser_name)
            bucket = _browser_bucket(browser_name)
            bucket["session_domains"].extend(entry["fields"].get("session_domains", []))
            bucket["session_titles"].extend(entry["fields"].get("session_titles", []))

    per_browser_summary = {
        browser_name: {
            "bookmark_titles": sorted(set(values["bookmark_titles"])),
            "bookmark_domains": sorted(set(values["bookmark_domains"])),
            "recent_history_domains": sorted(set(values["recent_history_domains"])),
            "download_domains": sorted(set(str(item) for item in values["download_domains"] if str(item).strip())),
            "download_file_extensions": sorted(
                set(str(item) for item in values["download_file_extensions"] if str(item).strip())
            ),
            "session_domains": sorted(set(str(item) for item in values["session_domains"] if str(item).strip())),
            "session_titles": sorted(set(str(item) for item in values["session_titles"] if str(item).strip())),
            "top_history_domains": sorted(
                (
                    {
                        "domain": domain,
                        "visit_count": sum(
                            int(item.get("visit_count", 0))
                            for item in values["top_history_domains"]
                            if item.get("domain") == domain
                        ),
                    }
                    for domain in sorted({str(item.get("domain")) for item in values["top_history_domains"] if item.get("domain")})
                ),
                key=lambda item: (-item["visit_count"], item["domain"]),
            )[:10],
            "top_search_queries": _merge_search_queries(values["top_search_queries"], limit=10),
            "extensions": sorted(
                {
                    json.dumps(item, sort_keys=True): item
                    for item in values["extensions"]
                    if isinstance(item, dict) and item.get("name")
                }.values(),
                key=lambda item: (str(item.get("name", "")).lower(), str(item.get("id", "")).lower()),
            )[:100],
        }
        for browser_name, values in sorted(per_browser.items())
    }

    git_repos = []
    for entry in raw_evidence:
        if entry["entity_kind"] != "git_repo_config":
            continue
        git_repos.append(
            {
                "path": str(Path(entry["path"]).parent.parent),
                "remote_urls": entry["fields"].get("remote_urls", []),
            }
        )

    workspace_files = []
    for entry in raw_evidence:
        if entry["entity_kind"] != "workspace_file":
            continue
        workspace_files.append(
            {
                "path": entry["path"],
                "folder_paths": entry["fields"].get("folder_paths", []),
            }
        )

    recent_editor_workspaces = []
    seen_editor_workspace_paths: set[str] = set()
    for entry in editor_workspace_entries:
        editor = entry["fields"].get("editor")
        for path in entry["fields"].get("recent_workspaces", []):
            normalized = str(path)
            key = normalized.lower()
            if key in seen_editor_workspace_paths:
                continue
            seen_editor_workspace_paths.add(key)
            recent_editor_workspaces.append(
                {
                    "path": normalized,
                    "editor": editor,
                }
            )
    for entry in cursor_state_entries:
        for path in entry["fields"].get("recent_workspaces", []):
            normalized = str(path)
            key = normalized.lower()
            if key in seen_editor_workspace_paths:
                continue
            seen_editor_workspace_paths.add(key)
            recent_editor_workspaces.append(
                {
                    "path": normalized,
                    "editor": "cursor",
                }
            )
    recent_editor_workspaces.sort(key=lambda item: (item["path"].lower(), str(item["editor"]).lower()))

    shells: list[str] = []
    recent_commands: list[str] = []
    command_counts: dict[str, int] = {}
    for entry in shell_history_entries:
        shell = entry["fields"].get("shell")
        if shell:
            shells.append(str(shell))
        recent_commands.extend(entry["fields"].get("recent_commands", []))
        for item in entry["fields"].get("top_commands", []):
            command = item.get("command")
            count = item.get("count", 0)
            if not command:
                continue
            command_counts[str(command)] = command_counts.get(str(command), 0) + int(count)
    top_commands = [
        {"command": command, "count": count}
        for command, count in sorted(command_counts.items(), key=lambda item: (-item[1], item[0].lower()))[:10]
    ]

    installed_apps = []
    seen_apps: set[tuple[str, str, str]] = set()
    for entry in installed_app_entries:
        for app in entry["fields"].get("apps", []):
            name = str(app.get("name") or "")
            version = str(app.get("version") or "")
            publisher = str(app.get("publisher") or "")
            key = (name.lower(), version.lower(), publisher.lower())
            if not name or key in seen_apps:
                continue
            seen_apps.add(key)
            installed_apps.append(app)
    installed_apps.sort(key=lambda item: str(item.get("name", "")).lower())
    creative_tools_profile = derive_creative_tools_profile(installed_apps)
    hardware_profile = derive_hardware_profile(installed_apps)
    privacy_security_profile = derive_privacy_security_profile(installed_apps)

    sqlite_artifacts = []
    for entry in raw_evidence:
        if entry["entity_kind"] != "sqlite_database":
            continue
        sqlite_artifacts.append(
            {
                "path": entry["path"],
                "valid": entry["fields"].get("valid", True),
                "tables": entry["fields"].get("tables", []),
                "row_counts": entry["fields"].get("row_counts", {}),
                "error": entry["fields"].get("error"),
            }
        )
    sqlite_artifacts.sort(
        key=lambda item: (
            0 if item["valid"] else 1,
            -sum(max(int(value), 0) for value in item["row_counts"].values()),
            -len(item["tables"]),
            item["path"].lower(),
        )
    )
    sqlite_profile = {
        "total_count": len(sqlite_artifacts),
        "top_artifacts": sqlite_artifacts[:10],
    }

    installed_app_names = [str(item.get("name", "")) for item in installed_apps if item.get("name")]
    installed_app_names_lower = [name.lower() for name in installed_app_names]
    tooling_allowlist = {
        "claude",
        "codex",
        "python",
        "node",
        "npm",
        "git",
        "cargo",
        "winget",
        "pip",
        "uv",
        "pnpm",
        "yarn",
        "powershell",
        "pwsh",
        "cmd",
    }
    shell_tooling = sorted(
        {
            item["command"].lower()
            for item in top_commands
            if str(item.get("command", "")).strip().lower() in tooling_allowlist
        }
    )
    developer_signals: set[str] = set()
    if git_repos:
        developer_signals.add("git")
    if any(item.get("editor") == "vscode" for item in recent_editor_workspaces) or any(
        "visual studio code" in name for name in installed_app_names_lower
    ):
        developer_signals.add("vscode")
    if any(item.get("editor") == "cursor" for item in recent_editor_workspaces) or any(
        "cursor" in name for name in installed_app_names_lower
    ):
        developer_signals.add("cursor")
    if any(command in shell_tooling for command in {"python", "node", "git", "npm", "cargo"}):
        developer_signals.add("shell_dev_tools")

    ai_tool_families: set[str] = set()
    if claude_present or claude_settings_entries or any("claude" in name for name in installed_app_names_lower):
        ai_tool_families.add("claude")
    if codex_present or any("codex" in name for name in installed_app_names_lower):
        ai_tool_families.add("codex")
    if any("cursor" in name for name in installed_app_names_lower):
        ai_tool_families.add("cursor")
    if cursor_state_entries:
        ai_tool_families.add("cursor")
    if any("openai" in domain for domain in recent_history_domains):
        ai_tool_families.add("openai")

    cursor_composer_count = sum(int(entry["fields"].get("composer_count", 0)) for entry in cursor_state_entries)
    cursor_agentic_count = sum(int(entry["fields"].get("agentic_count", 0)) for entry in cursor_state_entries)
    cursor_modes = sorted(
        {
            str(mode)
            for entry in cursor_state_entries
            for mode in entry["fields"].get("modes", [])
            if str(mode).strip()
        }
    )

    ai_desktop_tools = sorted(
        {
            name.lower()
            for name in installed_app_names
            if any(keyword in name.lower() for keyword in {"claude", "cursor", "codex"})
        }
    )

    developer_profile = {
        "is_developer": bool(git_repos or recent_editor_workspaces or developer_signals),
        "signals": sorted(developer_signals),
        "repo_count": len(git_repos),
        "editor_families": sorted({str(item.get("editor")) for item in recent_editor_workspaces if item.get("editor")}),
        "shell_tooling": shell_tooling,
        "language_tooling": sorted(
            [
                *(["dotnet"] if (nuget_user_entries or dotnet_global_tool_entries) else []),
                *(["javascript"] if (npm_user_entries or pnpm_user_entries or yarn_user_entries) else []),
                *(["java"] if (maven_user_entries or gradle_user_entries) else []),
                *(["python"] if (pip_user_entries or conda_user_entries or poetry_user_entries or uv_user_entries or uv_credentials_entries) else []),
                *(["rust"] if (cargo_user_entries or cargo_credentials_entries or rustup_settings_entries) else []),
            ]
        ),
        "package_tooling": sorted(
            [
                *(["cargo"] if (cargo_user_entries or cargo_credentials_entries) else []),
                *(["conda"] if conda_user_entries else []),
                *(["gradle"] if gradle_user_entries else []),
                *(["maven"] if maven_user_entries else []),
                *(["nuget"] if nuget_user_entries else []),
                *(["npm"] if npm_user_entries else []),
                *(["pnpm"] if pnpm_user_entries else []),
                *(["pip"] if pip_user_entries else []),
                *(["poetry"] if poetry_user_entries else []),
                *(["uv"] if (uv_user_entries or uv_credentials_entries) else []),
                *(["yarn"] if yarn_user_entries else []),
            ]
        ),
    }

    gaming_profile = {
        "is_gamer": bool(all_games or any("steam" in name or "epic" in name for name in installed_app_names_lower)),
        "platforms": platforms,
        "installed_game_names": sorted({str(item["name"]) for item in all_games if item.get("name")}),
        "game_count": len(all_games),
        "steam_game_count": len(steam_games),
        "steam_app_ids": sorted(steam_app_ids),
        "steam_install_dir_names": sorted(steam_install_dir_names),
        "steam_sizes_on_disk": sorted(steam_sizes_on_disk),
        "steam_state_flags": sorted(steam_state_flags),
        "steam_library_roots": steam_library_roots,
        "steam_manifest_count": steam_manifest_count,
        "epic_game_count": len(epic_games),
        "epic_catalog_item_ids": sorted(epic_catalog_item_ids),
        "epic_app_names": sorted(epic_app_names),
        "epic_install_locations": sorted(epic_install_locations),
        "epic_launch_executables": sorted(epic_launch_executables),
        "epic_install_sizes": sorted(epic_install_sizes),
        "epic_technical_types": sorted(epic_technical_types),
        "epic_executable_path_count": epic_executable_path_count,
        "gog_game_count": sum(len(entry["fields"].get("games", [])) for entry in gog_entries),
        "gog_game_ids": sorted(gog_game_ids),
        "gog_install_locations": sorted(gog_install_locations),
        "battle_net_game_count": sum(len(entry["fields"].get("games", [])) for entry in battle_net_product_entries),
        "battle_net_game_ids": sorted(battle_net_game_ids),
        "battle_net_product_codes": sorted(battle_net_product_codes),
        "battle_net_install_locations": sorted(battle_net_install_locations),
        "ea_game_count": sum(len(entry["fields"].get("games", [])) for entry in origin_entries),
        "ea_game_ids": sorted(ea_game_ids),
        "ea_install_locations": sorted(ea_install_locations),
    }

    ai_tools_profile = {
        "uses_ai_tools": bool(ai_tool_families),
        "tool_families": sorted(ai_tool_families),
        "desktop_tools": ai_desktop_tools,
        "configured_mcp_servers": sorted(set(claude_mcp_servers + mcp_servers)),
        "cursor_signals": {
            "composer_count": cursor_composer_count,
            "agentic_count": cursor_agentic_count,
            "modes": cursor_modes,
        },
    }

    knowledge_tool_families: set[str] = set()
    if obsidian_global_entries or obsidian_vault_entries:
        knowledge_tool_families.add("obsidian")
    if joplin_entries:
        knowledge_tool_families.add("joplin")

    knowledge_tools_profile = {
        "present": bool(knowledge_tool_families),
        "tool_families": sorted(knowledge_tool_families),
        "vault_count": len(obsidian_vault_entries),
        "vault_names": sorted(
            {
                str(entry["fields"].get("vault_name") or "").strip()
                for entry in obsidian_vault_entries
                if str(entry["fields"].get("vault_name") or "").strip()
            }
        ),
        "note_count": sum(int(entry["fields"].get("note_count", 0)) for entry in obsidian_vault_entries),
        "core_plugins": sorted(
            {
                str(plugin).strip()
                for entry in obsidian_vault_entries
                for package in entry["fields"].get("core_plugins", [])
                if str(plugin).strip()
            }
        ),
        "community_plugins": sorted(
            {
                str(plugin).strip()
                for entry in obsidian_vault_entries
                for package in entry["fields"].get("community_plugins", [])
                if str(plugin).strip()
            }
        ),
        "joplin_profile_count": len(joplin_entries),
        "joplin_sync_targets": sorted(
            {
                int(entry["fields"]["sync_target"])
                for entry in joplin_entries
                if entry["fields"].get("sync_target") is not None
            }
        ),
        "joplin_sync_paths": sorted(
            {
                str(entry["fields"].get("sync_path") or "").strip()
                for entry in joplin_entries
                if str(entry["fields"].get("sync_path") or "").strip()
            }
        ),
        "joplin_locales": sorted(
            {
                str(entry["fields"].get("locale") or "").strip()
                for entry in joplin_entries
                if str(entry["fields"].get("locale") or "").strip()
            }
        ),
        "joplin_theme_ids": sorted(
            {
                int(entry["fields"]["theme"])
                for entry in joplin_entries
                if entry["fields"].get("theme") is not None
            }
        ),
        "joplin_theme_auto_detect": next(
            (
                entry["fields"].get("theme_auto_detect")
                for entry in joplin_entries
                if entry["fields"].get("theme_auto_detect") is not None
            ),
            None,
        ),
        "joplin_resource_download_modes": sorted(
            {
                str(entry["fields"].get("resource_download_mode") or "").strip()
                for entry in joplin_entries
                if str(entry["fields"].get("resource_download_mode") or "").strip()
            }
        ),
        "joplin_ocr_enabled": next(
            (
                entry["fields"].get("ocr_enabled")
                for entry in joplin_entries
                if entry["fields"].get("ocr_enabled") is not None
            ),
            None,
        ),
        "joplin_plugin_count": sum(int(entry["fields"].get("plugin_count", 0)) for entry in joplin_entries),
        "joplin_plugin_ids": sorted(
            {
                str(plugin_id).strip()
                for entry in joplin_entries
                for plugin_id in entry["fields"].get("plugin_ids", [])
                if str(plugin_id).strip()
            }
        ),
        "joplin_custom_css_files": sorted(
            {
                str(css_name).strip()
                for entry in joplin_entries
                for css_name in entry["fields"].get("custom_css_files", [])
                if str(css_name).strip()
            }
        ),
        "joplin_database_present": any(entry["fields"].get("database_present") is True for entry in joplin_entries),
    }

    creator_tool_families: set[str] = set()
    if obs_studio_profile_entries or obs_studio_scene_entries:
        creator_tool_families.add("obs_studio")

    creator_profile = {
        "present": bool(creator_tool_families),
        "tool_families": sorted(creator_tool_families),
        "profile_names": sorted(
            {
                str(entry["fields"].get("profile_name") or "").strip()
                for entry in obs_studio_profile_entries
                if str(entry["fields"].get("profile_name") or "").strip()
            }
        ),
        "scene_collection_names": sorted(
            {
                str(entry["fields"].get("collection_name") or "").strip()
                for entry in obs_studio_scene_entries
                if str(entry["fields"].get("collection_name") or "").strip()
            }
        ),
        "recording_formats": sorted(
            {
                str(entry["fields"].get("recording_format") or "").strip()
                for entry in obs_studio_profile_entries
                if str(entry["fields"].get("recording_format") or "").strip()
            }
        ),
        "streaming_services": sorted(
            {
                str(service).strip()
                for entry in obs_studio_profile_entries
                for service in entry["fields"].get("streaming_services", [])
                if str(service).strip()
            }
        ),
        "scene_count": sum(int(entry["fields"].get("scene_count", 0)) for entry in obs_studio_scene_entries),
        "source_types": sorted(
            {
                str(source_type).strip()
                for entry in obs_studio_scene_entries
                for source_type in entry["fields"].get("source_types", [])
                if str(source_type).strip()
            }
        ),
    }

    container_tools_profile = {
        "present": bool(docker_desktop_entries),
        "tool_families": ["docker_desktop"] if docker_desktop_entries else [],
        "uses_wsl_engine": any(entry["fields"].get("uses_wsl_engine") is True for entry in docker_desktop_entries),
        "kubernetes_enabled": any(entry["fields"].get("kubernetes_enabled") is True for entry in docker_desktop_entries),
        "extensions_enabled": any(entry["fields"].get("extensions_enabled") is True for entry in docker_desktop_entries),
        "marketplace_only_extensions": any(
            entry["fields"].get("marketplace_only_extensions") is True for entry in docker_desktop_entries
        ),
        "model_runner_enabled": any(entry["fields"].get("model_runner_enabled") is True for entry in docker_desktop_entries),
        "model_runner_tcp_enabled": any(
            entry["fields"].get("model_runner_tcp_enabled") is True for entry in docker_desktop_entries
        ),
        "model_runner_tcp_port": next(
            (
                int(entry["fields"]["model_runner_tcp_port"])
                for entry in docker_desktop_entries
                if entry["fields"].get("model_runner_tcp_port") is not None
            ),
            None,
        ),
        "desktop_terminal_enabled": any(
            entry["fields"].get("desktop_terminal_enabled") is True for entry in docker_desktop_entries
        ),
        "exposes_docker_api_tcp_2375": any(
            entry["fields"].get("exposes_docker_api_tcp_2375") is True for entry in docker_desktop_entries
        ),
        "enhanced_container_isolation": any(
            entry["fields"].get("enhanced_container_isolation") is True for entry in docker_desktop_entries
        ),
    }

    all_wsl_distros = [
        distro
        for entry in wsl_distribution_entries
        for distro in entry["fields"].get("distros", [])
        if isinstance(distro, dict)
    ]
    linux_runtime_profile = {
        "present": bool(wsl_global_entries or wsl_distribution_entries),
        "tool_families": ["wsl"] if (wsl_global_entries or wsl_distribution_entries) else [],
        "default_distro": next(
            (
                str(entry["fields"].get("default_distro"))
                for entry in wsl_distribution_entries
                if entry["fields"].get("default_distro")
            ),
            None,
        ),
        "distro_names": sorted(
            {
                str(distro.get("name") or "").strip()
                for distro in all_wsl_distros
                if str(distro.get("name") or "").strip()
            }
        ),
        "running_distros": sorted(
            {
                str(distro.get("name") or "").strip()
                for distro in all_wsl_distros
                if str(distro.get("name") or "").strip()
                and str(distro.get("state") or "").strip().lower() == "running"
            }
        ),
        "wsl_versions": sorted(
            {
                int(distro.get("version"))
                for distro in all_wsl_distros
                if distro.get("version") is not None
            }
        ),
        "memory_limit": next(
            (entry["fields"].get("memory_limit") for entry in wsl_global_entries if entry["fields"].get("memory_limit")),
            None,
        ),
        "processor_count": next(
            (
                int(entry["fields"]["processor_count"])
                for entry in wsl_global_entries
                if entry["fields"].get("processor_count") is not None
            ),
            None,
        ),
        "localhost_forwarding": next(
            (
                entry["fields"].get("localhost_forwarding")
                for entry in wsl_global_entries
                if entry["fields"].get("localhost_forwarding") is not None
            ),
            None,
        ),
        "networking_mode": next(
            (
                entry["fields"].get("networking_mode")
                for entry in wsl_global_entries
                if entry["fields"].get("networking_mode")
            ),
            None,
        ),
        "nested_virtualization": next(
            (
                entry["fields"].get("nested_virtualization")
                for entry in wsl_global_entries
                if entry["fields"].get("nested_virtualization") is not None
            ),
            None,
        ),
        "swap_size": next(
            (entry["fields"].get("swap_size") for entry in wsl_global_entries if entry["fields"].get("swap_size")),
            None,
        ),
        "auto_memory_reclaim": next(
            (
                entry["fields"].get("auto_memory_reclaim")
                for entry in wsl_global_entries
                if entry["fields"].get("auto_memory_reclaim")
            ),
            None,
        ),
        "sparse_vhd": next(
            (
                entry["fields"].get("sparse_vhd")
                for entry in wsl_global_entries
                if entry["fields"].get("sparse_vhd") is not None
            ),
            None,
        ),
    }

    social_tools_profile = {
        "present": bool(discord_entries),
        "tool_families": ["discord"] if discord_entries else [],
        "discord_open_on_startup": next(
            (
                entry["fields"].get("open_on_startup")
                for entry in discord_entries
                if "open_on_startup" in entry["fields"]
            ),
            None,
        ),
        "discord_theme": next(
            (entry["fields"].get("theme") for entry in discord_entries if entry["fields"].get("theme")),
            None,
        ),
        "discord_status": next(
            (entry["fields"].get("status") for entry in discord_entries if entry["fields"].get("status")),
            None,
        ),
        "discord_locale": next(
            (entry["fields"].get("locale") for entry in discord_entries if entry["fields"].get("locale")),
            None,
        ),
    }

    collaboration_profile = {
        "present": bool(teams_entries),
        "tool_families": ["microsoft_teams"] if teams_entries else [],
        "teams_client_variants": sorted(
            {
                str(entry["fields"].get("client_variant") or "").strip()
                for entry in teams_entries
                if str(entry["fields"].get("client_variant") or "").strip()
            }
        ),
        "teams_open_at_login": next(
            (
                entry["fields"].get("open_at_login")
                for entry in teams_entries
                if entry["fields"].get("open_at_login") is not None
            ),
            None,
        ),
        "teams_open_as_hidden": next(
            (
                entry["fields"].get("open_as_hidden")
                for entry in teams_entries
                if entry["fields"].get("open_as_hidden") is not None
            ),
            None,
        ),
        "teams_running_on_close": next(
            (
                entry["fields"].get("running_on_close")
                for entry in teams_entries
                if entry["fields"].get("running_on_close") is not None
            ),
            None,
        ),
        "teams_disable_gpu": next(
            (
                entry["fields"].get("disable_gpu")
                for entry in teams_entries
                if entry["fields"].get("disable_gpu") is not None
            ),
            None,
        ),
        "teams_themes": sorted(
            {
                str(entry["fields"].get("theme") or "").strip()
                for entry in teams_entries
                if str(entry["fields"].get("theme") or "").strip()
            }
        ),
        "teams_locales": sorted(
            {
                str(entry["fields"].get("locale") or "").strip()
                for entry in teams_entries
                if str(entry["fields"].get("locale") or "").strip()
            }
        ),
    }

    dropbox_accounts = [
        account
        for entry in dropbox_entries
        for account in entry["fields"].get("accounts", [])
        if isinstance(account, dict)
    ]
    sync_storage_profile = {
        "present": bool(dropbox_entries or onedrive_entries or nextcloud_entries or syncthing_entries),
        "tool_families": sorted(
            [
                *(["dropbox"] if dropbox_entries else []),
                *(["onedrive"] if onedrive_entries else []),
                *(["nextcloud"] if nextcloud_entries else []),
                *(["syncthing"] if syncthing_entries else []),
            ]
        ),
        "dropbox_account_types": sorted(
            {
                str(account.get("account_type") or "").strip()
                for account in dropbox_accounts
                if str(account.get("account_type") or "").strip()
            }
        ),
        "dropbox_paths": sorted(
            {
                str(account.get("path") or "").strip()
                for account in dropbox_accounts
                if str(account.get("path") or "").strip()
            }
        ),
        "dropbox_hosts": sorted(
            {
                str(account.get("host") or "").strip()
                for account in dropbox_accounts
                if str(account.get("host") or "").strip()
            }
        ),
        "onedrive_account_slots": sorted(
            {
                str(entry["fields"].get("account_slot") or "").strip()
                for entry in onedrive_entries
                if str(entry["fields"].get("account_slot") or "").strip()
            }
        ),
        "onedrive_account_types": sorted(
            {
                str(entry["fields"].get("account_type") or "").strip()
                for entry in onedrive_entries
                if str(entry["fields"].get("account_type") or "").strip()
            }
        ),
        "onedrive_mount_points": sorted(
            {
                str(entry["fields"].get("mount_point") or "").strip()
                for entry in onedrive_entries
                if str(entry["fields"].get("mount_point") or "").strip()
            }
        ),
        "onedrive_tenants": sorted(
            {
                str(entry["fields"].get("tenant_name") or "").strip()
                for entry in onedrive_entries
                if str(entry["fields"].get("tenant_name") or "").strip()
            }
        ),
        "onedrive_site_titles": sorted(
            {
                str(entry["fields"].get("site_title") or "").strip()
                for entry in onedrive_entries
                if str(entry["fields"].get("site_title") or "").strip()
            }
        ),
        "onedrive_files_on_demand_enabled": next(
            (
                entry["fields"].get("files_on_demand_enabled")
                for entry in onedrive_entries
                if entry["fields"].get("files_on_demand_enabled") is not None
            ),
            None,
        ),
        "onedrive_coauth_enabled": next(
            (
                entry["fields"].get("coauth_enabled")
                for entry in onedrive_entries
                if entry["fields"].get("coauth_enabled") is not None
            ),
            None,
        ),
        "nextcloud_urls": sorted(
            {
                str(account.get("url") or "").strip()
                for entry in nextcloud_entries
                for account in entry["fields"].get("accounts", [])
                if isinstance(account, dict) and str(account.get("url") or "").strip()
            }
        ),
        "nextcloud_display_names": sorted(
            {
                str(account.get("display_name") or "").strip()
                for entry in nextcloud_entries
                for account in entry["fields"].get("accounts", [])
                if isinstance(account, dict) and str(account.get("display_name") or "").strip()
            }
        ),
        "nextcloud_local_paths": sorted(
            {
                str(local_path).strip()
                for entry in nextcloud_entries
                for account in entry["fields"].get("accounts", [])
                if isinstance(account, dict)
                for local_path in account.get("local_paths", [])
                if str(local_path).strip()
            }
        ),
        "nextcloud_launch_on_startup": next(
            (
                entry["fields"].get("launch_on_system_startup")
                for entry in nextcloud_entries
                if entry["fields"].get("launch_on_system_startup") is not None
            ),
            None,
        ),
        "nextcloud_move_to_trash": next(
            (
                entry["fields"].get("move_to_trash")
                for entry in nextcloud_entries
                if entry["fields"].get("move_to_trash") is not None
            ),
            None,
        ),
        "nextcloud_paused_folder_count": sum(int(entry["fields"].get("paused_folder_count", 0)) for entry in nextcloud_entries),
        "syncthing_folder_ids": sorted(
            {
                str(folder.get("id") or "").strip()
                for entry in syncthing_entries
                for folder in entry["fields"].get("folders", [])
                if isinstance(folder, dict) and str(folder.get("id") or "").strip()
            }
        ),
        "syncthing_folder_paths": sorted(
            {
                str(folder.get("path") or "").strip()
                for entry in syncthing_entries
                for folder in entry["fields"].get("folders", [])
                if isinstance(folder, dict) and str(folder.get("path") or "").strip()
            }
        ),
        "syncthing_folder_types": sorted(
            {
                str(folder.get("type") or "").strip()
                for entry in syncthing_entries
                for folder in entry["fields"].get("folders", [])
                if isinstance(folder, dict) and str(folder.get("type") or "").strip()
            }
        ),
        "syncthing_device_names": sorted(
            {
                str(device.get("name") or "").strip()
                for entry in syncthing_entries
                for device in entry["fields"].get("devices", [])
                if isinstance(device, dict) and str(device.get("name") or "").strip()
            }
        ),
        "syncthing_device_count": len(
            {
                str(device.get("id") or "").strip()
                for entry in syncthing_entries
                for device in entry["fields"].get("devices", [])
                if isinstance(device, dict) and str(device.get("id") or "").strip()
            }
        ),
        "syncthing_gui_enabled": next(
            (
                entry["fields"].get("gui_enabled")
                for entry in syncthing_entries
                if entry["fields"].get("gui_enabled") is not None
            ),
            None,
        ),
        "syncthing_gui_tls": next(
            (
                entry["fields"].get("gui_tls")
                for entry in syncthing_entries
                if entry["fields"].get("gui_tls") is not None
            ),
            None,
        ),
        "syncthing_gui_theme": next(
            (
                entry["fields"].get("gui_theme")
                for entry in syncthing_entries
                if entry["fields"].get("gui_theme")
            ),
            None,
        ),
        "syncthing_global_discovery_enabled": next(
            (
                entry["fields"].get("global_announce_enabled")
                for entry in syncthing_entries
                if entry["fields"].get("global_announce_enabled") is not None
            ),
            None,
        ),
        "syncthing_local_discovery_enabled": next(
            (
                entry["fields"].get("local_announce_enabled")
                for entry in syncthing_entries
                if entry["fields"].get("local_announce_enabled") is not None
            ),
            None,
        ),
        "syncthing_relays_enabled": next(
            (
                entry["fields"].get("relays_enabled")
                for entry in syncthing_entries
                if entry["fields"].get("relays_enabled") is not None
            ),
            None,
        ),
    }

    ide_profile = {
        "present": bool(jetbrains_entries),
        "tool_families": ["jetbrains"] if jetbrains_entries else [],
        "jetbrains_products": sorted(
            {
                str(entry["fields"].get("product") or "").strip()
                for entry in jetbrains_entries
                if str(entry["fields"].get("product") or "").strip()
            }
        ),
        "recent_project_paths": sorted(
            {
                str(path).strip()
                for entry in jetbrains_entries
                for path in entry["fields"].get("recent_project_paths", [])
                if str(path).strip()
            }
        ),
    }
    ide_profile["recent_project_count"] = len(ide_profile["recent_project_paths"])

    terminal_tools_profile = {
        "present": bool(windows_terminal_entries),
        "tool_families": ["windows_terminal"] if windows_terminal_entries else [],
        "default_profile": next(
            (
                entry["fields"].get("default_profile")
                for entry in windows_terminal_entries
                if entry["fields"].get("default_profile")
            ),
            None,
        ),
        "profile_names": sorted(
            {
                str(profile.get("name") or "").strip()
                for entry in windows_terminal_entries
                for profile in entry["fields"].get("profiles", [])
                if isinstance(profile, dict) and str(profile.get("name") or "").strip()
            }
        ),
        "profile_sources": sorted(
            {
                str(profile.get("source") or "").strip()
                for entry in windows_terminal_entries
                for profile in entry["fields"].get("profiles", [])
                if isinstance(profile, dict) and str(profile.get("source") or "").strip()
            }
        ),
    }

    ssh_profile = {
        "present": bool(ssh_entries),
        "host_aliases": sorted(
            {
                str(host.get("alias") or "").strip()
                for entry in ssh_entries
                for host in entry["fields"].get("hosts", [])
                if isinstance(host, dict) and str(host.get("alias") or "").strip()
            }
        ),
        "identity_files": sorted(
            {
                str(identity).strip()
                for entry in ssh_entries
                for host in entry["fields"].get("hosts", [])
                if isinstance(host, dict)
                for identity in host.get("identity_files", [])
                if str(identity).strip()
            }
        ),
    }
    ssh_profile["host_count"] = len(ssh_profile["host_aliases"])

    kubernetes_profile = {
        "present": bool(kubeconfig_entries),
        "current_context": next(
            (
                entry["fields"].get("current_context")
                for entry in kubeconfig_entries
                if entry["fields"].get("current_context")
            ),
            None,
        ),
        "context_names": sorted(
            {
                str(name).strip()
                for entry in kubeconfig_entries
                for name in entry["fields"].get("context_names", [])
                if str(name).strip()
            }
        ),
        "cluster_names": sorted(
            {
                str(name).strip()
                for entry in kubeconfig_entries
                for name in entry["fields"].get("cluster_names", [])
                if str(name).strip()
            }
        ),
        "user_names": sorted(
            {
                str(name).strip()
                for entry in kubeconfig_entries
                for name in entry["fields"].get("user_names", [])
                if str(name).strip()
            }
        ),
        "namespace_names": sorted(
            {
                str(name).strip()
                for entry in kubeconfig_entries
                for name in entry["fields"].get("namespace_names", [])
                if str(name).strip()
            }
        ),
    }

    docker_current_context = next(
        (
            entry["fields"].get("current_context")
            for entry in docker_cli_entries
            if entry["fields"].get("current_context")
        ),
        None,
    )
    docker_context_names = sorted(
        {
            str(entry["fields"].get("name") or "").strip()
            for entry in docker_context_entries
            if str(entry["fields"].get("name") or "").strip()
        }
    )
    docker_context_hosts = sorted(
        {
            str(entry["fields"].get("docker_host") or "").strip()
            for entry in docker_context_entries
            if str(entry["fields"].get("docker_host") or "").strip()
        }
    )
    if docker_current_context or docker_context_names or docker_context_hosts:
        container_tools_profile["present"] = True
        container_tools_profile["tool_families"] = sorted(
            set(container_tools_profile["tool_families"]) | {"docker_cli"}
        )
    container_tools_profile["docker_current_context"] = docker_current_context
    container_tools_profile["docker_context_names"] = docker_context_names
    container_tools_profile["docker_context_hosts"] = docker_context_hosts

    aws_profile_names = sorted(
        {
            str(profile.get("name") or "").strip()
            for entry in aws_cli_entries
            for profile in entry["fields"].get("profiles", [])
            if isinstance(profile, dict) and str(profile.get("name") or "").strip()
        }
    )
    aws_regions = sorted(
        {
            str(profile.get("region") or "").strip()
            for entry in aws_cli_entries
            for profile in entry["fields"].get("profiles", [])
            if isinstance(profile, dict) and str(profile.get("region") or "").strip()
        }
    )
    aws_outputs = sorted(
        {
            str(profile.get("output") or "").strip()
            for entry in aws_cli_entries
            for profile in entry["fields"].get("profiles", [])
            if isinstance(profile, dict) and str(profile.get("output") or "").strip()
        }
    )
    aws_sso_sessions = sorted(
        {
            str(session).strip()
            for entry in aws_cli_entries
            for session in entry["fields"].get("sso_sessions", [])
            if str(session).strip()
        }
        | {
            str(profile.get("sso_session") or "").strip()
            for entry in aws_cli_entries
            for profile in entry["fields"].get("profiles", [])
            if isinstance(profile, dict) and str(profile.get("sso_session") or "").strip()
        }
    )
    azure_default_subscription = next(
        (
            entry["fields"].get("default_subscription")
            for entry in azure_cli_entries
            if entry["fields"].get("default_subscription")
        ),
        None,
    )
    azure_subscription_names = sorted(
        {
            str(subscription.get("name") or "").strip()
            for entry in azure_cli_entries
            for subscription in entry["fields"].get("subscriptions", [])
            if isinstance(subscription, dict) and str(subscription.get("name") or "").strip()
        }
    )
    azure_cloud_names = sorted(
        {
            str(subscription.get("cloud_name") or "").strip()
            for entry in azure_cli_entries
            for subscription in entry["fields"].get("subscriptions", [])
            if isinstance(subscription, dict) and str(subscription.get("cloud_name") or "").strip()
        }
    )
    gcloud_active_configuration = next(
        (
            entry["fields"].get("active_configuration")
            for entry in gcloud_active_entries
            if entry["fields"].get("active_configuration")
        ),
        None,
    )
    gcloud_configuration_names = sorted(
        {
            str(entry["fields"].get("configuration_name") or "").strip()
            for entry in gcloud_cli_entries
            if str(entry["fields"].get("configuration_name") or "").strip()
        }
    )
    gcloud_projects = sorted(
        {
            str(entry["fields"].get("project") or "").strip()
            for entry in gcloud_cli_entries
            if str(entry["fields"].get("project") or "").strip()
        }
    )
    gcloud_regions = sorted(
        {
            str(entry["fields"].get("region") or "").strip()
            for entry in gcloud_cli_entries
            if str(entry["fields"].get("region") or "").strip()
        }
    )
    gcloud_zones = sorted(
        {
            str(entry["fields"].get("zone") or "").strip()
            for entry in gcloud_cli_entries
            if str(entry["fields"].get("zone") or "").strip()
        }
    )
    cloud_tool_families: list[str] = []
    if aws_profile_names or aws_regions or aws_outputs or aws_sso_sessions:
        cloud_tool_families.append("aws_cli")
    if azure_default_subscription or azure_subscription_names or azure_cloud_names:
        cloud_tool_families.append("azure_cli")
    if (
        gcloud_active_configuration
        or gcloud_configuration_names
        or gcloud_projects
        or gcloud_regions
        or gcloud_zones
    ):
        cloud_tool_families.append("gcloud_cli")
    cloud_tools_profile = {
        "present": bool(cloud_tool_families),
        "tool_families": cloud_tool_families,
        "aws_profile_names": aws_profile_names,
        "aws_regions": aws_regions,
        "aws_outputs": aws_outputs,
        "aws_sso_sessions": aws_sso_sessions,
        "azure_default_subscription": azure_default_subscription,
        "azure_subscription_names": azure_subscription_names,
        "azure_cloud_names": azure_cloud_names,
        "gcloud_active_configuration": gcloud_active_configuration,
        "gcloud_configuration_names": gcloud_configuration_names,
        "gcloud_projects": gcloud_projects,
        "gcloud_regions": gcloud_regions,
        "gcloud_zones": gcloud_zones,
    }

    gh_hosts = sorted(
        {
            str(host.get("host") or "").strip()
            for entry in github_cli_hosts_entries
            for host in entry["fields"].get("hosts", [])
            if isinstance(host, dict) and str(host.get("host") or "").strip()
        }
    )
    gh_users = sorted(
        {
            str(host.get("user") or "").strip()
            for entry in github_cli_hosts_entries
            for host in entry["fields"].get("hosts", [])
            if isinstance(host, dict) and str(host.get("user") or "").strip()
        }
    )
    gh_git_protocols = sorted(
        {
            str(protocol).strip()
            for protocol in [
                *[
                    entry["fields"].get("git_protocol")
                    for entry in github_cli_config_entries
                    if entry["fields"].get("git_protocol")
                ],
                *[
                    host.get("git_protocol")
                    for entry in github_cli_hosts_entries
                    for host in entry["fields"].get("hosts", [])
                    if isinstance(host, dict) and host.get("git_protocol")
                ],
            ]
            if str(protocol).strip()
        }
    )
    gh_authenticated_hosts = sorted(
        {
            str(host.get("host") or "").strip()
            for entry in github_cli_hosts_entries
            for host in entry["fields"].get("hosts", [])
            if isinstance(host, dict)
            and host.get("oauth_token_present") is True
            and str(host.get("host") or "").strip()
        }
    )
    source_control_profile = {
        "present": bool(gh_hosts or gh_users or gh_git_protocols or git_global_entries or npm_user_entries),
        "tool_families": sorted(
            [
                *(
                    ["github_cli"]
                    if (gh_hosts or gh_users or gh_git_protocols)
                    else []
                ),
                *(["git"] if git_global_entries else []),
                *(["npm"] if npm_user_entries else []),
            ]
        ),
        "gh_hosts": gh_hosts,
        "gh_users": gh_users,
        "gh_git_protocols": gh_git_protocols,
        "gh_authenticated_hosts": gh_authenticated_hosts,
        "gh_editor": next(
            (
                entry["fields"].get("editor")
                for entry in github_cli_config_entries
                if entry["fields"].get("editor")
            ),
            None,
        ),
        "gh_prompt": next(
            (
                entry["fields"].get("prompt")
                for entry in github_cli_config_entries
                if entry["fields"].get("prompt")
            ),
            None,
        ),
        "git_user_name": next(
            (
                entry["fields"].get("user_name")
                for entry in git_global_entries
                if entry["fields"].get("user_name")
            ),
            None,
        ),
        "git_user_email": next(
            (
                entry["fields"].get("user_email")
                for entry in git_global_entries
                if entry["fields"].get("user_email")
            ),
            None,
        ),
        "git_default_branch": next(
            (
                entry["fields"].get("default_branch")
                for entry in git_global_entries
                if entry["fields"].get("default_branch")
            ),
            None,
        ),
        "git_editor": next(
            (
                entry["fields"].get("editor")
                for entry in git_global_entries
                if entry["fields"].get("editor")
            ),
            None,
        ),
        "git_pull_rebase": next(
            (
                entry["fields"].get("pull_rebase")
                for entry in git_global_entries
                if entry["fields"].get("pull_rebase") is not None
            ),
            None,
        ),
        "git_github_user": next(
            (
                entry["fields"].get("github_user")
                for entry in git_global_entries
                if entry["fields"].get("github_user")
            ),
            None,
        ),
        "cargo_registry_names": sorted(
            {
                str(name).strip()
                for entry in cargo_credentials_entries
                for name in entry["fields"].get("registry_names", [])
                if str(name).strip()
            }
        ),
        "cargo_credentials_present": bool(
            sum(int(entry["fields"].get("credential_count", 0)) for entry in cargo_credentials_entries)
        ),
        "npm_registry": next(
            (
                entry["fields"].get("registry")
                for entry in npm_user_entries
                if entry["fields"].get("registry")
            ),
            None,
        ),
        "npm_scope_registries": sorted(
            {
                str(scope).strip()
                for entry in npm_user_entries
                for scope in entry["fields"].get("scope_registries", [])
                if str(scope).strip()
            }
        ),
        "npm_save_exact": next(
            (
                entry["fields"].get("save_exact")
                for entry in npm_user_entries
                if entry["fields"].get("save_exact") is not None
            ),
            None,
        ),
        "npm_prefix": next(
            (
                entry["fields"].get("prefix")
                for entry in npm_user_entries
                if entry["fields"].get("prefix")
            ),
            None,
        ),
    }

    rust_tooling_families = sorted(
        [
            *(["cargo"] if (cargo_user_entries or cargo_credentials_entries) else []),
            *(["rustup"] if rustup_settings_entries else []),
        ]
    )
    rust_tooling_profile = {
        "present": bool(rust_tooling_families),
        "tool_families": rust_tooling_families,
        "cargo_target_dir": next(
            (
                entry["fields"].get("target_dir")
                for entry in cargo_user_entries
                if entry["fields"].get("target_dir")
            ),
            None,
        ),
        "cargo_term_verbose": next(
            (
                entry["fields"].get("term_verbose")
                for entry in cargo_user_entries
                if entry["fields"].get("term_verbose") is not None
            ),
            None,
        ),
        "cargo_default_registry": next(
            (
                entry["fields"].get("default_registry")
                for entry in cargo_user_entries
                if entry["fields"].get("default_registry")
            ),
            None,
        ),
        "cargo_crates_io_protocol": next(
            (
                entry["fields"].get("crates_io_protocol")
                for entry in cargo_user_entries
                if entry["fields"].get("crates_io_protocol")
            ),
            None,
        ),
        "cargo_git_fetch_with_cli": next(
            (
                entry["fields"].get("git_fetch_with_cli")
                for entry in cargo_user_entries
                if entry["fields"].get("git_fetch_with_cli") is not None
            ),
            None,
        ),
        "cargo_net_retry": next(
            (
                entry["fields"].get("net_retry")
                for entry in cargo_user_entries
                if entry["fields"].get("net_retry") is not None
            ),
            None,
        ),
        "cargo_credentials_present": bool(
            sum(int(entry["fields"].get("credential_count", 0)) for entry in cargo_credentials_entries)
        ),
        "cargo_registry_names": sorted(
            {
                str(name).strip()
                for entry in cargo_credentials_entries
                for name in entry["fields"].get("registry_names", [])
                if str(name).strip()
            }
        ),
        "rustup_default_toolchain": next(
            (
                entry["fields"].get("default_toolchain")
                for entry in rustup_settings_entries
                if entry["fields"].get("default_toolchain")
            ),
            None,
        ),
        "rustup_profile": next(
            (
                entry["fields"].get("profile")
                for entry in rustup_settings_entries
                if entry["fields"].get("profile")
            ),
            None,
        ),
        "rustup_override_count": sum(int(entry["fields"].get("override_count", 0)) for entry in rustup_settings_entries),
        "rustup_settings_version": next(
            (
                entry["fields"].get("version")
                for entry in rustup_settings_entries
                if entry["fields"].get("version")
            ),
            None,
        ),
    }

    jvm_tooling_families = sorted(
        [
            *(["gradle"] if gradle_user_entries else []),
            *(["maven"] if maven_user_entries else []),
        ]
    )
    jvm_tooling_profile = {
        "present": bool(jvm_tooling_families),
        "tool_families": jvm_tooling_families,
        "maven_local_repository": next(
            (
                entry["fields"].get("local_repository")
                for entry in maven_user_entries
                if entry["fields"].get("local_repository")
            ),
            None,
        ),
        "maven_offline": next(
            (
                entry["fields"].get("offline")
                for entry in maven_user_entries
                if entry["fields"].get("offline") is not None
            ),
            None,
        ),
        "maven_plugin_groups": sorted(
            {
                str(item).strip()
                for entry in maven_user_entries
                for item in entry["fields"].get("plugin_groups", [])
                if str(item).strip()
            }
        ),
        "maven_mirror_ids": sorted(
            {
                str(item).strip()
                for entry in maven_user_entries
                for item in entry["fields"].get("mirror_ids", [])
                if str(item).strip()
            }
        ),
        "maven_mirror_urls": sorted(
            {
                str(item).strip()
                for entry in maven_user_entries
                for item in entry["fields"].get("mirror_urls", [])
                if str(item).strip()
            }
        ),
        "maven_server_ids": sorted(
            {
                str(item).strip()
                for entry in maven_user_entries
                for item in entry["fields"].get("server_ids", [])
                if str(item).strip()
            }
        ),
        "maven_credentials_present": bool(
            {
                str(item).strip()
                for entry in maven_user_entries
                for item in entry["fields"].get("credential_server_ids", [])
                if str(item).strip()
            }
        ),
        "maven_active_profiles": sorted(
            {
                str(item).strip()
                for entry in maven_user_entries
                for item in entry["fields"].get("active_profiles", [])
                if str(item).strip()
            }
        ),
        "maven_proxy_hosts": sorted(
            {
                str(item).strip()
                for entry in maven_user_entries
                for item in entry["fields"].get("proxy_hosts", [])
                if str(item).strip()
            }
        ),
        "gradle_caching": next(
            (
                entry["fields"].get("caching")
                for entry in gradle_user_entries
                if entry["fields"].get("caching") is not None
            ),
            None,
        ),
        "gradle_parallel": next(
            (
                entry["fields"].get("parallel")
                for entry in gradle_user_entries
                if entry["fields"].get("parallel") is not None
            ),
            None,
        ),
        "gradle_configuration_cache": next(
            (
                entry["fields"].get("configuration_cache")
                for entry in gradle_user_entries
                if entry["fields"].get("configuration_cache") is not None
            ),
            None,
        ),
        "gradle_daemon": next(
            (
                entry["fields"].get("daemon")
                for entry in gradle_user_entries
                if entry["fields"].get("daemon") is not None
            ),
            None,
        ),
        "gradle_jvmargs": next(
            (
                entry["fields"].get("jvmargs")
                for entry in gradle_user_entries
                if entry["fields"].get("jvmargs")
            ),
            None,
        ),
        "gradle_java_home": next(
            (
                entry["fields"].get("java_home")
                for entry in gradle_user_entries
                if entry["fields"].get("java_home")
            ),
            None,
        ),
    }

    dotnet_tooling_families = sorted(
        [
            *(["dotnet_tools"] if dotnet_global_tool_entries else []),
            *(["nuget"] if nuget_user_entries else []),
        ]
    )
    dotnet_tooling_profile = {
        "present": bool(dotnet_tooling_families),
        "tool_families": dotnet_tooling_families,
        "nuget_global_packages_folder": next(
            (
                entry["fields"].get("global_packages_folder")
                for entry in nuget_user_entries
                if entry["fields"].get("global_packages_folder")
            ),
            None,
        ),
        "nuget_default_push_source": next(
            (
                entry["fields"].get("default_push_source")
                for entry in nuget_user_entries
                if entry["fields"].get("default_push_source")
            ),
            None,
        ),
        "nuget_signature_validation_mode": next(
            (
                entry["fields"].get("signature_validation_mode")
                for entry in nuget_user_entries
                if entry["fields"].get("signature_validation_mode")
            ),
            None,
        ),
        "nuget_package_source_names": sorted(
            {
                str(item).strip()
                for entry in nuget_user_entries
                for item in entry["fields"].get("package_source_names", [])
                if str(item).strip()
            }
        ),
        "nuget_package_source_urls": sorted(
            {
                str(item).strip()
                for entry in nuget_user_entries
                for item in entry["fields"].get("package_source_urls", [])
                if str(item).strip()
            }
        ),
        "nuget_disabled_sources": sorted(
            {
                str(item).strip()
                for entry in nuget_user_entries
                for item in entry["fields"].get("disabled_sources", [])
                if str(item).strip()
            }
        ),
        "nuget_credentials_present": bool(
            {
                str(item).strip()
                for entry in nuget_user_entries
                for item in entry["fields"].get("credential_sources", [])
                if str(item).strip()
            }
        ),
        "nuget_credential_sources": sorted(
            {
                str(item).strip()
                for entry in nuget_user_entries
                for item in entry["fields"].get("credential_sources", [])
                if str(item).strip()
            }
        ),
        "global_tool_commands": sorted(
            {
                str(item).strip()
                for entry in dotnet_global_tool_entries
                for item in entry["fields"].get("commands", [])
                if str(item).strip()
            }
        ),
        "global_tool_count": sum(int(entry["fields"].get("command_count", 0)) for entry in dotnet_global_tool_entries),
    }

    python_tooling_families = sorted(
        [
            *(["conda"] if conda_user_entries else []),
            *(["pip"] if pip_user_entries else []),
            *(["poetry"] if poetry_user_entries else []),
            *(["uv"] if (uv_user_entries or uv_credentials_entries) else []),
        ]
    )
    python_tooling_profile = {
        "present": bool(python_tooling_families),
        "tool_families": python_tooling_families,
        "pip_index_url": next(
            (
                entry["fields"].get("index_url")
                for entry in pip_user_entries
                if entry["fields"].get("index_url")
            ),
            None,
        ),
        "pip_trusted_hosts": sorted(
            {
                str(host).strip()
                for entry in pip_user_entries
                for host in entry["fields"].get("trusted_hosts", [])
                if str(host).strip()
            }
        ),
        "pip_extra_index_urls": sorted(
            {
                str(url).strip()
                for entry in pip_user_entries
                for url in entry["fields"].get("extra_index_urls", [])
                if str(url).strip()
            }
        ),
        "pip_timeout": next(
            (
                entry["fields"].get("timeout")
                for entry in pip_user_entries
                if entry["fields"].get("timeout") is not None
            ),
            None,
        ),
        "pip_disable_version_check": next(
            (
                entry["fields"].get("disable_version_check")
                for entry in pip_user_entries
                if entry["fields"].get("disable_version_check") is not None
            ),
            None,
        ),
        "conda_channels": sorted(
            {
                str(channel).strip()
                for entry in conda_user_entries
                for channel in entry["fields"].get("channels", [])
                if str(channel).strip()
            }
        ),
        "conda_envs_dirs": sorted(
            {
                str(path).strip()
                for entry in conda_user_entries
                for path in entry["fields"].get("envs_dirs", [])
                if str(path).strip()
            }
        ),
        "conda_auto_activate_base": next(
            (
                entry["fields"].get("auto_activate_base")
                for entry in conda_user_entries
                if entry["fields"].get("auto_activate_base") is not None
            ),
            None,
        ),
        "conda_changeps1": next(
            (
                entry["fields"].get("changeps1")
                for entry in conda_user_entries
                if entry["fields"].get("changeps1") is not None
            ),
            None,
        ),
        "conda_show_channel_urls": next(
            (
                entry["fields"].get("show_channel_urls")
                for entry in conda_user_entries
                if entry["fields"].get("show_channel_urls") is not None
            ),
            None,
        ),
        "poetry_virtualenvs_create": next(
            (
                entry["fields"].get("virtualenvs_create")
                for entry in poetry_user_entries
                if entry["fields"].get("virtualenvs_create") is not None
            ),
            None,
        ),
        "poetry_virtualenvs_in_project": next(
            (
                entry["fields"].get("virtualenvs_in_project")
                for entry in poetry_user_entries
                if entry["fields"].get("virtualenvs_in_project") is not None
            ),
            None,
        ),
        "poetry_virtualenvs_path": next(
            (
                entry["fields"].get("virtualenvs_path")
                for entry in poetry_user_entries
                if entry["fields"].get("virtualenvs_path")
            ),
            None,
        ),
        "poetry_installer_parallel": next(
            (
                entry["fields"].get("installer_parallel")
                for entry in poetry_user_entries
                if entry["fields"].get("installer_parallel") is not None
            ),
            None,
        ),
        "poetry_installer_max_workers": next(
            (
                entry["fields"].get("installer_max_workers")
                for entry in poetry_user_entries
                if entry["fields"].get("installer_max_workers") is not None
            ),
            None,
        ),
        "poetry_system_git_client": next(
            (
                entry["fields"].get("system_git_client")
                for entry in poetry_user_entries
                if entry["fields"].get("system_git_client") is not None
            ),
            None,
        ),
        "uv_index_url": next(
            (
                entry["fields"].get("index_url")
                for entry in uv_user_entries
                if entry["fields"].get("index_url")
            ),
            None,
        ),
        "uv_extra_index_urls": sorted(
            {
                str(url).strip()
                for entry in uv_user_entries
                for url in entry["fields"].get("extra_index_urls", [])
                if str(url).strip()
            }
        ),
        "uv_cache_dir": next(
            (
                entry["fields"].get("cache_dir")
                for entry in uv_user_entries
                if entry["fields"].get("cache_dir")
            ),
            None,
        ),
        "uv_python_preference": next(
            (
                entry["fields"].get("python_preference")
                for entry in uv_user_entries
                if entry["fields"].get("python_preference")
            ),
            None,
        ),
        "uv_native_tls": next(
            (
                entry["fields"].get("native_tls")
                for entry in uv_user_entries
                if entry["fields"].get("native_tls") is not None
            ),
            None,
        ),
        "uv_offline": next(
            (
                entry["fields"].get("offline")
                for entry in uv_user_entries
                if entry["fields"].get("offline") is not None
            ),
            None,
        ),
        "uv_preview": next(
            (
                entry["fields"].get("preview")
                for entry in uv_user_entries
                if entry["fields"].get("preview") is not None
            ),
            None,
        ),
        "uv_pip_index_url": next(
            (
                entry["fields"].get("pip_index_url")
                for entry in uv_user_entries
                if entry["fields"].get("pip_index_url")
            ),
            None,
        ),
        "uv_auth_present": bool(
            sum(int(entry["fields"].get("credential_count", 0)) for entry in uv_credentials_entries)
        ),
        "uv_auth_service_urls": sorted(
            {
                str(credential.get("url") or "").strip()
                for entry in uv_credentials_entries
                for credential in entry["fields"].get("credentials", [])
                if isinstance(credential, dict) and str(credential.get("url") or "").strip()
            }
        ),
        "uv_auth_usernames": sorted(
            {
                str(credential.get("username") or "").strip()
                for entry in uv_credentials_entries
                for credential in entry["fields"].get("credentials", [])
                if isinstance(credential, dict) and str(credential.get("username") or "").strip()
            }
        ),
    }

    javascript_tooling_families = sorted(
        [
            *(["pnpm"] if pnpm_user_entries else []),
            *(["yarn"] if yarn_user_entries else []),
        ]
    )
    javascript_tooling_profile = {
        "present": bool(javascript_tooling_families),
        "tool_families": javascript_tooling_families,
        "pnpm_registry": next(
            (
                entry["fields"].get("registry")
                for entry in pnpm_user_entries
                if entry["fields"].get("registry")
            ),
            None,
        ),
        "pnpm_scope_registries": sorted(
            {
                str(scope).strip()
                for entry in pnpm_user_entries
                for scope in entry["fields"].get("scope_registries", [])
                if str(scope).strip()
            }
        ),
        "pnpm_global_dir": next(
            (
                entry["fields"].get("global_dir")
                for entry in pnpm_user_entries
                if entry["fields"].get("global_dir")
            ),
            None,
        ),
        "pnpm_store_dir": next(
            (
                entry["fields"].get("store_dir")
                for entry in pnpm_user_entries
                if entry["fields"].get("store_dir")
            ),
            None,
        ),
        "pnpm_package_import_method": next(
            (
                entry["fields"].get("package_import_method")
                for entry in pnpm_user_entries
                if entry["fields"].get("package_import_method")
            ),
            None,
        ),
        "pnpm_node_linker": next(
            (
                entry["fields"].get("node_linker")
                for entry in pnpm_user_entries
                if entry["fields"].get("node_linker")
            ),
            None,
        ),
        "pnpm_shamefully_hoist": next(
            (
                entry["fields"].get("shamefully_hoist")
                for entry in pnpm_user_entries
                if entry["fields"].get("shamefully_hoist") is not None
            ),
            None,
        ),
        "yarn_node_linker": next(
            (
                entry["fields"].get("node_linker")
                for entry in yarn_user_entries
                if entry["fields"].get("node_linker")
            ),
            None,
        ),
        "yarn_npm_registry_server": next(
            (
                entry["fields"].get("npm_registry_server")
                for entry in yarn_user_entries
                if entry["fields"].get("npm_registry_server")
            ),
            None,
        ),
        "yarn_enable_global_cache": next(
            (
                entry["fields"].get("enable_global_cache")
                for entry in yarn_user_entries
                if entry["fields"].get("enable_global_cache") is not None
            ),
            None,
        ),
        "yarn_enable_telemetry": next(
            (
                entry["fields"].get("enable_telemetry")
                for entry in yarn_user_entries
                if entry["fields"].get("enable_telemetry") is not None
            ),
            None,
        ),
        "yarn_global_folder": next(
            (
                entry["fields"].get("global_folder")
                for entry in yarn_user_entries
                if entry["fields"].get("global_folder")
            ),
            None,
        ),
        "yarn_path": next(
            (
                entry["fields"].get("yarn_path")
                for entry in yarn_user_entries
                if entry["fields"].get("yarn_path")
            ),
            None,
        ),
        "yarn_npm_scope_names": sorted(
            {
                str(name).strip()
                for entry in yarn_user_entries
                for name in entry["fields"].get("npm_scope_names", [])
                if str(name).strip()
            }
        ),
        "yarn_npm_scope_registries": sorted(
            {
                str(url).strip()
                for entry in yarn_user_entries
                for url in entry["fields"].get("npm_scope_registries", [])
                if str(url).strip()
            }
        ),
    }

    interest_tags: set[str] = set()
    if developer_profile["is_developer"]:
        interest_tags.add("developer_tools")
    if gaming_profile["is_gamer"]:
        interest_tags.add("gaming")
    if ai_tools_profile["uses_ai_tools"]:
        interest_tags.add("ai_tools")
    if knowledge_tools_profile["present"]:
        interest_tags.add("knowledge_management")
    if creator_profile["present"]:
        interest_tags.add("content_creation")
    if container_tools_profile["present"]:
        interest_tags.add("containers")
    if linux_runtime_profile["present"]:
        interest_tags.add("linux_runtime")
    if social_tools_profile["present"]:
        interest_tags.add("chat_community_tools")
    if collaboration_profile["present"]:
        interest_tags.add("collaboration_tools")
    if ide_profile["present"]:
        interest_tags.add("ide_workflow")
    if terminal_tools_profile["present"]:
        interest_tags.add("terminal_workflow")
    if ssh_profile["present"]:
        interest_tags.add("remote_access")
    if kubernetes_profile["present"]:
        interest_tags.add("kubernetes")
    if cloud_tools_profile["present"]:
        interest_tags.add("cloud_tooling")
    if source_control_profile["present"]:
        interest_tags.add("source_control")
    if dotnet_tooling_profile["present"]:
        interest_tags.add("dotnet_tooling")
    if jvm_tooling_profile["present"]:
        interest_tags.add("jvm_tooling")
    if rust_tooling_profile["present"]:
        interest_tags.add("rust_tooling")
    if python_tooling_profile["present"]:
        interest_tags.add("python_tooling")
    if javascript_tooling_profile["present"]:
        interest_tags.add("javascript_tooling")
    if sync_storage_profile["present"]:
        interest_tags.add("sync_storage")
    if any("github.com" == domain or domain.endswith("github.com") for domain in recent_history_domains + bookmark_domains):
        interest_tags.add("open_source")
    if any("bilibili.com" == domain or domain.endswith("bilibili.com") for domain in recent_history_domains + bookmark_domains):
        interest_tags.add("video")

    activitywatch_bucket_count = sum(int(entry["fields"].get("bucket_count", 0)) for entry in activitywatch_entries)
    activitywatch_watchers = sorted(
        {
            str(watcher)
            for entry in activitywatch_entries
            for watcher in entry["fields"].get("watchers", [])
            if str(watcher).strip()
        }
    )
    all_browser_domains = sorted(set(bookmark_domains + recent_history_domains))
    weighted_browser_domains: dict[str, int] = {}
    for browser_values in per_browser_summary.values():
        for item in browser_values["top_history_domains"]:
            domain = str(item.get("domain") or "").strip()
            if not domain:
                continue
            weighted_browser_domains[domain] = weighted_browser_domains.get(domain, 0) + int(item.get("visit_count", 0))
    weighted_top_domains = [
        {"domain": domain, "visit_count": visit_count}
        for domain, visit_count in sorted(weighted_browser_domains.items(), key=lambda item: (-item[1], item[0]))[:50]
    ]
    merged_search_queries = _merge_search_queries(
        [
            query
            for browser_values in per_browser_summary.values()
            for query in browser_values["top_search_queries"]
        ],
        limit=50,
    )
    search_engines = sorted(
        {
            str(engine)
            for query in merged_search_queries
            for engine in query.get("engines", [])
            if str(engine).strip()
        }
    )
    intent_profile = _derive_intent_profile(
        merged_search_queries=merged_search_queries,
        all_browser_domains=all_browser_domains,
    )
    web_interest_tags: set[str] = set()
    if any(
        domain.endswith(suffix)
        for domain in all_browser_domains
        for suffix in ("developer.mozilla.org", "mozilla.org", "docs.example.com", "support.mozilla.org")
    ):
        web_interest_tags.add("developer_docs")
    if any("github.com" == domain or domain.endswith("github.com") for domain in all_browser_domains):
        web_interest_tags.add("open_source")
    if any(
        domain.endswith(suffix)
        for domain in all_browser_domains
        for suffix in ("openai.com", "platform.openai.com", "auth.openai.com", "perplexity.ai", "krea.ai", "manus.im")
    ):
        web_interest_tags.add("ai_tools")
    if any("bilibili.com" == domain or domain.endswith("bilibili.com") for domain in all_browser_domains):
        web_interest_tags.add("video")
    if any("microsoft.com" == domain or domain.endswith("microsoft.com") for domain in all_browser_domains):
        web_interest_tags.add("microsoft_ecosystem")

    activitywatch_profile = {
        "present": bool(activitywatch_entries),
        "bucket_count": activitywatch_bucket_count,
        "watchers": activitywatch_watchers,
    }
    meaningful_activity_profile = derive_meaningful_activity_profile(
        gaming_profile=gaming_profile,
        developer_profile=developer_profile,
        ai_tools_profile=ai_tools_profile,
        knowledge_tools_profile=knowledge_tools_profile,
        creator_profile=creator_profile,
        sync_storage_profile=sync_storage_profile,
        terminal_tools_profile=terminal_tools_profile,
        cloud_tools_profile=cloud_tools_profile,
        linux_runtime_profile=linux_runtime_profile,
        container_tools_profile=container_tools_profile,
        activitywatch_profile=activitywatch_profile,
    )
    local_signal_coverage = derive_local_signal_coverage(raw_evidence)
    agent_config_profile = derive_agent_config_profile(raw_evidence)
    downloads_profile = derive_downloads_profile(raw_evidence)
    office_documents_profile = derive_office_documents_profile(raw_evidence)
    recent_documents_profile = derive_recent_documents_profile(raw_evidence)
    steam_playtime_profile = derive_steam_playtime_profile(raw_evidence)
    next_questions = derive_next_questions(
        local_signal_coverage=local_signal_coverage,
        meaningful_activity_profile=meaningful_activity_profile,
    )
    llm_context = derive_llm_context(
        gaming_profile=gaming_profile,
        developer_profile=developer_profile,
        ai_tools_profile=ai_tools_profile,
        meaningful_activity_profile=meaningful_activity_profile,
        local_signal_coverage=local_signal_coverage,
        next_questions=next_questions,
    )

    return {
        "tools": {
            "claude": {
                "present": claude_present,
                "entrypoints": [entry["path"] for entry in raw_evidence if entry["entity_kind"] == "claude_entrypoint"],
                "default_models": sorted(set(claude_default_models)),
                "mcp_servers": sorted(set(claude_mcp_servers)),
            },
            "codex": {
                "present": codex_present,
                "trusted_projects": sorted(set(trusted_projects)),
                "mcp_servers": sorted(set(mcp_servers)),
                "auth_present": bool(codex_auth_entries),
                "approved_rule_count": approved_rule_count,
            },
        },
        "game_ecosystem": {
            "platforms": platforms,
            "installed_games": all_games,
        },
        "game_index": game_index,
        "browser_activity": {
            "browsers": sorted(set(browsers)),
            "bookmark_titles": sorted(set(bookmark_titles)),
            "bookmark_domains": sorted(set(bookmark_domains)),
            "recent_history_domains": sorted(set(recent_history_domains)),
            "download_domains": sorted(set(download_domains)),
            "download_file_extensions": sorted(
                {
                    str(item)
                    for values in per_browser_summary.values()
                    for item in values["download_file_extensions"]
                    if str(item).strip()
                }
            ),
            "extension_names": sorted(set(extension_names)),
            "session_domains": sorted(set(session_domains)),
            "session_titles": sorted(set(session_titles)),
            "per_browser": per_browser_summary,
        },
        "web_interest_profile": {
            "interest_tags": sorted(web_interest_tags),
            "top_domains": weighted_top_domains,
        },
        "search_activity": {
            "present": bool(merged_search_queries),
            "engines": search_engines,
            "top_queries": merged_search_queries,
        },
        "intent_profile": intent_profile,
        "active_workspaces": {
            "git_repos": git_repos,
            "workspace_files": workspace_files,
            "recent_editor_workspaces": recent_editor_workspaces,
        },
        "shell_activity": {
            "shells": sorted(set(shells)),
            "recent_commands": recent_commands[-50:],
            "top_commands": top_commands,
        },
        "installed_apps": installed_apps[:200],
        "creative_tools_profile": creative_tools_profile,
        "hardware_profile": hardware_profile,
        "privacy_security_profile": privacy_security_profile,
        "agent_config_profile": agent_config_profile,
        "downloads_profile": downloads_profile,
        "office_documents_profile": office_documents_profile,
        "recent_documents_profile": recent_documents_profile,
        "developer_profile": developer_profile,
        "gaming_profile": gaming_profile,
        "steam_playtime_profile": steam_playtime_profile,
        "ai_tools_profile": ai_tools_profile,
        "knowledge_tools_profile": knowledge_tools_profile,
        "creator_profile": creator_profile,
        "container_tools_profile": container_tools_profile,
        "linux_runtime_profile": linux_runtime_profile,
        "social_tools_profile": social_tools_profile,
        "collaboration_tools_profile": collaboration_profile,
        "sync_storage_profile": sync_storage_profile,
        "ide_profile": ide_profile,
        "terminal_tools_profile": terminal_tools_profile,
        "ssh_profile": ssh_profile,
        "kubernetes_profile": kubernetes_profile,
        "cloud_tools_profile": cloud_tools_profile,
        "source_control_profile": source_control_profile,
        "dotnet_tooling_profile": dotnet_tooling_profile,
        "jvm_tooling_profile": jvm_tooling_profile,
        "rust_tooling_profile": rust_tooling_profile,
        "python_tooling_profile": python_tooling_profile,
        "javascript_tooling_profile": javascript_tooling_profile,
        "activitywatch_profile": activitywatch_profile,
        "meaningful_activity_profile": meaningful_activity_profile,
        "local_signal_coverage": local_signal_coverage,
        "next_questions": next_questions,
        "llm_context": llm_context,
        "interest_tags": sorted(interest_tags),
        "sqlite_artifacts": sqlite_profile,
    }


def _merge_search_queries(items: list[dict], limit: int) -> list[dict]:
    query_counts: dict[str, int] = {}
    query_engines: dict[str, set[str]] = {}
    query_variants: dict[str, set[str]] = {}
    query_sources: dict[str, set[str]] = {}

    for item in items:
        raw_query = str(item.get("query") or "")
        query = _canonicalize_query(raw_query)
        if not query:
            continue
        query_counts[query] = query_counts.get(query, 0) + int(item.get("visit_count", 0))
        engines = query_engines.get(query)
        if engines is None:
            engines = set()
            query_engines[query] = engines
        variants = query_variants.get(query)
        if variants is None:
            variants = set()
            query_variants[query] = variants
        sources = query_sources.get(query)
        if sources is None:
            sources = set()
            query_sources[query] = sources
        upstream_variants = item.get("normalized_from", [])
        if isinstance(upstream_variants, list) and upstream_variants:
            for variant in upstream_variants:
                normalized_variant = _display_normalized_variant(str(variant))
                if normalized_variant:
                    variants.add(normalized_variant)
        else:
            canonical_variant = _display_normalized_variant(raw_query)
            if canonical_variant:
                variants.add(canonical_variant)
        for engine in item.get("engines", []):
            engine_name = str(engine).strip()
            if engine_name:
                engines.add(engine_name)
        for source in item.get("sources", []):
            source_name = str(source).strip()
            if source_name:
                sources.add(source_name)

    merged_items: list[dict] = []
    for query, visit_count in sorted(query_counts.items(), key=lambda item: (-item[1], item[0]))[:limit]:
        entry = {
            "query": query,
            "visit_count": visit_count,
            "engines": sorted(query_engines.get(query, set())),
        }
        variants = sorted(query_variants.get(query, {query}))
        if variants != [query]:
            entry["normalized_from"] = variants
        sources = sorted(query_sources.get(query, set()))
        if sources:
            entry["sources"] = sources
        merged_items.append(entry)
    return merged_items


def _canonicalize_query(query: str) -> str:
    normalized = " ".join(query.strip().lower().split())
    if not normalized:
        return ""

    compact = normalized.replace(" ", "")
    if normalized in {"bilibili", "b \u7ad9", "b\u7ad9"} or compact == "b\u7ad9":
        return "bilibili"
    if normalized.startswith("discord"):
        return "discord"
    if normalized == "steam" or normalized == "steam deck":
        return "steam"
    return normalized


def _display_normalized_variant(query: str) -> str:
    normalized = " ".join(query.strip().lower().split())
    if not normalized:
        return ""

    compact = normalized.replace(" ", "")
    if normalized in {"b \u7ad9", "b\u7ad9"} or compact == "b\u7ad9":
        return "b station"
    return normalized


def _derive_intent_profile(merged_search_queries: list[dict], all_browser_domains: list[str]) -> dict:
    domain_set = {str(domain).strip().lower() for domain in all_browser_domains if str(domain).strip()}
    intent_scores: dict[str, int] = {}
    query_examples: list[dict] = []

    for item in merged_search_queries:
        query = str(item.get("query") or "").strip()
        visit_count = int(item.get("visit_count", 0))
        engines = sorted(str(engine) for engine in item.get("engines", []) if str(engine).strip())
        if not query or visit_count <= 0:
            continue

        normalized_query = query.lower()
        intent_tags: list[str] = []

        if any(keyword in normalized_query for keyword in {"openai", "claude", "codex", "agentic", "llm", "gpt"}):
            intent_tags.append("ai_tools")
        if any(
            keyword in normalized_query
            for keyword in {
                "playwright",
                "fixture",
                "config",
                "docs",
                "developer",
                "engineering",
                "pattern",
                "memory system",
                "context",
            }
        ):
            intent_tags.append("developer_research")
        if any(keyword in normalized_query for keyword in {"windows", "system", "powershell", "registry", "driver", "gpu"}):
            intent_tags.append("system_ops")
        if any(keyword in normalized_query for keyword in {"bilibili", "youtube", "netflix", "anime", "movie", "video"}):
            intent_tags.append("video_media")
        if any(keyword in normalized_query for keyword in {"discord", "kook", "telegram", "wechat", "server", "community"}):
            intent_tags.append("chat_community")
        if any(keyword in normalized_query for keyword in {"steam", "epic", "game", "gaming", "deck"}):
            intent_tags.append("gaming")
        if any(keyword in normalized_query for keyword in {"buy", "price", "sale", "shop", "store", "dock"}):
            intent_tags.append("shopping")

        if "ai_tools" not in intent_tags and any(
            domain.endswith(suffix)
            for domain in domain_set
            for suffix in ("openai.com", "platform.openai.com", "auth.openai.com", "claude.ai")
        ) and any(keyword in normalized_query for keyword in {"config", "agent", "assistant"}):
            intent_tags.append("ai_tools")
        if "developer_research" not in intent_tags and any(
            domain.endswith(suffix)
            for domain in domain_set
            for suffix in ("developer.mozilla.org", "docs.example.com", "github.com")
        ) and any(keyword in normalized_query for keyword in {"config", "pattern", "context"}):
            intent_tags.append("developer_research")

        if not intent_tags:
            continue

        for tag in intent_tags:
            intent_scores[tag] = intent_scores.get(tag, 0) + visit_count

        query_examples.append(
            {
                "query": query,
                "visit_count": visit_count,
                "engines": engines,
                **({"normalized_from": item["normalized_from"]} if item.get("normalized_from") else {}),
                **({"sources": item["sources"]} if item.get("sources") else {}),
                "intent_tags": intent_tags,
            }
        )

    top_intents = [
        {"intent": intent, "score": score}
        for intent, score in sorted(intent_scores.items(), key=lambda item: (-item[1], item[0]))[:10]
    ]

    return {
        "present": bool(top_intents),
        "intent_tags": sorted(intent_scores),
        "top_intents": top_intents,
        "query_examples": sorted(query_examples, key=lambda item: (-item["visit_count"], item["query"]))[:20],
    }


def _build_confidence_summary(raw_evidence: list[dict]) -> dict:
    if not raw_evidence:
        return {"average": 0.0, "count": 0}
    average = sum(float(entry["confidence"]) for entry in raw_evidence) / len(raw_evidence)
    return {"average": round(average, 3), "count": len(raw_evidence)}


def _build_open_questions(raw_evidence: list[dict]) -> list[str]:
    questions: list[str] = []
    if not any(entry["entity_kind"] == "claude_entrypoint" for entry in raw_evidence):
        questions.append("Claude context entrypoint was not found in explored roots.")
    if not any(entry["entity_kind"] == "codex_config" for entry in raw_evidence):
        questions.append("Codex config was not found in explored roots.")
    return questions


def _apply_redactions(raw_evidence: list[dict]) -> list[dict]:
    redactions: list[dict] = []
    exact_sensitive_fields = {"apiKey", "access_token", "token", "api_key"}

    for entry in raw_evidence:
        fields = entry.get("fields")
        if not isinstance(fields, dict):
            continue
        _redact_value(
            value=fields,
            path_parts=[],
            exact_sensitive_fields=exact_sensitive_fields,
            entry_path=entry["path"],
            entity_kind=entry["entity_kind"],
            redactions=redactions,
        )

    return redactions


def _redact_value(
    value: object,
    path_parts: list[str],
    exact_sensitive_fields: set[str],
    entry_path: str,
    entity_kind: str,
    redactions: list[dict],
) -> None:
    if isinstance(value, dict):
        for key, nested in list(value.items()):
            key_name = str(key)
            next_path = [*path_parts, key_name]
            if _is_sensitive_field_name(key_name, exact_sensitive_fields):
                if nested not in (None, "", "[REDACTED]"):
                    value[key] = "[REDACTED]"
                    redactions.append(
                        {
                            "path": entry_path,
                            "entity_kind": entity_kind,
                            "field": ".".join(next_path),
                        }
                    )
                continue
            _redact_value(
                value=nested,
                path_parts=next_path,
                exact_sensitive_fields=exact_sensitive_fields,
                entry_path=entry_path,
                entity_kind=entity_kind,
                redactions=redactions,
            )
        return

    if isinstance(value, list):
        for index, item in enumerate(value):
            _redact_value(
                value=item,
                path_parts=[*path_parts, str(index)],
                exact_sensitive_fields=exact_sensitive_fields,
                entry_path=entry_path,
                entity_kind=entity_kind,
                redactions=redactions,
            )


def _is_sensitive_field_name(field_name: str, exact_sensitive_fields: set[str]) -> bool:
    normalized = field_name.strip()
    if normalized in exact_sensitive_fields:
        return True

    upper_name = normalized.upper()
    return upper_name.endswith("_TOKEN") or upper_name.endswith("_KEY") or upper_name.endswith("_SECRET")


def build_effective_roots(config: ScoutConfig) -> list[Path]:
    roots: list[Path] = []
    seen: set[str] = set()

    def _add(path: Path | None) -> None:
        if path is None:
            return
        resolved = path.resolve()
        key = str(resolved).lower()
        if key in seen:
            return
        seen.add(key)
        roots.append(resolved)

    for root in config.roots:
        _add(root)

    if config.home:
        home = config.home.resolve()
        _add(home)
        _add(home / ".claude")
        _add(home / ".codex")
        _add(home / "AppData" / "Local")
        _add(home / "AppData" / "Roaming")
        _add(home / "Documents")
        _add(home / "Desktop")

    return roots
