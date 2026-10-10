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

mod tools;

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

#[cfg(test)]
mod tests {
    use super::*;
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

    /// THE CHECKS TAKE TURNS at the server: the wish is one for the whole process.
    fn take_your_turn() -> std::sync::MutexGuard<'static, ()> {
        static TURNS: Mutex<()> = Mutex::new(());
        TURNS.lock().expect("the turns of the checks are not held by a poisoned lock")
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

    /// A ROUND MADE WHILE THE PROGRAM'S FRAMES RUN: the round is made on a thread of its own, and the
    /// frames keep coming until it answers - a tool that touches the program is answered only from its
    /// frames, so without them the round would never come back.
    fn ask_while_the_program_runs<T: Send + 'static>(program: &mut qymcad::LiveLoop, asking: impl FnOnce() -> T + Send + 'static) -> T {
        let (said, told) = std::sync::mpsc::channel();
        std::thread::spawn(move || drop(said.send(asking())));
        let began = std::time::Instant::now();
        loop {
            if let Ok(answer) = told.try_recv() {
                return answer;
            }
            assert!(began.elapsed() < Duration::from_secs(180), "the round was never answered");
            program.frame();
        }
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
        let png = base64::engine::general_purpose::STANDARD.decode(said["content"][0]["data"].as_str().expect("the picture is carried as data")).expect("the picture's data is base64");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "the picture's bytes are not a PNG file");
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
        let said = call_a_tool(&mut program, port, &session, "call-tool", &serde_json::json!({ "name": "sketch.line", "parameters": {} }).to_string());
        let refused = said["content"][0]["text"].as_str().expect("the refusal is said in words");
        assert!(refused.contains("sketch.line") && refused.contains("command of the program"), "a known command was not said to be one: {refused}");
        wish(false, port, "the-token");
    }
}
