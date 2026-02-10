# Interface Expansion Implementation Plan

This document provides a detailed, implementation-ready plan for adding **Web**, **Telegram**, and **Discord** interfaces to OxideAgent.

It is designed to be executed as the immediate next phase after interface scaffolding (enum/config/CLI support) and follows existing project patterns:
- event-driven architecture
- orchestrator as core coordinator
- interface trait abstractions
- session persistence and tool approval security model

---

## 1) Goals and non-goals

### Goals
- Implement production-ready Web, Telegram, and Discord interfaces that use the existing orchestrator and event model.
- Preserve current TUI behavior and avoid regressions.
- Keep business logic centralized in `core/` and interface-specific behavior in `interfaces/`.
- Reuse existing session/tool approval/multi-agent logic wherever possible.

### Non-goals
- Rewriting orchestrator internals.
- Replacing current tool execution security model.
- Introducing provider-specific core logic into interface modules.

---

## 2) Cross-interface foundation work (must happen first)

These steps unblock all three interfaces and should be implemented once.

### 2.1 Interface capability model
Add an internal capability contract that defines per-interface support for:
- streaming output
- interactive tool approvals
- agent switching UX
- session switching UX
- rich message rendering

**Why:** keeps feature parity explicit and prevents ad-hoc branching.

### 2.2 Shared interface adapter utilities
Create reusable helpers under `src/interfaces/` for:
- event translation (`AppEvent` <-> transport payload)
- normalized error rendering
- approval flow state machine wrappers
- message chunk buffering for transports that are not naturally streaming-friendly

### 2.3 Approval flow abstraction
Standardize a transport-neutral approval interaction API:
- request approval
- approve once
- approve always (global)
- approve always (session)
- deny

Then map that API to each transport (HTTP/WS actions, Telegram callbacks, Discord interactions).

### 2.4 Interface-specific configuration sections
Extend `OxideConfig` with optional nested sections:
- `[web]`
- `[telegram]`
- `[discord]`

Each includes tokens/ports/webhook details/rate limit knobs.

### 2.5 Unified end-to-end test harness
Build interface conformance tests that assert the same behavior across interfaces:
- user input -> orchestrator -> response
- tool request lifecycle
- session persistence
- agent switching behavior

---

## 3) Web interface implementation plan

## 3.1 User stories
- As a local user, I can open a browser and chat with OxideAgent.
- I can view streaming responses in real time.
- I can approve/deny tool calls from the web UI.
- I can switch agents and sessions from the web interface.

### 3.2 Architecture
- Backend in Rust (same binary) exposes:
  - HTTP endpoints for command actions and snapshots
  - WebSocket/SSE channel for streaming events
- Web client consumes event stream and posts commands.
- The backend delegates all intelligence/tool workflow to orchestrator.

### 3.3 Proposed modules
- `src/interfaces/web/mod.rs` (implements `Interface` trait)
- `src/interfaces/web/server.rs` (HTTP/WS server lifecycle)
- `src/interfaces/web/protocol.rs` (DTOs for payloads)
- `src/interfaces/web/state.rs` (connection/session state)

### 3.4 Phased delivery

#### Phase W1: API + event stream skeleton
- Start server
- health endpoint
- connect event stream
- send basic user input to orchestrator
- render agent messages

#### Phase W2: streaming + history restoration
- stream chunk handling and completion
- initial history snapshot on connect
- reconnect behavior

#### Phase W3: tool approvals + sessions + agents
- approval prompts and decision actions
- session switch/list operations
- active agent display and switching controls

#### Phase W4: hardening
- auth for non-local mode
- CORS controls
- rate limiting and payload limits
- structured observability

### 3.5 Web testing strategy
- unit tests for protocol serialization/deserialization
- integration tests for HTTP/WS routes
- browser-level smoke test for send/receive/approve
- regression tests for stream ordering

### 3.6 Web docs to add
- runbook (`--interface web` usage)
- config examples (`[web]` section)
- troubleshooting (port conflicts, CORS, reverse proxy)

---

## 4) Telegram interface implementation plan

### 4.1 User stories
- As a Telegram user, I can converse with the agent in chat.
- I receive incremental or batched responses depending on Telegram limits.
- I can approve/deny tool requests using inline buttons.
- My chat context maps to persistent OxideAgent sessions.

### 4.2 Architecture
- Telegram bot transport layer (polling first, webhook optional later).
- Incoming updates mapped to `AppEvent` inputs.
- Outgoing orchestrator events mapped to Telegram messages/edits/callback replies.

### 4.3 Proposed modules
- `src/interfaces/telegram/mod.rs`
- `src/interfaces/telegram/bot.rs` (transport + lifecycle)
- `src/interfaces/telegram/mapping.rs` (event mapping)
- `src/interfaces/telegram/approval_ui.rs` (inline keyboard workflows)

### 4.4 Session and identity mapping
- `telegram_user_id` + optional `chat_id` -> session key convention
- support group and private chat modes
- configurable isolation mode:
  - per-user
  - per-chat
  - per-thread/topic (if applicable)

### 4.5 Phased delivery

#### Phase T1: command + plain chat support
- `/start`, `/help`
- message to orchestrator and full-response echo

#### Phase T2: streaming adaptation
- chunk coalescing into periodic message edits
- fallback to final-only mode for long replies

#### Phase T3: approvals + sessions + agents
- inline keyboard for tool approval actions
- slash commands for session list/switch and agent switch

#### Phase T4: robustness
- retry handling for Telegram API errors
- flood limit/backoff handling
- idempotency guard for duplicate updates

### 4.6 Telegram testing strategy
- mapper unit tests (update -> events, events -> messages)
- approval callback state transition tests
- integration tests with mocked Telegram API
- long-message chunking tests

### 4.7 Telegram docs to add
- token setup and bot permissions
- polling vs webhook operation
- command reference and approval UX behavior

---

## 5) Discord interface implementation plan

### 5.1 User stories
- As a Discord user, I can interact via slash commands and channel messages.
- I can approve tools through Discord interactions.
- I can run separate sessions by guild/channel/thread context.

### 5.2 Architecture
- Discord gateway/events consumer + REST interaction responses.
- adapter maps Discord events to orchestrator inputs and orchestrator events back to Discord outputs.
- interaction component handlers for tool approvals and controls.

### 5.3 Proposed modules
- `src/interfaces/discord/mod.rs`
- `src/interfaces/discord/bot.rs`
- `src/interfaces/discord/mapping.rs`
- `src/interfaces/discord/components.rs`

### 5.4 Session mapping model
- default: guild_id + channel_id + user_id tuple
- optional thread-scoped sessions for busy channels
- configurable policy to avoid cross-user context leakage

### 5.5 Phased delivery

#### Phase D1: slash command MVP
- `/chat` command
- basic message response flow

#### Phase D2: streaming and message updates
- deferred responses and followups
- progressive edit strategy for streamed tokens

#### Phase D3: approvals + session/agent controls
- button/select components for approval options
- commands for session switch/list and agent switch

#### Phase D4: production readiness
- permission scope validation
- shard-ready architecture notes
- moderation and safety guardrails

### 5.6 Discord testing strategy
- event mapping unit tests
- component interaction tests
- mocked integration tests for command lifecycle
- permission failure path tests

### 5.7 Discord docs to add
- bot app setup and intents
- command registration flow
- permission and moderation recommendations

---

## 6) Security and compliance checklist (all interfaces)

- Never bypass existing tool approval policy.
- Preserve per-session and global approval semantics exactly.
- Validate and sanitize all user-provided payloads.
- Enforce message length and rate limits.
- Redact secrets in logs.
- Add explicit threat notes for each transport:
  - Web: CSRF/CORS/session auth
  - Telegram: webhook authenticity, token leakage
  - Discord: interaction signature validation, permission boundaries

---

## 7) Observability and operations

- Add interface-specific tracing spans (`interface=web|telegram|discord`).
- Include correlation IDs across inbound update -> orchestrator action -> outbound response.
- Emit counters for:
  - messages handled
  - tool approvals requested/approved/denied
  - transport failures/retries

---

## 8) CI and quality gates

Before enabling any interface by default:
- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -D warnings`
- `cargo test`
- interface-specific integration suite must pass
- docs updated with usage/config examples

For each interface milestone, require:
- at least one integration test
- approval workflow test coverage
- session isolation regression test

---

## 9) Delivery sequence recommendation

1. **Foundation** (Section 2)
2. **Web** (fastest local validation loop)
3. **Telegram** (simpler interaction surface)
4. **Discord** (interaction-rich, highest complexity)

This sequence maximizes code reuse and reduces integration risk.

---

## 10) Definition of done per interface

An interface is considered complete when all are true:
- fully selectable via `--interface` and config
- supports user input, responses, streaming behavior
- supports tool approval end-to-end
- supports session and agent operations
- has integration tests and docs
- passes CI gates

---

## 11) Immediate next tasks (implementation-ready)

1. Add config structs: `WebInterfaceConfig`, `TelegramInterfaceConfig`, `DiscordInterfaceConfig`.
2. Add interface capability abstraction and shared adapters in `src/interfaces/`.
3. Implement `interfaces/web` Phase W1-W2.
4. Add web integration tests and docs.
5. Implement Telegram Phase T1.
6. Implement Discord Phase D1.

These tasks should be opened as tracked issues/epics and executed incrementally.
