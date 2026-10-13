//! A stored profile id that resolves to no region any more is never silent: the node carries a lost-profile
//! warning beside today's fallback behaviour, and the warning clears the moment the selection resolves
//! again. A loft section is the same loss beside the refusal the loft already gives.

use qymcad_core::errors::CoreError;
use qymcad_core::feature::{FeatureKind, Purpose, Reach, SketchPlane};
use qymcad_core::model::{Project, WorkPlane};

/// A scene standing: the project, the sketch's index and id, both region ids, and the node over the first.
struct Scene {
    p: Project,
    si: usize,
    sid: u64,
    a: u64,
    b: u64,
    node: u64,
}

/// A part with a rectangle A (selected by an extrude) and a smaller rectangle B beside it, so a fallback
/// region stands when A is deleted. Returns the project, the sketch place, both region ids, the node.
fn scene() -> Scene {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("s");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "s");
    p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    p.add_rect_entity(si, 100.0, 0.0, 130.0, 20.0, Purpose::Real);
    p.regen_sketch(si);
    let by_x = |p: &Project, cid: u64| p.contours[p.contour_index(cid).unwrap()].centroid().x;
    let both: Vec<u64> = p.sketches[si].contour_ids.iter().copied().filter(|c| p.contour_profile_xy(*c).is_some()).collect();
    let (a, b) = if by_x(&p, both[0]) < by_x(&p, both[1]) { (both[0], both[1]) } else { (both[1], both[0]) };
    let node = p.add_extrude_multi(sid, vec![a], 10.0, Reach::Forward, 0.0, vec![]);
    Scene { p, si, sid, a, b, node }
}

/// The boundary of the selected region deleted: the region opens and vanishes, the node carries the
/// lost-profile warning, and the body keeps today's fallback - the sketch's first closed region.
#[test]
fn a_lost_profile_is_reported_and_the_fallback_stands() {
    let Scene { mut p, si, sid, a, b, node } = scene();
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "the start builds: {}", report.errors.iter().map(|(id, e)| format!("{id}: {e}")).collect::<Vec<_>>().join("; "));
    assert!(!p.regen_warnings.contains_key(&node), "nothing is lost at the start");
    // every side of the selected rectangle goes: the region opens and is gone
    let gone: Vec<u64> = p.sketches[si].entities.iter().map(|e| e.id).take(4).collect();
    p.delete_entities(si, &gone);
    p.mark_sketch_dirty(sid);
    let (report, shapes) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "the fallback builds the region that stands: {}", report.errors.iter().map(|(id, e)| format!("{id}: {e}")).collect::<Vec<_>>().join("; "));
    match p.regen_warnings.get(&node) {
        Some(CoreError::ProfilesDropped { asked, dropped }) => {
            assert_eq!((*asked, *dropped), (1, 1), "one stored contour was asked for and lost, region {a} being dead");
        }
        other => panic!("the node carries a lost-profile warning, found {other:?}"),
    }
    // today's fallback: the sketch's first closed region is used - the body is B's, not nothing
    let v = shapes.get(&node).map(|s| s.volume()).unwrap_or(0.0);
    assert!((v - 30.0 * 20.0 * 10.0).abs() < 5.0, "the fallback extrudes the region that stands (30 x 20 by 10), got V={v:.1}");
    // a live region picked again: the warning is gone
    for n in &mut p.timeline {
        if n.id == node {
            if let FeatureKind::Extrude { profiles, .. } = &mut n.kind {
                *profiles = vec![b];
            }
        }
    }
    p.mark_sketch_dirty(sid);
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "the repair builds: {}", report.errors.iter().map(|(id, e)| format!("{id}: {e}")).collect::<Vec<_>>().join("; "));
    assert!(!p.regen_warnings.contains_key(&node), "the warning clears once the selection resolves again");
}

/// A loft section that no longer resolves is a lost profile beside the refusal the loft already gives: the
/// section reference holds one contour, which no cover can widen.
#[test]
fn a_lost_loft_section_is_reported_beside_the_refusal() {
    let mut p = Project::default();
    p.new_document();
    let si1 = p.new_sketch("one");
    let sid1 = p.sketches[si1].id;
    p.add_sketch_node(sid1, "one");
    p.add_rect_entity(si1, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    let si2 = p.new_sketch("two");
    let sid2 = p.sketches[si2].id;
    // the upper section on a datum plane at z = 10, so the loft has height to stand on
    let pl = p.add_plane(WorkPlane { id: 0, name: "z10".into(), origin: [0.0, 0.0, 10.0], normal: [0.0, 0.0, 1.0], rot_deg: 0.0, tilt: Default::default(), def: Default::default() });
    p.sketches[si2].plane = SketchPlane::Datum(pl);
    p.add_sketch_node(sid2, "two");
    p.add_rect_entity(si2, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    p.regen_sketch(si1);
    p.regen_sketch(si2);
    let c1 = p.sketches[si1].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).unwrap();
    let c2 = p.sketches[si2].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).unwrap();
    let node = p.add_loft(vec![sid1, sid2], vec![c1, c2], false, 0, 1, false);
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "the loft builds: {}", report.errors.iter().map(|(id, e)| format!("{id}: {e}")).collect::<Vec<_>>().join("; "));
    // the first section's rectangle loses its sides: its contour no longer resolves
    let gone: Vec<u64> = p.sketches[si1].entities.iter().map(|e| e.id).take(4).collect();
    p.delete_entities(si1, &gone);
    p.mark_sketch_dirty(sid1);
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.iter().any(|(id, _)| *id == node), "the loft refuses to build without its section");
    match p.regen_warnings.get(&node) {
        Some(CoreError::ProfilesDropped { asked, dropped }) => {
            assert_eq!((*asked, *dropped), (2, 1), "two sections were named, the first one is lost");
        }
        other => panic!("the node carries a lost-profile warning beside the refusal, found {other:?}"),
    }
}
