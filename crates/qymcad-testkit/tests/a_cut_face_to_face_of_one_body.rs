//! A CUT FROM FACE TO FACE OF ONE BODY: the tool spans exactly the band between the two picked faces,
//! wherever the sketch plane itself sits - on the start face, inside the band, above it - and each end
//! lands on a face of the body as a one-sided cut's does: no cap is left over a coincident end, so a
//! band across the whole height parts the block.
use qymcad_core::feature::{Extent, FaceRef, Reach, SketchPlane, Stop};
use qymcad_core::model::{CombineSpan, Project, WorkPlane};
use qymcad_core::refs::{Fingerprint, Ref};

/// A CUT BAND LAID ON A BLOCK: the project it lives in and the cut's output body.
struct Band {
    p: Project,
    cut: u64,
}

/// A 40x30x10 block (base z = 0) with a 4-wide slot across it (crossing both long sides) cut from its
/// bottom face (z = 0) to its top face (z = 10), the sketch plane standing at `z`.
fn band_cut(z: f64) -> Band {
    let mut p = Project::default();
    p.new_document();
    let block = p.add_box(40.0, 30.0, 10.0);
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "the block did not build: {:?}", report.errors);
    let faces = p.regen_faces.get(&block).expect("the block names its faces").clone();
    let pick = |at_z: f64| -> FaceRef {
        let f = faces.iter().find(|f| (f.centroid.z - at_z).abs() < 1e-6).unwrap_or_else(|| panic!("no face at z={at_z}: {:?}", faces.iter().map(|f| f.centroid.z).collect::<Vec<_>>()));
        FaceRef { body: block, face: Ref::one(f.id, Fingerprint { centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal }) }
    };
    let si = p.new_sketch("Tool");
    let sid = p.sketches[si].id;
    if z != 0.0 {
        let pl = p.add_plane(WorkPlane { id: 0, name: "band".into(), origin: [0.0, 0.0, z], normal: [0.0, 0.0, 1.0], rot_deg: 0.0, tilt: Default::default(), def: Default::default() });
        p.sketches[si].plane = SketchPlane::Datum(pl);
    }
    p.add_sketch_node(sid, "Tool");
    p.add_rect_entity(si, 3.0, -20.0, 7.0, 20.0, qymcad_core::feature::Purpose::Real);
    p.regen_sketch(si);
    let c: Vec<u64> = p.sketches[si].contour_ids.iter().copied().filter(|c| p.contour_profile_xy(*c).is_some()).collect();
    let cut =
        p.add_combine_multi_op(block, sid, c, CombineSpan { height: 0.0, down: 0.0, extent: Extent { stop: Stop::UpToFace(pick(10.0)), reach: Reach::Forward }, from: Some(pick(0.0)), fill: &[] }, 0);
    Band { p, cut }
}

/// The slot runs from face to face across the whole height, so it parts the block; the pieces are the
/// same wherever the sketch plane stood.
#[test]
fn a_band_across_the_whole_height_parts_the_block_wherever_the_plane_sits() {
    for (z, d_stop) in [(0.0, 10.0), (5.0, 5.0), (20.0, -10.0)] {
        let Band { mut p, cut } = band_cut(z);
        let (report, shapes) = qymcad_testkit::regenerate(&mut p);
        assert!(report.errors.is_empty(), "the cut at plane z={z} did not build: {:?}", report.errors);
        let node = p.timeline.iter().find(|n| n.kind.body() == Some(cut)).expect("the cut node");
        let pieces = node.kind.bodies();
        assert_eq!(pieces.len(), 2, "at plane z={z} the band parts the block in two - an end capped by coincidence would keep it one; the pieces are {pieces:?}");
        let vols: Vec<f64> = pieces.iter().map(|&b| shapes.get(&b).map(|s| s.volume()).unwrap_or(0.0)).collect();
        // the slot is 4 x 30 across the whole 10 of height: 1200 out of the 12000 of the block, 6900 and 3900
        assert!(
            (vols.iter().sum::<f64>() - 10800.0).abs() < 1.0 && (vols[0] - 6900.0).abs() < 1.0 && (vols[1] - 3900.0).abs() < 1.0,
            "at plane z={z} the pieces hold the whole block minus the slot (d_stop={d_stop}): {vols:?}"
        );
    }
}
