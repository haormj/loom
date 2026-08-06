# Playbook — Enterprise Best-Practice Layer for Loom

Loom ships a generic "best practice" core: coding references, design guidance,
review rules, commit conventions, and the logic that decides *which* reference
applies to *which* task. That core is useful for the open-source default, but
every enterprise has its own standards. This document describes the **Playbook**
abstraction that lifts the entire business-practice layer out of Rust source
code into a self-describing, versionable, enterprise-editable data unit.

> Status: design accepted. Implementation is staged (see
> [Migration](#migration)). This branch delivers Step 1 (registry extraction,
> behavior-equivalent).

## Why a Playbook

Today Loom's reference knowledge is hard-wired into Rust:

- The reference taxonomy (`ref_id` namespace such as `tech.code.java.security`,
  `bk.spring.security`, `fe.react.core`) is built from `format!` and string
  literals inside `contracts/{code_quality,api_quality,ui_quality,browser_quality}.rs`.
- The `ref_id → file path` mapping is scattered across the `*_reference_load_plan`
  helpers.
- The selection logic ("which references does a given task need?") is a large
  block of `match` / `if has_focus(...)` branches in Rust.
- The stack recognizer (language/framework detection from a `TechnicalBaseline`
  selection) is a chain of `contains_any` keyword checks.
- The installed reference file list is a ~200-entry `&[&str]` constant in
  `setup/lib.rs`.

The runtime that *reads* reference content (`state/request_resolver.rs:
read_reference_value`) is already decoupled — it treats markdown/txt as opaque
text. The coupling is entirely in *how ref_ids and paths are produced and
selected*. A Playbook externalizes that.

A Playbook is **not** just "a folder of markdown". It is a self-describing unit
containing the taxonomy, the selection rules, the stack matchers, **and** the
content — so a business can orchestrate its own practices without touching Rust.

## What a Playbook Contains

```
playbooks/<name>/
  manifest.toml      name, loom_version_range, mode (base | extend)
  registry.yaml      ref_id -> { path, category, reason }   (taxonomy + paths)
  rules.yaml         when { lang, frameworks, focus, actions } -> select [ref_id...]
  matchers.yaml      keywords -> { language, frameworks, roles }
  references/**      markdown content (layered: project -> user -> builtin)
```

### registry.yaml — taxonomy + path table

The single source of truth for every `ref_id`. Replaces all `format!` and
literal `ref_id`/`path` strings in the `*_load_plan` functions and the
`*_enum_refs` enumerations. Adding or removing a reference is a data edit; no
Rust change, and the install file list is derived from it.

```yaml
code_refs:
  springboot:
    security: { ref_id: bk.spring.security, path: tech/backend/springboot/security.md, reason: "Selected Spring Boot security framework quality reference for this task." }
  react:
    core: { ref_id: fe.react.core, path: tech/frontend/react/core.md, reason: "Selected React core frontend framework quality reference for this task." }
  # an enterprise adds:
  acme:
    sec-checklist: { ref_id: acme.sec-checklist, path: acme/sec-checklist.md, reason: "Selected Acme security checklist for this task." }
```

### rules.yaml — selection rules (Step 2)

Replaces the `match`/`if` branches in `reference_items_for_signal`,
`signal_applies_to_task`, and `task_focus_tags`. The engine evaluates every
rule against the task context and unions the selected `ref_id`s.

```yaml
rules:
  - when: { language: java, focus: [security] }
    select: [tech.code.java.security]
  - when: { frameworks: [spring_boot], focus: [security] }
    select: [bk.spring.security]
  # enterprise extends instead of replacing:
  - when: { language: java, focus: [security] }
    select: [acme.sec-checklist]
```

### matchers.yaml — stack recognizer (Step 3)

Replaces the `contains_any` keyword chain in `signal_from_selection`. An ordered
list of `any`/`unless` keyword rules that label a raw stack selection with
`{ language, frameworks, roles }`.

```yaml
matchers:
  - any: [spring, jpa, hibernate, mybatis plus]
    unless: [kotlin, ktor, android]
    set: { language: java, frameworks: [spring_framework], roles: [backend] }
  - any: [acme-rpc, acme-framework]
    set: { language: java, frameworks: [acme_core], roles: [backend] }
```

### references/** — content (Step 4 overlay)

Layered resolution, reusing Loom's existing project/user scoping:

1. `<project>/.loom/playbook/references/<path>`  — project-level, committable
2. `~/.loom/playbooks/<active>/references/<path>` — user-level default
3. `<install>/references/<path>`                  — builtin (this repo's content)

### manifest.toml — pack metadata

```toml
name = "acme"
loom_version_range = ">=0.2.7"
mode = "extend"   # "base" replaces builtin entirely; "extend" merges on top
```

`extend` does a key-level merge of `registry`/`rules`/`matchers` (override same
key, append to lists). `base` replaces them wholesale — useful when an enterprise
wants only its own subset of the ~200 builtin references.

## Engine Responsibility

Rust keeps three jobs and nothing more:

1. **Load** the active Playbook (builtin embedded via `include_str!`; enterprise
   overlays loaded from disk in Step 4).
2. **Evaluate** `rules.yaml` against the task context → a set of `ref_id`s.
3. **Resolve** each `ref_id` via `registry.yaml` → a `path`, then read bytes.

The delivery state machine, MCP protocol, `.loom/` state format, and the
`referenceLoadPlan` request field are untouched. `ref_id` and `path` remain the
stable contract; only *how they are produced* changes from code to data.

## How a Business Orchestrates Its Own Content

| Need | How |
| --- | --- |
| Different standards per enterprise | One Playbook per enterprise; `mode: base` or `extend` |
| Add a best-practice reference | Add `ref_id` to `registry.yaml`, a `when→select` rule, and the markdown under `references/` |
| Too many builtin references | `mode: base` and declare only the refs you use — unlisted refs are not installed, selected, or read |
| Decouple from source | Code only reads `registry`/`rules`/`matchers`; editing a Playbook never touches Rust |
| Share across teams | A Playbook directory is self-describing and versionable; commit it into the repo for team consistency |

## Migration

**Core principle: the builtin Playbook is a byte-for-byte mirror of the current
Rust behavior.** Each step is behavior-equivalent and verified by the existing
selection / non-selection test suite before moving on.

### Step 1 — Extract registry (this branch)

- Define `PlaybookRegistry` (`contracts/playbook.rs`) with a serde model and a
  `code_ref(group_key, group) -> Option<RefEntry>` lookup.
- Generate `playbooks/default/registry.yaml` enumerating every
  `(group_key, group) -> {ref_id, path, reason}` derived from the current
  `code_quality_enum_refs` taxonomy and `reference_load_plan_item` mapping.
- Wire `code_reference_load_plan` and `reference_load_plan_item` to consult the
  registry first, falling back to the prior logic as a safety net.
- Verify: `cargo test -p contracts` stays green (equivalence).

### Step 2 — Extract rules

- Translate `reference_items_for_signal` / `signal_applies_to_task` /
  `task_focus_tags` into `playbooks/default/rules.yaml`.
- Add `contracts/engine.rs`: `evaluate_selection(task_context, &rules,
  &registry) -> Vec<ref_id>`.
- Replace `code_reference_selection_for_task_with_context` with engine
  evaluation.
- Verify: existing selection/non-selection assertions stay green.

### Step 3 — Extract matchers

- Translate `signal_from_selection` keyword rules into
  `playbooks/default/matchers.yaml`.
- Verify: `code_stack_signals_from_baseline` tests stay green.

### Step 4 — Open pack overlay and orchestration

- `manifest.toml` `mode: base | extend`; key-level merge for `extend`.
- `.loom/playbook.toml` (project) declares `active_playbook`; `setup --pack
  <path>` installs an enterprise Playbook.
- Tooling: `loom-setup playbook init/lint/dry-run`.
- Verify: new overlay tests (`pack_override.rs`, `pack_base.rs`) + equivalence
  regression (builtin-only == current behavior).

## Risks and Controls

- **Rule expressiveness**: nested conditions (`a || (b && c)`) are expressed with
  `and`/`or`/`any` combinators; translation is checked rule-by-rule against
  tests.
- **Matcher ordering**: the current `if/else if` chain is order-sensitive;
  `matchers.yaml` preserves order and the engine matches in sequence.
- **Performance**: the Playbook is loaded once and cached; rule evaluation is a
  linear scan over ~hundreds of rules — negligible.
- **Type safety**: serde + startup validation (unknown `ref_id` references,
  missing paths fail fast) + the existing two-way test suite. Slightly weaker
  than hard-coded, but buys business self-orchestration.
- **Blast radius**: changes are confined to `contracts` (engine + data-driven
  load helpers) and `setup` (file-list source). `core`/`state`/`workflow`/
  `mcp-server` are untouched.

## Naming Note

"Playbook" is chosen over "Reference Pack" because the unit contains more than
references — it holds the taxonomy, selection rules, and stack matchers that let
a business *orchestrate* its practices. It also avoids collision with Loom's
existing `package` (release artifacts) and `reference` (load-plan / deploy
profile) vocabulary. The CLI reads naturally: `loom playbook init`, `.loom/
playbook.toml`, `~/.loom/playbooks/<name>/`.
