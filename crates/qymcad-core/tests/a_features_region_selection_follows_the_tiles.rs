//! A feature's selected area survives a re-tiling of its sketch. The selection names an area; when an edit
//! divides a selected region, the references become the set of new regions that together cover the old one
//! exactly, and when an edit merges selected regions, they become the one region covering them. A closed
//! region newly created inside the area joins it filled, so new geometry inside does not silently become a
//! hole. Exactness gates everything: a moved boundary is followed as it is, never rewritten.

use qymcad_core::feature::{Extent, FeatureKind, Purpose, Reach};
use qymcad_core::geom::Point2;
use qymcad_core::model::{CombineSpan, Id, Project};

/// The sketch and the region a scene's feature is built over.
struct Over {
    sketch: Id,
    region: Id,
}

/// A scene standing: the project, its sketch's index and id, and the node of the feature over the region.
struct Scene {
    p: Project,
    si: usize,
    sid: Id,
    node: Id,
}

/// The selection lists a node stores: the profiles, and the filled contours beside them.
struct Selection {
    profiles: Vec<Id>,
    fill: Vec<Id>,
}

/// A part with one sketch holding a rectangle of 40 x 30, and a feature over the rectangle's region. The
/// node id doubles as the body id, as everywhere in the timeline.
fn scene(build: fn(&mut Project, Over) -> Id) -> Scene {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("s");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "s");
    p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    p.regen_sketch(si);
    let cid = one_region_of_area(&p, si, (1200.0, 1.0));
    let node = build(&mut p, Over { sketch: sid, region: cid });
    Scene { p, si, sid, node }
}

fn extrude(p: &mut Project, over: Over) -> Id {
    p.add_extrude_multi(over.sketch, vec![over.region], 10.0, Reach::Forward, 0.0, vec![])
}

/// The ids of the closed regions of the sketch whose area is within `tol` of `area`.
fn regions_of_area(p: &Project, si: usize, about: (f64, f64)) -> Vec<Id> {
    let (area, tol) = about;
    p.sketches[si].contour_ids.iter().copied().filter(|&cid| p.contour_index(cid).is_some_and(|ci| p.contours[ci].closed && (p.contours[ci].area() - area).abs() < tol)).collect()
}

fn one_region_of_area(p: &Project, si: usize, about: (f64, f64)) -> Id {
    let (area, _tol) = about;
    let found = regions_of_area(p, si, about);
    assert_eq!(found.len(), 1, "one region of about {area} is expected, the sketch offers {:?}", regions_of_area(p, si, (area, 1000.0)));
    found[0]
}

/// The area the selection resolves to: every selected region minus its holes.
fn resolved_area(p: &Project, sid: Id, lists: [&[Id]; 2]) -> f64 {
    let [profiles, fill] = lists;
    let area = |cid: Id| p.contours[p.contour_index(cid).expect("a live contour")].area();
    p.profile_faces(sid, profiles, fill).into_iter().map(|(outer, holes)| area(outer) - holes.iter().copied().map(area).sum::<f64>()).sum()
}

/// The selection lists the node stores: what the document saves and the edit command re-offers.
fn selection(p: &Project, node: Id) -> Selection {
    let n = p.timeline.iter().find(|n| n.id == node).expect("the node");
    match &n.kind {
        FeatureKind::Extrude { profiles, fill, .. } | FeatureKind::Combine { profiles, fill, .. } => Selection { profiles: profiles.clone(), fill: fill.clone() },
        FeatureKind::Revolve { profiles, .. } | FeatureKind::Sweep { profiles, .. } => Selection { profiles: profiles.clone(), fill: Vec::new() },
        FeatureKind::Loft { contours, .. } => Selection { profiles: contours.clone(), fill: Vec::new() },
        _ => panic!("not a feature holding regions"),
    }
}

fn sorted(mut v: Vec<Id>) -> Vec<Id> {
    v.sort_unstable();
    v.dedup();
    v
}

// A line from one side of the extruded rectangle to the opposite side divides it into two regions: the
// selection has to hold both, and the resolved area stays the whole rectangle. Before the fix it held one
// half, the body silently cut in half.
#[test]
fn a_line_across_the_extruded_region_keeps_both_halves() {
    let Scene { mut p, si, sid, node } = scene(extrude);
    p.add_line_entity(si, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let halves = regions_of_area(&p, si, (600.0, 1.0));
    assert_eq!(halves.len(), 2, "the rectangle is two regions of 600 now");
    let Selection { profiles, fill } = selection(&p, node);
    assert_eq!(sorted(profiles.clone()), sorted(halves), "the extrude holds both halves, not one of them");
    assert!(fill.is_empty(), "nothing is filled: the halves are regions of their own");
    assert!((resolved_area(&p, sid, [&profiles, &fill]) - 1200.0).abs() < 1.0, "the resolved area is the whole rectangle");
}

// One line crossing two regions of one selection: each divided region is followed by its own cover.
#[test]
fn one_line_through_two_selected_regions_divides_each() {
    let (mut p, si, sid) = {
        let mut p = Project::default();
        p.new_document();
        let si = p.new_sketch("s");
        let sid = p.sketches[si].id;
        p.add_sketch_node(sid, "s");
        p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
        p.add_rect_entity(si, 60.0, 0.0, 100.0, 30.0, Purpose::Real);
        p.regen_sketch(si);
        (p, si, sid)
    };
    let both = regions_of_area(&p, si, (1200.0, 1.0));
    assert_eq!(both.len(), 2, "two rectangles, two regions");
    let by_x = |cid: Id| p.contours[p.contour_index(cid).unwrap()].centroid().x;
    let (left, right) = if by_x(both[0]) < by_x(both[1]) { (both[0], both[1]) } else { (both[1], both[0]) };
    let node = p.add_extrude_multi(sid, vec![left, right], 10.0, Reach::Forward, 0.0, vec![]);
    p.add_line_entity(si, 0.0, 15.0, 100.0, 15.0, Purpose::Real);
    let Selection { profiles, fill } = selection(&p, node);
    let halves = regions_of_area(&p, si, (600.0, 1.0));
    assert_eq!(halves.len(), 4, "both rectangles are divided into two regions of 600");
    assert_eq!(sorted(profiles.clone()), sorted(halves), "the selection holds every resulting region");
    assert!(fill.is_empty());
    assert!((resolved_area(&p, sid, [&profiles, &fill]) - 2400.0).abs() < 2.0, "the resolved area is both rectangles");
}

// A diameter line across an extruded disc: the halves carry arc tessellation, and the cover still has to
// fire - the tolerance of the gate is measured on this case (see `COVER_TOL` in `tiles_covering`).
#[test]
fn a_line_across_an_extruded_disc_keeps_both_halves() {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("s");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "s");
    p.add_circle_entity(si, 0.0, 0.0, 10.0, Purpose::Real);
    p.regen_sketch(si);
    let cid = one_region_of_area(&p, si, (std::f64::consts::PI * 100.0, 2.0));
    let node = extrude(&mut p, Over { sketch: sid, region: cid });
    p.add_line_entity(si, -20.0, 0.0, 20.0, 0.0, Purpose::Real);
    let halves = regions_of_area(&p, si, (std::f64::consts::PI * 50.0, 2.0));
    assert_eq!(halves.len(), 2, "the disc is two half discs now");
    let Selection { profiles, .. } = selection(&p, node);
    assert_eq!(sorted(profiles), sorted(halves), "the extrude holds both halves of the disc");
}

// A circle drawn wholly inside the selected region does not become a hole: it joins the selection filled and
// the resolved area keeps the whole rectangle.
#[test]
fn a_circle_inside_the_selected_region_stays_solid() {
    let Scene { mut p, si, sid, node } = scene(extrude);
    p.add_circle_entity(si, 20.0, 15.0, 5.0, Purpose::Real);
    let disc = one_region_of_area(&p, si, (std::f64::consts::PI * 25.0, 2.0));
    let Selection { profiles, fill } = selection(&p, node);
    assert_eq!(sorted(profiles.clone()), vec![one_region_of_area(&p, si, (1200.0, 1.0))], "the region itself stands");
    assert_eq!(fill, vec![disc], "the new disc is marked filled, not left to be a hole");
    assert!((resolved_area(&p, sid, [&profiles, &fill]) - 1200.0).abs() < 1.0, "the resolved area is the whole rectangle, hole for hole");
}

// A boundary of the selected region moved so its area changes: no rewrite, the feature follows its region
// and the body changes with it. Geometry added beside the selection enters no list.
#[test]
fn a_moved_boundary_is_followed_and_not_rewritten() {
    let Scene { mut p, si, sid, node } = scene(extrude);
    for pt in &mut p.sketches[si].points {
        if (pt.x - 40.0).abs() < 1e-6 {
            pt.x = 50.0; // the right side of the rectangle moves out by 10
        }
    }
    p.regen_sketch(si);
    let Selection { profiles, fill } = selection(&p, node);
    assert_eq!(profiles.len(), 1, "no rewrite: the selection is the one region it was");
    assert!((resolved_area(&p, sid, [&profiles, &fill]) - 1500.0).abs() < 1.0, "the feature follows its region into the new area");
    p.add_rect_entity(si, 100.0, 0.0, 140.0, 30.0, Purpose::Real);
    let Selection { profiles: beside, fill: fill_beside } = selection(&p, node);
    assert_eq!(beside, profiles, "a region outside the selected area stays out of it");
    assert!(fill_beside.is_empty());
}

// Deleting the dividing line merges the parts back into one region: the selection collapses to it.
#[test]
fn deleting_the_dividing_line_collapses_the_selection_back() {
    let Scene { mut p, si, sid, node } = scene(extrude);
    let line = p.add_line_entity(si, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let Selection { profiles: widened, .. } = selection(&p, node);
    assert_eq!(widened.len(), 2, "the subdivision widened the selection first");
    p.delete_entities(si, &[line]);
    let merged = one_region_of_area(&p, si, (1200.0, 1.0));
    let Selection { profiles, fill } = selection(&p, node);
    assert_eq!(profiles, vec![merged], "the selection collapses to the one region covering the parts");
    assert!(fill.is_empty());
    assert!((resolved_area(&p, sid, [&profiles, &fill]) - 1200.0).abs() < 1.0);
}

// An ellipse lives outside the arrangement and nests inside a region instead of dividing it: drawn inside
// the selection it is filled, and the resolved area stands.
#[test]
fn an_ellipse_drawn_inside_the_selection_is_filled_not_holed() {
    let Scene { mut p, si, sid, node } = scene(extrude);
    p.add_ellipse_entity(si, Point2::new(20.0, 15.0), 5.0, 3.0, 0.0, Purpose::Real);
    let nested = one_region_of_area(&p, si, (std::f64::consts::PI * 15.0, 1.0));
    let Selection { profiles, fill } = selection(&p, node);
    assert_eq!(fill, vec![nested], "the fresh nested ellipse joins the selection as filled");
    assert!((resolved_area(&p, sid, [&profiles, &fill]) - 1200.0).abs() < 1.0, "the body stays solid");
}

// A nested region that existed before the edit and was left unselected is a deliberate hole: an unrelated
// edit does not fill it.
#[test]
fn a_preexisting_nested_region_stays_a_hole() {
    let (mut p, si, sid, node) = {
        let mut p = Project::default();
        p.new_document();
        let si = p.new_sketch("s");
        let sid = p.sketches[si].id;
        p.add_sketch_node(sid, "s");
        p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
        p.add_ellipse_entity(si, Point2::new(20.0, 15.0), 5.0, 3.0, 0.0, Purpose::Real);
        p.regen_sketch(si);
        let cid = one_region_of_area(&p, si, (1200.0, 1.0));
        let node = extrude(&mut p, Over { sketch: sid, region: cid });
        (p, si, sid, node)
    };
    p.add_rect_entity(si, 100.0, 0.0, 140.0, 30.0, Purpose::Real); // an edit beside the selection
    let Selection { profiles, fill } = selection(&p, node);
    assert_eq!(profiles.len(), 1, "the region stands as it was");
    assert!(fill.is_empty(), "the pre-existing nested ellipse was left unselected and stays a hole");
    let hole = std::f64::consts::PI * 15.0;
    assert!((resolved_area(&p, sid, [&profiles, &fill]) - (1200.0 - hole)).abs() < 1.0, "the body keeps its hole");
}

// The same hole through an edit that re-derives only the region round it - the windowed path of the loops,
// where the ellipse stands untouched beside the loops being replaced: still a hole, not a fill.
#[test]
fn a_preexisting_nested_region_stays_a_hole_through_a_windowed_edit() {
    let (mut p, si, sid, node) = {
        let mut p = Project::default();
        p.new_document();
        let si = p.new_sketch("s");
        let sid = p.sketches[si].id;
        p.add_sketch_node(sid, "s");
        p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
        p.add_ellipse_entity(si, Point2::new(20.0, 15.0), 5.0, 3.0, 0.0, Purpose::Real);
        p.regen_sketch(si);
        let cid = one_region_of_area(&p, si, (1200.0, 1.0));
        let node = extrude(&mut p, Over { sketch: sid, region: cid });
        (p, si, sid, node)
    };
    p.add_line_entity(si, 10.0, 0.0, 10.0, 30.0, Purpose::Real); // a line across the region, clear of the ellipse
    let Selection { profiles, fill } = selection(&p, node);
    assert_eq!(profiles.len(), 2, "the region was divided and the selection holds both halves");
    assert!(fill.is_empty(), "the pre-existing ellipse was left unselected and stays a hole through the windowed edit");
    let hole = std::f64::consts::PI * 15.0;
    assert!((resolved_area(&p, sid, [&profiles, &fill]) - (1200.0 - hole)).abs() < 1.0, "the body keeps its hole: the halves ring the ellipse");
}

// The widened selection is written into the feature's own parameters - what the document saves and the edit
// command re-offers - and undo, being bound to the whole operation, takes it back with the edit. The core
// check restores the snapshot the undo step holds; the acceptance run drives the window's own undo.
#[test]
fn the_widened_selection_is_written_back_and_undo_takes_it_back() {
    let Scene { mut p, si, node, .. } = scene(extrude);
    let undo_step = p.clone(); // the state before the edit, as the undo stack holds it
    p.add_line_entity(si, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let Selection { profiles: widened, .. } = selection(&p, node);
    assert_eq!(widened.len(), 2, "the stored selection - what re-editing the node offers - holds the parts");
    let Selection { profiles: back, .. } = selection(&undo_step, node);
    assert_eq!(back.len(), 1, "the snapshot before the edit holds the single original reference");
    let p = undo_step; // undo: the snapshot is put back
    let Selection { profiles: restored, .. } = selection(&p, node);
    assert_eq!(restored.len(), 1, "undoing the edit takes the widened selection back with it");
}

// Every kind whose selection holds any number of regions follows the same rule, each in its own right: a
// revolve, a sweep, and the extrude command's boolean form against an existing body.
#[test]
fn a_revolve_over_a_subdivided_region_holds_both_parts() {
    let Scene { mut p, si, node, .. } = scene(|p, over| p.add_revolve_axis(over.sketch, vec![over.region], 0, 360.0, 0, 0));
    p.add_line_entity(si, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let halves = regions_of_area(&p, si, (600.0, 1.0));
    let Selection { profiles, .. } = selection(&p, node);
    assert_eq!(sorted(profiles), sorted(halves), "the revolve follows the parts as an extrude does");
}

#[test]
fn a_sweep_over_a_subdivided_region_holds_both_parts() {
    let Scene { mut p, si, node, .. } = scene(|p, over| {
        let psi = p.new_sketch("path");
        let psid = p.sketches[psi].id;
        p.add_sketch_node(psid, "path");
        p.add_line_entity(psi, 0.0, -50.0, 0.0, 50.0, Purpose::Real);
        p.add_sweep(over.sketch, vec![over.region], psid, 0)
    });
    p.add_line_entity(si, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let halves = regions_of_area(&p, si, (600.0, 1.0));
    let Selection { profiles, .. } = selection(&p, node);
    assert_eq!(sorted(profiles), sorted(halves), "the sweep holds the parts in its selection");
}

#[test]
fn a_boolean_cut_over_a_subdivided_region_holds_both_parts() {
    let Scene { mut p, si, node, .. } = scene(|p, over| {
        let src = p.add_box(100.0, 100.0, 20.0);
        p.add_combine_multi_op(src, over.sketch, vec![over.region], CombineSpan { height: 10.0, down: 0.0, extent: Extent::default(), fill: &[] }, 0)
    });
    p.add_line_entity(si, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let halves = regions_of_area(&p, si, (600.0, 1.0));
    let Selection { profiles, fill } = selection(&p, node);
    assert_eq!(sorted(profiles), sorted(halves), "the cut keeps its shape: the tool follows the parts");
    assert!(fill.is_empty());
}

// A loft section is a single contour, and a re-tiled area cannot be folded back into one reference: the
// section keeps following its contour. Today's behaviour, asserted so it is a decision, not an accident.
#[test]
fn a_loft_section_keeps_following_its_one_contour() {
    let mut p = Project::default();
    p.new_document();
    let si1 = p.new_sketch("one");
    let sid1 = p.sketches[si1].id;
    p.add_sketch_node(sid1, "one");
    p.add_rect_entity(si1, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    let si2 = p.new_sketch("two");
    let sid2 = p.sketches[si2].id;
    p.add_sketch_node(sid2, "two");
    p.add_rect_entity(si2, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    p.regen_sketch(si1);
    p.regen_sketch(si2);
    let c1 = one_region_of_area(&p, si1, (1200.0, 1.0));
    let c2 = one_region_of_area(&p, si2, (1200.0, 1.0));
    let node = p.add_loft(vec![sid1, sid2], vec![c1, c2], false, 0, 1, false);
    p.add_line_entity(si1, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let Selection { profiles: contours, .. } = selection(&p, node);
    assert_eq!(contours, vec![c1, c2], "the loft's sections are not rewritten: one contour each");
    assert!(p.contour_index(c1).is_some(), "the section contour stays live - the loft keeps following it");
}
