# AI Local Scout

Standalone AI-first local evidence aggregator.

This app combines:

- deterministic bootstrap discovery for high-signal local artifacts
- AI-style follow-up expansion based on discovered evidence
- unified JSON output with `bootstrap_evidence`, `search_trace`, `raw_evidence`, and `derived_profile`

Current v1 coverage:

- Claude discovery via `CLAUDE.md`, `.claude/settings.json`, and `.claude/mcp.json`
- Codex discovery via `.codex/config.toml`, `.codex/auth.json`, and `.codex/rules/default.rules`
- project rule discovery via `AGENTS.md`
- browser discovery for Chrome, Edge, Brave, Arc, Opera, Opera GX, Vivaldi, Firefox, and Zen
- browser-derived aggregation for bookmarks, recent domains, downloads, extension inventories, per-browser activity, weighted top domains, and cross-browser search queries
- best-effort browser session summaries from readable Chromium/Firefox session snapshot files when available
- higher-level intent inference from search queries and browser context, including consumer sub-buckets such as video, chat, gaming, and shopping
- Git repo discovery via `.git/config`
- workspace discovery via `.code-workspace`, VS Code storage, Cursor storage, and Cursor `state.vscdb`
- shell history discovery via PowerShell `ConsoleHost_history.txt`
- installed app discovery via Windows registry probe or exported `apps.json`
- Steam discovery via `libraryfolders.vdf` and `appmanifest_*.acf`
- Epic discovery via launcher manifests plus Legendary and Heroic `installed.json`
- gaming summaries for supported launchers now include provider-specific install metadata such as Steam app IDs/library roots and Epic catalog/app/install fields when those local artifacts expose them
- Playnite library discovery via `%AppData%\\Playnite\\library\\games\\*.json`
- GOG Galaxy discovery via storage export JSON files when present
- Amazon Games discovery via `GameInstallInfo.sqlite` when present
- Xbox / Microsoft Store game discovery via `MicrosoftGame.config` when present
- itch.io app discovery via `butler.db` when present
- Ubisoft Connect discovery via launcher install registry entries when readable
- Battle.net discovery via launcher-related uninstall registry entries and native `Agent\\product.db` when readable
- Origin / EA discovery via `LocalContent\\*\\*.mfst` manifests when present
- ActivityWatch runtime discovery via local HTTP bucket probing
- Obsidian discovery via global vault registry plus per-vault `.obsidian` config
- OBS Studio discovery via `basic/profiles/*/basic.ini` and `basic/scenes/*.json`
- Docker Desktop discovery via `settings-store.json`
- WSL discovery via `.wslconfig` plus `wsl.exe --list --verbose`
- Discord discovery via Roaming `settings.json`
- Microsoft Teams discovery via classic `desktop-config.json` plus new client `settings.json`
- Dropbox discovery via Roaming `info.json`
- OneDrive discovery via `settings/*/global.ini`
- Joplin discovery via profile `settings.json`, package directory, and profile assets
- Nextcloud discovery via Roaming `nextcloud.cfg`
- Syncthing discovery via Local `config.xml`
- JetBrains IDE discovery via `options/recentProjects.xml`
- Windows Terminal discovery via `LocalState/settings.json`
- SSH discovery via `~/.ssh/config`
- Kubernetes discovery via `~/.kube/config`
- Docker CLI context discovery via `~/.docker/config.json` and `contexts/meta/*/meta.json`
- AWS CLI discovery via `~/.aws/config`
- Azure CLI discovery via `~/.azure/azureProfile.json`
- Google Cloud CLI discovery via `%AppData%\\gcloud\\active_config` and `configurations\\config_*`
- GitHub CLI discovery via `%AppData%\\GitHub CLI\\config.yml` and `hosts.yml`
- Git global config discovery via `~/.gitconfig`
- Cargo discovery via `~/.cargo\\config.toml|config` and `~/.cargo\\credentials.toml|credentials`
- Maven discovery via `~/.m2\\settings.xml`
- Gradle discovery via `~/.gradle\\gradle.properties`
- NuGet discovery via `%AppData%\\NuGet\\NuGet.Config`
- .NET global tool discovery via `~/.dotnet\\tools`
- npm user config discovery via `~/.npmrc`
- pnpm user config discovery via `%LocalAppData%\\pnpm\\config\\rc`
- pip user config discovery via `%AppData%\\pip\\pip.ini`
- conda user config discovery via `~/.condarc`
- Poetry user config discovery via `%AppData%\\pypoetry\\config.toml`
- rustup discovery via `~/.rustup\\settings.toml`
- uv user config discovery via `%AppData%\\uv\\uv.toml` and `%AppData%\\uv\\data\\credentials\\credentials.toml`
- Yarn user config discovery via `~/.yarnrc.yml`
- generic sqlite artifact discovery with configurable parse budget
- recursive redaction for nested secret-like fields such as `*_TOKEN`, `*_KEY`, and `*_SECRET`
- skip rules for noisy cache, temp, and vendor-like directories during discovery

Derived JSON profile currently includes:

- `tools`
- `game_ecosystem`
- `browser_activity`
- `web_interest_profile`
- `search_activity`
- `intent_profile`
- `active_workspaces`
- `shell_activity`
- `installed_apps`
- `creative_tools_profile`
- `hardware_profile`
- `privacy_security_profile`
- `agent_config_profile`
- `downloads_profile`
- `office_documents_profile`
- `recent_documents_profile`
- `developer_profile`
- `gaming_profile`
- `steam_playtime_profile`
- `ai_tools_profile`
- `activitywatch_profile`
- `knowledge_tools_profile`
- `creator_profile`
- `container_tools_profile`
- `linux_runtime_profile`
- `social_tools_profile`
- `collaboration_tools_profile`
- `sync_storage_profile`
- `ide_profile`
- `terminal_tools_profile`
- `ssh_profile`
- `kubernetes_profile`
- `cloud_tools_profile`
- `source_control_profile`
- `dotnet_tooling_profile`
- `jvm_tooling_profile`
- `rust_tooling_profile`
- `python_tooling_profile`
- `javascript_tooling_profile`
- `meaningful_activity_profile`
- `local_signal_coverage`
- `next_questions`
- `llm_context`
- `interest_tags`
- `sqlite_artifacts`

Meaningful activity profile notes:

- this profile is derived from already-collected one-shot evidence; it does not add background monitoring
- broad mode and setup hints summarize games, dev tools, AI tools, notes, creator tooling, sync, terminal, cloud, Linux, and container signals when present
- gaming style hints come from installed game names and should be treated as lightweight library hints, not live playtime
- evidence quality labels distinguish strong install/config evidence from weak recent-activity signals

Local signal coverage notes:

- `local_signal_coverage` summarizes which broad local domains produced evidence, including games, browser, development, AI tools, knowledge tools, creator tools, sync storage, terminal/remote, cloud/runtime, language tooling, local apps, and generic data
- audio/voice and translation domains are explicitly marked out of scope so downstream users do not confuse this scout with voice separation or simultaneous interpretation tooling

Next question notes:

- `next_questions.ask_user` suggests concrete follow-up permissions or paths only when the current one-shot scan has weak or absent evidence
- `next_questions.do_not_ask` records explicit boundaries such as audio/voice separation and simultaneous interpretation being outside this project's scope

LLM context notes:

- `llm_context` is the compact handoff block for downstream agents
- it separates strong facts, weak hints, uncertainties, boundaries, and good follow-up questions
- it is generated deterministically from the derived profile; it does not call an LLM

Office document notes:

- Desktop `.docx`, `.pptx`, and `.xlsx` files are indexed as high-sensitivity `office_document` evidence with capped local previews
- `office_documents_profile.deep_read_candidates` includes `skill_routes` so downstream agents can call Weft `read_document` or the specialized `docx`/`pptx`/`xlsx` skills on demand
- `recent_documents_profile` resolves Windows Recent document shortcuts into high-sensitivity routing metadata without reading full document contents
- Scout remains the router/indexer here; it does not replace the document-reading skills or inject full document bodies into `llm_context`

Example:

```bash
python -m ai_local_scout.cli --root C:\Users\Admin --home C:\Users\Admin --output D:\weft\tmp\scout-report.json --max-depth 6 --max-sqlite-parse 48
```

Browser activity notes:

- download summaries are domain and file-extension oriented; they do not surface raw download target paths in the derived profile
- extension summaries capture names, ids, versions, and declared permissions when available
- session summaries are best-effort and currently support readable snapshot files plus Firefox `mozLz40` / `jsonlz4` session payloads when `lz4` is available
- unsupported binary session formats are skipped rather than guessed

Game ecosystem notes:

- Playnite is treated as a high-signal local aggregator and can contribute non-Steam / non-Epic platforms such as GOG and Battle.net when its library files are present
- GOG Galaxy support currently targets readable installed-game JSON exports when available
- Amazon Games support currently targets concrete installed rows from `GameInstallInfo.sqlite`
- Xbox / Microsoft Store support currently targets concrete `MicrosoftGame.config` files from AppX install locations or `XboxGames` content folders
- itch.io support currently targets concrete installed records from the app's `butler.db` caves table
- Ubisoft and Battle.net support currently prefers concrete launcher install artifacts over title guessing
- Battle.net `product.db` support currently reads concrete protobuf install records with `productCode` and `settings.installPath`
- Origin / EA support currently targets readable `LocalContent` manifest files and derives names from manifest fields or the concrete manifest folder name

Optional explicit system probes:

- `--system-steam-root <path>` to point at a Steam install root that contains `steamapps/libraryfolders.vdf`
- `--system-epic-manifest-dir <path>` to point at an Epic launcher manifests directory
- `--system-legendary-installed <path>` to point at a Legendary or Heroic `installed.json`
- `--system-amazon-games-install-info <path>` to point at Amazon Games `GameInstallInfo.sqlite`
- `--system-xbox-game-config <path>` to point at an Xbox / Microsoft Store `MicrosoftGame.config`
- `--system-itch-butler-db <path>` to point at itch.io `butler.db`
- `--system-battle-net-product-db <path>` to point at a Battle.net `Agent\\product.db`
- `--system-origin-local-content-dir <path>` to point at an Origin / EA `LocalContent` directory containing `*.mfst` manifests
- `--system-activitywatch-base-url <url>` to point at an ActivityWatch server such as `http://127.0.0.1:5600`
- `--system-obsidian-config <path>` to point at an Obsidian `obsidian.json` vault registry file
- `--system-obs-studio-basic-dir <path>` to point at an OBS Studio `basic` directory containing `profiles` and `scenes`
- `--system-docker-desktop-settings <path>` to point at a Docker Desktop `settings-store.json`
- `--system-wslconfig <path>` to point at a `.wslconfig` file
- `--system-discord-settings <path>` to point at a Discord `settings.json`
- `--system-teams-config <path>` to point at a Microsoft Teams `desktop-config.json` or `settings.json`
- `--system-dropbox-info <path>` to point at a Dropbox `info.json`
- `--system-onedrive-settings-root <path>` to point at a OneDrive `settings` directory containing `*/global.ini`
- `--system-joplin-profile <path>` to point at a Joplin profile directory
- `--system-nextcloud-config <path>` to point at a Nextcloud `nextcloud.cfg`
- `--system-syncthing-config <path>` to point at a Syncthing `config.xml`
- `--system-jetbrains-recent-projects <path>` to point at a JetBrains `recentProjects.xml`
- `--system-windows-terminal-settings <path>` to point at a Windows Terminal `settings.json`
- `--system-ssh-config <path>` to point at an SSH config file
- `--system-kubeconfig <path>` to point at a kubeconfig file
- `--system-docker-config <path>` to point at a Docker CLI `config.json`
- `--system-docker-context-meta <path>` to point at a Docker context `meta.json`
- `--system-aws-config <path>` to point at an AWS CLI `config`
- `--system-azure-profile <path>` to point at an Azure CLI `azureProfile.json`
- `--system-gcloud-config-root <path>` to point at a Google Cloud CLI config root
- `--system-github-cli-config-root <path>` to point at a GitHub CLI config root
- `--system-gitconfig <path>` to point at a global gitconfig file
- `--system-cargo-config <path>` to point at a Cargo `config.toml` or `config`
- `--system-cargo-credentials <path>` to point at a Cargo `credentials.toml` or `credentials`
- `--system-maven-settings <path>` to point at a Maven `settings.xml`
- `--system-gradle-properties <path>` to point at a Gradle `gradle.properties`
- `--system-nuget-config <path>` to point at a NuGet `NuGet.Config`
- `--system-dotnet-tools-dir <path>` to point at a .NET global tools directory
- `--system-npmrc <path>` to point at a user npmrc file
- `--system-pnpm-config <path>` to point at a pnpm user `rc` file
- `--system-pip-config <path>` to point at a pip user config file
- `--system-condarc <path>` to point at a conda user config file
- `--system-poetry-config <path>` to point at a Poetry `config.toml`
- `--system-rustup-settings <path>` to point at a rustup `settings.toml`
- `--system-uv-config <path>` to point at a uv `uv.toml`
- `--system-uv-credentials <path>` to point at a uv `credentials.toml`
- `--system-yarnrc-yml <path>` to point at a Yarn `.yarnrc.yml`

Example with explicit machine-specific probes:

```bash
python -m ai_local_scout.cli \
  --home C:\Users\Admin \
  --output D:\weft\tmp\scout-report.json \
  --system-steam-root D:\Steam \
  --system-epic-manifest-dir C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests \
  --system-legendary-installed C:\Users\Admin\AppData\Roaming\legendary\installed.json \
  --system-amazon-games-install-info C:\Users\Admin\AppData\Local\Amazon Games\Data\Games\Sql\GameInstallInfo.sqlite \
  --system-xbox-game-config D:\XboxGames\Avowed\Content\MicrosoftGame.config \
  --system-itch-butler-db C:\Users\Admin\AppData\Roaming\itch\db\butler.db \
  --system-battle-net-product-db C:\ProgramData\Battle.net\Agent\product.db \
  --system-origin-local-content-dir C:\ProgramData\Origin\LocalContent \
  --system-activitywatch-base-url http://127.0.0.1:5600 \
  --system-obsidian-config C:\Users\Admin\AppData\Roaming\Obsidian\obsidian.json \
  --system-obs-studio-basic-dir C:\Users\Admin\AppData\Roaming\obs-studio\basic \
  --system-docker-desktop-settings C:\Users\Admin\AppData\Roaming\Docker\settings-store.json \
  --system-wslconfig C:\Users\Admin\.wslconfig \
  --system-discord-settings C:\Users\Admin\AppData\Roaming\discord\settings.json \
  --system-teams-config C:\Users\Admin\AppData\Roaming\Microsoft\Teams\desktop-config.json \
  --system-dropbox-info C:\Users\Admin\AppData\Roaming\Dropbox\info.json \
  --system-onedrive-settings-root C:\Users\Admin\AppData\Local\Microsoft\OneDrive\settings \
  --system-joplin-profile C:\Users\Admin\.config\joplin-desktop \
  --system-nextcloud-config C:\Users\Admin\AppData\Roaming\Nextcloud\nextcloud.cfg \
  --system-syncthing-config C:\Users\Admin\AppData\Local\Syncthing\config.xml \
  --system-jetbrains-recent-projects C:\Users\Admin\AppData\Roaming\JetBrains\IntelliJIdea2025.1\options\recentProjects.xml \
  --system-windows-terminal-settings C:\Users\Admin\AppData\Local\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json \
  --system-ssh-config C:\Users\Admin\.ssh\config \
  --system-kubeconfig C:\Users\Admin\.kube\config \
  --system-docker-config C:\Users\Admin\.docker\config.json \
  --system-docker-context-meta C:\Users\Admin\.docker\contexts\meta\ctx1\meta.json \
  --system-aws-config C:\Users\Admin\.aws\config \
  --system-azure-profile C:\Users\Admin\.azure\azureProfile.json \
  --system-gcloud-config-root C:\Users\Admin\AppData\Roaming\gcloud \
  --system-github-cli-config-root C:\Users\Admin\AppData\Roaming\GitHub CLI \
  --system-gitconfig C:\Users\Admin\.gitconfig \
  --system-cargo-config C:\Users\Admin\.cargo\config.toml \
  --system-cargo-credentials C:\Users\Admin\.cargo\credentials.toml \
  --system-maven-settings C:\Users\Admin\.m2\settings.xml \
  --system-gradle-properties C:\Users\Admin\.gradle\gradle.properties \
  --system-nuget-config C:\Users\Admin\AppData\Roaming\NuGet\NuGet.Config \
  --system-dotnet-tools-dir C:\Users\Admin\.dotnet\tools \
  --system-npmrc C:\Users\Admin\.npmrc \
  --system-pnpm-config C:\Users\Admin\AppData\Local\pnpm\config\rc \
  --system-pip-config C:\Users\Admin\AppData\Roaming\pip\pip.ini \
  --system-condarc C:\Users\Admin\.condarc \
  --system-poetry-config C:\Users\Admin\AppData\Roaming\pypoetry\config.toml \
  --system-rustup-settings C:\Users\Admin\.rustup\settings.toml \
  --system-uv-config C:\Users\Admin\AppData\Roaming\uv\uv.toml \
  --system-uv-credentials C:\Users\Admin\AppData\Roaming\uv\data\credentials\credentials.toml \
  --system-yarnrc-yml C:\Users\Admin\.yarnrc.yml
```
