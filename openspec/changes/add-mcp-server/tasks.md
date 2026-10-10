## 1. The borrowable driving core (the riskiest patch, first and alone)

- [x] 1.1 Move `Session`'s gesture, pacing and read-out machinery into a core holding `&mut App` + `Window`, keeping `Session`'s public API byte-identical as a delegating owner; run the whole acceptance suite and `cargo test --workspace` green on the movement alone, in its own commit
- [x] 1.2 Expose the core publicly as `qymcad::AgentSession` (no stand-in, no own `App`), with its own doc-comment saying what it is for; check: a test in `qymcad` drives a `App::default()` through `AgentSession` the way `Hand` does and gets the same fillet the hand's own check gets
- [x] 1.3 Expose the command-catalogue data (codes, names, launches, workbenches) for derivation outside the crate; check: a test reads every catalogue entry through the new door

## 2. The queue and the pump (app side)

- [x] 2.1 Create `crates/qymcad/src/mcp.rs`: the process-static request queue (the `WATCH` precedent), `pub(crate) fn pump(&mut App, &egui::Context)`, and the single call line in `impl eframe::App for App::ui` before `draw_frame`; check: `god_object_ratchet` and `comment_ratchet` green, and a test pushes a no-op request and sees the pump take it in one frame
- [x] 2.2 Deference in the pump: a pointer button held in the live context, a text field holding the keyboard, or a chooser open holds the queue; check: a test with the hand mid-drag sees a queued request wait and resume after the release
- [x] 2.3 Time slices: the pump runs agent frames under a millisecond budget per live frame and re-requests repaint until the call answers; check: a test runs a long settle through the pump while live frames keep being counted, and the live frame count never stalls
- [x] 2.4 The "agent at work" indicator: the overlay the rebuild card uses, in the agent's words, raised while a call runs; check: a scenario test reads the indicator's words come and go with a call

## 3. The agent window

- [x] 3.1 The agent's own `Window` (own context, fonts installed) driven over the live `App` inside the pump, through the same `Window::run` the checks use; check: a test alternates live frames and agent frames on one `App` with `check_all`-style invariants after every step, and both windows stay coherent
- [x] 3.2 Choosers never: a pump slice that finds `asking_for_a_file` after an agent gesture answers the call with "the program is asking the person" and changes nothing; check: a test drives a gesture that would open a chooser and reads the answer
- [x] 3.3 Agent file operations take a path and call the io-job functions directly (the door a chooser's answer runs through); check: open, save and export each round-trip a real file with no chooser, in a test

## 4. The server crate and the discovery set

- [x] 4.1 Create `crates/qymcad-mcp` (depends on `qymcad`, as `qymcad-acceptance` does); verify rmcp's MSRV against the declared floor and record the dependency decision in the manifest comment; server off unless the setting is on
- [ ] 4.2 The rmcp streamable-HTTP server on a server thread (tokio never on the main thread), bound to `127.0.0.1`, port from the settings; check: a loopback client connects when the setting is on and is refused when it is off, without a restart
- [ ] 4.3 The pairing token: generated per the settings, required on every request, shown beside the switch for copying; check: a request without or with a wrong token is refused and nothing in the program changes
- [ ] 4.4 The settings section and its i18n keys (all strings through the catalogue), the default off; check: `i18n/tests` and the ratchets green, and the section follows the settings-panel patterns
- [ ] 4.5 The native discovery set: `search-tool`, `call-tool`, `read-tool-help`, `get-tool-schema`, `get-document`, `screenshot`, `get-status`, `undo`, `redo`; check: a client lists exactly these, and each answers over a real agent session

## 5. The registry

- [ ] 5.1 The registry entry as data (code, names in each language, description from the help article, parameters, category, synonyms, executor) and its derivation from the command catalogue; check: a test derives the registry and finds every catalogue command in it
- [ ] 5.2 Parameter shapes from the live tool: taking a command and reading its bar's fields yields the parameter list; check: for every feature command, the bar's fields match the registry entry's parameters
- [ ] 5.3 `search-tool` matching over codes, names in each language, help titles and synonyms, workbench filter, current-workbench-first ordering (the command search's rules); check: a tool is found in each of the program's languages, and a miss answers in words
- [ ] 5.4 The ratchet guard: every catalogue command discoverable, every registry entry executable; check: the guard runs in the crate's tests and fails when an entry is unwired (tried red first)

## 6. The executors

- [ ] 6.1 The executor frame: validate against the entry, run through `AgentSession`, settle, run the post-call checks, answer `{status, problems, outcome}`; unknown name, missing or wrong-typed parameter, and refused operation each answer in words and leave the document as it stood (checks for each refusal)
- [ ] 6.2 The post-call checks: a chosen subset of the acceptance oracles over the same read-outs, problems carried in the result; check: a call that leaves "a tool waiting with no bar saying so" answers with that problem
- [ ] 6.3 Sketch operations: every drawing tool, the twelve constraints, the three dimensions, trim/extend/break/project, move/copy/rotate/mirror/offset, corner and chamfer; each with its own contract check through `AgentSession`
- [ ] 6.4 Part operations: every feature and primitive of the catalogue, datums, sketch-on-face; each with its own check
- [ ] 6.5 Assembly operations: joints of every kind, mates, relations, groups, ground, drag; each with its own check
- [ ] 6.6 Parameters, view (orbit, fit, section), measurement, import/export of every format the program offers; each with its own check
- [ ] 6.7 The UI-driver escape hatch: the person vocabulary over `AgentSession` (press a button by hint, click a point of the sketch or of space, fill a field, walk a menu, press a key, type); check: a scenario does something with no named entry using only the escape hatch
- [ ] 6.8 `screenshot`: `qymcad_render::rasterize_3d` of the current view as PNG image content; check: the answer's bytes open as a picture of what the window shows

## 7. End to end

- [ ] 7.1 The agent scenario, the `user_case` shape: a sketch on a face of another part -> holes -> shell -> a second part -> assembly -> a dimension edited in the middle, made wholly through MCP calls, with the invariants checked after every step
- [ ] 7.2 The help: one article for the settings section and one for the agent, in both languages, linked from the help map; check: `help_map_flow` green
- [ ] 7.3 The undo story: agent-created undo entries carry the agent's mark in their names; check: a scenario reads the undo list and tells the agent's steps from the person's
- [ ] 7.4 The gate: `cargo fmt --all`, `cargo clippy --workspace --all-targets`, `cargo test --workspace` and `cargo check --workspace --all-targets`, exit codes read; the dependency ratchet shows the new gap and the decision is recorded
