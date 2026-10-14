use qymcad_core::feature::{FeatureKind, Purpose, Reach};
use qymcad_core::geom::Point2;
use qymcad_core::model::{Id, Project};

struct Scene {
    project: Project,
    sketch: usize,
    sketch_id: Id,
}

impl Scene {
    fn new() -> Self {
        let mut project = Project::default();
        project.new_document();
        let sketch = project.new_sketch("profile");
        let sketch_id = project.sketches[sketch].id;
        project.add_sketch_node(sketch_id, "profile");
        Self { project, sketch, sketch_id }
    }

    fn regions(&self) -> Vec<Id> {
        self.project.sketches[self.sketch].contour_ids.iter().copied().filter(|id| self.project.contour_profile_xy(*id).is_some()).collect()
    }

    fn extrude(&mut self, profiles: Vec<Id>) -> Id {
        self.project.add_extrude_multi(self.sketch_id, profiles, 10.0, Reach::Forward, 0.0, vec![])
    }

    fn area(&self, node: Id) -> f64 {
        let selected = selection(&self.project, node);
        let area = |id: Id| self.project.contours[self.project.contour_index(id).expect("live contour")].area();
        self.project.profile_faces(self.sketch_id, &selected.profiles, &selected.fill).into_iter().map(|(outer, holes)| area(outer) - holes.iter().copied().map(area).sum::<f64>()).sum()
    }
}

#[derive(Debug, PartialEq)]
struct Selection {
    profiles: Vec<Id>,
    fill: Vec<Id>,
}

fn selection(project: &Project, node: Id) -> Selection {
    let kind = &project.timeline.iter().find(|n| n.id == node).expect("extrude node").kind;
    let FeatureKind::Extrude { profiles, fill, .. } = kind else { panic!("extrude expected") };
    Selection { profiles: profiles.clone(), fill: fill.clone() }
}

#[test]
fn dividing_a_ring_keeps_the_unselected_hole() {
    let mut scene = Scene::new();
    scene.project.add_circle_entity(scene.sketch, 0.0, 0.0, 10.0, Purpose::Real);
    scene.project.add_circle_entity(scene.sketch, 0.0, 0.0, 5.0, Purpose::Real);
    let outer = scene
        .regions()
        .into_iter()
        .max_by(|a, b| scene.project.contours[scene.project.contour_index(*a).unwrap()].area().total_cmp(&scene.project.contours[scene.project.contour_index(*b).unwrap()].area()))
        .expect("outer circle");
    let node = scene.extrude(vec![outer]);
    let before = scene.area(node);
    scene.project.add_line_entity(scene.sketch, -20.0, 0.0, 20.0, 0.0, Purpose::Real);
    assert_eq!(selection(&scene.project, node).profiles.len(), 2, "only the two half rings are selected");
    assert!((scene.area(node) - before).abs() < 1e-6, "the hole stays empty");
}

#[test]
fn dividing_only_a_hole_keeps_both_halves_unselected() {
    let mut scene = Scene::new();
    scene.project.add_rect_entity(scene.sketch, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    scene.project.add_circle_entity(scene.sketch, 20.0, 15.0, 5.0, Purpose::Real);
    let outer = scene
        .regions()
        .into_iter()
        .max_by(|a, b| scene.project.contours[scene.project.contour_index(*a).unwrap()].area().total_cmp(&scene.project.contours[scene.project.contour_index(*b).unwrap()].area()))
        .expect("outer rectangle");
    let node = scene.extrude(vec![outer]);
    let before = scene.area(node);
    scene.project.add_line_entity(scene.sketch, 15.0, 15.0, 25.0, 15.0, Purpose::Real);
    assert!(selection(&scene.project, node).fill.is_empty(), "a tile of an existing hole is not new filled material");
    assert!((scene.area(node) - before).abs() < 1e-6, "both halves of the hole stay empty");
}

#[test]
fn merging_regions_uses_each_features_own_selection() {
    let mut scene = Scene::new();
    scene.project.add_rect_entity(scene.sketch, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    let line = scene.project.add_line_entity(scene.sketch, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let halves = scene.regions();
    assert_eq!(halves.len(), 2);
    let first = scene.extrude(vec![halves[0]]);
    let second = scene.extrude(vec![halves[1]]);
    let whole = scene.extrude(halves);
    let first_selection = selection(&scene.project, first);
    let second_selection = selection(&scene.project, second);
    scene.project.delete_entities(scene.sketch, &[line]);
    assert_eq!(selection(&scene.project, first), first_selection, "a partial merge is not rewritten");
    assert_eq!(selection(&scene.project, second), second_selection, "another feature cannot complete this selection");
    assert_eq!(selection(&scene.project, whole).profiles.len(), 1, "the complete selection follows the merge");
}

#[test]
fn dividing_a_concave_profile_keeps_every_tile() {
    let mut scene = Scene::new();
    scene.project.fill_sketch_polyline(
        scene.sketch,
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(30.0, 0.0),
            Point2::new(30.0, 30.0),
            Point2::new(20.0, 30.0),
            Point2::new(20.0, 10.0),
            Point2::new(10.0, 10.0),
            Point2::new(10.0, 30.0),
            Point2::new(0.0, 30.0),
        ],
        true,
    );
    let node = scene.extrude(scene.regions());
    let before = scene.area(node);
    scene.project.add_line_entity(scene.sketch, 0.0, 5.0, 30.0, 5.0, Purpose::Real);
    assert_eq!(selection(&scene.project, node).profiles.len(), 2, "both concave and convex tiles are selected");
    assert!((scene.area(node) - before).abs() < 1e-6, "the concave profile keeps its area");
}

#[test]
fn pasted_nested_regions_are_all_filled() {
    let mut scene = Scene::new();
    scene.project.add_rect_entity(scene.sketch, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    let node = scene.extrude(scene.regions());
    let source = scene.project.new_sketch("copy");
    let outer = scene.project.add_circle_entity(source, 0.0, 0.0, 5.0, Purpose::Real);
    let inner = scene.project.add_circle_entity(source, 0.0, 0.0, 2.0, Purpose::Real);
    let clip = scene.project.copy_sketch_geometry(source, &[inner, outer], 0.0, 0.0);
    scene.project.paste_sketch_geometry(scene.sketch, &clip, 20.0, 15.0);
    assert_eq!(selection(&scene.project, node).fill.len(), 2, "fresh descendants are filled at every depth");
    assert!((scene.area(node) - 1200.0).abs() < 1e-6, "the pasted contours introduce no hole");
}

#[test]
fn fresh_regions_can_span_several_old_selected_faces() {
    let mut scene = Scene::new();
    scene.project.add_rect_entity(scene.sketch, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    let line = scene.project.add_line_entity(scene.sketch, 20.0, 0.0, 20.0, 30.0, Purpose::Real);
    let node = scene.extrude(scene.regions());
    scene.project.batch(|project| {
        project.delete_entities(scene.sketch, &[line]);
        project.add_circle_entity(scene.sketch, 20.0, 15.0, 5.0, Purpose::Real);
        project.add_circle_entity(scene.sketch, 20.0, 15.0, 2.0, Purpose::Real);
    });
    assert!((scene.area(node) - 1200.0).abs() < 1e-6, "fresh material belongs to the union of the old selected faces");
}

#[test]
fn a_fresh_filled_parent_keeps_its_existing_unselected_child() {
    let mut scene = Scene::new();
    scene.project.add_rect_entity(scene.sketch, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    scene.project.add_circle_entity(scene.sketch, 20.0, 15.0, 2.0, Purpose::Real);
    let outer = scene
        .regions()
        .into_iter()
        .max_by(|a, b| scene.project.contours[scene.project.contour_index(*a).unwrap()].area().total_cmp(&scene.project.contours[scene.project.contour_index(*b).unwrap()].area()))
        .expect("outer rectangle");
    let node = scene.extrude(vec![outer]);
    let before = scene.area(node);
    scene.project.batch(|project| {
        project.add_circle_entity(scene.sketch, 20.0, 15.0, 5.0, Purpose::Real);
        project.add_line_entity(scene.sketch, 10.0, 0.0, 10.0, 30.0, Purpose::Real);
    });
    assert_eq!(selection(&scene.project, node).profiles.len(), 2, "both halves surrounding the old hole stay selected");
    assert!((scene.area(node) - before).abs() < 1e-6, "filling a fresh parent preserves its existing unselected child");
}
