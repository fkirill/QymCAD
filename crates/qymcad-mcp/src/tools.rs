//! THE DISCOVERY SET: the nine tools a client of the agent sees - everything else the program does is
//! called by name through `call-tool`, so a client's context carries these schemas and no hundred beside
//! them.
//!
//! The tools that touch the program answer only from rest: their work is handed to the queue and taken
//! into the frames of the program - over the driving core, a real agent session - and what they answer
//! comes back from the frame thread, once the program has come to rest. The tools that read the program's
//! own sources - the command catalogue, the help articles - answer from them: they change nothing. The
//! search is of both kinds: it reads the sources for what there is to find, and asks the program one
//! thing - where it stands - because its own rule ranks the standing workbench first.
use std::time::Duration;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, JsonObject, ServerCapabilities, ServerConfig};
use rmcp::tool;
use rmcp::tool_handler;
use rmcp::tool_router;
use rmcp::ServerHandler;

use crate::TheAgent;

/// HOW LONG A TOOL WAITS for the program to take its call. A settle of the program is bounded by the
/// session's own budget; the wait stands above it, so a program whose frames keep running is never cut
/// short. A program that never takes the call - its window closed, its frames stopped - is said so in
/// words rather than held forever.
const THE_WAIT: Duration = Duration::from_secs(150);

/// ONE CALL INTO THE PROGRAM, answered from rest: the work runs on the frame thread over the driving
/// core - a real agent session - and what it answers comes back from there. A gesture of the work that
/// raised a chooser is answered with the words that the program is asking the person; nothing is
/// changed by such a call.
pub(crate) fn ask_the_program<T: Send + 'static>(work: impl FnOnce(&mut qymcad::AgentSession) -> T + Send + 'static) -> Result<T, String> {
    let (said, tell) = std::sync::mpsc::channel();
    let (answer, told) = std::sync::mpsc::channel();
    let mut work = Some(work);
    qymcad::hand_over(
        Box::new(move |core| {
            if let Some(work) = work.take() {
                let _ = said.send(work(core));
            }
            true
        }),
        answer,
    );
    match told.recv_timeout(THE_WAIT) {
        Ok(qymcad::Ended::Done) => tell.recv_timeout(THE_WAIT).map_err(|_| "the program took the call and never said its answer".into()),
        Ok(qymcad::Ended::Asking) => Err("the program is asking the person: a chooser stands open for them, and the agent never answers one".into()),
        Err(_) => Err("the program never took the call: its frames are not running".into()),
    }
}

/// THE SAME, AWAITED AS A TOOL AWAITS: the wait runs on the runtime's blocking pool, so the server thread
/// keeps serving while the frames of the program run the call.
async fn the_program_answers<T: Send + 'static>(work: impl FnOnce(&mut qymcad::AgentSession) -> T + Send + 'static) -> Result<T, String> {
    match tokio::task::spawn_blocking(move || ask_the_program(work)).await {
        Ok(said) => said,
        Err(_) => Err("the call into the program was abandoned".into()),
    }
}

/// A NAME NO OPERATION OF THE PROGRAM CARRIES, said in words: the name it was given, that there is no such
/// operation, and where to look. The document is left as it stood.
fn no_such_operation(name: &str) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(format!("{name}: no operation of the program is named so; search-tool finds what the program offers"))])
}

/// A COMMAND OF THE PROGRAM THE AGENT CANNOT RUN: the registry of callable operations is derived from the
/// program's own sources, and this command stands outside it. The name is known to the program - said so,
/// with where its help is - and nothing runs; the document is left as it stood.
fn not_served(command: &qymcad::Command) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(format!(
        "{} ({}) is a command of the program, and the agent serves no operation for it; read-tool-help tells what it does, search-tool finds what the program offers",
        command.name(),
        command.code
    ))])
}

/// WHAT search-tool TAKES.
#[derive(serde::Deserialize, schemars::JsonSchema, Default)]
struct SearchToolParams {
    /// The words to find the operation by: a part of its code, or of its name in any language the program
    /// speaks.
    query: String,
    /// Only operations of this workbench - `sketch`, `part`, `assembly`; every workbench when left out.
    workbench: Option<String>,
}

/// WHAT read-tool-help TAKES.
#[derive(serde::Deserialize, schemars::JsonSchema, Default)]
struct ReadToolHelpParams {
    /// The code of the operation, as search-tool answers it (`sketch.line`).
    name: String,
}

/// WHAT call-tool TAKES.
#[derive(serde::Deserialize, schemars::JsonSchema, Default)]
struct CallToolParams {
    /// The code of the operation to run, as search-tool answers it.
    name: String,
    /// The operation's parameters, as get-tool-schema describes them. The schema of the call names the
    /// field; the run that reads what they carry arrives with the registry of operations.
    parameters: Option<JsonObject>,
}

/// WHAT get-tool-schema TAKES.
#[derive(serde::Deserialize, schemars::JsonSchema, Default)]
struct GetToolSchemaParams {
    /// The code of the operation, as search-tool answers it.
    name: String,
}

/// ONE STEP BACK OR AGAIN, the keys a person presses for it, said with the status line as the program
/// stands after the step.
fn back_or_again(core: &mut qymcad::AgentSession, key: qymcad::Key) -> String {
    core.chord(qymcad::Modifiers::COMMAND, key);
    core.status()
}

#[tool_router]
impl TheAgent {
    /// THE STATUS LINE, as the program then reports it: answered once the program is at rest, so a
    /// rebuild the call began is waited out before a word is said.
    #[tool(name = "get-status", description = "The status line of the program as it reports it now, answered once the program is at rest")]
    async fn get_status(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(match the_program_answers(|core| core.status()).await {
            Ok(status) => CallToolResult::success(vec![ContentBlock::text(status)]),
            Err(said) => CallToolResult::error(vec![ContentBlock::text(said)]),
        })
    }

    /// THE DOCUMENT AS THE PROGRAM HOLDS IT: the tree, the sketches, the timeline, the bodies, the
    /// parameters, the joints, the undo steps - what a person could find out by looking.
    #[tool(name = "get-document", description = "The document as the program holds it: the parts, the sketches, the timeline, the bodies, the parameters, the joints, the undo steps")]
    async fn get_document(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(match the_program_answers(|core| core.document()).await {
            Ok(document) => match serde_json::to_value(&document) {
                Ok(said) => CallToolResult::success(vec![ContentBlock::json(said).expect("the document is plain data and serializes")]),
                Err(said) => CallToolResult::error(vec![ContentBlock::text(format!("the document did not serialize: {said}"))]),
            },
            Err(said) => CallToolResult::error(vec![ContentBlock::text(said)]),
        })
    }

    /// A PICTURE OF THE MODEL, as the window draws it now - the same picture the program's own
    /// picture-taking produces - PNG image content of the call.
    #[tool(name = "screenshot", description = "A picture of the model as the window draws it now, the same picture the program's own picture-taking produces, as PNG image content")]
    async fn screenshot(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        use base64::Engine as _;
        Ok(match the_program_answers(|core| core.snapshot().png()).await {
            Ok(png) => CallToolResult::success(vec![ContentBlock::image(base64::engine::general_purpose::STANDARD.encode(png), "image/png")]),
            Err(said) => CallToolResult::error(vec![ContentBlock::text(said)]),
        })
    }

    /// ONE STEP BACK, as the person's Undo does - the same keys, through the same interface. The answer
    /// is the status line as the program then reports it.
    #[tool(name = "undo", description = "Take one step back, as the person's Undo does; the answer is the status line as the program then reports it")]
    async fn undo(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(match the_program_answers(|core| back_or_again(core, qymcad::Key::Z)).await {
            Ok(status) => CallToolResult::success(vec![ContentBlock::text(status)]),
            Err(said) => CallToolResult::error(vec![ContentBlock::text(said)]),
        })
    }

    /// ONE STEP AGAIN, as the person's Redo does.
    #[tool(name = "redo", description = "Do one step again, as the person's Redo does; the answer is the status line as the program then reports it")]
    async fn redo(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(match the_program_answers(|core| back_or_again(core, qymcad::Key::Y)).await {
            Ok(status) => CallToolResult::success(vec![ContentBlock::text(status)]),
            Err(said) => CallToolResult::error(vec![ContentBlock::text(said)]),
        })
    }

    /// FIND AN OPERATION by any of its names: its code, its name in any language the program speaks, the
    /// title of its help article, or a word it is also known by. The workbench the program stands in ranks
    /// first, as the person's own search ranks it - so the program is asked where it stands, and the
    /// answer comes from rest. The answer carries each operation's code, its names, a one-line synopsis
    /// and the workbench it lives in; a miss is empty and says so in words, not in an error.
    #[tool(
        name = "search-tool",
        description = "Find operations of the program by a word of their code, of their name in any language the program speaks, of the title of their help article, or of a word they are also known by; the answer carries each operation's code, its names, a one-line synopsis and the workbench it lives in"
    )]
    async fn search_tool(&self, Parameters(SearchToolParams { query, workbench }): Parameters<SearchToolParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let now_in = match the_program_answers(|core| core.workbench()).await {
            Ok(now_in) => now_in,
            Err(said) => return Ok(CallToolResult::error(vec![ContentBlock::text(said)])),
        };
        let found: Vec<serde_json::Value> = crate::registry::the_hits(&query, workbench.as_deref(), &now_in)
            .into_iter()
            .map(|e| serde_json::json!({ "code": e.command.code, "names": e.names(), "synopsis": e.synopsis(), "workbench": e.command.workbench }))
            .collect();
        if found.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                "no operation of the program matches {query}; search-tool finds by code, by name in any language the program speaks, by the title of the help article, or by a word the operation is also known by"
            ))]));
        }
        Ok(CallToolResult::success(vec![ContentBlock::json(found).expect("the found operations are plain data and serialize")]))
    }

    /// THE HELP ARTICLE OF AN OPERATION, by its code: what to press, what happens, and what to do if it
    /// did not work - the same article F1 opens, with its title and its place on the web.
    #[tool(name = "read-tool-help", description = "The help article of an operation, by its code: what to press, what happens, and what to do if it did not work")]
    fn read_tool_help(&self, Parameters(ReadToolHelpParams { name }): Parameters<ReadToolHelpParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let Some(command) = qymcad::COMMANDS.iter().find(|c| c.code == name) else {
            return Ok(no_such_operation(&name));
        };
        let Some(article) = command.help_article() else {
            return Ok(CallToolResult::success(vec![ContentBlock::text(format!("{} ({}) has no help article of its own", command.name(), command.code))]));
        };
        Ok(CallToolResult::success(vec![ContentBlock::json(serde_json::json!({
            "code": command.code,
            "title": qymcad_help::title(article),
            "url": qymcad_help::web_url(article),
            "article": qymcad_help::article(article).unwrap_or_default(),
        }))
        .expect("the article's head is plain data and serializes")]))
    }

    /// RUN AN OPERATION OF THE PROGRAM by its code and parameters, as a person runs it through the same
    /// interface. The registry of runnable operations is derived from the program's own sources; a name
    /// outside it is refused in words, and a name the registry holds no run for is said so. The document
    /// is left as it stood by every refusal.
    #[tool(name = "call-tool", description = "Run an operation of the program by its code and parameters, as a person runs it through the same interface")]
    async fn call_tool(&self, Parameters(CallToolParams { name, parameters }): Parameters<CallToolParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let Some(entry) = crate::registry::by_code(&name) else {
            return Ok(no_such_operation(&name));
        };
        let Some(run) = entry.run else {
            return Ok(not_served(entry.command));
        };
        let code = entry.command.code;
        let parameters = serde_json::Value::Object(parameters.unwrap_or_default());
        Ok(match the_program_answers(move |core| run(core, code, &parameters)).await {
            Ok(said) => CallToolResult::success(vec![ContentBlock::text(said)]),
            Err(said) => CallToolResult::error(vec![ContentBlock::text(said)]),
        })
    }

    /// THE PARAMETERS AN OPERATION TAKES, by its code: each field's name, its kind, and what it says. The
    /// shapes are read from the tools themselves - the command is taken as a person takes it, the fields
    /// its bar puts up are answered, and the tool is put down again, the document as it stood. Where the
    /// command does not open, nothing to work on, no parameter is answered.
    #[tool(name = "get-tool-schema", description = "The parameters an operation takes, by its code: each field's name, its kind, and what it says")]
    async fn get_tool_schema(&self, Parameters(GetToolSchemaParams { name }): Parameters<GetToolSchemaParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let Some(entry) = crate::registry::by_code(&name) else {
            return Ok(no_such_operation(&name));
        };
        let code = entry.command.code;
        Ok(match the_program_answers(move |core| crate::registry::parameters(core, code)).await {
            Ok(fields) => {
                CallToolResult::success(vec![ContentBlock::json(serde_json::to_value(&fields).expect("the fields are plain data and serialize")).expect("the fields are plain data and serialize")])
            }
            Err(said) => CallToolResult::error(vec![ContentBlock::text(said)]),
        })
    }
}

#[tool_handler]
impl ServerHandler for TheAgent {
    /// WHO SERVES: the program itself, one agent for the document it hosts. What a client does with it:
    /// find an operation by any of its names, then call it by name.
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("QymCAD", env!("CARGO_PKG_VERSION")))
            .with_instructions("search-tool finds an operation of the program by any of its names; every operation is called through call-tool, by its code and its parameters.")
    }
}
