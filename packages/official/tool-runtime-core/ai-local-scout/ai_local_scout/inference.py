from __future__ import annotations

from collections import Counter


def derive_agent_config_profile(raw_evidence: list[dict]) -> dict:
    surface_order = [
        "claude_entrypoint",
        "claude_settings",
        "claude_mcp_config",
        "codex_config",
        "codex_auth",
        "codex_rules",
        "project_rules",
        "claude_context_file",
    ]
    agent_entries = [entry for entry in raw_evidence if entry.get("entity_kind") in surface_order]
    by_kind: dict[str, list[dict]] = {kind: [] for kind in surface_order}
    for entry in agent_entries:
        by_kind[str(entry.get("entity_kind"))].append(entry)

    claude_mcp_servers: set[str] = set()
    claude_default_models: set[str] = set()
    for entry in by_kind["claude_settings"]:
        fields = entry.get("fields", {})
        claude_mcp_servers.update(str(server) for server in fields.get("mcp_servers", []) if str(server).strip())
        if fields.get("defaultModel"):
            claude_default_models.add(str(fields["defaultModel"]))
    for entry in by_kind["claude_mcp_config"]:
        fields = entry.get("fields", {})
        claude_mcp_servers.update(str(server) for server in fields.get("server_names", []) if str(server).strip())

    codex_mcp_servers: set[str] = set()
    trusted_projects: set[str] = set()
    for entry in by_kind["codex_config"]:
        fields = entry.get("fields", {})
        codex_mcp_servers.update(str(server) for server in fields.get("mcp_servers", []) if str(server).strip())
        trusted_projects.update(str(project) for project in fields.get("trusted_projects", []) if str(project).strip())

    approved_rule_count = sum(int(entry.get("fields", {}).get("allowed_rule_count", 0)) for entry in by_kind["codex_rules"])
    project_rule_paths = sorted(str(entry.get("path")) for entry in by_kind["project_rules"] if str(entry.get("path") or "").strip())

    agent_families = []
    if by_kind["claude_entrypoint"] or by_kind["claude_settings"] or by_kind["claude_mcp_config"] or by_kind["claude_context_file"]:
        agent_families.append("claude")
    if by_kind["codex_config"] or by_kind["codex_auth"] or by_kind["codex_rules"]:
        agent_families.append("codex")
    if by_kind["project_rules"]:
        agent_families.append("project_agents")

    config_surfaces = [kind for kind in surface_order if by_kind[kind]]
    setup_hints = []
    if by_kind["claude_entrypoint"] or by_kind["claude_context_file"]:
        setup_hints.append("uses_claude_project_memory")
    if claude_mcp_servers or codex_mcp_servers:
        setup_hints.append("uses_mcp_servers")
    if trusted_projects:
        setup_hints.append("uses_codex_trusted_projects")
    if project_rule_paths:
        setup_hints.append("uses_project_agents_rules")

    return {
        "present": bool(agent_entries),
        "agent_families": agent_families,
        "config_surfaces": config_surfaces,
        "claude": {
            "entrypoint_count": len(by_kind["claude_entrypoint"]),
            "context_file_count": len(by_kind["claude_context_file"]),
            "default_models": sorted(claude_default_models),
            "mcp_servers": sorted(claude_mcp_servers),
        },
        "codex": {
            "config_count": len(by_kind["codex_config"]),
            "auth_present": bool(by_kind["codex_auth"]),
            "rules_file_count": len(by_kind["codex_rules"]),
            "approved_rule_count": approved_rule_count,
            "trusted_project_count": len(trusted_projects),
            "mcp_servers": sorted(codex_mcp_servers),
        },
        "project_rules": {
            "agents_file_count": len(project_rule_paths),
            "paths": project_rule_paths,
        },
        "setup_hints": setup_hints,
        "evidence_basis": "raw_evidence_agent_config",
    }


def derive_privacy_security_profile(installed_apps: list[dict]) -> dict:
    tool_definitions = {
        "1password": {
            "keywords": {"1password", "agilebits"},
            "domains": {"password_management"},
        },
        "bitwarden": {
            "keywords": {"bitwarden"},
            "domains": {"password_management"},
        },
        "dashlane": {
            "keywords": {"dashlane"},
            "domains": {"password_management"},
        },
        "keepass": {
            "keywords": {"keepass"},
            "domains": {"password_management"},
        },
        "tailscale": {
            "keywords": {"tailscale"},
            "domains": {"vpn_or_mesh_networking"},
        },
        "zerotier": {
            "keywords": {"zerotier", "zero tier"},
            "domains": {"vpn_or_mesh_networking"},
        },
        "proton_vpn": {
            "keywords": {"proton vpn", "protonvpn"},
            "domains": {"vpn_or_mesh_networking"},
        },
        "malwarebytes": {
            "keywords": {"malwarebytes"},
            "domains": {"endpoint_security"},
        },
        "veracrypt": {
            "keywords": {"veracrypt", "vera crypt"},
            "domains": {"encryption"},
        },
    }
    domain_order = ["password_management", "vpn_or_mesh_networking", "endpoint_security", "encryption"]
    tool_order = list(tool_definitions.keys())
    detected_tools: set[str] = set()
    detected_domains: set[str] = set()
    app_names: set[str] = set()

    for app in installed_apps:
        name = str(app.get("name") or "").strip()
        publisher = str(app.get("publisher") or "").strip()
        haystack = f"{name} {publisher}".lower()
        for tool, definition in tool_definitions.items():
            if any(keyword in haystack for keyword in definition["keywords"]):
                detected_tools.add(tool)
                detected_domains.update(definition["domains"])
                if name:
                    app_names.add(name)

    domains = [domain for domain in domain_order if domain in detected_domains]
    setup_hints = []
    if "password_management" in detected_domains:
        setup_hints.append("uses_password_manager")
    if "vpn_or_mesh_networking" in detected_domains:
        setup_hints.append("uses_private_networking")
    if "endpoint_security" in detected_domains:
        setup_hints.append("uses_endpoint_security_tooling")
    if "encryption" in detected_domains:
        setup_hints.append("uses_encryption_tooling")

    return {
        "present": bool(detected_tools),
        "tool_families": [tool for tool in tool_order if tool in detected_tools],
        "domains": domains,
        "app_names": sorted(app_names),
        "setup_hints": setup_hints,
        "evidence_basis": "installed_apps",
    }


def derive_office_documents_profile(raw_evidence: list[dict]) -> dict:
    documents = [entry for entry in raw_evidence if entry.get("entity_kind") == "office_document"]
    return _derive_document_route_profile(documents, evidence_basis="office_document_light_preview")


def derive_recent_documents_profile(raw_evidence: list[dict]) -> dict:
    documents = [entry for entry in raw_evidence if entry.get("entity_kind") == "recent_document"]
    return _derive_document_route_profile(documents, evidence_basis="windows_recent_shortcuts")


def derive_steam_playtime_profile(raw_evidence: list[dict]) -> dict:
    entries = [entry for entry in raw_evidence if entry.get("entity_kind") == "steam_public_playtime"]
    if not entries:
        return {
            "present": False,
            "source": None,
            "privacy_limited": False,
            "game_count": 0,
            "top_games_by_playtime": [],
            "evidence_basis": "steam_public_profile",
        }
    fields = entries[0].get("fields", {})
    games = fields.get("games", []) if isinstance(fields.get("games", []), list) else []
    top_games = sorted(
        [game for game in games if isinstance(game, dict)],
        key=lambda game: float(game.get("playtime_forever_hours") or 0),
        reverse=True,
    )[:20]
    return {
        "present": bool(games),
        "source": fields.get("source"),
        "privacy_limited": bool(fields.get("privacy_limited")),
        "game_count": len(games),
        "top_games_by_playtime": top_games,
        "evidence_basis": "steam_public_profile",
    }


def derive_downloads_profile(raw_evidence: list[dict]) -> dict:
    file_entries = [entry for entry in raw_evidence if entry.get("entity_kind") == "downloaded_file"]
    browser_entries = [entry for entry in raw_evidence if entry.get("entity_kind") == "browser_downloads"]
    extension_counts = Counter(str(entry.get("fields", {}).get("file_extension") or "") for entry in file_entries)
    extension_counts.pop("", None)
    category_counts = Counter(str(entry.get("fields", {}).get("category") or "other") for entry in file_entries)

    browser_downloads = []
    source_domains: set[str] = set()
    for entry in browser_entries:
        fields = entry.get("fields", {})
        for domain in fields.get("download_domains", []):
            if str(domain).strip():
                source_domains.add(str(domain))
        for download in fields.get("downloads", []):
            if isinstance(download, dict):
                browser_downloads.append(download)

    local_names = {str(entry.get("fields", {}).get("filename") or "").lower() for entry in file_entries}
    matched = []
    for download in browser_downloads:
        target_name = str(download.get("target_name") or "")
        if target_name.lower() not in local_names:
            continue
        matched.append(
            {
                "filename": target_name,
                "file_extension": download.get("file_extension"),
                "source_domains": download.get("source_domains", []),
            }
        )

    return {
        "present": bool(file_entries or browser_downloads),
        "filesystem_file_count": len(file_entries),
        "browser_record_count": len(browser_downloads),
        "extension_counts": dict(sorted(extension_counts.items())),
        "category_counts": dict(sorted(category_counts.items())),
        "browser_source_domains": sorted(source_domains),
        "matched_browser_downloads": matched[:50],
        "top_files": [entry.get("fields", {}) for entry in file_entries[:50]],
        "skill_coupling": {
            "routes_by_extension": {
                ".pdf": ["pdf-reader"],
                ".docx": ["docx"],
                ".pptx": ["pptx"],
                ".xlsx": ["office-document-specialist-suite"],
                ".png": ["ppocrv5"],
                ".jpg": ["ppocrv5"],
                ".jpeg": ["ppocrv5"],
            }
        },
        "evidence_basis": "downloads_folder_and_browser_downloads",
    }


def _derive_document_route_profile(documents: list[dict], *, evidence_basis: str) -> dict:
    type_counts = Counter(str(entry.get("fields", {}).get("document_type") or "unknown") for entry in documents)
    specialized_order = ["docx", "pptx", "xlsx"]
    specialized_skills = [skill for skill in specialized_order if type_counts.get(skill, 0)]

    candidates = []
    for entry in sorted(documents, key=lambda item: float(item.get("fields", {}).get("modified_time") or 0), reverse=True)[:10]:
        fields = entry.get("fields", {})
        candidates.append(
            {
                "path": entry.get("path"),
                "filename": fields.get("filename"),
                "document_type": fields.get("document_type"),
                "preview_text": fields.get("preview_text", ""),
                "recommended_skills": fields.get("recommended_skills", []),
                "skill_routes": fields.get("skill_routes", []),
                "modified_time": fields.get("modified_time"),
            }
        )

    return {
        "present": bool(documents),
        "document_count": len(documents),
        "type_counts": {key: type_counts[key] for key in specialized_order if type_counts.get(key, 0)},
        "deep_read_candidates": candidates,
        "skill_coupling": {
            "primary_skill": "specialized_document_skill",
            "specialized_skills": specialized_skills,
            "routing_contract": "Use skill_routes[].arguments.path to deep-read selected documents on demand.",
        },
        "evidence_basis": evidence_basis,
    }


def derive_creative_tools_profile(installed_apps: list[dict]) -> dict:
    tool_definitions = {
        "adobe": {
            "keywords": {"adobe", "photoshop", "illustrator", "premiere", "after effects", "lightroom"},
            "domains": {"image_design", "video_editing"},
        },
        "blender": {
            "keywords": {"blender"},
            "domains": {"3d_creation"},
        },
        "davinci_resolve": {
            "keywords": {"davinci resolve", "blackmagic design"},
            "domains": {"video_editing"},
        },
        "ableton": {
            "keywords": {"ableton", "live 12", "live 11"},
            "domains": {"audio_production"},
        },
        "fl_studio": {
            "keywords": {"fl studio", "image-line"},
            "domains": {"audio_production"},
        },
        "reaper": {
            "keywords": {"reaper"},
            "domains": {"audio_production"},
        },
        "audacity": {
            "keywords": {"audacity"},
            "domains": {"audio_production"},
        },
    }
    domain_order = ["audio_production", "image_design", "video_editing", "3d_creation"]
    tool_order = list(tool_definitions.keys())
    detected_tools: set[str] = set()
    detected_domains: set[str] = set()
    app_names: set[str] = set()

    for app in installed_apps:
        name = str(app.get("name") or "").strip()
        publisher = str(app.get("publisher") or "").strip()
        haystack = f"{name} {publisher}".lower()
        for tool, definition in tool_definitions.items():
            if any(keyword in haystack for keyword in definition["keywords"]):
                detected_tools.add(tool)
                detected_domains.update(definition["domains"])
                if name:
                    app_names.add(name)

    return {
        "present": bool(detected_tools),
        "tool_families": [tool for tool in tool_order if tool in detected_tools],
        "domains": [domain for domain in domain_order if domain in detected_domains],
        "app_names": sorted(app_names),
        "evidence_basis": "installed_apps",
    }


def derive_hardware_profile(installed_apps: list[dict]) -> dict:
    vendor_keywords = {
        "nvidia": {"nvidia", "geforce"},
        "amd": {"amd", "radeon", "ryzen master", "advanced micro devices"},
        "intel": {"intel graphics", "intel arc", "intel driver"},
        "logitech": {"logitech", "g hub"},
        "razer": {"razer", "synapse"},
        "corsair": {"corsair", "icue"},
        "elgato": {"elgato", "stream deck"},
        "steelseries": {"steelseries", "gg"},
    }
    gpu_vendors = {"nvidia", "amd", "intel"}
    peripheral_vendors = {"logitech", "razer", "corsair", "elgato", "steelseries"}
    detected: set[str] = set()

    for app in installed_apps:
        app_name = str(app.get("name") or "").lower()
        publisher = str(app.get("publisher") or "").lower()
        haystack = f"{app_name} {publisher}"
        for vendor, keywords in vendor_keywords.items():
            if vendor == "corsair" and "corsair" not in app_name and "icue" not in app_name:
                continue
            if any(keyword in haystack for keyword in keywords):
                detected.add(vendor)

    gpu_tooling = sorted(detected & gpu_vendors)
    peripheral_tooling = sorted(detected & peripheral_vendors)
    setup_hints = []
    if gpu_tooling:
        setup_hints.append("discrete_gpu_tooling")
    if peripheral_tooling:
        setup_hints.append("gaming_peripherals")
    if "elgato" in peripheral_tooling:
        setup_hints.append("stream_deck_or_capture_controls")

    return {
        "present": bool(detected),
        "vendor_families": sorted(detected),
        "gpu_tooling": gpu_tooling,
        "peripheral_tooling": peripheral_tooling,
        "setup_hints": setup_hints,
        "evidence_basis": "installed_apps",
    }


def derive_llm_context(
    *,
    gaming_profile: dict,
    developer_profile: dict,
    ai_tools_profile: dict,
    meaningful_activity_profile: dict,
    local_signal_coverage: dict,
    next_questions: dict,
) -> dict:
    primary_modes = meaningful_activity_profile.get("primary_modes", [])
    present_categories = local_signal_coverage.get("present_categories", [])
    mode_phrase = _format_natural_list(primary_modes[:3])
    strongest_categories = ", ".join(present_categories[:5])
    summary = f"One-shot local scout found {mode_phrase} signals. Strongest categories: {strongest_categories}."

    game_count = int(meaningful_activity_profile.get("breadth", {}).get("installed_game_count", 0))
    platform_count = int(meaningful_activity_profile.get("breadth", {}).get("game_platform_count", 0))
    strong_facts = []
    if game_count:
        game_word = "game" if game_count == 1 else "games"
        platform_word = "platform" if platform_count == 1 else "platforms"
        strong_facts.append(f"Installed game libraries contain {game_count} unique {game_word} across {platform_count} {platform_word}.")
    if developer_profile.get("is_developer"):
        strong_facts.append("Developer workflow signals are present from repositories, editors, workspaces, or shell history.")
    if ai_tools_profile.get("uses_ai_tools"):
        tools = ", ".join(ai_tools_profile.get("tool_families", []))
        strong_facts.append(f"AI tooling signals are present: {tools}.")
    strong_facts.append(
        f"Broad local coverage is {local_signal_coverage.get('breadth_score', 0)}/{local_signal_coverage.get('category_count', 0)} categories."
    )

    weak_hints = []
    if meaningful_activity_profile.get("evidence_quality", {}).get("recent_activity") == "weak":
        weak_hints.append("Recent activity is weak because this scout does not run as a background monitor.")

    uncertainties = [item["why"] for item in next_questions.get("ask_user", []) if item.get("topic") != "recent_activity"]
    return {
        "summary": summary,
        "strong_facts": strong_facts,
        "weak_hints": weak_hints,
        "uncertainties": uncertainties,
        "boundaries": [
            "Do not infer live playtime from installed game libraries.",
            "Do not infer private message contents from app presence.",
            "Do not treat this scout as audio/voice separation tooling.",
            "Do not treat this scout as simultaneous interpretation tooling.",
        ],
        "good_followups": [item["question"] for item in next_questions.get("ask_user", []) if item.get("question")],
    }


def derive_next_questions(*, local_signal_coverage: dict, meaningful_activity_profile: dict) -> dict:
    categories = local_signal_coverage.get("categories", {})
    evidence_quality = meaningful_activity_profile.get("evidence_quality", {})
    ask_user = []

    if evidence_quality.get("recent_activity") == "weak":
        ask_user.append(
            {
                "topic": "recent_activity",
                "question": "Do you want a one-shot ActivityWatch bucket check for recent app/window context, if ActivityWatch is already running?",
                "why": "Installed libraries are strong, but recent activity is weak without a local activity source.",
            }
        )
    if not categories.get("knowledge", {}).get("present"):
        ask_user.append(
            {
                "topic": "knowledge_tools",
                "question": "Do you want to include Obsidian or Joplin config paths in the next scan?",
                "why": "No knowledge-tool signal was found in this scan.",
            }
        )
    if not categories.get("creator", {}).get("present"):
        ask_user.append(
            {
                "topic": "creator_tools",
                "question": "Do you want to include OBS Studio profile paths in the next scan?",
                "why": "No creator-tool signal was found in this scan.",
            }
        )

    return {
        "ask_user": ask_user,
        "do_not_ask": [
            {
                "topic": "audio_voice",
                "reason": "Out of scope for AI Local Scout; this project should not drift into voice separation.",
            },
            {
                "topic": "translation",
                "reason": "Out of scope for AI Local Scout; this project should not drift into simultaneous interpretation.",
            },
        ],
    }


def derive_local_signal_coverage(raw_evidence: list[dict]) -> dict:
    category_entities = {
        "games": {
            "steam_game_manifest",
            "epic_game_manifest",
            "legendary_installed",
            "playnite_library_game",
            "gog_installed",
            "amazon_games_install_info",
            "xbox_game_config",
            "itch_butler_db",
            "ubisoft_launcher_installs",
            "battle_net_launcher_installs",
            "battle_net_product_db",
            "origin_localcontent_manifest",
            "steam_public_playtime",
        },
        "browser": {"browser_bookmarks", "browser_history", "browser_downloads", "browser_extensions", "browser_sessions"},
        "downloads": {"downloaded_file"},
        "development": {"git_repo_config", "workspace_file", "editor_recent_workspaces", "cursor_state_db", "shell_history"},
        "ai_tools": {"claude_entrypoint", "claude_settings", "claude_mcp_config", "codex_config", "codex_auth", "codex_rules", "cursor_state_db"},
        "knowledge": {"obsidian_global_config", "obsidian_vault", "joplin_profile"},
        "creator": {"obs_studio_profile", "obs_studio_scene_collection"},
        "communication": {"discord_settings", "teams_config"},
        "sync_storage": {"dropbox_info", "onedrive_global_config", "nextcloud_config", "syncthing_config"},
        "terminal_remote": {"windows_terminal_settings", "ssh_config", "kubeconfig"},
        "cloud_runtime": {
            "docker_desktop_settings",
            "docker_cli_config",
            "docker_context_meta",
            "wsl_global_config",
            "wsl_distribution_list",
            "aws_cli_config",
            "azure_cli_profile",
            "gcloud_active_config",
            "gcloud_cli_config",
        },
        "language_tooling": {
            "cargo_user_config",
            "cargo_credentials_store",
            "maven_user_settings",
            "gradle_user_properties",
            "nuget_user_config",
            "dotnet_global_tools",
            "npm_user_config",
            "pnpm_user_config",
            "pip_user_config",
            "conda_user_config",
            "poetry_user_config",
            "rustup_settings",
            "uv_user_config",
            "uv_credentials_store",
            "yarn_user_config",
        },
        "local_apps": {"installed_apps"},
        "generic_data": {"sqlite_database"},
        "office_documents": {"office_document"},
        "recent_documents": {"recent_document"},
    }
    entity_counts = Counter(str(entry.get("entity_kind")) for entry in raw_evidence)
    categories = {}
    for category, entity_kinds in category_entities.items():
        present_kinds = sorted(kind for kind in entity_kinds if entity_counts.get(kind, 0) > 0)
        evidence_count = sum(entity_counts[kind] for kind in present_kinds)
        categories[category] = {
            "present": bool(present_kinds),
            "entity_kinds": present_kinds,
            "evidence_count": evidence_count,
            "quality": quality_from_count(evidence_count),
        }

    categories["audio_voice"] = {
        "present": False,
        "entity_kinds": [],
        "evidence_count": 0,
        "quality": "absent",
        "reason": "out_of_scope_for_local_scout",
    }
    categories["translation"] = {
        "present": False,
        "entity_kinds": [],
        "evidence_count": 0,
        "quality": "absent",
        "reason": "out_of_scope_for_local_scout",
    }

    ordered_categories = [*category_entities.keys(), "audio_voice", "translation"]
    present_categories = [category for category in ordered_categories if categories[category]["present"]]
    absent_categories = [category for category in ordered_categories if not categories[category]["present"]]
    return {
        "category_count": len(ordered_categories),
        "breadth_score": len(present_categories),
        "present_categories": present_categories,
        "absent_categories": absent_categories,
        "categories": {category: categories[category] for category in ordered_categories},
    }


def derive_meaningful_activity_profile(
    *,
    gaming_profile: dict,
    developer_profile: dict,
    ai_tools_profile: dict,
    knowledge_tools_profile: dict,
    creator_profile: dict,
    sync_storage_profile: dict,
    terminal_tools_profile: dict,
    cloud_tools_profile: dict,
    linux_runtime_profile: dict,
    container_tools_profile: dict,
    activitywatch_profile: dict,
) -> dict:
    primary_modes = []
    setup_hints = []

    def add_mode(condition: bool, mode: str) -> None:
        if condition:
            primary_modes.append(mode)

    def add_hint(condition: bool, hint: str) -> None:
        if condition:
            setup_hints.append(hint)

    installed_game_names = [str(name).strip() for name in gaming_profile.get("installed_game_names", []) if str(name).strip()]
    game_count = len(set(installed_game_names))
    game_platform_count = len(gaming_profile.get("platforms", []))
    add_mode(bool(gaming_profile.get("is_gamer")), "gaming")
    add_mode(bool(developer_profile.get("is_developer")), "building")
    add_mode(bool(ai_tools_profile.get("uses_ai_tools")), "ai_tool_use")
    add_mode(bool(knowledge_tools_profile.get("present")), "knowledge_work")
    add_mode(bool(creator_profile.get("present")), "content_creation")
    add_mode(bool(sync_storage_profile.get("present")), "sync_storage")
    add_mode(bool(terminal_tools_profile.get("present")), "terminal_work")
    add_mode(bool(cloud_tools_profile.get("present")), "cloud_work")
    add_mode(bool(linux_runtime_profile.get("present")), "local_linux_runtime")
    add_mode(bool(container_tools_profile.get("present")), "container_work")

    add_hint(game_platform_count >= 2, "multi_launcher_game_library")
    add_hint(bool(developer_profile.get("is_developer")), "developer_workstation")
    add_hint(bool(ai_tools_profile.get("uses_ai_tools")), "ai_augmented_workstation")
    add_hint(bool(knowledge_tools_profile.get("present")), "notes_and_vaults")
    add_hint(bool(creator_profile.get("present")), "creator_streaming_setup")
    add_hint(bool(sync_storage_profile.get("present")), "cross_device_sync")
    add_hint(bool(terminal_tools_profile.get("present")), "terminal_centered_setup")
    add_hint(bool(cloud_tools_profile.get("present")), "cloud_cli_ready")
    add_hint(bool(linux_runtime_profile.get("present")), "local_linux_runtime")
    add_hint(bool(container_tools_profile.get("present")), "containerized_dev")

    gaming_style_hints = derive_gaming_style_hints(installed_game_names)
    evidence_quality = {
        "gaming": quality_from_count(game_count),
        "development": "strong" if developer_profile.get("is_developer") else "absent",
        "ai_tool_use": "strong" if ai_tools_profile.get("uses_ai_tools") else "absent",
        "knowledge_work": "strong" if knowledge_tools_profile.get("present") else "absent",
        "content_creation": "strong" if creator_profile.get("present") else "absent",
        "sync_storage": "strong" if sync_storage_profile.get("present") else "absent",
        "recent_activity": "medium" if activitywatch_profile.get("present") else "weak",
    }

    summary = build_meaningful_summary(
        primary_modes=primary_modes,
        gaming_style_hints=gaming_style_hints,
        game_count=game_count,
        game_platform_count=game_platform_count,
        evidence_quality=evidence_quality,
    )
    return {
        "primary_modes": primary_modes,
        "gaming_style_hints": gaming_style_hints,
        "pc_setup_hints": setup_hints,
        "evidence_quality": evidence_quality,
        "breadth": {
            "primary_mode_count": len(primary_modes),
            "setup_hint_count": len(setup_hints),
            "game_platform_count": game_platform_count,
            "installed_game_count": game_count,
        },
        "wholesome_summary": summary,
    }


def derive_gaming_style_hints(game_names: list[str]) -> list[str]:
    ordered_hints = [
        "cozy_indie",
        "competitive_multiplayer",
        "strategy",
        "rpg_story",
        "sandbox_survival",
        "sandbox_creative",
        "aaa_single_player",
        "modding_friendly",
    ]
    keyword_map = {
        "cozy_indie": {"stardew", "short hike", "celeste", "tiny sticker", "unpacking", "slime rancher"},
        "competitive_multiplayer": {"counter-strike", "dota", "valorant", "league of legends", "overwatch", "fortnite", "apex"},
        "strategy": {"civilization", "starcraft", "total war", "stellaris", "xcom", "age of empires"},
        "rpg_story": {"witcher", "mass effect", "cyberpunk", "baldur", "dragon age", "disco elysium", "avowed"},
        "sandbox_survival": {"rust", "valheim", "terraria", "subnautica", "ark", "palworld"},
        "sandbox_creative": {"minecraft", "factorio", "satisfactory", "rimworld", "cities skylines"},
        "aaa_single_player": {"assassin", "alan wake", "red dead", "god of war", "horizon", "spider-man"},
        "modding_friendly": {"skyrim", "fallout", "rimworld", "minecraft"},
    }
    normalized_names = [str(name).strip().lower() for name in game_names if str(name).strip()]
    hints = []
    for hint in ordered_hints:
        keywords = keyword_map[hint]
        if any(keyword in game_name for game_name in normalized_names for keyword in keywords):
            hints.append(hint)
    return hints


def quality_from_count(count: int) -> str:
    if count >= 1:
        return "strong"
    return "absent"


def build_meaningful_summary(
    *,
    primary_modes: list[str],
    gaming_style_hints: list[str],
    game_count: int,
    game_platform_count: int,
    evidence_quality: dict,
) -> list[str]:
    summary = []
    if primary_modes:
        summary.append("This PC shows a broad one-shot profile across " + ", ".join(primary_modes[:5]) + ".")
    if game_count:
        platform_text = "platform" if game_platform_count == 1 else "platforms"
        summary.append(
            f"Game signals come from install/library evidence: {game_count} installed games across {game_platform_count} {platform_text}."
        )
    if gaming_style_hints:
        summary.append("The installed games suggest " + ", ".join(gaming_style_hints[:5]) + " tastes, not live playtime.")
    if evidence_quality.get("recent_activity") == "weak":
        summary.append("Recent activity is intentionally weak here because the scout does not run as a background monitor.")
    return summary


def _format_natural_list(items: list[str]) -> str:
    if not items:
        return "no strong"
    label_overrides = {"ai_tool_use": "AI tool use"}
    labels = [label_overrides.get(item, item.replace("_", " ")) for item in items]
    if len(labels) == 1:
        return labels[0]
    if len(labels) == 2:
        return f"{labels[0]} and {labels[1]}"
    return f"{labels[0]}, {labels[1]}, and {labels[2]}"
