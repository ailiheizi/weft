from __future__ import annotations

import pytest

from ai_local_scout.inference import (
    derive_agent_config_profile,
    derive_creative_tools_profile,
    derive_hardware_profile,
    derive_gaming_style_hints,
    derive_llm_context,
    derive_local_signal_coverage,
    derive_meaningful_activity_profile,
    derive_next_questions,
    derive_privacy_security_profile,
)


def test_agent_config_profile_summarizes_claude_codex_and_project_rules() -> None:
    profile = derive_agent_config_profile(
        [
            {"entity_kind": "claude_entrypoint", "path": "C:\\Users\\Admin\\CLAUDE.md", "fields": {"referenced_paths": ["AI-Knowledge/as-me.md"]}},
            {"entity_kind": "claude_context_file", "path": "C:\\Users\\Admin\\AI-Knowledge\\as-me.md", "fields": {}},
            {"entity_kind": "claude_settings", "path": "C:\\Users\\Admin\\.claude\\settings.json", "fields": {"defaultModel": "claude-sonnet", "mcp_servers": ["memory"]}},
            {"entity_kind": "claude_mcp_config", "path": "C:\\Users\\Admin\\.claude\\mcp.json", "fields": {"server_names": ["memory", "calendar"]}},
            {"entity_kind": "codex_config", "path": "C:\\Users\\Admin\\.codex\\config.toml", "fields": {"trusted_projects": ["D:/weft"], "mcp_servers": ["context-sync"]}},
            {"entity_kind": "codex_auth", "path": "C:\\Users\\Admin\\.codex\\auth.json", "fields": {}},
            {"entity_kind": "codex_rules", "path": "C:\\Users\\Admin\\.codex\\rules\\default.rules", "fields": {"allowed_rule_count": 2}},
            {"entity_kind": "project_rules", "path": "D:\\weft\\AGENTS.md", "fields": {"heading_count": 1}},
            {"entity_kind": "installed_apps", "path": "apps.json", "fields": {}},
        ]
    )

    assert profile == {
        "present": True,
        "agent_families": ["claude", "codex", "project_agents"],
        "config_surfaces": ["claude_entrypoint", "claude_settings", "claude_mcp_config", "codex_config", "codex_auth", "codex_rules", "project_rules", "claude_context_file"],
        "claude": {
            "entrypoint_count": 1,
            "context_file_count": 1,
            "default_models": ["claude-sonnet"],
            "mcp_servers": ["calendar", "memory"],
        },
        "codex": {
            "config_count": 1,
            "auth_present": True,
            "rules_file_count": 1,
            "approved_rule_count": 2,
            "trusted_project_count": 1,
            "mcp_servers": ["context-sync"],
        },
        "project_rules": {
            "agents_file_count": 1,
            "paths": ["D:\\weft\\AGENTS.md"],
        },
        "setup_hints": ["uses_claude_project_memory", "uses_mcp_servers", "uses_codex_trusted_projects", "uses_project_agents_rules"],
        "evidence_basis": "raw_evidence_agent_config",
    }


def test_agent_config_profile_stays_absent_without_agent_config_evidence() -> None:
    profile = derive_agent_config_profile(
        [
            {"entity_kind": "installed_apps", "path": "apps.json", "fields": {}},
            {"entity_kind": "git_repo_config", "path": "D:\\repo\\.git\\config", "fields": {}},
        ]
    )

    assert profile == {
        "present": False,
        "agent_families": [],
        "config_surfaces": [],
        "claude": {
            "entrypoint_count": 0,
            "context_file_count": 0,
            "default_models": [],
            "mcp_servers": [],
        },
        "codex": {
            "config_count": 0,
            "auth_present": False,
            "rules_file_count": 0,
            "approved_rule_count": 0,
            "trusted_project_count": 0,
            "mcp_servers": [],
        },
        "project_rules": {
            "agents_file_count": 0,
            "paths": [],
        },
        "setup_hints": [],
        "evidence_basis": "raw_evidence_agent_config",
    }


def test_privacy_security_profile_groups_installed_privacy_and_security_apps() -> None:
    profile = derive_privacy_security_profile(
        [
            {"name": "1Password", "publisher": "AgileBits Inc."},
            {"name": "Bitwarden", "publisher": "Bitwarden Inc."},
            {"name": "Tailscale", "publisher": "Tailscale Inc."},
            {"name": "Proton VPN", "publisher": "Proton AG"},
            {"name": "Malwarebytes", "publisher": "Malwarebytes"},
            {"name": "Steam", "publisher": "Valve"},
        ]
    )

    assert profile == {
        "present": True,
        "tool_families": ["1password", "bitwarden", "tailscale", "proton_vpn", "malwarebytes"],
        "domains": ["password_management", "vpn_or_mesh_networking", "endpoint_security"],
        "app_names": ["1Password", "Bitwarden", "Malwarebytes", "Proton VPN", "Tailscale"],
        "setup_hints": ["uses_password_manager", "uses_private_networking", "uses_endpoint_security_tooling"],
        "evidence_basis": "installed_apps",
    }


def test_privacy_security_profile_stays_absent_without_privacy_or_security_apps() -> None:
    profile = derive_privacy_security_profile(
        [
            {"name": "Steam", "publisher": "Valve"},
            {"name": "Blender", "publisher": "Blender Foundation"},
        ]
    )

    assert profile == {
        "present": False,
        "tool_families": [],
        "domains": [],
        "app_names": [],
        "setup_hints": [],
        "evidence_basis": "installed_apps",
    }


def test_creative_tools_profile_groups_installed_creator_apps() -> None:
    profile = derive_creative_tools_profile(
        [
            {"name": "Blender", "publisher": "Blender Foundation"},
            {"name": "Adobe Photoshop 2025", "publisher": "Adobe Inc."},
            {"name": "DaVinci Resolve", "publisher": "Blackmagic Design"},
            {"name": "Ableton Live 12 Suite", "publisher": "Ableton"},
            {"name": "FL Studio", "publisher": "Image-Line"},
            {"name": "Steam", "publisher": "Valve"},
        ]
    )

    assert profile == {
        "present": True,
        "tool_families": ["adobe", "blender", "davinci_resolve", "ableton", "fl_studio"],
        "domains": ["audio_production", "image_design", "video_editing", "3d_creation"],
        "app_names": ["Ableton Live 12 Suite", "Adobe Photoshop 2025", "Blender", "DaVinci Resolve", "FL Studio"],
        "evidence_basis": "installed_apps",
    }


def test_creative_tools_profile_stays_absent_without_creator_apps() -> None:
    profile = derive_creative_tools_profile(
        [
            {"name": "Steam", "publisher": "Valve"},
            {"name": "NVIDIA Graphics Driver", "publisher": "NVIDIA Corporation"},
        ]
    )

    assert profile == {
        "present": False,
        "tool_families": [],
        "domains": [],
        "app_names": [],
        "evidence_basis": "installed_apps",
    }


def test_hardware_profile_infers_gpu_and_peripheral_hints_from_installed_apps() -> None:
    profile = derive_hardware_profile(
        [
            {"name": "NVIDIA Graphics Driver", "publisher": "NVIDIA Corporation"},
            {"name": "NVIDIA GeForce Experience", "publisher": "NVIDIA Corporation"},
            {"name": "AMD Ryzen Master", "publisher": "Advanced Micro Devices"},
            {"name": "Logitech G HUB", "publisher": "Logitech"},
            {"name": "Razer Synapse", "publisher": "Razer Inc."},
            {"name": "Elgato Stream Deck", "publisher": "Corsair Memory, Inc."},
            {"name": "Steam", "publisher": "Valve"},
        ]
    )

    assert profile == {
        "present": True,
        "vendor_families": ["amd", "elgato", "logitech", "nvidia", "razer"],
        "gpu_tooling": ["amd", "nvidia"],
        "peripheral_tooling": ["elgato", "logitech", "razer"],
        "setup_hints": ["discrete_gpu_tooling", "gaming_peripherals", "stream_deck_or_capture_controls"],
        "evidence_basis": "installed_apps",
    }


def test_hardware_profile_stays_absent_without_hardware_vendor_apps() -> None:
    profile = derive_hardware_profile(
        [
            {"name": "Steam", "publisher": "Valve"},
            {"name": "Visual Studio Code", "publisher": "Microsoft"},
        ]
    )

    assert profile == {
        "present": False,
        "vendor_families": [],
        "gpu_tooling": [],
        "peripheral_tooling": [],
        "setup_hints": [],
        "evidence_basis": "installed_apps",
    }


def test_meaningful_activity_profile_helper_keeps_weak_recent_activity_boundary() -> None:
    meaningful = derive_meaningful_activity_profile(
        gaming_profile={"is_gamer": True, "platforms": ["steam"], "installed_game_names": ["Stardew Valley"]},
        developer_profile={"is_developer": False},
        ai_tools_profile={"uses_ai_tools": False},
        knowledge_tools_profile={"present": False},
        creator_profile={"present": False},
        sync_storage_profile={"present": False},
        terminal_tools_profile={"present": False},
        cloud_tools_profile={"present": False},
        linux_runtime_profile={"present": False},
        container_tools_profile={"present": False},
        activitywatch_profile={"present": False},
    )

    assert meaningful["primary_modes"] == ["gaming"]
    assert meaningful["gaming_style_hints"] == ["cozy_indie"]
    assert meaningful["evidence_quality"] == {
        "gaming": "strong",
        "development": "absent",
        "ai_tool_use": "absent",
        "knowledge_work": "absent",
        "content_creation": "absent",
        "sync_storage": "absent",
        "recent_activity": "weak",
    }
    assert meaningful["breadth"] == {
        "primary_mode_count": 1,
        "setup_hint_count": 0,
        "game_platform_count": 1,
        "installed_game_count": 1,
    }
    assert meaningful["wholesome_summary"][-1] == "Recent activity is intentionally weak here because the scout does not run as a background monitor."


def test_meaningful_activity_profile_helper_marks_activitywatch_as_medium_recent_activity() -> None:
    meaningful = derive_meaningful_activity_profile(
        gaming_profile={"is_gamer": False, "platforms": [], "installed_game_names": []},
        developer_profile={"is_developer": False},
        ai_tools_profile={"uses_ai_tools": False},
        knowledge_tools_profile={"present": False},
        creator_profile={"present": False},
        sync_storage_profile={"present": False},
        terminal_tools_profile={"present": False},
        cloud_tools_profile={"present": False},
        linux_runtime_profile={"present": False},
        container_tools_profile={"present": False},
        activitywatch_profile={"present": True},
    )

    assert meaningful["evidence_quality"]["recent_activity"] == "medium"
    assert all("background monitor" not in line for line in meaningful["wholesome_summary"])


@pytest.mark.parametrize(
    ("game_name", "expected_hint"),
    [
        ("Stardew Valley", "cozy_indie"),
        ("Counter-Strike 2", "competitive_multiplayer"),
        ("Sid Meier's Civilization VI", "strategy"),
        ("The Witcher 3: Wild Hunt", "rpg_story"),
        ("Rust", "sandbox_survival"),
        ("Minecraft", "sandbox_creative"),
        ("Skyrim Special Edition", "modding_friendly"),
    ],
)
def test_gaming_style_hint_helper_maps_known_titles(game_name: str, expected_hint: str) -> None:
    assert expected_hint in derive_gaming_style_hints([game_name])


def test_gaming_style_hint_helper_does_not_overclassify_stardew_as_modding() -> None:
    hints = derive_gaming_style_hints(["Stardew Valley"])

    assert hints == ["cozy_indie"]


def test_local_signal_coverage_helper_maps_raw_evidence_to_categories() -> None:
    coverage = derive_local_signal_coverage(
        [
            {"entity_kind": "steam_game_manifest"},
            {"entity_kind": "browser_history"},
            {"entity_kind": "cursor_state_db"},
            {"entity_kind": "obsidian_vault"},
            {"entity_kind": "syncthing_config"},
            {"entity_kind": "aws_cli_config"},
            {"entity_kind": "sqlite_database"},
        ]
    )

    assert coverage["present_categories"] == [
        "games",
        "browser",
        "development",
        "ai_tools",
        "knowledge",
        "sync_storage",
        "cloud_runtime",
        "generic_data",
    ]
    assert coverage["categories"]["development"] == {
        "present": True,
        "entity_kinds": ["cursor_state_db"],
        "evidence_count": 1,
        "quality": "strong",
    }
    assert coverage["categories"]["audio_voice"] == {
        "present": False,
        "entity_kinds": [],
        "evidence_count": 0,
        "quality": "absent",
        "reason": "out_of_scope_for_local_scout",
    }


def test_next_questions_helper_asks_only_for_weak_or_missing_evidence() -> None:
    next_questions = derive_next_questions(
        local_signal_coverage={
            "categories": {
                "knowledge": {"present": False},
                "creator": {"present": True},
            }
        },
        meaningful_activity_profile={"evidence_quality": {"recent_activity": "weak"}},
    )

    assert [item["topic"] for item in next_questions["ask_user"]] == ["recent_activity", "knowledge_tools"]
    assert [item["topic"] for item in next_questions["do_not_ask"]] == ["audio_voice", "translation"]


def test_next_questions_helper_skips_recent_activity_when_evidence_is_not_weak() -> None:
    next_questions = derive_next_questions(
        local_signal_coverage={
            "categories": {
                "knowledge": {"present": True},
                "creator": {"present": True},
            }
        },
        meaningful_activity_profile={"evidence_quality": {"recent_activity": "medium"}},
    )

    assert next_questions["ask_user"] == []


def test_llm_context_helper_separates_facts_hints_boundaries_and_followups() -> None:
    context = derive_llm_context(
        gaming_profile={},
        developer_profile={"is_developer": True},
        ai_tools_profile={"uses_ai_tools": True, "tool_families": ["claude", "cursor"]},
        meaningful_activity_profile={
            "primary_modes": ["gaming", "building", "ai_tool_use"],
            "breadth": {"installed_game_count": 3, "game_platform_count": 2},
            "evidence_quality": {"recent_activity": "weak"},
        },
        local_signal_coverage={
            "present_categories": ["games", "development", "ai_tools"],
            "breadth_score": 3,
            "category_count": 15,
        },
        next_questions={
            "ask_user": [
                {
                    "topic": "recent_activity",
                    "question": "Ask about recent activity?",
                    "why": "Recent activity is weak.",
                },
                {
                    "topic": "creator_tools",
                    "question": "Ask about creator tools?",
                    "why": "No creator-tool signal was found in this scan.",
                },
            ]
        },
    )

    assert context["summary"] == "One-shot local scout found gaming, building, and AI tool use signals. Strongest categories: games, development, ai_tools."
    assert context["strong_facts"] == [
        "Installed game libraries contain 3 unique games across 2 platforms.",
        "Developer workflow signals are present from repositories, editors, workspaces, or shell history.",
        "AI tooling signals are present: claude, cursor.",
        "Broad local coverage is 3/15 categories.",
    ]
    assert context["weak_hints"] == ["Recent activity is weak because this scout does not run as a background monitor."]
    assert context["uncertainties"] == ["No creator-tool signal was found in this scan."]
    assert context["good_followups"] == ["Ask about recent activity?", "Ask about creator tools?"]
