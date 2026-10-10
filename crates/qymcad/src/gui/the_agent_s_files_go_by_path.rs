//! THE AGENT'S FILES GO BY PATH: opening, saving and exporting act through the same door a chooser's answer
//! runs through, with the path carried by the call itself - no chooser put up, on the person's screen or
//! anywhere else.
#[cfg(test)]
mod tests {
    use super::super::session::AgentSession;
    use super::super::App;

    /// OPEN, SAVE AND EXPORT ROUND-TRIP A REAL FILE, no chooser anywhere: a body is made by the agent's own
    /// gestures, saved to a project file, opened back from it - the document comes back with the same body -
    /// and exported to a STEP file that opens as a STEP file.
    #[test]
    fn open_save_and_export_round_trip_a_real_file() {
        let mut app = App::default();
        app.waiting.splash_until = None;
        let dir = std::env::temp_dir().join(format!("qymcad-agent-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("the directory for the files");
        let project = dir.join("by-the-agent.qcad");
        let step = dir.join("by-the-agent.step");
        let _ = std::fs::remove_file(&project);
        let _ = std::fs::remove_file(&step);
        {
            let mut core = AgentSession::over(&mut app, egui::vec2(1400.0, 900.0));
            // a body, made as a person makes one (the acceptance's own `build::block` flow): the start screen
            // put away with Esc, the first part stepped into from the tree, a rectangle drawn on XY, the
            // sketch finished, extruded by its hint
            core.key(egui::Key::Escape);
            let part = core.document().parts.iter().find(|p| !p.assembly).expect("the first part").name.clone();
            let row = core.find(&part, egui::pos2(0.0, 300.0)).expect("the first part's row in the tree");
            core.double_click(row.center());
            core.press_word(&core.word("plane-xy-table"));
            core.press_hint(&core.word("tb-rect-hint"));
            core.click_on_sketch(0.0, 0.0);
            core.click_on_sketch(40.0, 30.0);
            core.press_word(&core.word("wb-finish"));
            core.press_hint(&core.word("tb-extrude-hint"));
            core.key(egui::Key::Enter);
            core.settle();
            assert_eq!(core.document().bodies.len(), 1, "the square extruded into a body");
            // SAVE: the path is the call's own, the write is waited out, and no chooser went up
            core.save(&project.to_string_lossy());
            core.settle();
            assert!(!core.asking_for_a_file(), "saving put up a chooser");
            assert_eq!(core.document().path, Some(project.to_string_lossy().into_owned()), "the saved path became the document's own");
            // OPEN: the same file, read back - the body comes back with it
            core.open(&project.to_string_lossy());
            core.settle();
            assert!(!core.asking_for_a_file(), "opening put up a chooser");
            assert_eq!(core.document().bodies.len(), 1, "the opened project came back with its body");
            // EXPORT: a STEP file, written where the call said
            core.export_step(&step.to_string_lossy());
            core.settle();
            assert!(!core.asking_for_a_file(), "exporting put up a chooser");
        }
        assert!(project.exists(), "the project file was written");
        let bytes = std::fs::read(&step).expect("the STEP file was written");
        assert!(bytes.len() > 100, "the STEP file holds a model");
        assert!(bytes.starts_with(b"ISO-10303"), "the STEP file opens as a STEP file: {:?}", String::from_utf8_lossy(&bytes[..bytes.len().min(40)]));
    }
}
