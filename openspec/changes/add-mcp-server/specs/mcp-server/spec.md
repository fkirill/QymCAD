## Purpose

Hosts an MCP (Model Context Protocol) server inside the running QymCAD application so that an
external agent can work on the document the person has open, driving the program exactly as a
person drives it and deferring to the person whenever they are mid-gesture.

## ADDED Requirements

### Requirement: The server is switched on in the settings and reachable only on this machine

The application SHALL host an MCP server, switched off unless the person switches it on in the
settings, and while it is on it SHALL accept connections on `127.0.0.1` at a port the settings
name. It SHALL NOT bind any interface other than the loopback one. Switching the server on or
off takes effect without restarting the program.

#### Scenario: Off by default, on by the settings

- **WHEN** the program starts with the factory settings
- **THEN** nothing listens for MCP connections
- **WHEN** the person switches the server on in the settings
- **THEN** a client on the same machine can connect at `127.0.0.1` at the port the settings name
- **WHEN** the person switches it off again
- **THEN** the port closes and connections are refused, without a restart

#### Scenario: Never off the machine

- **WHEN** the server is on
- **THEN** it listens on the loopback interface only, and a connection from another machine cannot reach it

### Requirement: A connection is accepted only with the pairing token

While the server is on, the settings SHALL show a pairing token, and every request SHALL be
accepted only when it carries that token. A request without the token, or with a wrong one,
SHALL be refused with no other effect.

#### Scenario: A client without the token is refused

- **WHEN** a client connects and sends a request without the pairing token
- **THEN** the request is refused and nothing in the program changes

#### Scenario: The token is shown where the person can copy it

- **WHEN** the server is on and the person opens the settings
- **THEN** the pairing token is shown beside the server's switch, in a form the person can copy

### Requirement: One agent per hosted document

The server SHALL present one agent for the document the program hosts. Tool calls from every
connected client SHALL be served one at a time, in order, on that one document; the agent
shares the document, its undo history and the tool in hand with the person at the window. The
result of each call SHALL reflect the state after the calls before it.

#### Scenario: Calls are served in order on one document

- **WHEN** two tool calls arrive while the first is still running
- **THEN** the second waits, and runs only after the first has finished and answered

#### Scenario: What the agent does, the person can undo

- **WHEN** the agent makes a feature and the person presses Undo
- **THEN** the agent's feature is taken back, as a person's would be

### Requirement: The agent acts only through whole frames of the program

The agent SHALL drive the program the way the interface runs it - whole frames that decide who
receives a press, a key or an answer - and SHALL NOT set the program's fields or call its
handlers outside a frame. Everything the agent can do SHALL be something a person could do
through the same interface.

#### Scenario: A tool taken by the agent lands in hand as by a person

- **WHEN** the agent takes a command
- **THEN** the command's bar of options stands in the window as when a person takes it, and Enter applies it and Esc cancels it

#### Scenario: Nothing happens a person could not do

- **WHEN** the agent's calls are compared, one by one, with what the interface offers
- **THEN** every one of them is reachable by a person's own clicks and keys

### Requirement: The agent defers to the person

While the person is mid-gesture - a mouse button held, a drag under way, a text field taking
typing, or a file chooser open - the agent SHALL wait and run no frames of its own. When the
person's gesture ends, the waiting call continues.

#### Scenario: A call waits out a person's drag

- **WHEN** a tool call arrives while the person is dragging in the viewport
- **THEN** the call does not begin until the drag ends, and then runs whole

#### Scenario: The agent never answers a chooser

- **WHEN** an agent operation would put up the system's file chooser
- **THEN** no chooser is put up; the call is answered with the words that the program is asking the person, and nothing is changed by it

### Requirement: Agent file operations carry their path

The agent's file operations - opening, saving, exporting, importing - SHALL take a path as a
parameter and act through the same door a chooser's answer runs through, and SHALL NOT put up
a system chooser.

#### Scenario: An export writes the named file

- **WHEN** the agent exports the model to a path it named in the call
- **THEN** the file is written there, and no chooser appears on the person's screen

### Requirement: A call is answered from rest

A tool call SHALL be answered only once the program is at rest - no rebuild, load or write
under way - and the answer SHALL carry the status line as the program then reports it.

#### Scenario: The answer waits for a rebuild

- **WHEN** a call starts a rebuild and the client's answer would arrive while it runs
- **THEN** the answer is held until the rebuild has finished, and reports the model as it then stands

### Requirement: The window says when the agent is at work

While the server is on, the window SHALL show that it is on; while an agent call is running,
the window SHALL show that the agent is working, in the place the rebuild card is shown. The
person's window SHALL stay live - its frames keep being drawn - while the agent works.

#### Scenario: The indicator follows the agent

- **WHEN** the server is on and idle
- **THEN** the window shows the server is on, and not that the agent is working
- **WHEN** a tool call runs
- **THEN** the window shows the agent at work until the call is answered
