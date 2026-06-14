from __future__ import annotations

import argparse
import json
from pathlib import Path

from .runtime import ScoutConfig, run_scout


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="AI-first local scout report generator")
    parser.add_argument("command", nargs="?", choices=["run"], help="Optional tool-runtime command alias")
    parser.add_argument("--root", action="append", default=[], help="Root path to explore")
    parser.add_argument("--home", help="Home directory for tool-specific context")
    parser.add_argument("--output", help="Path to write the report JSON. If omitted, report JSON is written to stdout")
    parser.add_argument("--max-depth", type=int, default=6, help="Maximum recursive depth per root")
    parser.add_argument(
        "--max-sqlite-parse",
        type=int,
        default=32,
        help="Maximum number of sqlite-like databases to parse before switching to budget metadata only",
    )
    parser.add_argument(
        "--system-steam-root",
        action="append",
        default=[],
        help="Explicit Steam install root to probe for libraryfolders.vdf",
    )
    parser.add_argument(
        "--enable-steam-public-profile",
        action="store_true",
        help="Opt in to fetching public Steam Community game playtime from the most recent local Steam ID",
    )
    parser.add_argument(
        "--system-epic-manifest-dir",
        action="append",
        default=[],
        help="Explicit Epic launcher manifest directory to probe for *.item files",
    )
    parser.add_argument(
        "--system-legendary-installed",
        action="append",
        default=[],
        help="Explicit Legendary or Heroic installed.json path to probe",
    )
    parser.add_argument(
        "--system-amazon-games-install-info",
        action="append",
        default=[],
        help="Explicit Amazon Games GameInstallInfo.sqlite path to probe",
    )
    parser.add_argument(
        "--system-xbox-game-config",
        action="append",
        default=[],
        help="Explicit MicrosoftGame.config path to probe for Xbox / Microsoft Store games",
    )
    parser.add_argument(
        "--system-itch-butler-db",
        action="append",
        default=[],
        help="Explicit itch.io butler.db path to probe",
    )
    parser.add_argument(
        "--system-battle-net-product-db",
        action="append",
        default=[],
        help="Explicit Battle.net Agent product.db path to probe",
    )
    parser.add_argument(
        "--system-origin-local-content-dir",
        action="append",
        default=[],
        help="Explicit Origin / EA App LocalContent directory to probe for *.mfst manifests",
    )
    parser.add_argument(
        "--system-activitywatch-base-url",
        action="append",
        default=[],
        help="Explicit ActivityWatch base URL to probe for runtime buckets",
    )
    parser.add_argument(
        "--system-obsidian-config",
        action="append",
        default=[],
        help="Explicit Obsidian obsidian.json path to probe for registered vaults",
    )
    parser.add_argument(
        "--system-obs-studio-basic-dir",
        action="append",
        default=[],
        help="Explicit OBS Studio basic directory to probe for profiles and scene collections",
    )
    parser.add_argument(
        "--system-docker-desktop-settings",
        action="append",
        default=[],
        help="Explicit Docker Desktop settings-store.json path to probe",
    )
    parser.add_argument(
        "--system-wslconfig",
        action="append",
        default=[],
        help="Explicit .wslconfig path to probe for global WSL settings",
    )
    parser.add_argument(
        "--system-discord-settings",
        action="append",
        default=[],
        help="Explicit Discord settings.json path to probe",
    )
    parser.add_argument(
        "--system-teams-config",
        action="append",
        default=[],
        help="Explicit Microsoft Teams desktop-config.json or settings.json path to probe",
    )
    parser.add_argument(
        "--system-dropbox-info",
        action="append",
        default=[],
        help="Explicit Dropbox info.json path to probe",
    )
    parser.add_argument(
        "--system-onedrive-settings-root",
        action="append",
        default=[],
        help="Explicit OneDrive settings root directory containing */global.ini to probe",
    )
    parser.add_argument(
        "--system-joplin-profile",
        action="append",
        default=[],
        help="Explicit Joplin profile directory to probe",
    )
    parser.add_argument(
        "--system-nextcloud-config",
        action="append",
        default=[],
        help="Explicit Nextcloud nextcloud.cfg path to probe",
    )
    parser.add_argument(
        "--system-syncthing-config",
        action="append",
        default=[],
        help="Explicit Syncthing config.xml path to probe",
    )
    parser.add_argument(
        "--system-jetbrains-recent-projects",
        action="append",
        default=[],
        help="Explicit JetBrains recentProjects.xml path to probe",
    )
    parser.add_argument(
        "--system-windows-terminal-settings",
        action="append",
        default=[],
        help="Explicit Windows Terminal settings.json path to probe",
    )
    parser.add_argument(
        "--system-ssh-config",
        action="append",
        default=[],
        help="Explicit SSH config path to probe",
    )
    parser.add_argument(
        "--system-kubeconfig",
        action="append",
        default=[],
        help="Explicit kubeconfig path to probe",
    )
    parser.add_argument(
        "--system-docker-config",
        action="append",
        default=[],
        help="Explicit Docker CLI config.json path to probe",
    )
    parser.add_argument(
        "--system-docker-context-meta",
        action="append",
        default=[],
        help="Explicit Docker context meta.json path to probe",
    )
    parser.add_argument(
        "--system-aws-config",
        action="append",
        default=[],
        help="Explicit AWS CLI config path to probe",
    )
    parser.add_argument(
        "--system-azure-profile",
        action="append",
        default=[],
        help="Explicit Azure CLI azureProfile.json path to probe",
    )
    parser.add_argument(
        "--system-gcloud-config-root",
        action="append",
        default=[],
        help="Explicit Google Cloud CLI config root to probe",
    )
    parser.add_argument(
        "--system-github-cli-config-root",
        action="append",
        default=[],
        help="Explicit GitHub CLI config root to probe",
    )
    parser.add_argument(
        "--system-gitconfig",
        action="append",
        default=[],
        help="Explicit global gitconfig path to probe",
    )
    parser.add_argument(
        "--system-cargo-config",
        action="append",
        default=[],
        help="Explicit Cargo user config path to probe",
    )
    parser.add_argument(
        "--system-cargo-credentials",
        action="append",
        default=[],
        help="Explicit Cargo credentials store path to probe",
    )
    parser.add_argument(
        "--system-maven-settings",
        action="append",
        default=[],
        help="Explicit Maven user settings.xml path to probe",
    )
    parser.add_argument(
        "--system-gradle-properties",
        action="append",
        default=[],
        help="Explicit Gradle user gradle.properties path to probe",
    )
    parser.add_argument(
        "--system-nuget-config",
        action="append",
        default=[],
        help="Explicit NuGet user config path to probe",
    )
    parser.add_argument(
        "--system-dotnet-tools-dir",
        action="append",
        default=[],
        help="Explicit .NET global tools directory to probe",
    )
    parser.add_argument(
        "--system-npmrc",
        action="append",
        default=[],
        help="Explicit user npmrc path to probe",
    )
    parser.add_argument(
        "--system-pnpm-config",
        action="append",
        default=[],
        help="Explicit pnpm user rc path to probe",
    )
    parser.add_argument(
        "--system-pip-config",
        action="append",
        default=[],
        help="Explicit pip user config path to probe",
    )
    parser.add_argument(
        "--system-condarc",
        action="append",
        default=[],
        help="Explicit conda user config path to probe",
    )
    parser.add_argument(
        "--system-poetry-config",
        action="append",
        default=[],
        help="Explicit Poetry user config.toml path to probe",
    )
    parser.add_argument(
        "--system-rustup-settings",
        action="append",
        default=[],
        help="Explicit rustup settings.toml path to probe",
    )
    parser.add_argument(
        "--system-uv-config",
        action="append",
        default=[],
        help="Explicit uv user config path to probe",
    )
    parser.add_argument(
        "--system-uv-credentials",
        action="append",
        default=[],
        help="Explicit uv credentials.toml path to probe",
    )
    parser.add_argument(
        "--system-yarnrc-yml",
        action="append",
        default=[],
        help="Explicit Yarn .yarnrc.yml path to probe",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    roots = [Path(root).resolve() for root in args.root]
    config = ScoutConfig(
        roots=roots,
        home=Path(args.home).resolve() if args.home else None,
        max_depth=args.max_depth,
        max_sqlite_parse=args.max_sqlite_parse,
        system_steam_roots=[Path(path).resolve() for path in args.system_steam_root],
        system_epic_manifest_dirs=[Path(path).resolve() for path in args.system_epic_manifest_dir],
        system_legendary_installed_paths=[Path(path).resolve() for path in args.system_legendary_installed],
        system_amazon_games_install_info_paths=[Path(path).resolve() for path in args.system_amazon_games_install_info],
        system_xbox_game_config_paths=[Path(path).resolve() for path in args.system_xbox_game_config],
        system_itch_butler_db_paths=[Path(path).resolve() for path in args.system_itch_butler_db],
        system_battle_net_product_db_paths=[Path(path).resolve() for path in args.system_battle_net_product_db],
        system_origin_local_content_dirs=[Path(path).resolve() for path in args.system_origin_local_content_dir],
        system_activitywatch_base_urls=list(args.system_activitywatch_base_url),
        system_obsidian_config_paths=[Path(path).resolve() for path in args.system_obsidian_config],
        system_obs_studio_basic_dirs=[Path(path).resolve() for path in args.system_obs_studio_basic_dir],
        system_docker_desktop_settings_paths=[Path(path).resolve() for path in args.system_docker_desktop_settings],
        system_wslconfig_paths=[Path(path).resolve() for path in args.system_wslconfig],
        system_discord_settings_paths=[Path(path).resolve() for path in args.system_discord_settings],
        system_teams_config_paths=[Path(path).resolve() for path in args.system_teams_config],
        system_dropbox_info_paths=[Path(path).resolve() for path in args.system_dropbox_info],
        system_onedrive_settings_roots=[Path(path).resolve() for path in args.system_onedrive_settings_root],
        system_joplin_profile_paths=[Path(path).resolve() for path in args.system_joplin_profile],
        system_nextcloud_config_paths=[Path(path).resolve() for path in args.system_nextcloud_config],
        system_syncthing_config_paths=[Path(path).resolve() for path in args.system_syncthing_config],
        system_jetbrains_recent_projects_paths=[Path(path).resolve() for path in args.system_jetbrains_recent_projects],
        system_windows_terminal_settings_paths=[Path(path).resolve() for path in args.system_windows_terminal_settings],
        system_ssh_config_paths=[Path(path).resolve() for path in args.system_ssh_config],
        system_kubeconfig_paths=[Path(path).resolve() for path in args.system_kubeconfig],
        system_docker_config_paths=[Path(path).resolve() for path in args.system_docker_config],
        system_docker_context_meta_paths=[Path(path).resolve() for path in args.system_docker_context_meta],
        system_aws_config_paths=[Path(path).resolve() for path in args.system_aws_config],
        system_azure_profile_paths=[Path(path).resolve() for path in args.system_azure_profile],
        system_gcloud_config_root_paths=[Path(path).resolve() for path in args.system_gcloud_config_root],
        system_github_cli_config_root_paths=[Path(path).resolve() for path in args.system_github_cli_config_root],
        system_gitconfig_paths=[Path(path).resolve() for path in args.system_gitconfig],
        system_cargo_config_paths=[Path(path).resolve() for path in args.system_cargo_config],
        system_cargo_credentials_paths=[Path(path).resolve() for path in args.system_cargo_credentials],
        system_maven_settings_paths=[Path(path).resolve() for path in args.system_maven_settings],
        system_gradle_properties_paths=[Path(path).resolve() for path in args.system_gradle_properties],
        system_nuget_config_paths=[Path(path).resolve() for path in args.system_nuget_config],
        system_dotnet_tools_dirs=[Path(path).resolve() for path in args.system_dotnet_tools_dir],
        system_npmrc_paths=[Path(path).resolve() for path in args.system_npmrc],
        system_pnpm_config_paths=[Path(path).resolve() for path in args.system_pnpm_config],
        system_pip_config_paths=[Path(path).resolve() for path in args.system_pip_config],
        system_condarc_paths=[Path(path).resolve() for path in args.system_condarc],
        system_poetry_config_paths=[Path(path).resolve() for path in args.system_poetry_config],
        system_rustup_settings_paths=[Path(path).resolve() for path in args.system_rustup_settings],
        system_uv_config_paths=[Path(path).resolve() for path in args.system_uv_config],
        system_uv_credentials_paths=[Path(path).resolve() for path in args.system_uv_credentials],
        system_yarnrc_yml_paths=[Path(path).resolve() for path in args.system_yarnrc_yml],
        enable_steam_public_profile=args.enable_steam_public_profile,
        output_path=Path(args.output).resolve() if args.output else None,
    )
    report = run_scout(config)
    output_path = config.output_path
    if output_path is None:
        print(json.dumps(report, ensure_ascii=True))
        return 0
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(report, indent=2, ensure_ascii=True), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
