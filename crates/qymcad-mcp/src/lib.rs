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
use rmcp::model::ServerConfig;
use rmcp::ServerHandler;
use tokio_util::sync::CancellationToken;

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

/// THE AGENT ITSELF, as the protocol sees it: one agent for the document the program hosts. The
/// toolset is served by its own module; what the handler says here is who the agent is.
struct TheAgent;

impl ServerHandler for TheAgent {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(rmcp::model::ServerCapabilities::builder().build())
    }
}

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
}
