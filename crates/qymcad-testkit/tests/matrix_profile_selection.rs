//! MATRIX: the selected area of a feature against the REAL OCCT kernel through a re-tiling of its sketch.
//! A line across the region, a line deleted, a circle drawn inside - the body's volume does not move,
//! because the selection follows the tiles. The same measurement runs through a revolve and through the
//! extrude command's boolean cut. All failures accumulate and are reported at once.

use qymcad_core::feature::{Extent, Purpose, Reach};
use qymcad_core::model::{CombineSpan, Project};

const PI: f64 = std::f64::consts::PI;

/// A rectangle scene standing: the project, the sketch's index and id, and the id of its region.
struct RectScene {
    p: Project,
    si: usize,
    sid: u64,
    region: u64,
}

/// A part with one sketch of a 40 x 30 rectangle over 0..40 x 0..30, its id, and the id of its region.
fn rect_scene() -> RectScene {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("s");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "s");
    p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).unwrap();
    RectScene { p, si, sid, region: cid }
}

/// Rebuild after an edit and read the body's volume, with the regen errors reported beside.
fn volume_after_edit(p: &mut Project, sid: u64, label: &str, body: u64, fails: &mut Vec<String>) -> f64 {
    p.mark_sketch_dirty(sid);
    let (report, shapes) = qymcad_testkit::regenerate(p);
    for (id, er) in &report.errors {
        fails.push(format!("{label}: REGEN ERROR {id}: {er}"));
    }
    shapes.get(&body).map(|s| s.volume()).unwrap_or(0.0)
}

fn near(fails: &mut Vec<String>, label: &str, volumes: (f64, f64)) {
    let (got, want) = volumes;
    if ((got - want) / want).abs() > 0.01 {
        fails.push(format!("{label}: V={got:.1}, expected {want:.1}"));
    }
}

/// An extruded rectangle keeps its volume through a subdivision, a merge back, and a circle drawn inside.
#[test]
fn matrix_extrude_volume_through_a_retiling() {
    let mut fails: Vec<String> = Vec::new();
    let RectScene { mut p, si, sid, region: cid } = rect_scene();
    let e = p.add_extrude_multi(sid, vec![cid], 10.0, Reach::Forward, 0.0, vec![]);
    let body = p.finish_base_body(e, 1);
    let (report, shapes) = qymcad_testkit::regenerate(&mut p);
    for (id, er) in &report.errors {
        fails.push(format!("the start: REGEN ERROR {id}: {er}"));
    }
    let v0 = shapes.get(&body).map(|s| s.volume()).unwrap_or(0.0);
    near(&mut fails, "the start", (v0, 12000.0));
    // a line across the region: the selection holds both halves, the body is the same box
    let line = p.add_line_entity(si, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let v = volume_after_edit(&mut p, sid, "line across", body, &mut fails);
    near(&mut fails, "line across", (v, 12000.0));
    // the line deleted: the parts merge back, the selection collapses, the body is the same box
    p.delete_entities(si, &[line]);
    let v = volume_after_edit(&mut p, sid, "line deleted", body, &mut fails);
    near(&mut fails, "line deleted", (v, 12000.0));
    // a circle drawn wholly inside: it joins the selection filled, the body stays solid
    p.add_circle_entity(si, 20.0, 15.0, 5.0, Purpose::Real);
    let v = volume_after_edit(&mut p, sid, "circle inside", body, &mut fails);
    near(&mut fails, "circle inside", (v, 12000.0));
    assert!(fails.is_empty(), "\nMATRIX FAILURES ({}):\n{}", fails.len(), fails.join("\n"));
}

/// A revolve follows its region the same way: the ring the rectangle sweeps keeps its volume through a
/// subdivision of the region.
#[test]
fn matrix_revolve_volume_through_a_retiling() {
    let mut fails: Vec<String> = Vec::new();
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("s");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "s");
    // off the X axis, so the revolve sweeps a ring and no boundary touches the axis it turns about
    p.add_rect_entity(si, 10.0, 5.0, 40.0, 35.0, Purpose::Real);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).unwrap();
    let e = p.add_revolve_axis(sid, vec![cid], 0, 360.0, 0, 0);
    let body = p.finish_base_body(e, 1);
    let (report, shapes) = qymcad_testkit::regenerate(&mut p);
    for (id, er) in &report.errors {
        fails.push(format!("the start: REGEN ERROR {id}: {er}"));
    }
    let want = PI * (35.0 * 35.0 - 5.0 * 5.0) * 30.0;
    near(&mut fails, "the start", (shapes.get(&body).map(|s| s.volume()).unwrap_or(0.0), want));
    p.add_line_entity(si, 25.0, 5.0, 25.0, 35.0, Purpose::Real);
    let v = volume_after_edit(&mut p, sid, "line across", body, &mut fails);
    near(&mut fails, "line across", (v, want));
    assert!(fails.is_empty(), "\nMATRIX FAILURES ({}):\n{}", fails.len(), fails.join("\n"));
}

/// The extrude command's boolean cut keeps its shape through a subdivision of the tool's region.
#[test]
fn matrix_combine_volume_through_a_retiling() {
    let mut fails: Vec<String> = Vec::new();
    let mut p = Project::default();
    p.new_document();
    let bsi = p.new_sketch("base");
    let bsid = p.sketches[bsi].id;
    p.add_sketch_node(bsid, "base");
    p.add_rect_entity(bsi, -20.0, -20.0, 80.0, 80.0, Purpose::Real);
    p.regen_sketch(bsi);
    let bcid = p.sketches[bsi].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).unwrap();
    let e = p.add_extrude_multi(bsid, vec![bcid], 20.0, Reach::Forward, 0.0, vec![]);
    let plate = p.finish_base_body(e, 1);
    let (mut p, si, sid, cid) = {
        // the tool scene, laid into the same project after the plate
        let si = p.new_sketch("tool");
        let sid = p.sketches[si].id;
        p.add_sketch_node(sid, "tool");
        p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
        p.regen_sketch(si);
        let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).unwrap();
        (p, si, sid, cid)
    };
    let node = p.add_combine_multi_op(plate, sid, vec![cid], CombineSpan { height: 30.0, down: 0.0, extent: Extent::default(), fill: &[] }, 0);
    let (report, shapes) = qymcad_testkit::regenerate(&mut p);
    for (id, er) in &report.errors {
        fails.push(format!("the start: REGEN ERROR {id}: {er}"));
    }
    let v0 = shapes.get(&node).map(|s| s.volume()).unwrap_or(0.0);
    near(&mut fails, "the start", (v0, 100.0 * 100.0 * 20.0 - 40.0 * 30.0 * 20.0));
    p.add_line_entity(si, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let v = volume_after_edit(&mut p, sid, "line across", node, &mut fails);
    near(&mut fails, "line across", (v, 100.0 * 100.0 * 20.0 - 40.0 * 30.0 * 20.0));
    assert!(fails.is_empty(), "\nMATRIX FAILURES ({}):\n{}", fails.len(), fails.join("\n"));
}
