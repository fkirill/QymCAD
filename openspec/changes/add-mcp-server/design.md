## Context

The program has one object (`App`, private to the `qymcad` crate) driven by whole frames:
the live eframe window calls `App::draw_frame` on the main thread, and every check - the
in-crate `Hand` and the public `Session` of the acceptance crate - drives the same
`draw_frame` through `gui::window::Window::run`, a headless egui context fed synthetic
input. `draw_frame` runs entirely off the context it is given, which is why one `App` can
be driven by a context that is not the live window's. OCCT is serialized process-wide by
the kernel gate. `Session` is the only public door from outside the crate; it owns its own
`App` and installs a thread-local stand-in for the system's dialogs - both are wrong for an
agent that shares the person's live document. The `god_object_ratchet` counts `impl App`
blocks, methods, fields and their size; `impl eframe::App for App` is not among them.
`dead_code` is denied tree-wide: nothing may be written without a path from the interface.

## Goals / Non-Goals

**Goals:**

- The agent shares the live document and is visible to the person; one agent, calls
  serialized in order.
- The agent's every gesture runs through whole frames - the tested door - so the
  acceptance machinery keeps speaking about agent-driven runs as it does about
  person-driven ones.
- The registry is derived from existing sources with a guard, not written beside them.
- The person's window stays live while the agent works.

**Non-Goals:**

- No per-operation native MCP tools (no `tools/list` with two hundred schemas).
- No headless-only `--mcp` CLI mode, no separate agent process, no second document.
- No concurrent multi-agent editing; no per-agent view or camera split (one shared view).
- No changes to how the checks use `Session`: its public shape is kept as it stands.
- No writing of MCP protocol plumbing by hand where the official SDK serves.

## Decisions

### D1: An invisible co-pilot window - a second headless context over the live App

The agent gets its own `gui::window::Window` (own egui context, own pointer, keyboard and
clock, fonts installed) driven over `&mut App` of the live program. The MCP thread never
touches `App`; it only enqueues requests and wakes the live context
(`Context::request_repaint()` from another thread is the documented egui mechanism, already
used by the app's background jobs). All `App` access stays on the main thread, inside
frames; kernel work happens there and contends for nothing new.

```
      main thread                                    server thread
┌────────────────────────────────────────┐   ┌────────────────────────┐
│ eframe::App::ui()                      │   │ rmcp server            │
│   crate::mcp::pump(self, ctx)  (1 line)│   │ 127.0.0.1:PORT/mcp    │
│     take one request slice             │   │ streamable HTTP + token│
│     defer if person mid-gesture/chooser│   └───────────┬────────────┘
│     agent Window::run(&mut app, events)│              │ request/result
│       = whole frames over the live App │   queue: static in mcp module
│ live frame renders after the pump      │   (the WATCH precedent)     │
└────────────────────────────────────────┘
```

Alternatives rejected: injecting synthetic events into the live eframe loop (no stable
egui/eframe mechanism; the frame's parsing would be bypassed); running a `Session` beside
the live window (a second document, and its thread-local stand-in plus the process-wide
`CHECKED` guard would break the person's real choosers); a separate process (no shared
document, against the chosen hosting).

### D2: The pump - one line, a static queue, time slices

`crate::mcp::pump(&mut App, &egui::Context)` is called first in
`impl eframe::App for App::ui`. It is the single added call line (the ratchet's counted
blocks are untouched; no `App` fields or methods are added). The queue is a process static
in the `mcp` module, as `WATCH` is in `session.rs`.

A tool call is answered only from rest, but the pump runs a bounded slice of agent frames
per live frame (a frame budget of a few milliseconds) and re-requests repaint until the
call finishes - the same citizen pattern as the background rebuild. The person's window
never blocks on the agent; heavy settles read exactly like today's rebuilds.

The pump defers the agent while the person is mid-gesture: a pointer button held in the
live context, a text field holding the keyboard, or a chooser open
(`App::asking_for_a_file`). Deference is checked per live frame; the waiting call resumes
when the gesture ends.

### D3: `Session` becomes an owning shell over a borrowable driving core

The gesture, pacing and read-out machinery of `Session` (clicks a second apart, rest
before a hint, settle, bring-into-view, document/snapshot/words read-outs) moves into a
core that holds `&mut App` plus a `Window`. `Session` keeps its public API byte-identical
and delegates - the acceptance crate does not change. The MCP crate drives the same core
over the live `App`, without the stand-in, through a new public door `qymcad::AgentSession`.
`window.rs`'s own charter - one driver for everything that runs the program by its frames,
"so the two cannot come to disagree about what a frame is" - already names this third
consumer. This is pure movement with the suite green, in its own commit; see risks.

### D4: One crate for the server, a small module in the app

- `crates/qymcad-mcp` (new; depends on `qymcad`, as `qymcad-acceptance` does): the rmcp
  server, the curated native tools, the registry, the executors over `AgentSession`, the
  post-call checks. The protocol lives here so it is testable without a window.
- `crates/qymcad/src/mcp.rs` (small): the static queue, `pump`, the settings wiring, the
  indicator. This is all that must see `App` from inside.
- `qymcad` exposes: `AgentSession` (D3) and the command-catalogue data the registry is
  derived from.

The registry data itself is app data (it names the app's tools), so derivation happens
where the catalogue lives and the result is passed to `qymcad-mcp` as values; no cycle.

### D5: The registry is derived, with a ratchet

Each registry entry: code (the catalogue's `sketch.line` style code), names (from the
catalogue/help, each of the program's languages), description (from the help article),
parameters (from the acceptance `contract::Tool` fields and modes: caption key, class,
typical, lo/hi, zero/negative rules), category (workbench + layer), synonyms, executor (a
function over `&mut AgentSession`).

A guard holds both directions: every catalogue command is findable through `search-tool`'s
matching, and every registry entry executes through `call-tool`. The guard joins the
existing ratchet family (`comment_ratchet`, `god_object_ratchet`) - the same rule shape the
catalogue itself was built under ("a search with its own list would be behind the toolbar
on the very first new feature").

Where the acceptance contracts cannot be read at app build time (the acceptance crate
depends on the app, not the other way), the parameter shapes are read from the live tool
itself: taking a command puts its bar up, and the bar's fields are the parameters. The
contract data remains the check of the shapes, not their source at runtime.

### D6: rmcp, streamable HTTP, localhost, token

rmcp (the official Rust SDK, streamable-HTTP server feature, a Tower service) is the only
transport that fits a server inside an already-running app: stdio requires the client to
spawn the program. Bind `127.0.0.1` only; require the pairing token on every request; show
the token beside the switch in the settings. tokio runs on the server thread only - never
on the main thread, which is winit's. Alternative considered and rejected: hand-rolled
JSON-RPC over a local socket (zero new deps, but owns protocol-version negotiation, SSE
streams and session handling - a trap the SDK already solved).

### D7: Tool call lifecycle

`call-tool(name, params)` -> validate against the registry entry -> enqueue -> wake ->
pump slices agent frames (the entry's executor composes person-level gestures:
take-command, pick, fill, Enter) -> settle -> run the post-call checks (a chosen subset of
the acceptance oracles, over the same read-outs the checks use) -> answer with
`{status, problems, outcome}` and, where asked, a screenshot
(`qymcad_render::rasterize_3d` over the app's painting, PNG as image content).

Agent file operations carry a path and call the io-job functions directly - the same door
a chooser's answer continuation runs through (`poll_file_ask`'s `then`). If an agent
gesture would raise a chooser, the call is answered with "the program is asking the
person" (`asking_for_a_file`), and nothing changed.

### D8: Visible collaboration

While a call runs, the pump raises the same kind of overlay the rebuild card uses, in the
agent's words ("the agent is working"), and the status line names the agent's last
outcome. Undo entries the agent creates carry the agent's mark in their names, so the
person can tell whose step they are undoing. All strings through the `i18n/` catalogue,
one help article for the settings section and one for the agent, as every feature has.

## Risks / Trade-offs

- [The `Session` refactor touches the acceptance door] -> It is the first task and its own
  commit: pure movement, no behaviour change, the whole acceptance suite green before
  anything MCP lands on top. If movement is not pure, the plan stops there.
- [Two egui contexts over one `App` share transient state - tool in hand, selection,
  camera, open windows] -> This is the chosen collaboration model, documented in the help
  article; the deference policy (D2) keeps the worst interleavings out; the pump defers
  while the person holds a gesture. A per-agent view split is a later change if the shared
  camera proves painful, and is a non-goal here.
- [An agent frame runs `draw_frame` on the agent's context; App-owned window state is
  shared, so an agent gesture that opens a window opens it for the person too] -> Accepted
  and visible (visible collaboration); the semantic layer avoids window-opening paths, and
  the escape hatch documents it.
- [rmcp brings tokio + axum/hyper into a tree that has neither] -> A deliberate dependency
  decision recorded here; the server is a feature-gated dependency of `qymcad-mcp` only,
  nothing else in the tree sees it. MSRV is verified against the declared floor (1.88)
  before adoption; if it does not fit, the fallback is pinning an older rmcp line, not
  hand-rolling the protocol.
- [Time-sliced pump adds machinery] -> The first working version may run a call
  synchronously under a millisecond cap per pump call; the slice target is kept in the task
  list as its own step, so the mechanism lands tested rather than assumed.
- [Screenshot fidelity: CPU raster vs the GPU viewport] -> `rasterize_3d` is the same
  picture the checks compare against (`card_matches_raster`); where the GPU picture
  differs, the CPU one is the honest one for an agent - it is the one the checks trust.

## Migration Plan

Lands as one change on `main`; no document format changes, no settings migration beyond a
new section with a default of off. Rollback is reverting the merge: the server is off by
default, so an ordinary user's program is unchanged by the revert. The `Session` refactor
commit (D3) is valuable on its own and stays even if the rest were reverted.

## Open Questions

- The exact post-call check subset (which oracles run per call, which only on demand) -
  chosen during implementation; the spec fixes only that problems are checked and
  returned.
- The default port (or port-ephemeral with the actual port shown in the settings) -
  decided when the settings section is built.
- Whether `screenshot` also offers the 2D sheet picture beside the 3D one - a small
  addition once the first form works.
