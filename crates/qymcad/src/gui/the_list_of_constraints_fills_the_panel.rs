//! THE LIST OF CONSTRAINTS AND DIMENSIONS FILLS THE PANEL IT STANDS IN.
//!
//! Reported behaviour: the list was capped at a fixed 360 points of height, and below it a third of
//! the properties panel stood empty whatever the window's height - the rows scrolled inside their
//! strip while the panel had room to spare.
#[cfg(test)]
mod tests {
    use super::super::App;
    use qymcad_core::feature::{Purpose, SketchPlane};

    /// A sketch of many rectangles: every side brings its own constraints, so the list holds far more
    /// rows than any panel shows at once.
    fn many_rows() -> App {
        let mut app = App::default();
        let si = app.create_sketch_on(SketchPlane::default());
        for k in 0..12 {
            let x = 30.0 * f64::from(k);
            app.project.add_rect_entity(si, x, 0.0, x + 20.0, 12.0, Purpose::Real);
        }
        app.project.regen_sketch(si);
        app.enter_sketch_edit(si);
        app
    }

    /// The captions of a frame with the rect each is clipped to, shapes nested in vectors included.
    /// A row of the list is clipped to the list's own viewport, so the deepest clip among the rows
    /// ends where the list ends on screen.
    fn clipped_captions(s: &egui::epaint::Shape, clip: egui::Rect, out: &mut Vec<(String, egui::Rect)>) {
        match s {
            egui::epaint::Shape::Text(t) => out.push((t.galley.text().to_string(), clip)),
            egui::epaint::Shape::Vec(v) => v.iter().for_each(|x| clipped_captions(x, clip, out)),
            _ => {}
        }
    }

    /// THE LIST ENDS NEAR THE BOTTOM OF THE PANEL. The panel is drawn into a frame 900 points tall;
    /// the viewport that clips the last visible row must end within 90 points of that bottom. A list
    /// of a fixed height stopped about a third short of it, and the rest of the panel stood empty.
    #[test]
    fn the_list_ends_near_the_bottom_of_the_panel() {
        let prev = crate::i18n::language();
        crate::i18n::set_language("en");
        // the row marker is asked of the catalogue, as a person reads it: "Horizontal: Line 3"
        let line_word = crate::i18n::tr("ent-line");
        let mut app = many_rows();
        let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1400.0, 900.0));
        let input = egui::RawInput { screen_rect: Some(screen), ..Default::default() };
        let ctx = egui::Context::default();
        super::super::install_fonts(&ctx);
        // two passes: the first lays the panel out, the second is the one a person sees
        let _ = ctx.run_ui(input.clone(), |ui| app.properties_panel(ui));
        let out = ctx.run_ui(input, |ui| app.properties_panel(ui));
        let mut captions = Vec::new();
        for cs in &out.shapes {
            clipped_captions(&cs.shape, cs.clip_rect, &mut captions);
        }
        crate::i18n::set_language(&prev);
        let rows: Vec<&(String, egui::Rect)> = captions.iter().filter(|(t, _)| t.contains(&line_word) && t.contains(": ")).collect();
        assert!(!rows.is_empty(), "the frame holds no constraint rows - the check has nothing to look at");
        let bottom = rows.iter().map(|(_, r)| r.bottom()).fold(f32::NEG_INFINITY, f32::max);
        let words: Vec<&str> = rows.iter().map(|(t, _)| t.as_str()).collect();
        assert!(bottom > screen.bottom() - 90.0, "the list ends at y={bottom} of a panel {:.0} tall - it does not fill the panel (rows on screen: {words:?})", screen.height());
    }
}
