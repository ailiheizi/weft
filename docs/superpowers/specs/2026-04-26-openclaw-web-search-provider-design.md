# OpenClaw Web Search Provider Design

## Goal

Replace WEFT's hard-coded DDG-to-Brave-HTML fallback with an OpenClaw-style `web_search` provider chain.

## Selected Approach

Use provider auto-detection:

- If `BRAVE_API_KEY` is configured, use the official Brave Search API first.
- If no API-backed provider is configured, fall back to DuckDuckGo HTML search as the keyless provider.
- Do not use Brave HTML scraping.
- Keep result caching to reduce bot-triggering traffic.
- Surface provider, source, and failure notes explicitly.

## Components

- `skills/src/lib.rs`: owns the provider chain for `search_web`.
- DuckDuckGo provider: keyless fallback using non-JavaScript HTML results, cache, parser, and challenge detection.
- Brave provider: official API only, using `BRAVE_API_KEY` from env/config when available.
- Tests: update search contracts so they assert provider order, no Bing, no Brave HTML scraping, cache behavior, and challenge fallback.

## Data Flow

1. `search_web` receives query and limit.
2. Resolve available providers by OpenClaw-like priority.
3. Try configured API provider first.
4. If no configured API provider exists, use DDG HTML fallback.
5. Cache successful search payloads for repeated queries.
6. Return JSON with `provider`, `source`, `results`, `notes`, and explicit error state when all providers fail.

## Error Handling

- DDG bot challenge is a provider failure, not a valid empty result.
- Missing Brave key is an unavailable provider, not a runtime crash.
- If a provider was explicitly selected and fails, do not silently fallback.
- If provider selection is automatic, fallback is allowed.

## Testing

- Contract tests verify provider order and no removed fallbacks.
- Parser tests use fixtures rather than live search.
- Live tests classify failures as network/provider/challenge instead of claiming success.
