use super::*;

/// THE HOUSING EXTRUDED, ITS SKETCH RE-ENTERED AND A LINE LAID ACROSS THE PROFILE: the edit session
/// closed, the selection follows the tiles of the re-divided region, the body keeps its shape, and the
/// picture is looked at through the rasteriser the help draws by.
pub(super) fn the_housing_extruded_and_divided(app: &mut App, si: usize, problems: &mut Vec<String>) {
    app.chosen.sel = crate::gui::Sel::Sketch(si);
    let mut hand = Hand::new(app);
    hand.look_at([30.0, 20.0, 10.0], 8.0).tool(1).set("height", 25.0).enter();
    check_all(app, "the housing was extruded", problems);
    let volume = |app: &App| app.project.timeline.iter().rev().find_map(|n| n.kind.body()).and_then(|b| app.project.bodies.iter().find(|x| x.id == b)).map(|b| b.mesh.volume()).unwrap_or(0.0);
    let v_before = volume(app);
    app.enter_sketch_edit(si);
    let mut hand = Hand::new(app);
    hand.sk_tool(1).click2d(30.0, 0.0).click2d(30.0, 40.0).key(egui::Key::Escape);
    app.finish_sketch_edit();
    check_all(app, "a line was laid across the extruded profile", problems);
    let v_after = volume(app);
    if (v_after - v_before).abs() > 1.0 {
        problems.push(format!("[divided profile] the body did not keep its shape through the re-tiling: it was {v_before:.1}, it is {v_after:.1}; status: {}", app.status));
    }
    // the widened selection is what the node stores - both halves, not one of them
    let sid = app.project.sketches[si].id;
    let holds = app.project.timeline.iter().find_map(|n| match &n.kind {
        qymcad_core::feature::FeatureKind::Extrude { sketch, profiles, .. } if *sketch == sid => Some(profiles.len()),
        _ => None,
    });
    if holds != Some(2) {
        problems.push(format!("[divided profile] the extrusion holds {holds:?} profile contours, both halves were expected; status: {}", app.status));
    }
    app.viewing.mode_3d = true;
    app.viewing.cam.init = true;
    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(700.0, 600.0));
    crate::gui::fit3d(&mut app.viewing.cam, &app.project, rect);
    let basis = app.viewing.cam.basis();
    let picture = qymcad_render::rasterize_3d(&app.painting(), rect, &basis, 1.0, 1.0).and_then(|img| crate::gui::color_image_to_png(&img));
    if picture.as_ref().is_none_or(|png| png.is_empty()) {
        problems.push(format!("[divided profile] the housing drew no picture; status: {}", app.status));
    }
}
