//! THE AGENT'S CORE DRIVES A PROGRAM IT DOES NOT OWN: the same corner fillet the hand's own check gets, made
//! with the same person-level gestures over a borrowed program - no session around it, no stand-in for the
//! system, no window on the person's screen.
#[cfg(test)]
mod tests {
    use super::super::session::AgentSession;
    use super::super::App;
    use qymcad_core::feature::SketchPlane;
    use qymcad_core::model::{Constraint, EntityKind};

    /// The radii of the arcs of the sketch.
    fn arc_radii(app: &App, si: usize) -> Vec<f64> {
        let s = &app.project.sketches[si];
        let at = |id: u64| s.points.iter().find(|q| q.id == id).map(|q| (q.x, q.y)).expect("a point");
        s.entities
            .iter()
            .filter_map(|e| match e.kind {
                EntityKind::Arc { center, a, .. } => {
                    let (c, p) = (at(center), at(a));
                    Some((p.0 - c.0).hypot(p.1 - c.1))
                }
                _ => None,
            })
            .collect()
    }

    /// A SET OF CORNERS IS ROUNDED BY ITS CHORD through the agent's core, as the hand rounds it (`a_corner_set_
    /// takes_its_size_whole`): a square drawn with the line tool, the corner tool by its hint, the chord mode of
    /// its bar pressed, three lines of the square named with Shift, the value typed and Enter - and the same two
    /// arcs of the same radius, held by the same chords, come out of the program the core does not own.
    #[test]
    fn a_set_of_corners_is_rounded_by_its_chord() {
        let mut app = App::default();
        // the greeting is over, as a person past it is; the sketch is entered as the check's own setup enters it
        app.waiting.splash_until = None;
        let si = app.create_sketch_on(SketchPlane::default());
        let mut core = AgentSession::over(&mut app, egui::vec2(1400.0, 900.0));
        core.press_hint(&core.word("tb-line-hint"));
        for &(x, y) in &[(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0), (0.0, 0.0)] {
            core.click_on_sketch(x, y);
        }
        core.key(egui::Key::Escape);
        core.press_hint(&core.word("tb-fillet-sketch-hint"));
        core.press_word(&core.word("opt-fillet-chord"));
        // the middle of each of the three lines the hand names: the left, the bottom, the right
        for &(x, y) in &[(0.0, 15.0), (20.0, 0.0), (40.0, 15.0)] {
            let at = core.on_sketch(x, y);
            core.click_with(at, egui::PointerButton::Primary, egui::Modifiers::SHIFT);
        }
        core.type_text("3");
        core.key(egui::Key::Enter);
        let radii = arc_radii(&app, si);
        let r = 3.0 / (2.0 * std::f64::consts::FRAC_PI_4.sin());
        assert!(radii.len() == 2 && radii.iter().all(|x| (x - r).abs() < 1e-6), "two corners of a chord of 3 have arcs of radius {r}: {radii:?}");
        let s = &app.project.sketches[si];
        let chords = s.constraints.iter().filter(|c| matches!(c, Constraint::Distance { d, .. } if (d - 3.0).abs() < 1e-9)).count();
        let held_by_radius = s.constraints.iter().filter(|c| matches!(c, Constraint::Diameter { .. })).count();
        assert_eq!((chords, held_by_radius), (2, 0), "each corner keeps its chord of 3 and no radius: {:?}", s.constraints);
    }
}
