//! THE AGENT'S SERVER: the MCP protocol over the program's session door.
//!
//! rmcp serves the streamable HTTP transport on the loopback interface, one agent for the document the
//! program hosts; every tool call is handed to the frames of the program through the queue and answered
//! from rest. The server runs on a thread of its own; the tokio runtime never touches the main thread,
//! which stays the window's. The server follows the settings as the frames keep them: nothing is bound
//! while they say off, and what they say takes effect without a restart. A request is served only when
//! it carries the pairing token; one without it, or with a wrong one, is refused and nothing in the
//! program changes.
use std::sync::Mutex;
use std::time::Duration;

use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;

mod general;
mod registry;
mod tools;
mod ui_driver;

/// THE HEADER A CLIENT'S REQUEST MUST CARRY: the pairing token, as the settings show it beside the
/// switch. Lower-case, as HTTP headers travel.
pub const THE_TOKEN_HEADER: &str = "x-pairing-token";

/// THE WISH THE SERVER FOLLOWS: the program's own settings, or whatever a check stood in for them.
static THE_WISH: Mutex<fn() -> qymcad::Serving> = Mutex::new(qymcad::the_wish);

/// STAND THE SERVER UP, on a thread of its own, to follow the program's settings. Called once, from the
/// executable, before the window opens.
pub fn stand_up() {
    stand_up_on(qymcad::the_wish);
}

/// THE SAME, ON A WISH OF A CHECK'S OWN: the server follows whatever the `wish` answers, so a check can
/// drive it without a program running.
pub fn stand_up_on(wish: fn() -> qymcad::Serving) {
    if let Ok(mut the_wish) = THE_WISH.lock() {
        *the_wish = wish;
    }
    let _ = std::thread::Builder::new().name("the agent's server".into()).spawn(serve);
}

/// ONE SERVER, BOUND: the port it stands on, and what lets it go when the settings say off.
struct Bound {
    port: u16,
    let_go: CancellationToken,
}

/// THE SERVER'S OWN THREAD: a runtime of its own (the main thread stays the window's), watching the wish
/// and serving what it says - a bind at a time, the port taken as it stands, the token read as each
/// request arrives.
fn serve() {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("the agent's server builds its runtime");
    runtime.block_on(async {
        let mut bound: Option<Bound> = None;
        loop {
            let wish = the_wish();
            let wants = wish.on && !wish.token.is_empty();
            let port_moved = bound.as_ref().is_some_and(|b| b.port != wish.port);
            if bound.is_some() && (!wants || port_moved) {
                if let Some(b) = bound.take() {
                    b.let_go.cancel();
                }
            }
            if bound.is_none() && wants {
                bound = Some(Bound { port: wish.port, let_go: bind(wish.port) });
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    });
}

/// THE WISH, taken as it stands; a wish left unreadable by a poisoned lock serves nothing rather than
/// serving wrongly.
fn the_wish() -> qymcad::Serving {
    match THE_WISH.lock() {
        Ok(wish) => wish(),
        Err(poisoned) => poisoned.into_inner()(),
    }
}

/// BIND THE SERVER to the loopback interface at `port`, serving until the returned token is cancelled:
/// the streamable HTTP transport of MCP at `/mcp`, every request refused unless it carries the pairing
/// token.
fn bind(port: u16) -> CancellationToken {
    let config = StreamableHttpServerConfig::default().with_json_response(true).with_sse_keep_alive(None).with_cancellation_token(CancellationToken::new());
    let let_go = config.cancellation_token.clone();
    let going = let_go.clone();
    let service = StreamableHttpService::new(|| Ok(TheAgent), std::sync::Arc::new(LocalSessionManager::default()), config);
    let router = axum::Router::new().nest_service("/mcp", service).layer(middleware::from_fn(the_pairing_token));
    tokio::spawn(async move {
        let Ok(listener) = tokio::net::TcpListener::bind(("127.0.0.1", port)).await else {
            return; // the port is taken: nothing serves until the person names another
        };
        let serve = axum::serve(listener, router);
        let _ = serve.with_graceful_shutdown(async move { going.cancelled_owned().await }).await;
    });
    let_go
}

/// THE PAIRING TOKEN, as every request must carry it: the one the settings show, in the header
/// [`THE_TOKEN_HEADER`]. A request without it, or with a wrong one, is refused - and nothing in the
/// program changes, because nothing of the program is reached.
async fn the_pairing_token(request: axum::extract::Request, next: Next) -> Response {
    let want = the_wish().token;
    let wrong = request.headers().get(THE_TOKEN_HEADER).and_then(|t| t.to_str().ok()).is_none_or(|t| t != want);
    if wrong {
        return (axum::http::StatusCode::UNAUTHORIZED, "the pairing token is missing or wrong").into_response();
    }
    next.run(request).await
}

/// THE AGENT ITSELF, as the protocol sees it: one agent for the document the program hosts. The toolset it
/// serves - the discovery set, and the calling by name - is its own module.
struct TheAgent;

/// WHAT THE CHECKS OF THIS CRATE SHARE: the agent's queue and its server are one for the whole process,
/// so the checks take turns at them; and every round into the program is made while a check's own frames
/// keep the program turning underneath it.
#[cfg(test)]
pub(crate) mod checks {
    /// THE CHECKS TAKE TURNS at the agent's machinery: the queue and the server are one for the whole
    /// process, and checks beside each other would take each other's calls. Each begins with the
    /// agent's own hands put down, so one check's window does not leak into the next check's program.
    pub(crate) fn take_your_turn() -> std::sync::MutexGuard<'static, ()> {
        static TURNS: std::sync::Mutex<()> = std::sync::Mutex::new(());
        qymcad::the_agent_begins_again();
        TURNS.lock().expect("the turns of the checks are not held by a poisoned lock")
    }

    /// A CALL INTO THE PROGRAM while its frames run: the call is made on a thread of its own, and the
    /// frames keep coming until it answers - work that touches the program is answered only from its
    /// frames, so without them the call would never come back.
    pub(crate) fn ask_while_the_program_runs<T: Send + 'static>(program: &mut qymcad::LiveLoop, asking: impl FnOnce() -> T + Send + 'static) -> T {
        let (said, told) = std::sync::mpsc::channel();
        std::thread::spawn(move || drop(said.send(asking())));
        let began = std::time::Instant::now();
        loop {
            if let Ok(answer) = told.try_recv() {
                return answer;
            }
            assert!(began.elapsed() < std::time::Duration::from_secs(180), "the call into the program was never answered");
            program.frame();
        }
    }

    /// WORK OVER THE AGENT'S SESSION, answered from rest: the work is handed to the queue as a tool's
    /// work is, while the check's own frames keep the program turning underneath it.
    pub(crate) fn ask_the_core<T: Send + 'static>(program: &mut qymcad::LiveLoop, work: impl FnOnce(&mut qymcad::AgentSession) -> T + Send + 'static) -> Result<T, String> {
        ask_while_the_program_runs(program, || crate::tools::ask_the_program(work))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::{ask_while_the_program_runs, take_your_turn};
    use std::io::{Read, Write};
    use std::net::TcpStream;

    /// THE CHECK'S OWN WISH, standing in for the program's settings.
    static CHECK_WISH: Mutex<qymcad::Serving> = Mutex::new(qymcad::Serving { on: false, port: 51907, token: String::new() });

    fn the_check_s_wish() -> qymcad::Serving {
        match CHECK_WISH.lock() {
            Ok(wish) => qymcad::Serving { on: wish.on, port: wish.port, token: wish.token.clone() },
            Err(poisoned) => {
                let wish = poisoned.into_inner();
                qymcad::Serving { on: wish.on, port: wish.port, token: wish.token.clone() }
            }
        }
    }

    fn wish(on: bool, port: u16, token: &str) {
        match CHECK_WISH.lock() {
            Ok(mut wish) => *wish = qymcad::Serving { on, port, token: token.into() },
            Err(poisoned) => *poisoned.into_inner() = qymcad::Serving { on, port, token: token.into() },
        }
    }

    /// ONE REQUEST over the loopback, as a client would send it: answered with the status line, or
    /// `None` when nothing is bound to the port.
    fn ask_the_server(port: u16, token: &str) -> Option<String> {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
        stream.set_read_timeout(Some(Duration::from_secs(5))).expect("the timeout is set");
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"the check","version":"1"}}}"#;
        let ask = format!("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{THE_TOKEN_HEADER}: {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        stream.write_all(ask.as_bytes()).expect("the request is written");
        let mut said = String::new();
        stream.read_to_string(&mut said).ok()?;
        said.split("\r\n\r\n").next().map(|status| status.to_string())
    }

    /// WHAT THE PORT ANSWERS NOW, waiting out the server's watching beat: `true` when a request is
    /// answered, `false` when nothing is bound.
    fn the_port_answers(port: u16, token: &str, want: bool) -> bool {
        let began = std::time::Instant::now();
        loop {
            if ask_the_server(port, token).is_some() == want {
                return true;
            }
            if began.elapsed() > Duration::from_secs(10) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// A CLIENT CONNECTS WHEN THE SETTING IS ON and is refused when it is off, without a restart: the
    /// server follows the wish as it changes, binding and letting go.
    #[test]
    fn a_client_connects_when_the_setting_is_on_and_is_refused_when_it_is_off() {
        let _turn = take_your_turn();
        stand_up_on(the_check_s_wish);
        let port = 51907;
        wish(false, port, "the-token");
        assert!(the_port_answers(port, "the-token", false), "something was bound while the setting said off");
        wish(true, port, "the-token");
        assert!(the_port_answers(port, "the-token", true), "nothing answered while the setting said on");
        assert!(ask_the_server(port, "the-token").expect("the server answered").contains("HTTP/1.1 "), "the answer was not an HTTP response");
        wish(false, port, "the-token");
        assert!(the_port_answers(port, "the-token", false), "the port stayed open after the setting said off");
    }

    /// A REQUEST WITHOUT THE TOKEN, OR WITH A WRONG ONE, IS REFUSED - and nothing in the program
    /// changes, because nothing of the program is reached.
    #[test]
    fn a_request_without_the_token_is_refused() {
        let _turn = take_your_turn();
        stand_up_on(the_check_s_wish);
        let port = 51908;
        wish(true, port, "the-token");
        assert!(the_port_answers(port, "the-token", true), "nothing answered while the setting said on");
        for refused in ["", "wrong"] {
            let said = ask_the_server(port, refused).unwrap_or_default();
            assert!(said.contains("401"), "a request with the token {refused:?} was not refused: {said:?}");
        }
        wish(false, port, "the-token");
    }

    /// ONE ROUND OF A CLIENT over the loopback, beyond the hello the first check sends: the whole said -
    /// the head and the body - or `None` when nothing is bound to the port. Carries the pairing token
    /// and, once the server has given one, the session it gave.
    fn round(port: u16, session: Option<&str>, id: u8, method: &str, params: &str) -> Option<(String, String)> {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
        stream.set_read_timeout(Some(Duration::from_secs(150))).expect("the timeout is set");
        let body = format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#);
        let the_session = session.map(|s| format!("Mcp-Session-Id: {s}\r\n")).unwrap_or_default();
        let ask = format!("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{THE_TOKEN_HEADER}: the-token\r\n{the_session}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        stream.write_all(ask.as_bytes()).expect("the request is written");
        let mut said = String::new();
        stream.read_to_string(&mut said).ok()?;
        said.split_once("\r\n\r\n").map(|(head, body)| (head.to_string(), body.to_string()))
    }

    /// THE SESSION THE SERVER GAVE, read from the head of its answer to initialize: what every round
    /// after the first carries, as the transport says a client must.
    fn the_session_of(head: &str) -> String {
        head.lines()
            .find(|l| l.to_ascii_lowercase().starts_with("mcp-session-id:"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .expect("the server gave no session with its answer")
    }

    /// THE JSON OF A ROUND'S BODY: one JSON object, or the data an SSE answer carries - the transport may
    /// answer either.
    fn the_said_of(body: &str) -> serde_json::Value {
        let said = if body.starts_with('{') { body.to_string() } else { body.lines().filter(|l| l.starts_with("data: ")).map(|l| &l[6..]).collect::<String>() };
        serde_json::from_str(&said).unwrap_or_else(|e| panic!("the body is not one JSON answer: {e}: {body:?}"))
    }

    /// CALL A TOOL as a client does, with the session and the token, while the program's frames run under
    /// the round. The tool's result is answered; a protocol error fails the check where it stands.
    fn call_a_tool(program: &mut qymcad::LiveLoop, port: u16, session: &str, name: &str, arguments: &str) -> serde_json::Value {
        let session = session.to_string();
        let params = format!(r#"{{"name":"{name}","arguments":{arguments}}}"#);
        let (_, body) = ask_while_the_program_runs(program, move || round(port, Some(session.as_str()), 3, "tools/call", &params)).expect("the call was answered");
        let said = the_said_of(&body);
        assert!(said.get("error").is_none(), "the call of {name} answered a protocol error: {said}");
        said["result"].clone()
    }

    /// THE DISCOVERY SET IS WHAT A CLIENT SEES, and each tool answers over a real agent session: the
    /// check's own program - a `LiveLoop` standing in for the person's window - takes the agent's calls
    /// into its frames, and the tools that touch the program are answered from them (a call no frame took
    /// would never answer at all); the tools that read the program's own sources answer from them; a name
    /// no operation carries is refused in words.
    #[test]
    fn a_client_lists_the_discovery_set_and_each_answers_over_a_real_agent_session() {
        let _turn = take_your_turn();
        stand_up_on(the_check_s_wish);
        let port = 51909;
        wish(true, port, "the-token");
        assert!(the_port_answers(port, "the-token", true), "nothing answered while the setting said on");
        let mut program = qymcad::LiveLoop::start();
        // initialize, as a client must, and the session the server gives
        let hello = r#"{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"the check","version":"1"}}"#;
        let (head, body) = ask_while_the_program_runs(&mut program, move || round(port, None, 1, "initialize", hello)).expect("initialize was answered");
        assert!(head.contains("200"), "initialize was not answered with success: {head:?}");
        let session = the_session_of(&head);
        let said = the_said_of(&body);
        assert_eq!(said["result"]["serverInfo"]["name"], "QymCAD", "the server did not say who serves: {said}");
        // THE SET, EXACTLY: the nine tools of discovery, and nothing per-operation beside them
        let session_of_the_list = session.clone();
        let (_, body) = ask_while_the_program_runs(&mut program, move || round(port, Some(session_of_the_list.as_str()), 2, "tools/list", "{}")).expect("tools/list was answered");
        let mut seen: Vec<String> = the_said_of(&body)["result"]["tools"].as_array().expect("the tools are a list").iter().map(|t| t["name"].as_str().expect("a tool is named").to_string()).collect();
        seen.sort();
        assert_eq!(
            seen,
            ["call-tool", "get-document", "get-status", "get-tool-schema", "read-tool-help", "redo", "screenshot", "search-tool", "undo"],
            "the discovery set is not what the client sees: {seen:?}"
        );
        // THE PROGRAM'S OWN STATE, each answered from rest by the frames of the check's program
        let said = call_a_tool(&mut program, port, &session, "get-status", "{}");
        assert!(!said["isError"].as_bool().unwrap_or(true), "the status line answered as an error: {said}");
        let said = call_a_tool(&mut program, port, &session, "get-document", "{}");
        let document = said["content"][0]["text"].as_str().expect("the document is said in words");
        assert!(document.contains("\"parts\"") && document.contains("\"sketches\""), "the document does not carry the tree and the sketches: {document}");
        let said = call_a_tool(&mut program, port, &session, "screenshot", "{}");
        assert_eq!(said["content"][0]["type"], "image", "the picture is not image content: {said}");
        use base64::Engine as _;
        let the_picture = |said: &serde_json::Value| -> qymcad::Picture {
            let data = said["content"][0]["data"].as_str().expect("the picture is carried as data");
            let png = base64::engine::general_purpose::STANDARD.decode(data).expect("the picture's data is base64");
            qymcad::Picture::from_png(&png).expect("the picture's bytes open as a picture")
        };
        // THE PIXELS THE MODEL COVERS in a picture of the view: where the raster drew something.
        let covered = |picture: &qymcad::Picture| picture.rgba.chunks(4).filter(|px| px[3] > 128).count();
        let empty = the_picture(&said);
        assert!(empty.width > 100 && empty.height > 100, "the picture of the view is of no window: {} x {}", empty.width, empty.height);
        assert_eq!(covered(&empty), 0, "an empty program already covers pixels of its view");
        for name in ["undo", "redo"] {
            let said = call_a_tool(&mut program, port, &session, name, "{}");
            assert!(!said["isError"].as_bool().unwrap_or(true), "{name} answered as an error: {said}");
        }
        // THE SOURCES: search over the catalogue in either language, a miss said in words, the help
        // article of a command, and the refusals that name what was asked
        let line = qymcad::COMMANDS.iter().find(|c| c.code == "sketch.line").expect("the catalogue has the line");
        for language in qymcad_help::languages() {
            let word = line.name_in(&language);
            let said = call_a_tool(&mut program, port, &session, "search-tool", &serde_json::json!({ "query": word }).to_string());
            let found = said["content"][0]["text"].as_str().expect("the found operations are said in words");
            assert!(found.contains("sketch.line"), "the word {word:?} of {language} did not find sketch.line: {found}");
        }
        let said = call_a_tool(&mut program, port, &session, "search-tool", &serde_json::json!({ "query": "qqqqzzzz" }).to_string());
        let miss = said["content"][0]["text"].as_str().expect("a miss is said in words");
        assert!(miss.contains("no operation") && !said["isError"].as_bool().unwrap_or(true), "a miss must say so in words, not as an error: {said}");
        let said = call_a_tool(&mut program, port, &session, "read-tool-help", &serde_json::json!({ "name": "part.extrude" }).to_string());
        let article = the_said_of(said["content"][0]["text"].as_str().expect("the article is said as JSON"));
        assert_eq!(article["code"], "part.extrude", "the article is not of the extrude: {article}");
        assert!(!article["article"].as_str().unwrap_or_default().is_empty(), "the article carries no text: {article}");
        // THE REFUSALS, IN WORDS: a name no operation of the program carries, and one the program knows
        // and the agent serves no operation for
        for name in ["call-tool", "get-tool-schema"] {
            let said = call_a_tool(&mut program, port, &session, name, &serde_json::json!({ "name": "no.such-thing" }).to_string());
            let refused = said["content"][0]["text"].as_str().expect("the refusal is said in words");
            assert!(refused.contains("no.such-thing") && refused.contains("search-tool"), "{name} did not name what was asked and suggest searching: {refused}");
            assert!(said["isError"].as_bool().unwrap_or(false), "the refusal must be an error the client sees: {said}");
        }
        // A COMMAND OF THE CATALOGUE RUNS: the registry wires every command to a run - the take itself -
        // so call-tool lands the command's bar in the person's window, exactly as a person's take does,
        // and answers from rest with the status line and the outcome; a command that cannot work on what
        // stands (a line with no sketch to draw in) is refused in the program's own words
        let said = call_a_tool(&mut program, port, &session, "call-tool", &serde_json::json!({ "name": "part.box", "parameters": {} }).to_string());
        assert!(!said["isError"].as_bool().unwrap_or(true), "the take of a command of the catalogue answered as an error: {said}");
        let taken = said["content"][0]["text"].as_str().expect("the take is said in words");
        assert!(taken.contains("\"outcome\"") && taken.contains("\"status\""), "the answer does not carry the frame's parts: {taken}");
        // AND THE PICTURE IS OF WHAT THE WINDOW SHOWS: a box made by a call, the view fitted to it, and
        // the picture covers the pixels the box stands on - the same raster the checks trust
        let said =
            call_a_tool(&mut program, port, &session, "call-tool", &serde_json::json!({ "name": "part.box", "parameters": { "Length X": "40", "Width Y": "30", "Height Z": "10" } }).to_string());
        assert!(!said["isError"].as_bool().unwrap_or(true), "the box did not run: {said}");
        let said = call_a_tool(&mut program, port, &session, "call-tool", &serde_json::json!({ "name": "view.fit", "parameters": {} }).to_string());
        assert!(!said["isError"].as_bool().unwrap_or(true), "the fit did not run: {said}");
        let said = call_a_tool(&mut program, port, &session, "screenshot", "{}");
        let picture = the_picture(&said);
        assert!(
            (picture.width, picture.height) == (empty.width, empty.height),
            "the picture changed its size with the model: {} x {} against {} x {}",
            picture.width,
            picture.height,
            empty.width,
            empty.height
        );
        assert!(covered(&picture) > 1000, "a fitted block covers only {} pixels of its view", covered(&picture));
        wish(false, port, "the-token");
    }

    // ONE CALL, answered - and checked: a step that runs leaves no problem standing.
    fn step(program: &mut qymcad::LiveLoop, code: &str, given: serde_json::Value) -> String {
        let code = code.to_string();
        let named = code.clone();
        let answered = crate::checks::ask_the_core(program, move |core| crate::registry::call(core, &code, &given)).expect("the call answered");
        match answered {
            crate::registry::TheCall::Answered { outcome, problems, .. } => {
                assert!(problems.is_empty(), "{named} left problems standing: {problems:?}");
                outcome
            }
            refused => panic!("{named} was refused: {refused:?}"),
        }
    }

    // WHAT THE PROGRAM HOLDS, read between the steps.
    fn read<T: Send + 'static>(program: &mut qymcad::LiveLoop, read: impl FnOnce(&mut qymcad::AgentSession) -> T + Send + 'static) -> T {
        crate::checks::ask_the_core(program, read).expect("the program was read")
    }

    // THE WORD OF A KEY, as the window shows it.
    fn button(program: &mut qymcad::LiveLoop, key: &str) -> String {
        let key = key.to_string();
        read(program, move |core| core.word(&key))
    }

    // THE NAME OF A FIELD the call wants, read from the operation's own schema - the way a
    // client reads get-tool-schema before it calls.
    fn the_field(program: &mut qymcad::LiveLoop, code: &str, needle: &str) -> String {
        let code = code.to_string();
        let needle = needle.to_string();
        read(program, move |core| crate::registry::parameters(core, &code).into_iter().find(|f| f.name.to_lowercase().contains(&needle)).expect("the field is in the schema").name)
    }

    /// THE AGENT'S OWN SCENARIO, the `user_case` shape, made wholly through calls - the door
    /// `call-tool` runs - with the invariants read after every step: a sketch held at a parameter,
    /// a housing grown from it, holes bored, a shell hollowing it, a second part whose sketch sits
    /// on the face of the first, an assembly of the two, and a dimension edited in the middle of it
    /// all. Every step answers with no problem standing; what each leaves in the document is
    /// counted, not assumed.
    #[test]
    fn a_box_with_a_lid_is_built_whole_through_the_agent_s_calls() {
        let _turn = take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        // THE HOUSING: a part, a sketch on the world plane, a rectangle held at w = 40 by a dimension
        step(&mut program, "ui.key", serde_json::json!({ "key": "escape" }));
        let new_part = button(&mut program, "tb-new-part-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": new_part }));
        let pencil = button(&mut program, "g-sketch-pick-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": pencil }));
        step(&mut program, "ui.click", serde_json::json!({ "at": [0.0, 0.0, 0.0] }));
        step(&mut program, "parameters.set", serde_json::json!({ "name": "w", "expression": "40" }));
        step(&mut program, "sketch.rect", serde_json::json!({ "points": [[10.0, 10.0], [50.0, 40.0]] }));
        step(&mut program, "sketch.dim", serde_json::json!({ "points": [[30.0, 10.0], [30.0, 2.0]], "value": "w" }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+enter" }));
        let held = read(&mut program, |core| core.document().sketches[0].max[0] - core.document().sketches[0].min[0]);
        assert!((held - 40.0).abs() < 1e-6, "the dimension did not hold the rectangle at w = 40: it spans {held}");
        // THE HOUSING GROWN, HOLES BORED, THE SHELL HOLLOWING IT - each leaving its node green. The
        // volume is read off the working body - the last solid standing of its own, as the oracles
        // read it - for the document keeps the bodies a boolean worked on.
        step(&mut program, "part.extrude", serde_json::json!({ "Length": "10" }));
        let the_volume =
            |program: &mut qymcad::LiveLoop| read(program, |core| core.document().bodies.into_iter().filter(|b| !b.consumed && !b.sheet).rfind(|b| b.volume > 0.0).expect("a body stands").volume);
        let grown = the_volume(&mut program);
        assert!((grown - 12000.0).abs() < 1.0, "the housing holds {grown}, not the 12000 of 40 x 30 x 10");
        let (diameter, depth) = (the_field(&mut program, "part.hole", "diameter"), the_field(&mut program, "part.hole", "depth"));
        step(&mut program, "part.hole", serde_json::json!({ "picks": [[30.0, 25.0, 10.0]], diameter: "10", depth: "12" }));
        let holed = the_volume(&mut program);
        assert!(holed < grown - 700.0, "the hole bored nothing: the housing still holds {holed}");
        let thickness = the_field(&mut program, "part.shell", "thickness");
        step(&mut program, "part.shell", serde_json::json!({ "picks": [[20.0, 25.0, 10.0]], thickness: "2" }));
        let shelled = the_volume(&mut program);
        assert!(read(&mut program, |core| core.document().features.iter().filter(|f| f.error.is_some()).count()) == 0, "a red node stands in the housing after the shell");
        assert!(shelled < holed, "the shell hollowed nothing: the housing still holds {shelled}");
        // A SECOND PART, ITS SKETCH ON THE FACE OF THE FIRST: the housing's part is finished into
        // the assembly (the toolbar's own New-part button stands there, not in a part), and the
        // pencil then clicks the front of the housing while the second part is the one being drawn
        // in - the sheet belongs to the second part, the face to the first. The new-part word is
        // read again, for its plate counts the parts.
        step(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+enter" }));
        let new_part_again = button(&mut program, "tb-new-part-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": new_part_again }));
        // THE NEIGHBOUR SEEN FROM WITHIN THE SECOND PART: "in context" ticked on, the first part's
        // faces stand to be picked - a sketch of the second part on the first's face becomes a live
        // reference between the two
        let in_context = read(&mut program, |core| {
            let label = core.word("wb-in-context");
            core.widgets().into_iter().find(|w| w.kind == qymcad::Kind::CheckBox && w.label.ends_with(&label)).map(|w| w.checked == Some(false)).unwrap_or(true)
        });
        if in_context {
            let the_switch = button(&mut program, "wb-in-context");
            step(&mut program, "ui.button", serde_json::json!({ "hint": the_switch }));
        }
        step(&mut program, "ui.button", serde_json::json!({ "hint": pencil }));
        // THE FRONT OF THE HOUSING, off every base plane of the world - a face a pencil click can
        // tell from the grids, unlike a box seated on the origin
        step(&mut program, "ui.click", serde_json::json!({ "at": [30.0, 10.0, 5.0] }));
        let seat = read(&mut program, |core| core.document().sketches.last().cloned());
        let seat = seat.expect("the sketch on the face stands");
        assert!(seat.seat.starts_with("face of"), "the second part's first sketch sits on {} rather than the face of the first", seat.seat);
        assert_eq!(seat.part, read(&mut program, |core| core.document().parts.last().cloned().map(|p| p.name)), "the sketch on the face belongs to another part than the second one");
        step(&mut program, "sketch.line", serde_json::json!({ "points": [[5.0, 2.0], [5.0, 8.0]] }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+enter" }));
        // THE LID ITSELF: a sketch of the second part's own, on the world plane beside the housing
        step(&mut program, "ui.button", serde_json::json!({ "hint": pencil }));
        step(&mut program, "ui.click", serde_json::json!({ "at": [70.0, 15.0, 0.0] }));
        step(&mut program, "sketch.rect", serde_json::json!({ "points": [[60.0, 0.0], [100.0, 30.0]] }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+enter" }));
        step(&mut program, "part.extrude", serde_json::json!({ "Length": "10" }));
        let (parts, lid) = read(&mut program, |core| {
            let d = core.document();
            (d.parts.len(), d.bodies.into_iter().filter(|b| !b.consumed && !b.sheet).rfind(|b| b.volume > 0.0))
        });
        let lid = lid.expect("the lid's body stands");
        assert_eq!(parts, 2, "the document holds {parts} parts, not two");
        assert_eq!(lid.part, read(&mut program, |core| core.document().parts.last().cloned().map(|p| p.name)), "the last body standing belongs to the second part");
        // THE ASSEMBLY: the lid's part finished into it, and the housing and the lid held by a
        // rigid joint, the housing grounded by it
        step(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+enter" }));
        step(&mut program, "assembly.joint", serde_json::json!({ "kind": "Rigid", "picks": [[45.0, 10.0, 5.0], [80.0, 15.0, 10.0]] }));
        let joints = read(&mut program, |core| core.document().joints);
        assert_eq!(joints.len(), 1, "the joint did not come to stand");
        assert!(!joints[0].violated, "the joint does not hold: {:?}", joints[0]);
        // A DIMENSION EDITED IN THE MIDDLE: w carried to 30, the rectangle follows, the housing with
        // it, the sketch on the housing's face with the housing, the joint holds, nothing goes red
        step(&mut program, "parameters.set", serde_json::json!({ "name": "w", "expression": "30" }));
        let document = read(&mut program, |core| core.document());
        let span = document.sketches[0].max[0] - document.sketches[0].min[0];
        assert!((span - 30.0).abs() < 1e-6, "carrying w to 30 left the rectangle spanning {span}");
        let housing = read(&mut program, |core| {
            let d = core.document();
            let first = d.parts[0].name.clone();
            d.bodies.into_iter().filter(|b| !b.consumed && !b.sheet).rfind(|b| b.part.as_deref() == Some(first.as_str())).expect("the housing stands").volume
        });
        assert!(housing < shelled, "carrying w to 30 left the housing as large as it was: {housing} against {shelled}");
        assert!(
            document.features.iter().all(|f| f.error.is_none()),
            "a red node stands after the dimension was edited: {:?}",
            document.features.iter().filter(|f| f.error.is_some()).collect::<Vec<_>>()
        );
        assert_eq!(document.joints.len(), 1, "the joint did not survive the edit");
        assert!(!document.joints[0].violated, "the joint does not hold after the edit");
        assert_eq!(document.parts.len(), 2, "the edit lost a part");
    }

    /// A ROW OF THE TREE OPENS BY ITS NAME: a sketch's row is double-clicked to edit it again, and
    /// two calls of a pick are two clicks too far apart to be one - the double click runs inside the
    /// one call, as the person's double click does. Without it a layered part - a second extrusion
    /// over the same sketch - had no way back to the contour picking once the first was applied.
    #[test]
    fn a_row_of_the_tree_opens_by_its_name() {
        let _turn = take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        step(&mut program, "ui.key", serde_json::json!({ "key": "escape" }));
        let new_part = button(&mut program, "tb-new-part-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": new_part }));
        let pencil = button(&mut program, "g-sketch-pick-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": pencil }));
        step(&mut program, "ui.click", serde_json::json!({ "at": [0.0, 0.0, 0.0] }));
        step(&mut program, "sketch.circle", serde_json::json!({ "points": [[10.0, 10.0], [30.0, 10.0]] }));
        step(&mut program, "sketch.circle", serde_json::json!({ "points": [[40.0, 10.0], [60.0, 10.0]] }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+enter" }));
        // THE FIRST EXTRUSION over one contour
        let _ = crate::checks::ask_the_core(&mut program, move |core| crate::registry::call(core, "part.extrude", &serde_json::json!({}))).expect("the take answered");
        step(&mut program, "ui.click", serde_json::json!({ "at": [10.0, 10.0] }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        step(&mut program, "ui.field", serde_json::json!({ "label": "Length", "text": "5" }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        // THE SKETCH OPENS AGAIN by its name, and the SECOND extrusion follows over the other contour
        step(&mut program, "ui.open", serde_json::json!({ "name": "Sketch 1" }));
        let editing = read(&mut program, |core| core.document().editing.is_some());
        assert!(editing, "the open pick left the sketch closed");
        step(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+enter" }));
        let _ = crate::checks::ask_the_core(&mut program, move |core| crate::registry::call(core, "part.extrude", &serde_json::json!({}))).expect("the second take answered");
        // the second contour AND the lens the circles overlap in, or the pieces do not touch
        step(&mut program, "ui.click", serde_json::json!({ "at": [45.0, 10.0] }));
        step(&mut program, "ui.click", serde_json::json!({ "at": [25.0, 10.0] }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        step(&mut program, "ui.field", serde_json::json!({ "label": "Length", "text": "5" }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        let volume = read(&mut program, |core| core.document().bodies.into_iter().filter(|b| !b.consumed && !b.sheet).map(|b| b.volume).sum::<f64>());
        let want = (2.0 * std::f64::consts::PI * 400.0 - 181.3) * 5.0;
        assert!((volume - want).abs() < 80.0, "two extrusions of two circles hold {volume}, not the {want} of their union");
    }

    /// A PLACE FAR ON THE SHEET IS BROUGHT INTO THE CANVAS, not given up on: a sketch opened at the
    /// scale the sheet starts at, and a circle drawn far from where the view stands - the place is
    /// dragged nearer, turn by turn, until the click lands. Reported behaviour: the program came
    /// down whole ("could not be brought into the canvas") when the place stood farther than the
    /// wheel's reach and the wheel itself moved nothing.
    #[test]
    fn a_place_far_on_the_sheet_is_brought_into_the_canvas() {
        let _turn = take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        step(&mut program, "ui.key", serde_json::json!({ "key": "escape" }));
        let new_part = button(&mut program, "tb-new-part-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": new_part }));
        let pencil = button(&mut program, "g-sketch-pick-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": pencil }));
        step(&mut program, "ui.click", serde_json::json!({ "at": [0.0, 0.0, 0.0] }));
        // A CIRCLE FAR FROM WHERE THE VIEW STANDS: centre 300 mm out, the radius point 100 mm further
        step(&mut program, "sketch.circle", serde_json::json!({ "points": [[300.0, 300.0], [400.0, 300.0]] }));
        let document = read(&mut program, |core| core.document());
        assert_eq!(document.sketches[0].circles, 1, "the far circle did not come to stand");
    }

    /// A ROW OF THE TREE IS PICKED BY ITS NAME: a second take over the same sketch needs the sketch
    /// selected again, and the agent reaches the row as the person does - its own words clicked.
    /// Reported behaviour: no gesture could select a finished sketch, so a second extrusion over
    /// one sketch was impossible over the server.
    #[test]
    fn a_row_of_the_tree_is_picked_by_its_name() {
        let _turn = take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        step(&mut program, "ui.key", serde_json::json!({ "key": "escape" }));
        let new_part = button(&mut program, "tb-new-part-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": new_part }));
        let pencil = button(&mut program, "g-sketch-pick-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": pencil }));
        step(&mut program, "ui.click", serde_json::json!({ "at": [0.0, 0.0, 0.0] }));
        step(&mut program, "sketch.circle", serde_json::json!({ "points": [[10.0, 10.0], [20.0, 10.0]] }));
        step(&mut program, "sketch.circle", serde_json::json!({ "points": [[25.0, 10.0], [35.0, 10.0]] }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+enter" }));
        // THE FIRST EXTRUSION over one contour of the sketch, the whole ladder
        let _ = crate::checks::ask_the_core(&mut program, move |core| crate::registry::call(core, "part.extrude", &serde_json::json!({}))).expect("the take answered");
        step(&mut program, "ui.click", serde_json::json!({ "at": [10.0, 10.0] }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        step(&mut program, "ui.field", serde_json::json!({ "label": "Length", "text": "5" }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        // THE ROW OF THE SKETCH picked by its own words, and the SECOND extrusion over the other
        // contour of the same sketch follows
        step(&mut program, "ui.pick", serde_json::json!({ "name": "Sketch 1" }));
        let _ = crate::checks::ask_the_core(&mut program, move |core| crate::registry::call(core, "part.extrude", &serde_json::json!({}))).expect("the second take answered");
        // the second contour AND the lens the circles overlap in: apart they would leave the part
        // in separate pieces - a part is one body, the addition must touch it
        step(&mut program, "ui.click", serde_json::json!({ "at": [25.0, 10.0] }));
        step(&mut program, "ui.click", serde_json::json!({ "at": [17.5, 10.0] }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        step(&mut program, "ui.field", serde_json::json!({ "label": "Length", "text": "5" }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        let (extrusions, combines, volume) = read(&mut program, |core| {
            let d = core.document();
            (
                d.features.iter().filter(|f| f.kind == "Extrude").count(),
                d.features.iter().filter(|f| f.kind == "Combine" && f.error.is_none()).count(),
                d.bodies.into_iter().filter(|b| !b.consumed && !b.sheet).map(|b| b.volume).sum::<f64>(),
            )
        });
        assert_eq!(
            (extrusions, combines),
            (1, 1),
            "the second extrusion over the picked row did not join the first: {:?}",
            read(&mut program, |core| {
                let d = core.document();
                (d.features.iter().map(|f| (f.kind.clone(), f.name.clone(), f.error.clone())).collect::<Vec<_>>(), d.bodies.iter().map(|b| (b.volume, b.consumed)).collect::<Vec<_>>(), core.status())
            })
        );
        // two circles of r = 10, centres 15 apart, both 5 thick, their union one body
        let want = (2.0 * std::f64::consts::PI * 100.0 - 45.32) * 5.0;
        assert!((volume - want).abs() < 30.0, "the joined body holds {volume}, not the {want} of the two circles' union");
    }

    /// A CLICK OF THE FLAT SHEET NEEDS NO OPEN SKETCH: the extrude's bar waits for contours over a
    /// finished sketch - the canvas flat, nothing editing - and the agent's pair [x, y] must reach
    /// it as the person's click does. Reported behaviour: the pair was refused ("no sketch is
    /// open"), the triple too ("the canvas is not the 3D view"), and no gesture of the agent could
    /// pick a contour at all.
    #[test]
    fn a_click_of_the_flat_sheet_needs_no_open_sketch() {
        let _turn = take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        step(&mut program, "ui.key", serde_json::json!({ "key": "escape" }));
        let new_part = button(&mut program, "tb-new-part-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": new_part }));
        let pencil = button(&mut program, "g-sketch-pick-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": pencil }));
        step(&mut program, "ui.click", serde_json::json!({ "at": [0.0, 0.0, 0.0] }));
        step(&mut program, "sketch.rect", serde_json::json!({ "points": [[10.0, 10.0], [50.0, 40.0]] }));
        step(&mut program, "sketch.rect", serde_json::json!({ "points": [[100.0, 10.0], [160.0, 40.0]] }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+enter" }));
        // THE TAKE OPENS THE BAR AND IT WAITS: two closed contours stand, neither taken at once,
        // and the take answers the waiting words - the bar stays in hand for the calls that follow
        let taken = crate::checks::ask_the_core(&mut program, move |core| crate::registry::call(core, "part.extrude", &serde_json::json!({}))).expect("the take answered");
        match taken {
            crate::registry::TheCall::Refused(words) => assert!(words.contains("click a closed contour"), "the waiting bar said something else: {words}"),
            other => panic!("the take over two contours did not wait for one: {other:?}"),
        }
        // THE CLICK REACHES THE CONTOUR the person would point at; Enter sets the size - the bar's
        // own ladder, one rung a call - the length fills into the field that then stands at the
        // geometry, and the extrude makes the body of THAT contour alone
        step(&mut program, "ui.click", serde_json::json!({ "at": [30.0, 25.0] }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        step(&mut program, "ui.field", serde_json::json!({ "label": "Length", "text": "10" }));
        step(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        let volume = read(&mut program, |core| core.document().bodies.into_iter().filter(|b| !b.consumed && !b.sheet).rfind(|b| b.volume > 0.0).map(|b| b.volume));
        assert_eq!(volume, Some(12000.0), "the extrude over the clicked contour did not make the 40 x 30 x 10 body alone: {volume:?}");
    }

    /// A CLICK OF THE MODEL'S SPACE OFF THE 3D VIEW IS REFUSED, NOT FATAL: a sketch's sheet stands
    /// flat, a place of the space of the model has nothing to name there, and the call says so in
    /// words; the document stands and the next call is served. Reported behaviour: the click brought
    /// the program down, whole, from a server call.
    #[test]
    fn a_click_of_the_space_off_the_3d_view_is_refused_not_fatal() {
        let _turn = take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        step(&mut program, "ui.key", serde_json::json!({ "key": "escape" }));
        let new_part = button(&mut program, "tb-new-part-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": new_part }));
        let pencil = button(&mut program, "g-sketch-pick-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": pencil }));
        step(&mut program, "ui.click", serde_json::json!({ "at": [0.0, 0.0, 0.0] }));
        // THE SHEET STANDS FLAT: a triple names a place of the space of the model, and the 3D view is
        // not the canvas - the call is refused in words naming it, the sketch stands open
        let answered = crate::checks::ask_the_core(&mut program, move |core| crate::registry::call(core, "ui.click", &serde_json::json!({ "at": [5.0, 3.0, 7.0] }))).expect("the call answered");
        let crate::registry::TheCall::Refused(words) = answered else { panic!("the click off the 3D view was not refused: {answered:?}") };
        assert!(words.contains("the canvas is not the 3D view"), "the refusal does not name the canvas: {words}");
        let open = read(&mut program, |core| core.document().editing.is_some());
        assert!(open, "the refused click closed the sketch");
    }

    /// WHOSE STEP IS WHOSE in the undo list: every step the agent's calls lay carries the agent's
    /// name, and the person's own steps carry none. The person takes the agent's step back with
    /// their own Ctrl+Z - the marked name moves to redo - and makes a step of their own through
    /// their own window, the command search and a box; the list tells the three apart.
    #[test]
    fn the_agent_s_steps_carry_its_mark_and_the_person_s_carry_none() {
        let _turn = take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        // THE AGENT MAKES A PART AND A CYLINDER IN IT: whatever steps its calls lay, all carry the mark
        step(&mut program, "ui.key", serde_json::json!({ "key": "escape" }));
        let new_part = button(&mut program, "tb-new-part-hint");
        step(&mut program, "ui.button", serde_json::json!({ "hint": new_part }));
        step(&mut program, "part.cylinder", serde_json::json!({ "Radius": "10", "Height": "10" }));
        let mark = format!(" ({})", qymcad_i18n::tr("mcp-the-agent"));
        let undo = read(&mut program, |core| core.document().undo);
        assert!(!undo.is_empty(), "the agent's work laid no undo step at all");
        assert!(undo.iter().all(|s| s.ends_with(&mark)), "a step of the agent's carries no mark: {undo:?}");
        // THE PERSON TAKES THE AGENT'S STEP BACK with their own Ctrl+Z: the marked name moves to redo
        let had = undo.len();
        let was_last = undo.last().cloned().expect("the step stood");
        program.the_person_s_key(qymcad::Modifiers::COMMAND, qymcad::Key::Z);
        let (undo, redo) = read(&mut program, |core| {
            let d = core.document();
            (d.undo, d.redo)
        });
        assert_eq!(undo.len() + 1, had, "the person's undo took no step back: {undo:?}");
        assert_eq!(redo.last(), Some(&was_last), "the step the person took back is not the agent's marked one: {redo:?}");
        // THE PERSON MAKES A STEP OF THEIR OWN through their own window: the command search by their
        // own keys, a box, applied with its own Enter - a step with no mark beside the agent's
        let box_name = read(&mut program, |_core| qymcad::COMMANDS.iter().find(|c| c.code == "part.box").expect("the box is in the catalogue").name());
        program.the_person_s_key(qymcad::Modifiers::COMMAND, qymcad::Key::K);
        program.the_person_types(&box_name);
        program.the_person_s_key(qymcad::Modifiers::default(), qymcad::Key::Enter);
        program.the_person_s_key(qymcad::Modifiers::default(), qymcad::Key::Enter);
        let undo = read(&mut program, |core| core.document().undo);
        let last = undo.last().cloned().unwrap_or_else(|| panic!("the person's box laid no step: {undo:?}"));
        assert!(!last.ends_with(&mark), "the person's own step carries the agent's mark: {undo:?}");
        assert!(undo.iter().rev().skip(1).all(|s| s.ends_with(&mark)), "the agent's steps lost their mark under the person's: {undo:?}");
    }
}
