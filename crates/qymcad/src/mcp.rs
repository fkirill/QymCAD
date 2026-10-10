//! THE AGENT'S WAY IN: the queue its calls wait in, and the pump that takes them into the frames of the
//! program.
//!
//! All touching of the program happens on the frame thread, inside whole frames: whoever asks puts work into
//! the queue and wakes the live window, and the answer comes back from the frame thread. The pump is the
//! first act of a LIVE frame (`eframe::App::ui`), never of a frame the agent itself runs - a frame of the
//! agent's going back to the queue would be the agent asking itself.
use std::collections::VecDeque;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use crate::gui::session::AgentSession;
use crate::gui::App;

/// WHAT THE AGENT DOES WITH THE PROGRAM: work over the driving core, whatever program the core holds.
type Work = dyn for<'a> FnOnce(&mut AgentSession<'a>) + Send;

/// ONE CALL OF THE AGENT: the work it asks of the program, and where its answer lands once the work has run.
struct Call {
    /// The work, run on the frame thread over the driving core.
    work: Box<Work>,
    /// Told once the work has run: the answer door of whoever asked.
    answer: Sender<()>,
}

/// THE CALLS WAITING for a frame of the program to take them, oldest first.
static WAITING: Mutex<VecDeque<Call>> = Mutex::new(VecDeque::new());

/// THE LIVE CONTEXT, as the last frame left it: what a call wakes, so the frame that takes it comes at once
/// rather than when the person next moves the mouse.
static WAKE: Mutex<Option<egui::Context>> = Mutex::new(None);

/// TAKE ONE CALL OF THE AGENT into this frame of the program: the work runs over the driving core - the
/// program driven by whole frames, on a window of the agent's own - and the answer is told once it has run.
/// One call to a frame; what is left waits for the next one, asked for at once.
pub(crate) fn pump(app: &mut App, ctx: &egui::Context) {
    if let Ok(mut wake) = WAKE.lock() {
        *wake = Some(ctx.clone());
    }
    let call = WAITING.lock().ok().and_then(|mut waiting| waiting.pop_front());
    let Some(call) = call else { return };
    // the agent's window opens at the size of the person's, or at the program's opening size before the first
    // frame has said one
    let size = ctx.input(|i| i.raw.screen_rect.unwrap_or(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0)))).size();
    let mut core = AgentSession::over(app, size);
    (call.work)(&mut core);
    let _ = call.answer.send(());
    if WAITING.lock().is_ok_and(|waiting| !waiting.is_empty()) {
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE PUMP TAKES A CALL IN ONE FRAME: what the queue holds is run by one call of the pump - the first
    /// act of a live frame - and its answer comes back with nothing left waiting.
    #[test]
    fn the_pump_takes_a_call_in_one_frame() {
        let mut app = App::default();
        let (answer, told) = std::sync::mpsc::channel();
        WAITING.lock().expect("the queue of the agent's calls is not poisoned").push_back(Call { work: Box::new(|_| ()), answer });
        pump(&mut app, &egui::Context::default());
        assert!(told.try_recv().is_ok(), "one call of the pump did not run the waiting call");
        assert!(WAITING.lock().expect("the queue of the agent's calls is not poisoned").is_empty(), "the call was left waiting");
    }
}
