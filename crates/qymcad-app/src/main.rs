//! QymCAD - the executable: the program itself is the `qymcad` library, and the agent's server is stood
//! up beside it before the window opens.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    qymcad_mcp::stand_up();
    qymcad::run()
}
