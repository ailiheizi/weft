# WEFT vNext Blueprint: Product Generation Manager

## 1. Purpose

This document defines the WEFT Product Generation Manager direction.

The goal is to make product package usage feel simple, safe, and stable by managing product instances as immutable, switchable generations. The model is intentionally closer to Nix-style profiles and generations than to a dependency toggle panel.

The key rule is:

**Core owns product generation truth. Package Server owns package supply truth. UI, CLI, shell scripts, and AI are only clients.**

## 2. Motivation

WEFT needs to support:

- multiple dependency scenarios for the same product instance
- exact package, version, source, and digest binding
- safe candidate generation before activation
- fast switching between known-good generations
- rollback without depending on a live Package Server
- shell/CLI-first control that AI and GUI can reuse

This should not be modeled as arbitrary dependency switches. Most users do not want to manage low-level packages directly. They want to choose a usable product state and know that it can be verified, activated, and rolled back.

## 3. Relationship to existing vNext documents

This document extends, but does not replace:

- `../foundations/13-product-package-instance-model.md`
- `04-declaration-config-lock.md`
- `05-binding-and-resolution.md`
- `06-profiles-and-policy.md`
- `16-package-service.md`
- `18-self-hosted-package-server-blueprint.md`

The existing ownership split remains unchanged:

- Package is the source-side published object.
- Product Package expresses product default intent.
- Instance owns local runtime config, lock, active generation, and rollback history.
- Package Server supplies catalog, candidates, closure metadata, artifact metadata, and verification metadata.
- Core performs final resolve, writes locks, verifies candidates, activates generations, and rolls back.

## 4. Core concepts

### 4.1 Product Generation Manager

The Product Generation Manager is not a new source object. It is Core's runtime responsibility for managing generations under a product instance.

It owns:

- candidate generation creation
- generation verification
- active generation selection
- previous generation tracking
- rollback to a known-good generation
- exact lock identity
- local materialization metadata

It does not own package catalog truth. That remains Package Server responsibility.

### 4.2 Scene

A scene is a named dependency and binding scenario under an instance.

Examples:

- `stable`
- `team`
- `experimental-mcp`
- `safe-mode`

A scene answers:

**Which feature and binding choices should be resolved into a generation?**

A scene may contain:

- enabled and disabled product features
- capability binding pins
- exact package pins
- a base generation
- user-readable intent metadata

Scenes are not source packages. They are instance-local runtime inputs.

### 4.3 Profile

A profile remains a policy and execution trust mode.

Examples:

- `safe`
- `developer`
- `trusted`

Profile and scene must not be conflated:

- profile controls what is allowed
- scene controls what dependency and binding scenario is requested

For example, `scene = "team"` can be used with `profile = "developer"`, while `scene = "safe-mode"` can be used with `profile = "safe"`.

### 4.4 Binding Set

A binding set is the normalized result of capability provider choices for a scene.

It records mappings such as:

```text
agent.runtime  -> agent-runtime@0.1.0 sha512:...
memory.store   -> memory-store@0.2.4 sha512:...
team.delegate  -> team-runtime@0.1.0 sha512:...
```

A binding set must be digestible. The same normalized binding input should produce the same `binding_set_id`.

### 4.5 Closure

A closure is the complete package set required by a resolved scene.

It includes direct and transitive product, feature, foundation, and provider packages. It must include exact package identity:

- package name
- version
- source
- manifest digest
- artifact digest
- content hash
- provenance or verification metadata when available

The closure is recorded in the lock and represented by a stable `closure_id` or `closure_digest`.

### 4.6 Generation

A generation is an immutable, verifiable product instance result.

A generation records:

- product instance
- scene
- profile
- parent generation
- binding set identity
- closure identity
- lock digest
- verification results
- status
- creation source, such as `cli`, `gui`, `ai`, or `import`

Changing feature selection, package versions, binding pins, profile, or product declaration inputs produces a new generation.

### 4.7 Active pointer

Activation should be modeled as changing the active generation pointer after verification.

Activation must not mutate the previous generation. A previous active generation remains available as a rollback candidate as long as its lock and local materialized packages remain available.

### 4.8 Content-addressed store object

A store object is a local materialization of a package artifact identified by content.

Conceptually:

```text
.weft/store/
  sha512-aaa-agent-runtime-0.1.0/
  sha512-bbb-memory-store-0.2.4/
  sha512-ccc-team-runtime-0.1.0/
```

The lock may record a local store path, but path is not the trust root. The trust root is digest identity:

- content hash
- manifest digest
- artifact digest
- signature/provenance metadata where available

## 5. Desired local instance shape

A mature instance can be represented like this:

```text
.weft/instances/weft-code/
  config.toml
  lock.toml
  active
  previous
  scenes/
    stable.toml
    team.toml
    experimental-mcp.toml
  generations/
    18.lock.toml
    19.lock.toml
    20.lock.toml
```

Current repository paths may continue to use `.weft/<product>/` during transition. The important semantics are:

- scenes are instance-local inputs
- generations are immutable results
- `active` is a pointer
- `lock.toml` mirrors or points to the active generation lock

## 6. Lifecycle

### 6.1 Create or update a scene

The user, CLI, GUI, or AI creates or updates a scene. This does not change the active generation.

### 6.2 Resolve scene

Core reads:

- product package declaration
- instance config
- selected scene
- package candidates and metadata
- profile and policy

Core then resolves the scene into a candidate binding set and closure.

### 6.3 Generate candidate

Core writes a candidate generation with an exact lock.

The current active generation remains unchanged.

### 6.4 Verify candidate

Core verifies the candidate generation. Verification may include:

- metadata coverage
- binding coverage
- digest/signature checks
- local materialization checks
- capability probes
- product-level smoke checks

### 6.5 Activate candidate

Only a verified generation can be activated. Activation changes the active generation pointer and records the previous active generation.

### 6.6 Roll back or switch

Rollback is activation of a previous known-good generation. Switching to any verified generation should use the same activation path.

## 7. CLI and shell as the primary control surface

The generation manager should be controllable from shell first.

Representative commands:

```bash
weft gen current weft-code
weft gen list weft-code
weft gen show weft-code 18

weft scene create weft-code team --from current
weft scene bind weft-code team \
  --capability team.delegate \
  --package team-runtime \
  --version 0.1.0 \
  --hash sha512:...

weft scene generate weft-code team
weft gen verify weft-code 19
weft gen activate weft-code 19
weft gen activate weft-code 18
weft gen diff weft-code 18 19

weft store verify
weft store gc --dry-run
```

The GUI should call the same APIs. AI should generate or explain the same commands and structured requests. AI must not get a privileged mutation path.

## 8. Package Server role

Package Server can support this model by returning advisory metadata:

- package candidates
- candidate dependency closures
- artifact set metadata
- manifest digest
- artifact digest
- verification metadata
- replay and rollback metadata

Package Server must not:

- write instance locks
- set active generation pointers
- activate generations
- decide rollback truth
- become mandatory for replaying an already locked and materialized generation

## 9. Lock metadata requirements

A generation lock should be able to record:

- `scene`
- `binding_set_id`
- `closure_id`
- `closure_digest`
- `store_generation_id`
- per-package artifact digest
- per-package manifest digest
- per-package store object identity
- per-binding package version and hash
- binding source layer

This lets WEFT answer:

- what is active now?
- why is this package included?
- what exact package set is locked?
- can this generation be replayed without the server?
- which previous generation can be activated safely?

## 10. Target on-disk contracts

This section describes the desired contract. The repository may keep the current transitional paths while the implementation is staged, but new behavior should move toward these semantics.

### 10.1 Instance directory

```text
.weft/<instance-name>/
  config.toml                  # user intent and active scene selection
  lock.toml                    # compatibility mirror of active generation lock
  active                       # text file containing active generation id
  previous                     # text file containing previous active generation id
  scenes/
    stable.toml
    team.toml
    safe-mode.toml
  generations/
    18.lock.toml
    19.lock.toml
    20.lock.toml
  generation-store.toml        # index only; not the trust root
```

Rules:

- `scenes/*.toml` are mutable inputs.
- `generations/*.lock.toml` are immutable outputs once verified.
- `active` is the activation pointer.
- `previous` is a convenience pointer used for one-step rollback.
- `lock.toml` remains as a compatibility mirror of the active generation during migration.
- The lock content, not the index file, is the source of replay and rollback truth.

### 10.2 Scene file shape

Example:

```toml
schema_version = 1
name = "team"
description = "Enable team delegation with a pinned agent runtime."
profile = "developer"
base_generation = 18

[features]
enabled = ["weft-code-team-feature"]
disabled = []

[[bindings]]
capability = "team.delegate"
package = "agent-runtime"
provider = "agent-runtime"
version = "0.1.0"
sha512 = "sha512:..."
reason = "Team scene delegates agent work through the official runtime."

[[packages]]
package = "agent-runtime"
version = "0.1.0"
sha512 = "sha512:..."
source = "local-index"
```

Scene rules:

- A scene may pin a capability binding, a package, or both.
- A scene may request features, but Core must still resolve and verify them against the product declaration and profile policy.
- A scene must not directly mutate the active generation.
- Missing versions or hashes are allowed only for preview or candidate resolution; verified generation locks must record exact identity.

### 10.3 Generation index shape

Example:

```toml
active = 20
previous = 19
next_id = 21

[[generations]]
id = 18
status = "verified"
scene = "stable"
profile = "safe"
lock_path = "generations/18.lock.toml"
created_by = "cli"
created_at = 1710000000

[[generations]]
id = 19
status = "verified"
scene = "team"
profile = "developer"
lock_path = "generations/19.lock.toml"
created_by = "gui"
created_at = 1710001000

[[generations]]
id = 20
status = "active"
scene = "team"
profile = "developer"
lock_path = "generations/20.lock.toml"
created_by = "cli"
created_at = 1710002000
```

Index rules:

- The index is a navigation structure, not a substitute for generation lock files.
- If the index and lock disagree, Core should prefer immutable generation locks and report the inconsistency.
- A generation may be listed as `candidate`, `verified`, `active`, `failed`, or `archived`.
- `rollback` should be treated as an action, not as a permanent generation status. Previous known-good generations remain `verified` unless active.

### 10.4 Generation lock shape

Example:

```toml
lock_version = 2
app = "weft-code"
generation = 20
status = "active"
profile = "developer"
scene = "team"
binding_set_id = "binding-set:sha256:..."
closure_id = "closure:sha256:..."
closure_digest = "sha256:..."
store_generation_id = "store-gen:sha256:..."

[inputs]
declaration_schema_version = 1
config_schema_version = 1
declaration_digest = "sha256:..."
config_digest = "sha256:..."
scene_digest = "sha256:..."

[assembly]
enabled_features = ["weft-code-team-feature"]
selected_packages = ["weft-code", "agent-runtime"]
scene = "team"
binding_set_id = "binding-set:sha256:..."
closure_id = "closure:sha256:..."
closure_digest = "sha256:..."

[[packages]]
name = "agent-runtime"
version = "0.1.0"
source = "local-index"
sha512 = "sha512:..."
manifest_digest = "sha256:..."
artifact_digest = "sha256:..."
artifact_set_id = "artifact-set:sha256:..."
store_object_id = "store:sha512:..."
store_path = ".weft/store/sha512-...-agent-runtime-0.1.0"
closure_id = "closure:sha256:..."
runtime_provider = "agent-runtime"
provides = ["team.delegate"]

[[bindings]]
capability = "team.delegate"
provider = "agent-runtime"
package = "agent-runtime"
package_version = "0.1.0"
package_sha512 = "sha512:..."
mutable = false
binding_source = "scene"
```

Lock rules:

- A verified lock must contain enough information to explain every package and binding.
- A verified lock must not rely on live Package Server state for replay.
- Package Server metadata may be copied into the lock as advisory evidence, but Core remains the writer.
- `lock.toml` at the instance root should be replaced atomically when activation succeeds.

## 11. Core data model plan

### 11.1 Transitional model names

The current code uses `App*` types for vNext product/package structures. The generation manager work should not perform a broad `App*` to `Product*` rename in the first rounds. Renaming is cosmetic and high-risk because it touches many APIs, tests, and frontend types.

For now:

- `AppManifest` continues to represent product package declaration.
- `AppConfigFile` continues to represent instance config.
- `AppLockFile` continues to represent instance or generation lock.
- `AppGeneration` continues to represent generation summary.
- Documentation may say “product” while code still says `App`.

### 11.2 Scene config additions

Add scene support to instance config in a backward-compatible way:

```rust
pub struct AppConfigFile {
    pub app_runtime: AppRuntimeConfig,
    pub features: AppFeaturesConfig,
    pub binding_overrides: HashMap<String, AppBindingOverride>,
    pub active_scene: String,
    pub scenes: HashMap<String, AppSceneConfig>,
}

pub struct AppSceneConfig {
    pub name: String,
    pub description: String,
    pub profile: String,
    pub base_generation: Option<u64>,
    pub enabled_features: Vec<String>,
    pub disabled_features: Vec<String>,
    pub binding_pins: Vec<AppSceneBindingPin>,
    pub package_pins: Vec<AppScenePackagePin>,
}
```

Compatibility rules:

- Existing configs without `active_scene` continue to load.
- Empty `active_scene` means “use legacy config-level feature and binding inputs.”
- Scene definitions are ignored until scene-aware generation is explicitly called.

### 11.3 Generation summary additions

`AppGeneration` should stay a summary object, not the full lock.

Target fields:

```rust
pub struct AppGeneration {
    pub id: u64,
    pub app_name: String,
    pub version: String,
    pub scene: String,
    pub profile: String,
    pub binding_set_id: String,
    pub closure_id: String,
    pub lock_digest: String,
    pub lock_path: String,
    pub parent_generation: Option<u64>,
    pub created_by: String,
    pub status: GenerationStatus,
    pub created_at: u64,
}
```

However, this should be introduced only when all test builders and API response expectations are updated in the same implementation slice. If adding these fields causes widespread literal churn, prefer a helper constructor in tests before expanding public fields.

### 11.4 Generation store evolution

Current store shape:

```rust
pub struct AppGenerationStore {
    pub active: Option<AppGeneration>,
    pub candidate: Option<AppGeneration>,
    pub rollback: Option<AppGeneration>,
    pub next_id: u64,
}
```

Target store shape:

```rust
pub struct AppGenerationStore {
    pub active: Option<u64>,
    pub previous: Option<u64>,
    pub candidate: Option<u64>,
    pub generations: BTreeMap<u64, AppGeneration>,
    pub next_id: u64,
}
```

Migration rules:

- Existing in-memory active/candidate/rollback behavior must continue during transition.
- First persistent version may write only `generation-store.toml` without changing activation semantics.
- Rollback should become “activate previous verified generation,” not “mark current as failed.”

## 12. Resolver and identity plan

### 12.1 Binding set normalization

Core should normalize binding choices before generating an id.

Inputs:

- capability name
- chosen provider
- package name
- version
- content hash or artifact digest
- binding source layer: declaration, feature, config override, scene, CLI

Normalization rules:

- Sort entries by capability name.
- Omit display-only fields such as reason text.
- Include exact package identity when available.
- Hash canonical JSON or canonical TOML, not Rust debug formatting.

Result:

```text
binding_set_id = "binding-set:sha256:<digest>"
```

### 12.2 Closure normalization

Core should normalize the final package set after all direct and transitive requirements are resolved.

Inputs per package:

- package name
- version
- source
- manifest digest
- artifact digest
- content hash
- provides/requires capabilities
- selected reason or feature source

Rules:

- Sort packages by name, version, and source.
- Include every package needed to replay the generation.
- Include direct and transitive packages.
- Treat missing digest as candidate-only unless policy explicitly allows local development mode.

Result:

```text
closure_id = "closure:sha256:<digest>"
closure_digest = "sha256:<digest>"
```

### 12.3 Store object identity

Store object identity should be content-addressed:

```text
store_object_id = "store:sha512:<digest>"
```

Rules:

- Store path is derived from identity and human-readable package metadata.
- Store path is not the trust root.
- A store object can be shared by many generations.
- Store object metadata must not require Package Server availability for verification once materialized.

## 13. API contract plan

The HTTP API should expose generation lifecycle as Core-owned operations. Names below are representative; exact paths can follow existing route style.

### 13.1 Read APIs

```http
GET /api/apps/:name/scenes
GET /api/apps/:name/scenes/:scene
GET /api/apps/:name/generations
GET /api/apps/:name/generations/current
GET /api/apps/:name/generations/:id
GET /api/apps/:name/generations/:id/diff/:other_id
```

Read API requirements:

- Return current active generation id.
- Return scene/profile/binding/closure summaries.
- Explain package inclusion reason when known.
- Never require mutation or Package Server calls for already locked generations.

### 13.2 Mutation APIs

```http
PUT  /api/apps/:name/scenes/:scene
POST /api/apps/:name/scenes/:scene/generate
POST /api/apps/:name/generations/:id/verify
POST /api/apps/:name/generations/:id/activate
POST /api/apps/:name/generations/rollback
```

Mutation API requirements:

- Scene update must not activate anything.
- Generate must create a candidate generation only.
- Verify must record validation results.
- Activate must require verified status unless an explicit development override exists.
- Rollback must use the same activation path as normal switching.

### 13.3 Error contract

Representative structured errors:

```json
{
  "error": "Candidate must be verified before activation",
  "reason": "generation_not_verified",
  "generation": 20
}
```

Common reasons:

- `scene_not_found`
- `package_not_found`
- `binding_unresolved`
- `digest_missing`
- `policy_denied`
- `generation_not_verified`
- `generation_lock_missing`
- `store_object_missing`
- `activation_failed`

## 14. CLI contract plan

CLI should be the first mutation surface. GUI and AI should call the same API or produce the same commands.

### 14.1 Scene commands

```bash
weft scene list weft-code
weft scene show weft-code team
weft scene create weft-code team --from current
weft scene enable-feature weft-code team weft-code-team-feature
weft scene disable-feature weft-code team legacy-feature
weft scene bind weft-code team \
  --capability team.delegate \
  --package agent-runtime \
  --version 0.1.0 \
  --hash sha512:...
```

### 14.2 Generation commands

```bash
weft gen current weft-code
weft gen list weft-code
weft gen show weft-code 20
weft gen generate weft-code --scene team
weft gen verify weft-code 20
weft gen activate weft-code 20
weft gen rollback weft-code
weft gen diff weft-code 19 20
```

### 14.3 Store commands

```bash
weft store verify
weft store verify --generation weft-code:20
weft store gc --dry-run
```

Initial CLI implementation may be thin wrappers around HTTP APIs. The important rule is that CLI semantics come before GUI mutation flows.

## 15. Package Server contract plan

Package Server remains advisory supply infrastructure.

### 15.1 Allowed Package Server responsibilities

Package Server may provide:

- package search
- product install/update preview
- provider candidates
- dependency closure preview
- artifact set metadata
- manifest and artifact digests
- provenance and signature metadata
- compatibility notes

### 15.2 Forbidden Package Server responsibilities

Package Server must not:

- write instance config
- write generation locks
- write active pointers
- activate or roll back generations
- decide local runtime truth
- become required to replay an already locked generation

### 15.3 Core integration rule

Core may use Package Server responses as input, but must copy required replay data into the generation lock before verification succeeds.

## 16. Verification and activation semantics

### 16.1 Candidate verification checks

Minimum checks:

- product declaration digest present
- config or scene digest present
- all required capabilities bound
- every binding maps to an included package
- exact package version recorded
- exact package hash or artifact digest recorded for verified mode
- profile policy allows selected packages and capabilities
- local store object exists when activation requires materialization

Development-mode exceptions may exist, but they must be visible in validation results.

### 16.2 Activation rules

Activation must:

- require verified generation status
- write immutable generation lock first
- atomically update active pointer
- update compatibility `lock.toml` mirror
- record previous active generation
- leave previous generation lock unchanged

Activation must not:

- mutate the previous generation lock
- fetch new Package Server state during pointer switch
- silently activate a candidate with failed validation

### 16.3 Rollback rules

Rollback is a specialized activation:

```text
previous verified generation -> active generation
```

Rollback must use the same checks as normal activation, except it should not re-resolve dependencies. It should verify local lock and store availability only.

## 17. GUI and AI contract

### 17.1 GUI role

GUI should first be read-only for generation manager state:

- current active generation
- scene list
- generation list
- package closure explanation
- differences between generations
- rollback availability

GUI mutation should come only after CLI/Core lifecycle is stable.

### 17.2 AI role

AI should be a client, not an authority.

AI may:

- explain what is installed and why
- propose scene changes
- generate CLI commands
- call the same APIs as GUI/CLI when explicitly authorized
- help diagnose failed verification

AI must not:

- bypass Core verification
- directly edit active pointers
- directly write locks as a privileged path
- invent package hashes or trust metadata

## 18. Migration plan

### Phase 0: Planning and boundaries

Deliverables:

- this architecture document
- explicit non-goals
- current behavior inventory

Acceptance:

- no runtime behavior changed
- Package Server/Core boundary remains unchanged
- implementation slices are separately reviewable

### Phase 1: Backward-compatible metadata fields

Deliverables:

- config structs can deserialize scene metadata
- lock structs can deserialize and serialize generation metadata
- existing config and lock files still load
- no new activation semantics

Acceptance:

- `cargo check` passes
- focused config/lock tests pass
- existing API responses remain compatible or documented

### Phase 2: Generation summary and test helpers

Deliverables:

- generation summary can reference scene, binding set, closure, lock path
- tests use helper constructors instead of large raw literals
- current active/candidate/rollback behavior remains intact

Acceptance:

- no widespread literal churn after this phase
- generation tests pass
- activation and rollback behavior unchanged except extra metadata

### Phase 3: Persistent generation index

Deliverables:

- generation index load/save
- active and previous pointer representation
- compatibility import from current active lock

Acceptance:

- existing instances can start without index
- Core can reconstruct active generation from existing lock
- index corruption produces a clear diagnostic, not silent activation

### Phase 4: Scene-aware candidate generation

Deliverables:

- resolve selected scene into candidate generation
- scene feature/binding/package pins participate in resolution
- candidate lock records scene digest

Acceptance:

- scene update does not activate
- candidate can be inspected before activation
- failed scene resolution returns structured error

### Phase 5: Binding set and closure identities

Deliverables:

- canonical binding set digest
- canonical closure digest
- lock records both identities

Acceptance:

- same inputs produce same ids
- package order does not change ids
- lock can explain why each package exists

### Phase 6: CLI-first lifecycle

Deliverables:

- scene list/show/create/bind commands
- generation list/show/generate/verify/activate/rollback commands

Acceptance:

- all mutation paths available from shell
- GUI is not required to operate lifecycle
- AI can safely produce command plans

### Phase 7: Store metadata and replay verification

Deliverables:

- store object identity fields
- local store verification command
- replay check for locked generation

Acceptance:

- already materialized verified generation can be checked offline
- missing local object blocks activation unless explicitly allowed by profile

### Phase 8: Package Server preview enrichment

Deliverables:

- closure preview metadata
- artifact digest metadata
- provenance metadata when available

Acceptance:

- Package Server still does not activate or write local truth
- Core copies required replay metadata into locks

### Phase 9: GUI read-only inspection, then mutation

Deliverables:

- read-only generation/scene dashboard
- diff and rollback visibility
- later, guarded mutation actions using same APIs

Acceptance:

- GUI displays the same truth as CLI
- GUI mutation cannot bypass verification

## 19. Implementation guardrails

For every implementation slice:

- Keep the diff small and focused.
- Do not introduce real artifact storage until store metadata is designed and tested.
- Do not make Package Server responsible for active runtime truth.
- Do not add AI-specific privileged mutation paths.
- Do not perform broad `App*` to `Product*` renames.
- Prefer additive schema changes with `serde(default)`.
- Add or update focused Rust tests for every behavior change.
- Run appropriate Cargo verification before claiming completion.

## 20. First implementation slice checklist

The next safe implementation slice should be metadata-only.

Allowed:

- Add scene structs to config with defaults.
- Add lock metadata fields with defaults.
- Preserve existing load/save behavior.
- Add tests proving old configs and locks still deserialize.
- Add tests proving new scene and lock metadata round-trip.

Not allowed yet:

- Change activation semantics.
- Change rollback semantics.
- Persist generation history.
- Add real store materialization.
- Add GUI mutation.
- Require Package Server for existing local generation startup.

Acceptance commands:

```bash
cargo check
cargo test app::config
cargo test app::generation
```

If integration tests compile against raw `AppGeneration` literals, introduce local test helpers before adding public generation fields.

## 21. Final architecture decisions

This section resolves the design questions that would otherwise block a complete implementation plan.

### 21.1 Scene storage decision

Decision:

- Scene definitions live in `scenes/*.toml`.
- `config.toml` stores only `active_scene` plus legacy compatibility inputs.
- During transition, `config.toml [scenes]` may be accepted for backward-compatible deserialization, but Core should save scenes as files.

Rationale:

- Scene files are easier to diff, copy, review, and generate from CLI or AI.
- `config.toml` remains the instance entrypoint instead of becoming a large generation database.
- This matches the goal that scenes are mutable inputs while generation locks are immutable outputs.

Implementation rule:

- If the same scene exists in both `config.toml` and `scenes/<name>.toml`, the file under `scenes/` wins and Core should report a non-fatal warning.

### 21.2 Active and previous pointer decision

Decision:

- `active` and `previous` are separate pointer files.
- `generation-store.toml` also records `active` and `previous` as an index cache.
- Pointer files are authoritative for activation state; the index is repairable metadata.

Rationale:

- Pointer files make activation atomic and easy to inspect from shell.
- The index can be regenerated from lock files and pointer files.
- Activation should not require rewriting a large index before the active pointer changes.

Implementation rule:

- Activation writes generation lock first, updates `previous`, then atomically replaces `active`, then updates `generation-store.toml` best-effort.
- If index update fails after pointer update, Core should report a repair-needed warning but keep the successful activation.

### 21.3 Root lock decision

Decision:

- Root `lock.toml` remains a full copy of the active generation lock during vNext transition.
- Later, it may become a pointer only after all consumers read `active` and `generations/*.lock.toml` directly.

Rationale:

- Existing startup paths already expect `.weft/<instance>/lock.toml`.
- Full-copy compatibility avoids breaking existing tooling and frontend APIs.
- The immutable generation lock remains the real historical artifact.

Implementation rule:

- `generations/<id>.lock.toml` is written first.
- After successful activation, root `lock.toml` is atomically replaced with the same content but status set to `active`.

### 21.4 Development digest policy decision

Decision:

- `safe` and `trusted` profiles require exact artifact or content digests for verified generations.
- `developer` profile may verify local path packages without artifact digests only when the lock records `dev_unsealed = true` and validation results explicitly mark this exception.
- `dev_unsealed` generations can be activated only under `developer` profile.

Rationale:

- Local development needs flexibility.
- Safe rollback requires visible trust boundaries.
- Users should never confuse unsealed local state with reproducible verified state.

Implementation rule:

- Any missing digest in `safe` or `trusted` profile fails verification with `digest_missing`.
- Any missing digest in `developer` profile produces a warning validation result and sets generation trust level to `dev-unsealed`.

### 21.5 Canonical hash format decision

Decision:

- Canonical identity hashes use canonical JSON encoded as UTF-8.
- Object keys are sorted lexicographically.
- Arrays that represent sets are sorted by stable identity fields.
- Arrays that represent ordered user intent remain ordered only when order changes semantics.
- Hash algorithm is SHA-256 for binding set, closure, scene, config, and declaration identities.
- Artifact content may use SHA-512 when package metadata already exposes SHA-512.

Rationale:

- Canonical JSON is easier to implement consistently across Rust, Node, and future CLI tooling than TOML canonicalization.
- SHA-256 is sufficient for structural identity; SHA-512 can remain artifact/content-specific.

Implementation rule:

```text
scene_digest       = "sha256:<canonical-json-digest>"
binding_set_id     = "binding-set:sha256:<canonical-json-digest>"
closure_id         = "closure:sha256:<canonical-json-digest>"
closure_digest     = "sha256:<canonical-json-digest>"
store_object_id    = "store:sha512:<content-digest>"
```

### 21.6 Package reason decision

Decision:

- Package inclusion reasons are represented as a list, not a single string.
- Each reason records source layer and source item.

Example:

```toml
[[packages.reasons]]
layer = "feature"
source = "weft-code-team-feature"
message = "Feature requires team delegation runtime."

[[packages.reasons]]
layer = "scene"
source = "team"
message = "Scene pins agent-runtime for team.delegate."
```

Rationale:

- The same package can be selected by product defaults, feature selection, scene pin, and transitive dependency.
- A list preserves explainability without hiding conflicts.

Implementation rule:

- Conflict resolution selects one final package identity, but all non-conflicting reasons remain visible.
- Conflicting exact pins fail resolution unless an explicit override rule chooses one.

### 21.7 Package Server trust decision

Decision:

- Core may trust Package Server for candidate discovery and advisory closure preview.
- Core must recompute local binding coverage and final selected package set.
- Core may copy Package Server digests/provenance into lock evidence.
- Core must not treat Package Server activation recommendations as runtime truth.

Rationale:

- Package Server is supply-side infrastructure.
- Core is runtime authority.
- Offline replay and rollback require local lock truth.

Implementation rule:

- Verification succeeds only against Core's local final lock.
- Package Server closure preview mismatch becomes a warning or failure depending on profile policy.

## 22. Complete state machine

### 22.1 Generation states

Allowed states:

```text
draft      # scene or config exists but no generated lock yet
candidate  # generated lock exists, verification not passed yet
verified   # verification passed, not currently active
active     # currently selected generation
failed     # verification failed or activation failed before pointer switch
archived   # hidden from normal switching but retained for audit
```

`rollback` is not a generation state. Rollback is an operation that activates a previous verified generation.

### 22.2 State transitions

```text
draft
  -> candidate          generate(scene)

candidate
  -> verified           verify(candidate) passes
  -> failed             verify(candidate) fails
  -> archived           discard candidate

verified
  -> active             activate(verified)
  -> archived           archive generation

active
  -> verified           another generation is activated
  -> failed             only if post-activation health policy marks it failed

failed
  -> candidate          regenerate from same scene
  -> archived           archive failed generation

archived
  -> verified           explicit unarchive after lock/store verification
```

### 22.3 Transition invariants

- There is at most one active generation per instance.
- Activation cannot target `candidate`, `failed`, or `archived` generations.
- `active` pointer must reference an existing generation lock.
- A verified generation lock is immutable.
- A generation id is never reused.
- A generation may be recreated from the same scene, but it receives a new id.
- Rollback never re-resolves dependencies.
- Switch and rollback use the same activation implementation.

## 23. Complete schemas

### 23.1 Scene schema v1

```toml
schema_version = 1
name = "team"
description = "Human-readable intent."
profile = "developer"
base_generation = 18

[features]
enabled = ["feature-a"]
disabled = ["feature-b"]

[[bindings]]
capability = "team.delegate"
package = "agent-runtime"
provider = "agent-runtime"
version = "0.1.0"
sha512 = "sha512:..."
source = "local-index"
reason = "Pin team delegate provider."

[[packages]]
package = "agent-runtime"
version = "0.1.0"
sha512 = "sha512:..."
source = "local-index"
reason = "Required by team scene."

[metadata]
created_by = "cli"
created_at = 1710000000
updated_at = 1710001000
```

Required fields for saving a named scene:

- `schema_version`
- `name`

Required fields for verified generation:

- every selected package must resolve to exact `name`, `version`, `source`, and digest identity in the lock.

### 23.2 Generation index schema v1

```toml
schema_version = 1
active = 20
previous = 19
next_id = 21

[[generations]]
id = 20
status = "active"
scene = "team"
profile = "developer"
binding_set_id = "binding-set:sha256:..."
closure_id = "closure:sha256:..."
lock_digest = "sha256:..."
lock_path = "generations/20.lock.toml"
parent_generation = 19
created_by = "cli"
created_at = 1710002000
trust_level = "verified"
```

Required fields:

- `schema_version`
- `next_id`
- `generations[].id`
- `generations[].status`
- `generations[].lock_path`

Repair behavior:

- Missing index with valid `active` and lock files: rebuild index.
- Missing pointer with valid index: require explicit repair command before activation.
- Pointer to missing lock: fail startup generation manager load with actionable diagnostic.

### 23.3 Generation lock schema v2 additions

`AppLockFile` should include these top-level fields:

```toml
scene = "team"
scene_digest = "sha256:..."
binding_set_id = "binding-set:sha256:..."
closure_id = "closure:sha256:..."
closure_digest = "sha256:..."
store_generation_id = "store-gen:sha256:..."
trust_level = "verified"
dev_unsealed = false
```

`inputs` should include:

```toml
declaration_digest = "sha256:..."
config_digest = "sha256:..."
scene_digest = "sha256:..."
package_index_digest = "sha256:..."
package_server_snapshot_digest = "sha256:..."
```

`packages[]` should include:

```toml
name = "agent-runtime"
version = "0.1.0"
source = "local-index"
sha512 = "sha512:..."
manifest_digest = "sha256:..."
artifact_digest = "sha256:..."
artifact_set_id = "artifact-set:sha256:..."
store_object_id = "store:sha512:..."
store_path = ".weft/store/sha512-...-agent-runtime-0.1.0"
closure_id = "closure:sha256:..."
runtime_provider = "agent-runtime"
provides = ["team.delegate"]
requires = []

[[packages.reasons]]
layer = "scene"
source = "team"
message = "Scene pins team.delegate."
```

`bindings[]` should include:

```toml
capability = "team.delegate"
provider = "agent-runtime"
package = "agent-runtime"
package_version = "0.1.0"
package_sha512 = "sha512:..."
mutable = false
binding_source = "scene"
```

## 24. Complete API response contracts

### 24.1 Generation list response

```json
{
  "app": "weft-code",
  "active": 20,
  "previous": 19,
  "generations": [
    {
      "id": 20,
      "status": "active",
      "scene": "team",
      "profile": "developer",
      "binding_set_id": "binding-set:sha256:...",
      "closure_id": "closure:sha256:...",
      "lock_digest": "sha256:...",
      "trust_level": "verified",
      "created_by": "cli",
      "created_at": 1710002000
    }
  ]
}
```

### 24.2 Generation detail response

```json
{
  "generation": {
    "id": 20,
    "status": "active",
    "scene": "team",
    "profile": "developer",
    "binding_set_id": "binding-set:sha256:...",
    "closure_id": "closure:sha256:...",
    "lock_path": "generations/20.lock.toml"
  },
  "features": ["weft-code-team-feature"],
  "bindings": [
    {
      "capability": "team.delegate",
      "provider": "agent-runtime",
      "package": "agent-runtime",
      "version": "0.1.0",
      "hash": "sha512:...",
      "source": "scene"
    }
  ],
  "packages": [
    {
      "name": "agent-runtime",
      "version": "0.1.0",
      "hash": "sha512:...",
      "reasons": [
        {"layer": "scene", "source": "team", "message": "Scene pins team.delegate."}
      ]
    }
  ],
  "validation_results": []
}
```

### 24.3 Activation response

```json
{
  "status": "activated",
  "app": "weft-code",
  "active": 20,
  "previous": 19,
  "lock_written": true,
  "pointer_written": true
}
```

### 24.4 Error response

```json
{
  "error": "Package digest is required for safe profile",
  "reason": "digest_missing",
  "app": "weft-code",
  "scene": "team",
  "generation": 20,
  "details": {
    "package": "agent-runtime",
    "profile": "safe"
  }
}
```

## 25. Complete CLI behavior

### 25.1 Command semantics

`weft scene create <app> <scene> --from current`

- Reads current active generation lock.
- Creates `scenes/<scene>.toml` with current feature and binding choices.
- Does not generate or activate.

`weft scene bind <app> <scene> ...`

- Updates a scene binding pin.
- Does not resolve immediately unless `--generate` is passed.

`weft gen generate <app> --scene <scene>`

- Resolves scene.
- Writes candidate generation lock.
- Writes/updates generation index candidate entry.
- Does not activate.

`weft gen verify <app> <id>`

- Runs verification checks.
- Updates validation results and status.
- Does not activate.

`weft gen activate <app> <id>`

- Requires verified status.
- Performs atomic activation.
- Updates root `lock.toml` compatibility mirror.

`weft gen rollback <app>`

- Reads `previous` pointer.
- Verifies previous lock/store availability.
- Activates previous generation through same activation path.

### 25.2 Exit codes

```text
0  success
1  generic failure
2  invalid command or arguments
3  scene not found
4  generation not found
5  verification failed
6  policy denied
7  lock or pointer corruption
8  store object missing
9  package server unavailable when required for new resolution
```

## 26. Complete failure handling

### 26.1 Activation failure points

Activation has ordered steps:

1. Read target generation summary.
2. Read target generation lock.
3. Verify target status and lock digest.
4. Verify local store availability when required.
5. Write immutable lock if it does not already exist.
6. Write temporary previous pointer.
7. Atomically replace active pointer.
8. Atomically replace root `lock.toml` mirror.
9. Update generation index.

Failure behavior:

- Failure before step 7 leaves active pointer unchanged.
- Failure after step 7 means activation succeeded but compatibility mirror or index may need repair.
- Core must surface repair-needed diagnostics instead of silently rolling back pointer changes.

### 26.2 Startup repair rules

At startup, Core should:

1. Read active pointer.
2. Read active generation lock.
3. Compare root `lock.toml` with active generation lock.
4. Rebuild missing generation index if locks and pointers are valid.
5. Warn if root `lock.toml` mirror is stale.
6. Refuse generation operations if active pointer targets a missing lock.

### 26.3 Package Server outage behavior

- Existing active generation startup must not require Package Server.
- Existing verified generation rollback must not require Package Server.
- New scene resolution may require Package Server only if local package index lacks required metadata.
- Package Server outage during advisory preview should degrade to local-only resolution when profile allows it.

## 27. Complete implementation task plan

### Task 0: Freeze architecture and add tests-first helpers

Files:

- `docs/architecture/vNext/runtime/19-generation-manager.md`
- `core/tests/integration.rs`
- focused test modules under `core/src/app/`

Work:

- Treat this document as the controlling plan.
- Add helper constructors for `AppGeneration` tests before expanding public fields.
- Add fixtures for legacy config/lock and new scene/lock schemas.

Acceptance:

- No behavior change.
- Existing tests still compile.

### Task 1: Scene schema load/save

Files:

- `core/src/app/config.rs`
- `core/src/app/mod.rs`

Work:

- Add `AppSceneConfig`, `AppSceneBindingPin`, `AppScenePackagePin`, scene metadata structs.
- Load scene files from `.weft/<instance>/scenes/*.toml`.
- Keep `config.toml` scene fields deserializable for transition.
- Add round-trip tests.

Acceptance:

- Legacy config loads.
- Scene file loads.
- Duplicate scene source chooses file over embedded config with warning.

### Task 2: Lock schema v2 metadata

Files:

- `core/src/app/config.rs`
- lock-related tests

Work:

- Add lock fields: scene digest, binding set id, closure id/digest, trust level, dev unsealed, package reasons.
- Preserve old lock deserialization with defaults.
- Add serialization tests for new fields.

Acceptance:

- Old locks load.
- New locks round-trip.
- No activation behavior changes.

### Task 3: Canonical identity hashing

Files:

- new `core/src/app/identity.rs` or equivalent
- `core/src/app/mod.rs`

Work:

- Implement canonical JSON helpers.
- Implement scene digest, binding set id, closure id.
- Add order-insensitivity tests.

Acceptance:

- Same semantic binding set produces same id regardless of map/package order.
- Display-only reason changes do not alter binding set id.
- Package identity changes alter closure id.

### Task 4: Generation summary expansion

Files:

- `core/src/app/generation.rs`
- `core/src/api/generations.rs`
- tests using generation literals

Work:

- Add summary metadata fields only after test helpers exist.
- Remove permanent `Rollback` status from future-facing paths or keep it only as legacy compatibility until store migration.
- Ensure API responses include scene/closure summaries.

Acceptance:

- Existing generation lifecycle tests pass.
- No raw literal churn remains in integration tests.

### Task 5: Persistent generation index

Files:

- `core/src/app/generation.rs`
- new generation index load/save module if useful
- startup code in `core/src/main.rs`

Work:

- Add index schema.
- Load existing active generation from root lock when index absent.
- Write active/previous pointer files.
- Add repair diagnostics.

Acceptance:

- Existing instances start unchanged.
- Missing index is rebuilt or ignored safely.
- Pointer to missing lock blocks generation operations with clear error.

### Task 6: Scene-aware candidate generation

Files:

- `core/src/api/generations.rs`
- resolver modules under `core/src/app/`

Work:

- Resolve selected scene over product declaration and config.
- Apply feature enable/disable and binding/package pins.
- Generate candidate lock with scene digest, binding set id, closure id.

Acceptance:

- Scene update alone does not activate.
- Generate writes candidate only.
- Conflicting pins fail with structured error.

### Task 7: Verification policy

Files:

- `core/src/app/generation.rs`
- policy/profile modules if present

Work:

- Enforce digest policy by profile.
- Add dev-unsealed warning path.
- Verify binding coverage and closure/package identity.

Acceptance:

- Safe profile rejects missing digest.
- Developer profile can create dev-unsealed verified generation with visible warning.
- Failed verification cannot activate.

### Task 8: Atomic activation and rollback

Files:

- `core/src/app/generation.rs`
- `core/src/api/generations.rs`

Work:

- Implement pointer-based activation.
- Make rollback activate `previous` through same path.
- Keep root lock compatibility mirror.

Acceptance:

- Previous active generation remains immutable.
- Rollback does not re-resolve dependencies.
- Failure before pointer switch leaves active unchanged.

### Task 9: CLI lifecycle

Files:

- CLI crate or command entrypoint to be selected from existing project layout

Work:

- Add scene and generation commands.
- Use Core APIs or direct Core library calls consistently.
- Return documented exit codes.

Acceptance:

- Full lifecycle works from shell without GUI.
- Commands print lock paths and generation ids.

### Task 10: Store metadata verification

Files:

- store module to be introduced

Work:

- Verify store object metadata.
- Do not implement GC until verification is stable.

Acceptance:

- Offline verification of active generation works.
- Missing store object blocks safe/trusted activation.

### Task 11: Package Server preview enrichment

Files:

- `server/src/package_server/*`
- Core package-service client models

Work:

- Add closure/artifact/provenance preview fields.
- Keep server non-mutating.

Acceptance:

- Server never writes local lock or active pointer.
- Core copies required evidence into lock.

### Task 12: GUI read-only, then guarded mutation

Files:

- frontend API client and dashboard components

Work:

- Show scene/generation/closure/diff state.
- Add mutation buttons only after CLI lifecycle passes.

Acceptance:

- GUI state matches CLI output.
- GUI cannot activate unverified generation.

## 28. End-to-end acceptance matrix

| Scenario | Expected result |
|---|---|
| Legacy instance with only `config.toml` and `lock.toml` | Starts and imports active generation |
| Create scene from current | Scene file written, active unchanged |
| Bind package in scene | Scene changes, active unchanged |
| Generate scene | Candidate lock written, active unchanged |
| Verify candidate with full digests in safe profile | Status becomes verified |
| Verify candidate missing digest in safe profile | Fails with `digest_missing` |
| Verify candidate missing digest in developer profile | Succeeds only as `dev-unsealed` with warning |
| Activate verified generation | Active pointer changes, previous recorded, root lock mirror updated |
| Activate unverified generation | Fails, active unchanged |
| Rollback | Previous verified generation becomes active without re-resolve |
| Package Server offline with active generation | Startup and rollback still work |
| Package Server offline for new unresolved package | Generate fails or degrades according to profile |
| Corrupt generation index but valid pointers/locks | Repair or warning, active generation usable |
| Active pointer targets missing lock | Generation operations blocked with repair diagnostic |

## 29. Verification commands by phase

Phase 1-2:

```bash
cargo check
cargo test app::config
cargo test app::generation
```

Phase 3-8:

```bash
cargo check
cargo test app::config
cargo test app::generation
cargo test --test integration generation
```

Package Server phases:

```bash
cargo check
cargo test package_server
```

Frontend read-only phase:

```bash
npm run typecheck:ui
npm run build
```

Full release gate:

```bash
cargo test
cargo clippy
npm run typecheck:ui
npm run build
```

## 30. Non-goals for the first implementation round

The first implementation round should not include:

- real artifact storage
- package garbage collection
- Package Server-owned activation
- GUI-first mutation workflows
- broad `App*` to `Product*` renaming
- AI-specific dependency management paths

The first implementation round should only make the data model ready for exact scene-aware generations while keeping current behavior intact.

## 31. Design conclusion

WEFT should evolve into a Nix-like Product Generation Manager:

```text
scene
  -> binding set
  -> exact package closure
  -> candidate lock
  -> verify
  -> activate or rollback
```

This gives users a simple model:

**Create a safe product scene, verify it, switch to it, and switch back if anything breaks.**

AI remains useful, but only as an optional planner or command generator. The trusted substrate is Core's deterministic generation lifecycle.
