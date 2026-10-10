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

/// DOES THE PERSON HOLD THE PROGRAM: a button of theirs down, a field of theirs holding the keyboard, a
/// chooser of theirs open. The program is theirs first - the agent's call waits in the queue until the
/// gesture is over, and the frame that ends the gesture is the one that takes it.
fn the_person_holds_it(app: &App, ctx: &egui::Context) -> bool {
    ctx.input(|i| i.pointer.any_down()) || ctx.egui_wants_keyboard_input() || app.asking_for_a_file()
}

/// TAKE ONE CALL OF THE AGENT into this frame of the program - unless the person is mid-gesture, when the
/// call waits in the queue for the gesture to end. The work runs over the driving core - the program driven
/// by whole frames, on a window of the agent's own - and the answer is told once it has run. One call to a
/// frame; what is left waits for the next one, asked for at once.
pub(crate) fn pump(app: &mut App, ctx: &egui::Context) {
    if let Ok(mut wake) = WAKE.lock() {
        *wake = Some(ctx.clone());
    }
    if the_person_holds_it(app, ctx) {
        return;
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

    /// THE PERSON'S BUTTON, down or up in a context of the live window's shape: a pass is begun and ended
    /// around the event, as the live loop does.
    fn the_person_s_button(pressed: bool, ctx: &egui::Context) {
        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::PointerButton { pos: egui::pos2(400.0, 300.0), button: egui::PointerButton::Primary, pressed, modifiers: Default::default() });
        ctx.begin_pass(raw);
        let _ = ctx.end_pass();
    }

    /// A CALL WAITS OUT A DRAG OF THE PERSON: while their button is held the pump takes nothing, and the
    /// frame after the release runs the waiting call whole.
    #[test]
    fn a_call_waits_out_a_drag_of_the_person() {
        let mut app = App::default();
        let ctx = egui::Context::default();
        the_person_s_button(true, &ctx);
        let (answer, told) = std::sync::mpsc::channel();
        WAITING.lock().expect("the queue of the agent's calls is not poisoned").push_back(Call { work: Box::new(|_| ()), answer });
        pump(&mut app, &ctx);
        assert!(told.try_recv().is_err(), "the agent's call ran while the person was mid-drag");
        assert!(!WAITING.lock().expect("the queue of the agent's calls is not poisoned").is_empty(), "the call was dropped instead of waiting out the drag");
        the_person_s_button(false, &ctx);
        pump(&mut app, &ctx);
        assert!(told.try_recv().is_ok(), "the call did not resume once the person let go");
        assert!(WAITING.lock().expect("the queue of the agent's calls is not poisoned").is_empty(), "the call was left waiting after the release");
    }
}
