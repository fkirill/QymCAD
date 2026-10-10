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

/// WHAT THE AGENT DOES WITH THE PROGRAM: a slice of work over the driving core. The slice returns when its
/// time is spent, saying whether the whole of the work has run - until it has, the next live frame takes up
/// the call again, and the person's window keeps its frames coming meanwhile.
pub(crate) type Work = dyn FnMut(&mut AgentSession) -> bool + Send;

/// ONE CALL OF THE AGENT: the work it asks of the program, and where its answer lands once the work has run.
struct Call {
    /// The work, run on the frame thread over the driving core, a slice to a live frame.
    work: Box<Work>,
    /// Told once the work has run: the answer door of whoever asked.
    answer: Sender<()>,
}

/// A CALL HALF-RUN: the hands its slices share - the agent's own window, the state of its gestures - and the
/// work still to do. The program itself belongs to the frame and is lent anew by each slice.
struct Running {
    hands: crate::gui::session::Hands,
    work: Box<Work>,
    answer: Sender<()>,
}

/// THE CALLS WAITING for a frame of the program to take them, oldest first.
static WAITING: Mutex<VecDeque<Call>> = Mutex::new(VecDeque::new());

/// THE CALL IN THE FRAME, half-run: taken up again by the next live frame, one slice at a time, until the
/// work says it is done.
static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

/// IS A CALL OF THE AGENT RUNNING - taken up, not yet answered: what the window says with the card of a
/// background job, so the person knows whose work the program is doing.
static AT_WORK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// THE AGENT AT WORK, said with the card a background job uses, over the frame just drawn: the person's
/// window keeps its frames coming under it, and the card goes the moment the call is answered.
pub(crate) fn the_agent_at_work_card(logo: &Option<egui::TextureHandle>, scheme: &qymcad_ui_state::SchemeUi, ctx: &egui::Context) {
    if !AT_WORK.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    qymcad_render::draw_splash(logo, scheme, ctx, &crate::i18n::tr("mcp-at-work"));
}

/// THE LIVE CONTEXT, as the last frame left it: what a call wakes, so the frame that takes it comes at once
/// rather than when the person next moves the mouse.
static WAKE: Mutex<Option<egui::Context>> = Mutex::new(None);

/// HAND A CALL OF THE AGENT to the frames of the program: it runs on the frame thread, a slice to a live
/// frame, and its answer comes back from there. The live window is woken for it. The server thread's door;
/// until the server lands, the checks drive it.
#[cfg(test)]
pub(crate) fn hand_over(work: Box<Work>, answer: Sender<()>) {
    if let Ok(mut waiting) = WAITING.lock() {
        waiting.push_back(Call { work, answer });
    }
    if let Some(ctx) = WAKE.lock().ok().and_then(|wake| wake.clone()) {
        ctx.request_repaint();
    }
}

/// DOES THE PERSON HOLD THE PROGRAM: a button of theirs down, a field of theirs holding the keyboard, a
/// chooser of theirs open. The program is theirs first - the agent's call waits in the queue until the
/// gesture is over, and the frame that ends the gesture is the one that takes it.
fn the_person_holds_it(app: &App, ctx: &egui::Context) -> bool {
    ctx.input(|i| i.pointer.any_down()) || ctx.egui_wants_keyboard_input() || app.asking_for_a_file()
}

/// THE SIZE OF THE AGENT'S OWN WINDOW: the person's, or the program's opening size before the first frame
/// has said one.
fn window_size(ctx: &egui::Context) -> egui::Vec2 {
    ctx.input(|i| i.raw.screen_rect.unwrap_or(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0)))).size()
}

/// TAKE A SLICE OF THE AGENT'S WORK into this frame of the program - unless the person is mid-gesture, when
/// the call waits for the gesture to end. The work runs over the driving core - the program driven by whole
/// frames, on a window of the agent's own - a slice to a live frame; a call half-done is taken up again by
/// the next frame, asked for at once, and its answer is told only when the whole of it has run.
pub(crate) fn pump(app: &mut App, ctx: &egui::Context) {
    if let Ok(mut wake) = WAKE.lock() {
        *wake = Some(ctx.clone());
    }
    if the_person_holds_it(app, ctx) {
        return;
    }
    let (mut work, answer, mut core) = match RUNNING.lock().ok().and_then(|mut running| running.take()) {
        Some(running) => (running.work, running.answer, AgentSession::with_hands(running.hands, app)),
        None => {
            let Some(call) = WAITING.lock().ok().and_then(|mut waiting| waiting.pop_front()) else { return };
            (call.work, call.answer, AgentSession::over(app, window_size(ctx)))
        }
    };
    AT_WORK.store(true, std::sync::atomic::Ordering::Relaxed);
    let done = work(&mut core);
    let hands = core.hands();
    if done {
        AT_WORK.store(false, std::sync::atomic::Ordering::Relaxed);
        let _ = answer.send(());
    } else {
        if let Ok(mut held) = RUNNING.lock() {
            *held = Some(Running { hands, work, answer });
        }
        ctx.request_repaint();
    }
}

/// THE CHECKS TAKE TURNS at the agent's machinery: the queue and the frame are one for the whole process,
/// and checks beside each other would take each other's calls.
#[cfg(test)]
pub(crate) fn take_your_turn() -> std::sync::MutexGuard<'static, ()> {
    static TURNS: Mutex<()> = Mutex::new(());
    TURNS.lock().expect("the turns of the checks are not held by a poisoned lock")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE PUMP TAKES A CALL IN ONE FRAME: what the queue holds is run by one call of the pump - the first
    /// act of a live frame - and its answer comes back with nothing left waiting.
    #[test]
    fn the_pump_takes_a_call_in_one_frame() {
        let _turn = take_your_turn();
        let mut app = App::default();
        let (answer, told) = std::sync::mpsc::channel();
        WAITING.lock().expect("the queue of the agent's calls is not poisoned").push_back(Call { work: Box::new(|_| true), answer });
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
        let _turn = take_your_turn();
        let mut app = App::default();
        let ctx = egui::Context::default();
        the_person_s_button(true, &ctx);
        let (answer, told) = std::sync::mpsc::channel();
        WAITING.lock().expect("the queue of the agent's calls is not poisoned").push_back(Call { work: Box::new(|_| true), answer });
        pump(&mut app, &ctx);
        assert!(told.try_recv().is_err(), "the agent's call ran while the person was mid-drag");
        assert!(!WAITING.lock().expect("the queue of the agent's calls is not poisoned").is_empty(), "the call was dropped instead of waiting out the drag");
        the_person_s_button(false, &ctx);
        pump(&mut app, &ctx);
        assert!(told.try_recv().is_ok(), "the call did not resume once the person let go");
        assert!(WAITING.lock().expect("the queue of the agent's calls is not poisoned").is_empty(), "the call was left waiting after the release");
    }

    /// THE WORDS OF THE CARD on a context of the live window's shape: a first pass lays the card out, the
    /// next draws it - an anchored window finds its place in one pass and paints in the following one.
    fn the_words_of_the_card(ctx: &egui::Context) -> Vec<String> {
        fn texts(s: &egui::Shape, out: &mut Vec<String>) {
            match s {
                egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
                _ => {}
            }
        }
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))), ..Default::default() };
        let card = |ui: &mut egui::Ui| the_agent_at_work_card(&None, &qymcad_ui_state::SchemeUi::default(), ui.ctx());
        let _ = ctx.run_ui(raw.clone(), card);
        let out = ctx.run_ui(raw, card);
        let mut words = Vec::new();
        for cs in &out.shapes {
            texts(&cs.shape, &mut words);
        }
        words
    }

    /// THE CARD SAYS THE AGENT IS AT WORK while a call runs, and says nothing once the call is answered: the
    /// words come with the first slice and go with the answer.
    #[test]
    fn the_card_says_the_agent_is_at_work_while_a_call_runs() {
        let _turn = take_your_turn();
        let mut app = App::default();
        let ctx = egui::Context::default();
        let runs = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counting = runs.clone();
        let (answer, told) = std::sync::mpsc::channel();
        hand_over(Box::new(move |_| counting.fetch_add(1, std::sync::atomic::Ordering::Relaxed) > 0), answer);
        pump(&mut app, &ctx);
        assert!(told.try_recv().is_err(), "the call of two slices answered after one");
        let words = the_words_of_the_card(&ctx);
        assert!(words.contains(&crate::i18n::tr("mcp-at-work")), "while a call runs the window does not say so: {words:?}");
        pump(&mut app, &ctx);
        assert!(told.try_recv().is_ok(), "the second slice did not answer the call");
        let words = the_words_of_the_card(&ctx);
        assert!(!words.contains(&crate::i18n::tr("mcp-at-work")), "after the call answered the window still says the agent is at work: {words:?}");
    }
}
