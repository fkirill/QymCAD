## Why

An AI agent today can only look at QymCAD from the outside: it reads files, guesses at
coordinates, and never touches the program the way a person does. The program already has a
complete person-level door - `Session`, "what a session gives is what a person has" - proven by
the acceptance checks. Serving that door over the Model Context Protocol lets an agent work
on a document in the running program: draw a sketch, extrude it, mate an assembly, measure,
export - and be checked by the same invariants that check a person.

## What Changes

- A built-in MCP server runs inside the running application, on its own thread, speaking the
  MCP streamable HTTP transport on `127.0.0.1` with a pairing token. It is switched on in the
  settings; while it works, an indicator in the window says so.
- One agent per hosted document: the agent shares the live `App` - the document, the undo
  history, the tool in hand - with the person at the window. All tool calls are serialized
  through one queue; the person watches every change appear in their window.
- The agent drives the program through whole frames on its own headless egui context - the
  same frame driver (`gui::window::Window`) the checks use - never by poking fields or calling
  handlers out of frame. It defers to the person: while a button is held, a field takes
  typing, or a file chooser is open, the agent waits.
- An exhaustive toolset (~150-200 operations) covering sketch drawing and constraints, part
  features and primitives, datums, assembly joints and mates, parameters, view, measurement,
  import/export, document queries, and a UI-driver escape hatch of the agent's whole
  person-vocabulary. The registry is derived from the existing sources - `command_catalog`,
  the acceptance `contract::Tool` data, and the help articles - never written beside them.
- A small curated set of native MCP tools for discovery: `search-tool` (fuzzy search over the
  registry), `call-tool` (dispatch by name and parameters), `read-tool-help`,
  `get-tool-schema`, `get-document`, `screenshot`, `get-status`, `undo`, `redo`. Everything
  else is called through `call-tool`, so a client's context carries a dozen schemas instead of
  two hundred.
- Every tool call ends with the program brought to rest and a subset of the acceptance
  invariants run; problems found are returned in the tool result, so an agent learns
  immediately what a person would have seen go wrong.
- Agent file operations (open, save, export) take a path as a parameter and call the io-job
  functions directly - the same door a chooser's answer runs through. The agent never puts a
  system chooser up; a gesture that would raise one is reported back as "the program is asking
  the person".
- `Session` is refactored into an owning shell over a borrowable driving core (pure movement;
  the acceptance door keeps its public shape), re-exported publicly as `AgentSession` for the
  MCP crate. The command catalog data becomes reachable outside the `qymcad` crate for
  registry derivation.

## Capabilities

### New Capabilities

- `mcp-server`: hosting and lifecycle of the built-in server - settings door, transport and
  pairing token, one agent per hosted document, whole-frame driving of the live document,
  deference to the person, the visible indicator, agent file operations without system
  choosers.
- `mcp-toolset`: the tool registry and its discovery - derivation from the command catalog,
  contracts and help with a guard holding the link; the `search-tool`/`call-tool` pattern and
  the curated native set; the executor layers (semantic operations, queries, UI-driver escape
  hatch); the post-call rest and invariants.

### Modified Capabilities

None - the repository has no existing specs.

## Impact

- New crate `crates/qymcad-mcp` (depends on `qymcad`, as `qymcad-acceptance` does): the server,
  the registry, the executors, the checks. New small module `crates/qymcad/src/mcp.rs`: the
  request queue (a process static, the `WATCH` precedent) and the pump. One line of call is
  added in `impl eframe::App for App::ui` - not an `impl App` block, so the `god_object_ratchet`
  mark does not move; no `App` fields or methods are added.
- `gui/session.rs` is refactored (owning shell + borrowable core) - the riskiest patch; it
  must be pure movement with the acceptance suite green, in its own commit.
- `command_catalog` data is exposed for registry derivation; `qymcad-ui-state::Settings`
  gains the server fields; `i18n/` gains the catalogue keys and `docs/help/` the articles;
  a registry ratchet guard joins the existing guards.
- Dependencies: `rmcp` (the official Rust MCP SDK) with the streamable HTTP server feature,
  bringing tokio and axum/hyper into the tree - a deliberate dependency decision; its MSRV is
  verified against the declared floor before adoption. The tokio runtime runs on the server
  thread only; the main thread stays winit's.
- The interface is not restructured: no panel moves, no `App` logic moves. The person's window
  gains the settings section, the token display, and the "agent at work" indicator.
