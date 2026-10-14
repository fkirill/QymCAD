//! The extent of a cut or a pad set by faces: a stop face, a start face, both remembered in the node and
//! re-read at every rebuild. The mock places a face of an extruded body at the height of its extrusion
//! (one vertex, normal +Z), which is what these tests measure against.

use super::*;

// The extent of a cut or a pad set by faces: a stop face, a start face, both remembered in the node and
// re-read at every rebuild. The mock places a face of an extruded body at the height of its extrusion
// (one vertex, normal +Z), which is what these tests measure against.

/// A cut whose stop is a face: the tool spans from the sketch plane to the face.
#[test]
fn a_cut_to_a_face_takes_its_distance() {
    use qymcad_core::feature::{Extent, FaceRef, Reach, Stop};
    use qymcad_core::model::CombineSpan;
    use qymcad_core::refs::{Fingerprint, Ref};
    let mut p = part_project();
    let sid = square(&mut p, "s");
    p.add_sketch_node(sid, "s");
    let plate = p.add_extrude(sid, 10.0);
    let k = MockKernel::default();
    p.regenerate(&k);
    // a face of the plate parallel to the sketch plane, six units along the normal
    p.regen_faces.insert(plate, vec![MeshFace { triangles: vec![], normal: [0.0, 0.0, 1.0], centroid: Point3::new(0.0, 0.0, 6.0), area: 4.0, id: 7 }]);
    let cut = p.add_combine_on(
        plate,
        sid,
        0,
        CombineSpan {
            height: 0.0,
            down: 0.0,
            extent: Extent { stop: Stop::UpToFace(FaceRef { body: plate, face: Ref::one(7, Fingerprint { centroid: [0.0, 0.0, 6.0], normal: [0.0, 0.0, 1.0] }) }), reach: Reach::Forward },
            from: None,
            fill: &[],
        },
        0,
    );
    k.spans.borrow_mut().clear();
    p.regenerate(&k);
    assert!(!p.regen_errors.contains_key(&cut), "the cut builds; the error is {:?}", p.regen_errors.get(&cut));
    let span = k.spans.borrow().last().copied().expect("the tool ran");
    assert!((span.start - 0.0).abs() < 1e-9 && (span.length - 6.0).abs() < 1e-6, "the tool spans from the plane to the face: start={} length={}", span.start, span.length);
}

/// A start face and a stop face: the tool spans exactly the band between them.
#[test]
fn a_cut_between_two_faces_spans_the_band() {
    use qymcad_core::feature::{Extent, FaceRef, Reach, Stop};
    use qymcad_core::model::CombineSpan;
    use qymcad_core::refs::{Fingerprint, Ref};
    let mut p = part_project();
    let sid = square(&mut p, "s");
    p.add_sketch_node(sid, "s");
    let plate = p.add_extrude(sid, 10.0);
    let k = MockKernel::default();
    p.regenerate(&k);
    p.regen_faces.insert(
        plate,
        vec![
            MeshFace { triangles: vec![], normal: [0.0, 0.0, 1.0], centroid: Point3::new(0.0, 0.0, 6.0), area: 4.0, id: 7 },
            MeshFace { triangles: vec![], normal: [0.0, 0.0, 1.0], centroid: Point3::new(0.0, 0.0, 2.0), area: 4.0, id: 8 },
        ],
    );
    let cut = p.add_combine_on(
        plate,
        sid,
        0,
        CombineSpan {
            height: 0.0,
            down: 0.0,
            extent: Extent { stop: Stop::UpToFace(FaceRef { body: plate, face: Ref::one(7, Fingerprint { centroid: [0.0, 0.0, 6.0], normal: [0.0, 0.0, 1.0] }) }), reach: Reach::Forward },
            from: Some(FaceRef { body: plate, face: Ref::one(8, Fingerprint { centroid: [0.0, 0.0, 2.0], normal: [0.0, 0.0, 1.0] }) }),
            fill: &[],
        },
        0,
    );
    k.spans.borrow_mut().clear();
    p.regenerate(&k);
    assert!(!p.regen_errors.contains_key(&cut), "the cut builds; the error is {:?}", p.regen_errors.get(&cut));
    let span = k.spans.borrow().last().copied().expect("the tool ran");
    assert!((span.start - 2.0).abs() < 1e-6 && (span.length - 4.0).abs() < 1e-6, "the tool spans the band between the faces: start={} length={}", span.start, span.length);
}

/// The distance follows the face: a taller body moves its face, and the cut rebuilds to the new place
/// even though the body it cuts did not change at all.
#[test]
fn a_cut_follows_a_face_of_another_body() {
    use qymcad_core::feature::{Extent, FaceRef, FeatureKind, Reach, Stop};
    use qymcad_core::model::CombineSpan;
    use qymcad_core::refs::{Fingerprint, Ref};
    let mut p = part_project();
    let sid1 = square(&mut p, "a");
    p.add_sketch_node(sid1, "a");
    let target = p.add_extrude(sid1, 10.0); // the body the cut works on
    let sid2 = square(&mut p, "b");
    p.add_sketch_node(sid2, "b");
    let other = p.add_extrude(sid2, 6.0); // holds the reached face: its top stands at 6
    let k = MockKernel::default();
    p.regenerate(&k);
    let cut = p.add_combine_on(
        target,
        sid1,
        0,
        CombineSpan {
            height: 0.0,
            down: 0.0,
            extent: Extent { stop: Stop::UpToFace(FaceRef { body: other, face: Ref::one(1, Fingerprint { centroid: [0.0, 0.0, 6.0], normal: [0.0, 0.0, 1.0] }) }), reach: Reach::Forward },
            from: None,
            fill: &[],
        },
        0,
    );
    p.regenerate(&k);
    assert!(!p.regen_errors.contains_key(&cut), "the cut builds; the error is {:?}", p.regen_errors.get(&cut));
    assert!((k.spans.borrow().last().unwrap().length - 6.0).abs() < 1e-6, "the tool first reaches the face at 6");
    // the other body grows: its face moves to 8, and the cut follows although its own src did not change
    if let Some(node) = p.timeline.iter_mut().find(|n| matches!(&n.kind, FeatureKind::Extrude { body, .. } if *body == other)) {
        if let FeatureKind::Extrude { height, .. } = &mut node.kind {
            *height = 8.0;
        }
        node.dirty = true;
    }
    k.spans.borrow_mut().clear();
    p.regenerate(&k);
    assert!((k.spans.borrow().last().unwrap().length - 8.0).abs() < 1e-6, "the cut rebuilds to the face at its new place: spans={:?}", k.spans.borrow());
}

/// A reached face that no longer resolves is a refusal naming the face it sought.
#[test]
fn a_cut_whose_face_is_lost_refuses() {
    use qymcad_core::feature::{Extent, FaceRef, Reach, Stop};
    use qymcad_core::model::CombineSpan;
    use qymcad_core::refs::{Fingerprint, Ref};
    let mut p = part_project();
    let sid = square(&mut p, "s");
    p.add_sketch_node(sid, "s");
    let plate = p.add_extrude(sid, 10.0);
    let k = MockKernel::default();
    p.regenerate(&k);
    let cut = p.add_combine_on(
        plate,
        sid,
        0,
        CombineSpan {
            height: 0.0,
            down: 0.0,
            extent: Extent { stop: Stop::UpToFace(FaceRef { body: plate, face: Ref::one(99, Fingerprint::default()) }), reach: Reach::Forward },
            from: None,
            fill: &[],
        },
        0,
    );
    p.regenerate(&k);
    assert!(p.regen_errors.contains_key(&cut), "a lost stop face refuses rather than cutting blind");
}

/// A reached face that turned out of parallelism is a refusal naming the face.
#[test]
fn a_cut_whose_face_turns_refuses() {
    use qymcad_core::feature::{Extent, FaceRef, Reach, Stop};
    use qymcad_core::model::CombineSpan;
    use qymcad_core::refs::{Fingerprint, Ref};
    let mut p = part_project();
    let sid = square(&mut p, "s");
    p.add_sketch_node(sid, "s");
    let plate = p.add_extrude(sid, 10.0);
    let k = MockKernel::default();
    p.regenerate(&k);
    p.regen_faces.insert(plate, vec![MeshFace { triangles: vec![], normal: [1.0, 0.0, 0.0], centroid: Point3::new(0.0, 0.0, 6.0), area: 4.0, id: 7 }]);
    let cut = p.add_combine_on(
        plate,
        sid,
        0,
        CombineSpan {
            height: 0.0,
            down: 0.0,
            extent: Extent { stop: Stop::UpToFace(FaceRef { body: plate, face: Ref::one(7, Fingerprint { centroid: [0.0, 0.0, 6.0], normal: [0.0, 0.0, 1.0] }) }), reach: Reach::Forward },
            from: None,
            fill: &[],
        },
        0,
    );
    p.regenerate(&k);
    assert!(p.regen_errors.contains_key(&cut), "a stop face no longer parallel to the sketch plane refuses");
}

/// A stop face that an edit moved behind the start face is a refusal.
#[test]
fn a_cut_whose_faces_cross_refuses() {
    use qymcad_core::feature::{Extent, FaceRef, Reach, Stop};
    use qymcad_core::model::CombineSpan;
    use qymcad_core::refs::{Fingerprint, Ref};
    let mut p = part_project();
    let sid = square(&mut p, "s");
    p.add_sketch_node(sid, "s");
    let plate = p.add_extrude(sid, 10.0);
    let k = MockKernel::default();
    p.regenerate(&k);
    p.regen_faces.insert(
        plate,
        vec![
            MeshFace { triangles: vec![], normal: [0.0, 0.0, 1.0], centroid: Point3::new(0.0, 0.0, 2.0), area: 4.0, id: 7 },
            MeshFace { triangles: vec![], normal: [0.0, 0.0, 1.0], centroid: Point3::new(0.0, 0.0, 6.0), area: 4.0, id: 8 },
        ],
    );
    let cut = p.add_combine_on(
        plate,
        sid,
        0,
        CombineSpan {
            height: 0.0,
            down: 0.0,
            extent: Extent { stop: Stop::UpToFace(FaceRef { body: plate, face: Ref::one(7, Fingerprint { centroid: [0.0, 0.0, 2.0], normal: [0.0, 0.0, 1.0] }) }), reach: Reach::Forward },
            from: Some(FaceRef { body: plate, face: Ref::one(8, Fingerprint { centroid: [0.0, 0.0, 6.0], normal: [0.0, 0.0, 1.0] }) }),
            fill: &[],
        },
        0,
    );
    p.regenerate(&k);
    assert!(p.regen_errors.contains_key(&cut), "a stop face behind the start face refuses");
}

/// A band that collapsed to nothing cuts nothing: the node stays well, says the faces met, and hands the
/// source body on unchanged.
#[test]
fn a_cut_whose_faces_met_is_a_visible_no_op() {
    use qymcad_core::feature::{Extent, FaceRef, Reach, Stop};
    use qymcad_core::model::CombineSpan;
    use qymcad_core::refs::{Fingerprint, Ref};
    let mut p = part_project();
    let sid = square(&mut p, "s");
    p.add_sketch_node(sid, "s");
    let plate = p.add_extrude(sid, 10.0);
    let k = MockKernel::default();
    p.regenerate(&k);
    p.regen_faces.insert(plate, vec![MeshFace { triangles: vec![], normal: [0.0, 0.0, 1.0], centroid: Point3::new(0.0, 0.0, 6.0), area: 4.0, id: 7 }]);
    let cut = p.add_combine_on(
        plate,
        sid,
        0,
        CombineSpan {
            height: 0.0,
            down: 0.0,
            extent: Extent { stop: Stop::UpToFace(FaceRef { body: plate, face: Ref::one(7, Fingerprint { centroid: [0.0, 0.0, 6.0], normal: [0.0, 0.0, 1.0] }) }), reach: Reach::Forward },
            from: Some(FaceRef { body: plate, face: Ref::one(7, Fingerprint { centroid: [0.0, 0.0, 6.0], normal: [0.0, 0.0, 1.0] }) }),
            fill: &[],
        },
        0,
    );
    k.spans.borrow_mut().clear();
    p.regenerate(&k);
    assert!(!p.regen_errors.contains_key(&cut), "a met pair is not an error: {:?}", p.regen_errors.get(&cut));
    assert!(p.regen_warnings.contains_key(&cut), "the node says the faces met");
    assert!(k.spans.borrow().is_empty(), "no tool is built for a band of zero");
}

/// "Through all" takes no start face: the pair is refused rather than half-honored.
#[test]
fn a_through_cut_with_a_start_face_refuses() {
    use qymcad_core::feature::{Extent, FaceRef, Reach, Stop};
    use qymcad_core::model::CombineSpan;
    use qymcad_core::refs::{Fingerprint, Ref};
    let mut p = part_project();
    let sid = square(&mut p, "s");
    p.add_sketch_node(sid, "s");
    let plate = p.add_extrude(sid, 10.0);
    let k = MockKernel::default();
    p.regenerate(&k);
    let cut = p.add_combine_on(
        plate,
        sid,
        0,
        CombineSpan {
            height: 0.0,
            down: 0.0,
            extent: Extent { stop: Stop::Through, reach: Reach::Forward },
            from: Some(FaceRef { body: plate, face: Ref::one(1, Fingerprint::default()) }),
            fill: &[],
        },
        0,
    );
    p.regenerate(&k);
    assert!(p.regen_errors.contains_key(&cut), "through all with a start face refuses");
}

/// A cut that grows BACKWARDS (its sketch on the top of the body, say) takes its stop face on the far
/// side of the plane: the length runs the other way, and the band is measured along the growth.
#[test]
fn a_backward_cut_measures_its_stop_along_its_growth() {
    use qymcad_core::feature::{Extent, FaceRef, Reach, Stop};
    use qymcad_core::model::CombineSpan;
    use qymcad_core::refs::{Fingerprint, Ref};
    let mut p = part_project();
    let sid = square(&mut p, "s");
    p.add_sketch_node(sid, "s");
    let plate = p.add_extrude(sid, 10.0);
    let k = MockKernel::default();
    p.regenerate(&k);
    p.regen_faces.insert(
        plate,
        vec![
            MeshFace { triangles: vec![], normal: [0.0, 0.0, 1.0], centroid: Point3::new(0.0, 0.0, 6.0), area: 4.0, id: 7 },
            MeshFace { triangles: vec![], normal: [0.0, 0.0, 1.0], centroid: Point3::new(0.0, 0.0, 2.0), area: 4.0, id: 8 },
        ],
    );
    let cut = p.add_combine_on(
        plate,
        sid,
        0,
        CombineSpan {
            height: 0.0,
            down: 0.0,
            extent: Extent { stop: Stop::UpToFace(FaceRef { body: plate, face: Ref::one(8, Fingerprint { centroid: [0.0, 0.0, 2.0], normal: [0.0, 0.0, 1.0] }) }), reach: Reach::Backward },
            from: Some(FaceRef { body: plate, face: Ref::one(7, Fingerprint { centroid: [0.0, 0.0, 6.0], normal: [0.0, 0.0, 1.0] }) }),
            fill: &[],
        },
        0,
    );
    k.spans.borrow_mut().clear();
    p.regenerate(&k);
    assert!(!p.regen_errors.contains_key(&cut), "the backward cut builds; the error is {:?}", p.regen_errors.get(&cut));
    let span = k.spans.borrow().last().copied().expect("the tool ran");
    assert!((span.start - 2.0).abs() < 1e-6 && (span.length - 4.0).abs() < 1e-6, "the tool sweeps from the stop face up to the start face: start={} length={}", span.start, span.length);
}
