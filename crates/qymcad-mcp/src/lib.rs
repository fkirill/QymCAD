//! THE AGENT'S SERVER: the MCP protocol over the program's session door.
//!
//! rmcp serves the streamable HTTP transport on the loopback interface, one agent for the document the
//! program hosts; every tool call is handed to the frames of the program through the queue and answered
//! from rest. The server runs on a thread of its own; the tokio runtime never touches the main thread,
//! which stays the window's.
