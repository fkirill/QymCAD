## Purpose

Defines the MCP toolset an agent works with: a small, fixed set of discovery tools over a
large registry of operations, derived from the program's own command catalogue and checked by
the program's own invariants after every call.

## ADDED Requirements

### Requirement: A small discovery set is native, everything else is called by name

The server SHALL expose, as MCP tools of its own: `search-tool`, `call-tool`,
`read-tool-help`, `get-tool-schema`, `get-document`, `screenshot`, `get-status`, `undo` and
`redo`. Every other operation of the toolset SHALL be called through `call-tool` with the
operation's name and its parameters, so a client need not hold a schema for each of them.

#### Scenario: A client discovers and calls without a hundred schemas

- **WHEN** a client connects and lists the tools
- **THEN** the discovery set is what it sees, and no per-operation tool is listed
- **WHEN** the client calls `call-tool` with a name that is in the registry and fitting parameters
- **THEN** the operation runs and its result is answered

### Requirement: search-tool finds an operation by any of its names

`search-tool` SHALL take a query and answer with the registry entries that match it - by
operation code, by name in each of the program's languages, by the title of its help article
and by its synonyms - each with its name, a one-line synopsis, a summary of its parameters and
the workbench it lives in. An optional workbench filter narrows the answer. The ordering
SHALL rank the current workbench's operations first, as the command search does.

#### Scenario: A tool is found by either language

- **WHEN** the client searches for the name of a tool in one language the program speaks
- **THEN** the tool is found, whichever of the program's languages the query is in

#### Scenario: A miss names what was tried

- **WHEN** the client searches for words no operation matches
- **THEN** the answer is empty and says so in words, not in an error

### Requirement: The registry is derived, never written beside its sources

The registry SHALL be derived from the program's command catalogue - the same one source the
command search and the panel read - with descriptions from the help articles and parameter
shapes from the tools' contracts. A guard SHALL hold the link in both directions: every
command of the catalogue is findable in the registry, and every entry of the registry runs.

#### Scenario: The guard holds the link

- **WHEN** the guard runs over the catalogue and the registry
- **THEN** every catalogue command is discoverable by `search-tool`, and every registry entry answers to `call-tool`

### Requirement: The toolset covers the program whole

The registry SHALL cover, at least: every sketch drawing tool, constraint and dimension;
every part feature, primitive and datum; the assembly joints, mates and relations; parameters;
the view and its section; measurement; import and export of every format the program offers;
document queries; undo and redo. For whatever is not yet a named operation, a UI-driver layer
SHALL expose the agent's whole person-vocabulary - pressing a button by its hint, clicking a
point of the sketch or of space, filling a field, walking a menu, pressing a key - so the set
is exhaustive by construction.

#### Scenario: A catalogue command is callable end to end

- **WHEN** the client searches for a command of the catalogue and calls it through `call-tool` with its parameters
- **THEN** the operation happens in the document, as if the person had taken the command

#### Scenario: What has no name yet is still reachable

- **WHEN** an operation has no named entry in the registry
- **THEN** the agent can still do it through the UI-driver layer, by the same gestures a person would make

### Requirement: Every call is checked before it answers

Every tool call SHALL end with the program at rest and a set of the program's own invariants
run over it - the same kind the acceptance checks run after every step of a person. Problems
found SHALL be returned in the call's result. A call that left the program in a state a person
would be warned about SHALL NOT answer as if all was well.

#### Scenario: A problem is carried in the answer

- **WHEN** a tool call leaves the program in a state the invariants name a problem in
- **THEN** the result says so, in the words the checks would use, beside whatever the operation returned

#### Scenario: A clean call answers clean

- **WHEN** a tool call ends with no problem found
- **THEN** the result says the program stands right, and the operation's own outcome is the whole of it

### Requirement: screenshot returns a picture of the model

`screenshot` SHALL answer with a picture of the model as it is drawn - the same picture the
program's own picture-taking produces - as image content of the call, with the view as it
stands.

#### Scenario: A picture comes back as image content

- **WHEN** the client calls `screenshot`
- **THEN** the answer carries a picture of the current view that can be looked at as an image

### Requirement: Results are structured and errors say what was expected

Every tool result SHALL be structured: the status line, the problems found, and the outcome of
the operation itself. A call with an unknown name, a missing parameter, a wrong-typed
parameter, or an operation the program refuses SHALL be answered with an error naming what was
expected and what was wrong, and the program SHALL be left as it stood.

#### Scenario: An unknown name is said in words

- **WHEN** the client calls `call-tool` with a name that is not in the registry
- **THEN** the answer names the name it was given, says there is no such operation, and suggests searching

#### Scenario: A refusal changes nothing

- **WHEN** an operation is refused - a pick that is not there, a value the tool will not take
- **THEN** the answer says the words the program itself says, and the document is as it was before the call
