//! THE AGENT'S FRAMES AND THE PERSON'S stay coherent on one program: the agent drives its own window through
//! the pump - the same frame driver the checks use - while the person's window keeps its frames, and every
//! step of either leaves the program whole.
#[cfg(test)]
mod tests {
    use super::super::session::AgentSession;
    use super::super::window::Window;
    use super::super::App;
    use qymcad_core::feature::SketchPlane;

    /// ONE PROGRAM, TWO WINDOWS: the agent draws a chain of lines, one gesture to a slice of the pump, while
    /// the person's window keeps its frames over the same program. After every step - each gesture of the
    /// agent, each frame of the person - the program stands right: no key of the catalogue on either screen,
    /// no letter the font lacks, nothing waiting for a click with no bar saying so, the sketch in the
    /// person's tree, and the lines the agent drew the lines the program holds.
    #[test]
    fn the_agent_s_frames_and_the_person_s_stay_coherent() {
        let _turn = crate::mcp::take_your_turn();
        let mut app = App::default();
        app.waiting.splash_until = None;
        let si = app.create_sketch_on(SketchPlane::default());
        let sketch = crate::i18n::name(&app.project.sketches[si].name);
        // the person's window: the frame driver the checks use, standing in for the live one
        let mut person = Window::new(egui::vec2(1400.0, 900.0));
        super::super::install_fonts(&person.ctx);
        // the agent's work, one gesture a slice, the invariants looked at after each - the oracles' own kind
        let steps = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counting = steps.clone();
        let work: Box<crate::mcp::Work> = Box::new(move |core| {
            fn the_program_is_right(core: &mut AgentSession) {
                assert!(core.words_that_are_keys().is_empty(), "a key of the catalogue stands on the agent's screen instead of words");
                assert!(core.words_without_glyphs().is_empty(), "the agent's screen draws a letter the font does not have");
                assert!(core.waiting_without_bar().is_none(), "on the agent's screen something waits for a click with no bar saying so");
            }
            match counting.fetch_add(1, std::sync::atomic::Ordering::Relaxed) {
                0 => {
                    core.press_hint(&core.word("tb-line-hint"));
                }
                1 => {
                    core.click_on_sketch(0.0, 0.0);
                }
                2 => {
                    core.click_on_sketch(40.0, 0.0);
                }
                3 => {
                    core.click_on_sketch(40.0, 30.0);
                }
                _ => {
                    core.key(egui::Key::Escape);
                    the_program_is_right(core);
                    return true;
                }
            }
            the_program_is_right(core);
            false
        });
        let (answer, told) = std::sync::mpsc::channel();
        crate::mcp::hand_over(work, answer);
        let mut guard = 0;
        while told.try_recv().is_err() {
            guard += 1;
            assert!(guard < 50, "the call was never answered");
            person.run(&mut app, egui::Modifiers::default(), Vec::new(), false);
            // the frame of the person's window drew the program the agent is changing: the sketch is in their tree
            assert!(person.drawn.iter().any(|(t, _)| t == &sketch), "the person's window does not show the sketch the agent draws in");
            crate::mcp::pump(&mut app, &person.ctx);
        }
        let lines = app.project.sketches[si].entities.iter().filter(|e| matches!(e.kind, qymcad_core::model::EntityKind::Line { .. })).count();
        assert_eq!(lines, 2, "the agent's three clicks drew two lines");
    }
}
