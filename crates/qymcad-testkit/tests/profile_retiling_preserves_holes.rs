use qymcad_core::feature::{Purpose, Reach};
use qymcad_core::geom::Point2;
use qymcad_core::model::Project;

struct Scene {
    project: Project,
    sketch: usize,
    body: u64,
}

struct Case {
    name: &'static str,
    build: fn() -> Scene,
    edit: fn(&mut Scene),
    expected_volume: f64,
}

fn extrude_selected(mut project: Project, sketch: usize) -> Scene {
    let sketch_id = project.sketches[sketch].id;
    project.add_sketch_node(sketch_id, "Profile");
    let profile = project.sketches[sketch]
        .contour_ids
        .iter()
        .copied()
        .filter(|id| project.contour_profile_xy(*id).is_some())
        .max_by(|a, b| {
            let area = |id| project.contours[project.contour_index(id).expect("the contour exists")].area();
            area(*a).total_cmp(&area(*b))
        })
        .expect("a closed profile exists");
    let feature = project.add_extrude_multi(sketch_id, vec![profile], 10.0, Reach::Forward, 0.0, vec![]);
    let body = project.finish_base_body(feature, 1);
    Scene { project, sketch, body }
}

fn ring() -> Scene {
    let mut project = Project::default();
    project.new_document();
    let sketch = project.new_sketch("Ring");
    project.batch(|project| {
        project.add_circle_entity(sketch, 0.0, 0.0, 10.0, Purpose::Real);
        project.add_circle_entity(sketch, 0.0, 0.0, 5.0, Purpose::Real);
    });
    extrude_selected(project, sketch)
}

fn concave_u() -> Scene {
    let mut project = Project::default();
    project.new_document();
    let corners = [(0.0, 0.0), (30.0, 0.0), (30.0, 30.0), (20.0, 30.0), (20.0, 10.0), (10.0, 10.0), (10.0, 30.0), (0.0, 30.0)];
    let sketch_id = project.add_line_sketch("U", corners.into_iter().map(|(x, y)| Point2::new(x, y)).collect(), true);
    let sketch = project.sketch_index(sketch_id).expect("the U sketch exists");
    extrude_selected(project, sketch)
}

fn rectangle() -> Scene {
    let mut project = Project::default();
    project.new_document();
    let sketch = project.new_sketch("Rectangle");
    project.add_rect_entity(sketch, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    extrude_selected(project, sketch)
}

fn rectangle_with_hole() -> Scene {
    let mut project = Project::default();
    project.new_document();
    let sketch = project.new_sketch("Rectangle with hole");
    project.batch(|project| {
        project.add_rect_entity(sketch, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
        project.add_circle_entity(sketch, 20.0, 15.0, 5.0, Purpose::Real);
    });
    extrude_selected(project, sketch)
}

fn split_ring(scene: &mut Scene) {
    scene.project.add_line_entity(scene.sketch, -10.0, 0.0, 10.0, 0.0, Purpose::Real);
}

fn split_u(scene: &mut Scene) {
    scene.project.add_line_entity(scene.sketch, 0.0, 5.0, 30.0, 5.0, Purpose::Real);
}

fn split_existing_hole(scene: &mut Scene) {
    scene.project.add_line_entity(scene.sketch, 15.0, 15.0, 25.0, 15.0, Purpose::Real);
}

fn paste_nested_circles(scene: &mut Scene) {
    let mut donor = Project::default();
    donor.new_document();
    let sketch = donor.new_sketch("Circles");
    let outer = donor.add_circle_entity(sketch, 0.0, 0.0, 5.0, Purpose::Real);
    let inner = donor.add_circle_entity(sketch, 0.0, 0.0, 2.0, Purpose::Real);
    let clip = donor.copy_sketch_geometry(sketch, &[outer, inner], 0.0, 0.0);
    let pasted = scene.project.paste_sketch_geometry(scene.sketch, &clip, 20.0, 15.0);
    assert_eq!(pasted.len(), 2, "both concentric circles are pasted together");
}

fn batch_nested_circles(scene: &mut Scene) {
    scene.project.batch(|project| {
        project.add_circle_entity(scene.sketch, 20.0, 15.0, 5.0, Purpose::Real);
        project.add_circle_entity(scene.sketch, 20.0, 15.0, 2.0, Purpose::Real);
    });
}

fn check_volume(scene: &mut Scene, case: &Case, stage: &str, failures: &mut Vec<String>) {
    let sketch_id = scene.project.sketches[scene.sketch].id;
    scene.project.mark_sketch_dirty(sketch_id);
    let (report, shapes) = qymcad_testkit::regenerate(&mut scene.project);
    for error in &report.errors {
        failures.push(format!("{} / {stage}: regeneration error {}: {}", case.name, error.0, error.1));
    }
    let Some(shape) = shapes.get(&scene.body) else {
        failures.push(format!("{} / {stage}: the extrusion has no OCCT shape", case.name));
        return;
    };
    let volume = shape.volume();
    if !volume.is_finite() || (volume - case.expected_volume).abs() > 0.01 {
        failures.push(format!("{} / {stage}: volume {volume:.6} mm^3, expected {:.6} mm^3", case.name, case.expected_volume));
    }
}

#[test]
fn an_extrusion_keeps_its_selected_material_through_retiling() {
    let cases = [
        Case { name: "a diameter divides a ring without filling its hole", build: ring, edit: split_ring, expected_volume: std::f64::consts::PI * (100.0 - 25.0) * 10.0 },
        Case { name: "a line divides a concave U without losing either arm", build: concave_u, edit: split_u, expected_volume: 7000.0 },
        Case { name: "two circles pasted into a solid stay filled", build: rectangle, edit: paste_nested_circles, expected_volume: 12000.0 },
        Case { name: "two circles batched into a solid stay filled", build: rectangle, edit: batch_nested_circles, expected_volume: 12000.0 },
        Case {
            name: "a diameter divides an existing hole without filling either half",
            build: rectangle_with_hole,
            edit: split_existing_hole,
            expected_volume: 12000.0 - std::f64::consts::PI * 25.0 * 10.0,
        },
    ];
    let mut failures = Vec::new();
    for case in &cases {
        let mut scene = (case.build)();
        check_volume(&mut scene, case, "before the edit", &mut failures);
        (case.edit)(&mut scene);
        check_volume(&mut scene, case, "after the edit", &mut failures);
    }
    assert!(failures.is_empty(), "profile retiling failures ({}):\n{}", failures.len(), failures.join("\n"));
}
