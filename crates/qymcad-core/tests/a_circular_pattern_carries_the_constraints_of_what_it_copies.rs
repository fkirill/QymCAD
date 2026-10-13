//! A CIRCULAR PATTERN CARRIES THE CONSTRAINTS OF WHAT IT COPIES: a copy keeps a constraint when it holds of
//! the copy as laid out - an end that stays on the same circle keeps its tie to it, a junction coincidence
//! is kept when the copy's end lands on the neighbour's end, a turned copy keeps its level only when it lies
//! level again - and nothing is cloned that does not hold: no copy is dragged, no clone references the
//! source elements, no clone repeats what the entities already say about themselves.
use qymcad_core::feature::{Purpose, Winding};
use qymcad_core::geom::Point2;
use qymcad_core::model::{constraint_point_ids, Constraint, EntityKind, Id, PatternKind, Project};

fn new_sketch() -> (Project, usize) {
    let mut p = Project::default();
    p.new_document();
    let sid = p.add_sketch("t", vec![], None);
    p.add_sketch_node(sid, "Sketch");
    let si = p.sketch_index(sid).unwrap();
    (p, si)
}

/// The arc laid by `add_arc_entity`, which answers nothing: the one entity the call added.
fn add_arc(p: &mut Project, si: usize, c: Point2, a: Point2, b: Point2) -> Id {
    let before: std::collections::HashSet<Id> = p.sketches[si].entities.iter().map(|e| e.id).collect();
    p.add_arc_entity(si, c, a, b, Winding::Ccw, Purpose::Real);
    p.sketches[si].entities.iter().find(|e| !before.contains(&e.id) && matches!(e.kind, EntityKind::Arc { .. })).map(|e| e.id).unwrap()
}

/// The endpoints of arc `eid`, in the order the arc holds them.
fn arc_ends(p: &Project, si: usize, eid: Id) -> (Id, Id) {
    match p.sketches[si].entity(eid).map(|e| e.kind) {
        Some(EntityKind::Arc { a, b, .. }) => (a, b),
        _ => panic!("no arc {eid}"),
    }
}

/// The ends of line entity `eid`, in the order the line holds them.
fn line_ends(p: &Project, si: usize, eid: Id) -> (Id, Id) {
    match p.sketches[si].entity(eid).map(|e| e.kind) {
        Some(EntityKind::Line { a, b }) => (a, b),
        _ => panic!("no line {eid}"),
    }
}

/// The constraints of the kinds `want`, added after place `from` in the list.
fn new_of(p: &Project, si: usize, from: usize, want: fn(&Constraint) -> bool) -> Vec<&Constraint> {
    p.sketches[si].constraints[from..].iter().filter(|c| want(c)).collect()
}

fn is_coincident(c: &Constraint) -> bool {
    matches!(c, Constraint::Coincident { .. })
}

fn is_point_on_circle(c: &Constraint) -> bool {
    matches!(c, Constraint::PointOnCircle { .. })
}

fn is_horizontal(c: &Constraint) -> bool {
    matches!(c, Constraint::Horizontal { .. })
}

/// TWO ARCS OF ONE CIRCLE: the first from 0 to 30 degrees, the second from 31 to 60 - their nearest ends a
/// degree apart, then tied by a coincidence, as the constraint button ties ends drawn apart. The junction
/// settles on the circle between the two. Answered with the first arc's id.
fn tied_arcs(p: &mut Project, si: usize) -> Id {
    let first = add_arc(p, si, Point2::new(0.0, 0.0), Point2::new(20.0, 0.0), Point2::new(17.320508, 10.0));
    let second = add_arc(p, si, Point2::new(0.0, 0.0), Point2::new(17.143357, 10.300767), Point2::new(10.0, 17.320508));
    let (end_of_first, _) = arc_ends(p, si, first);
    let (start_of_second, _) = arc_ends(p, si, second);
    p.sketches[si].constraints.push(Constraint::Coincident { a: end_of_first, b: start_of_second });
    p.solve_sketch(si); // the tie holds before the pattern is laid, or no copy can be judged against it
    first
}

/// THE POINTS OF THE SOURCE ELEMENTS, whose ids no clone may reference: a clone points at the copy's own
/// points and at the geometry the copy landed on, never back at what was copied.
fn source_points(p: &Project, si: usize, eids: &[Id]) -> std::collections::HashSet<Id> {
    eids.iter().flat_map(|&e| qymcad_core::model::entity_points(p.sketches[si].entity(e).unwrap())).collect()
}

// THE JUNCTION COINCIDENCE, carried through the quick edit tool: the copy that comes round the full turn to
// the source lands its end on the neighbour's end, and keeps the tie. The copies that land on empty sheet
// carry nothing - their counterpart is not where they landed.
#[test]
fn a_circular_pattern_carries_a_junction_coincidence_through_the_quick_edit_tool() {
    let (mut p, si) = new_sketch();
    let first = tied_arcs(&mut p, si);
    let from = p.sketches[si].constraints.len();
    p.array_circular(si, &[first], 0.0, 0.0, 7, 420.0); // a full turn and one step of 60 degrees: the sixth copy lands on the source
    assert_carried_one_junction_coincidence_and_no_clone_names_the_source(&p, si, first, from);
    assert!(p.sketch_conflicts(si).is_empty(), "the sketch solves with the carried coincidence");
}

// THE SAME through the pattern bar: both doors give the same result.
#[test]
fn a_circular_pattern_carries_a_junction_coincidence_through_the_pattern_bar() {
    let (mut p, si) = new_sketch();
    let first = tied_arcs(&mut p, si);
    let from = p.sketches[si].constraints.len();
    let _ = p.add_pattern(si, &[first], PatternKind::Circular { cx: 0.0, cy: 0.0, count: 7, total_deg: 420.0 });
    assert_carried_one_junction_coincidence_and_no_clone_names_the_source(&p, si, first, from);
    assert!(p.sketch_conflicts(si).is_empty(), "the sketch solves with the carried coincidence");
}

/// One junction coincidence is carried - the copy that comes round the whole turn keeps the tie - and no
/// CLONE names a point of the source elements. The pattern's pins are not clones and are excepted by kind:
/// they speak about the source's anchor pair by design, that being the relation the rotation is; the
/// whole-turn pin ties the copy to its source points by coincidence.
fn assert_carried_one_junction_coincidence_and_no_clone_names_the_source(p: &Project, si: usize, first: Id, from: usize) {
    let source = source_points(p, si, &[first]);
    let carried = p.sketches[si].constraints[from..].iter().filter(|c| is_coincident(c) && !constraint_point_ids(c).iter().any(|id| source.contains(id))).count();
    assert!(carried == 1, "the copy that lands on the neighbour keeps the coincidence; {} clones were carried", carried);
    for c in p.sketches[si].constraints[from..].iter() {
        let pin =
            matches!(c, Constraint::Angle { .. } | Constraint::AngleLines { .. } | Constraint::Equal { .. }) || (is_coincident(c) && constraint_point_ids(c).iter().any(|id| source.contains(id)));
        if pin {
            continue;
        }
        for id in constraint_point_ids(c) {
            assert!(!source.contains(&id), "a clone references the point {id} of the source elements");
        }
    }
}

// A TURNED COPY KEEPS ITS LEVEL ONLY WHERE IT IS LEVEL AGAIN: the copy turned by a half turn lies level and
// carries the horizontal; the copies turned to stand on their side carry nothing, and nothing pulls them
// level.
#[test]
fn a_turned_copy_keeps_its_level_only_where_it_is_level_again() {
    let (mut p, si) = new_sketch();
    let line = p.add_line_entity(si, 30.0, 0.0, 50.0, 0.0, Purpose::Real);
    let (a, b) = match p.sketches[si].entity(line).map(|e| e.kind) {
        Some(EntityKind::Line { a, b }) => (a, b),
        _ => panic!("no line"),
    };
    p.sketches[si].constraints.push(Constraint::Horizontal { a, b });
    let from = p.sketches[si].constraints.len();
    p.array_circular(si, &[line], 0.0, 0.0, 4, 360.0); // quarter turns
    let carried = new_of(&p, si, from, is_horizontal);
    assert!(carried.len() == 1, "only the copy that lies level again carries the horizontal; {} clones were carried", carried.len());
    // the clone holds the copy turned by a half turn: its points stand to the left of the centre, level
    let ids: std::collections::HashSet<Id> = carried.iter().flat_map(|c| constraint_point_ids(c)).collect();
    for id in &ids {
        let q = p.sketches[si].points.iter().find(|q| &q.id == id).unwrap();
        assert!(q.y.abs() < 5e-3 && q.x < 0.0, "the horizontal was carried onto a copy that does not lie level: ({}, {})", q.x, q.y);
    }
    // the copies on their side carry no level: only the pins - the pattern's own relations, not a carried
    // constraint - name their points
    for e in p.sketches[si].entities.iter().skip(1) {
        let (ca, cb) = match e.kind {
            EntityKind::Line { a, b } => (a, b),
            _ => continue,
        };
        if ids.contains(&ca) || ids.contains(&cb) {
            continue;
        }
        for c in p.sketches[si].constraints[from..].iter().filter(|c| is_horizontal(c)) {
            for id in constraint_point_ids(c) {
                assert!(id != ca && id != cb, "a copy turned onto its side carries a horizontal");
            }
        }
    }
}

// A COPY GAINS NO POINT-ON-CIRCLE OF ITS OWN ARC: the end of every arc is on its own circle by the entity's
// own rule, and a clone of that tie would say it a second time - the solver would call it implied.
#[test]
fn a_copy_gains_no_point_on_circle_of_its_own_arc() {
    let (mut p, si) = new_sketch();
    let first = add_arc(&mut p, si, Point2::new(0.0, 0.0), Point2::new(20.0, 0.0), Point2::new(17.320508, 10.0));
    let (end, _) = arc_ends(&p, si, first);
    let centre = match p.sketches[si].entity(first).map(|e| e.kind) {
        Some(EntityKind::Arc { center, .. }) => center,
        _ => panic!("no arc"),
    };
    p.sketches[si].constraints.push(Constraint::PointOnCircle { p: end, c: centre });
    let from = p.sketches[si].constraints.len();
    p.array_circular(si, &[first], 0.0, 0.0, 4, 360.0);
    assert!(new_of(&p, si, from, is_point_on_circle).is_empty(), "a copy gained a point-on-circle of its own arc, which the arc already says about itself");
}

// A RING OF ARCS ON ITS CIRCLE: the ends of the source are tied to the circle they are drawn on, and every
// copy's ends land on that same circle - the ring closes by constraint, and the copies stay where the
// rotation put them.
#[test]
fn a_ring_of_arcs_on_its_circle_keeps_its_ends_tied_to_the_circle() {
    let (mut p, si) = new_sketch();
    let circle = p.add_circle_entity(si, 0.0, 0.0, 20.0, Purpose::Construction);
    let cc = match p.sketches[si].entity(circle).map(|e| e.kind) {
        Some(EntityKind::Circle { center, .. }) => center,
        _ => panic!("no circle"),
    };
    let first = add_arc(&mut p, si, Point2::new(0.0, 0.0), Point2::new(20.0, 0.0), Point2::new(10.0, 17.320508));
    let (start, end) = arc_ends(&p, si, first);
    p.sketches[si].constraints.push(Constraint::PointOnCircle { p: start, c: cc });
    p.sketches[si].constraints.push(Constraint::PointOnCircle { p: end, c: cc });
    p.solve_sketch(si);
    let laid: Vec<(Id, f64, f64)> = p.sketches[si].points.iter().map(|q| (q.id, q.x, q.y)).collect();
    let from = p.sketches[si].constraints.len();
    let _ = p.add_pattern(si, &[first], PatternKind::Circular { cx: 0.0, cy: 0.0, count: 6, total_deg: 360.0 });
    // every copy carries the tie of both its ends to the circle: 5 copies, 2 ties each
    let carried = new_of(&p, si, from, is_point_on_circle);
    assert!(carried.len() == 10, "every end of every copy stays tied to the circle; {} clones were carried", carried.len());
    for c in &carried {
        match **c {
            Constraint::PointOnCircle { c: tied, .. } => assert!(tied == cc, "a carried tie points at a circle other than the one the source is tied to"),
            _ => unreachable!("the filter says these are ties"),
        }
    }
    assert!(p.sketch_conflicts(si).is_empty(), "the ring solves");
    // THE COPIES STAY WHERE THE ROTATION PUT THEM, to the landing tolerance: the solve confirms, it does not move
    const TOL: f64 = 5e-3;
    for q in p.sketches[si].points.iter().skip(laid.len()) {
        let near = laid.iter().any(|(_, x, y)| {
            (0..6).map(|k| k as f64 * 60.0).any(|deg| {
                let (sn, cs) = deg.to_radians().sin_cos();
                (q.x - (*x * cs - *y * sn)).hypot(q.y - (*x * sn + *y * cs)) < TOL
            })
        });
        assert!(near, "the point ({}, {}) is not where any turn of the source put it", q.x, q.y);
    }
}

// THE PATTERN'S CLONES FOLLOW RE-LAYOUT AND REMOVAL: the count changed re-lays the copies and re-derives
// their ties, with no tie left pointing at a copy that no longer exists; the pattern deleted leaves the
// source with its own ties and nothing more.
#[test]
fn editing_and_deleting_the_pattern_re_derives_and_removes_the_clones() {
    let (mut p, si) = new_sketch();
    let circle = p.add_circle_entity(si, 0.0, 0.0, 20.0, Purpose::Construction);
    let cc = match p.sketches[si].entity(circle).map(|e| e.kind) {
        Some(EntityKind::Circle { center, .. }) => center,
        _ => panic!("no circle"),
    };
    let first = add_arc(&mut p, si, Point2::new(0.0, 0.0), Point2::new(20.0, 0.0), Point2::new(10.0, 17.320508));
    let (start, end) = arc_ends(&p, si, first);
    p.sketches[si].constraints.push(Constraint::PointOnCircle { p: start, c: cc });
    p.sketches[si].constraints.push(Constraint::PointOnCircle { p: end, c: cc });
    p.solve_sketch(si);
    p.add_pattern(si, &[first], PatternKind::Circular { cx: 0.0, cy: 0.0, count: 6, total_deg: 360.0 });
    assert!(p.sketches[si].constraints.iter().filter(|c| is_point_on_circle(c)).count() == 12, "six arcs, two ties each");
    p.update_pattern(si, 0, PatternKind::Circular { cx: 0.0, cy: 0.0, count: 8, total_deg: 360.0 });
    assert!(p.sketches[si].constraints.iter().filter(|c| is_point_on_circle(c)).count() == 16, "eight arcs, two ties each");
    let alive: std::collections::HashSet<Id> = p.sketches[si].points.iter().map(|q| q.id).collect();
    for c in p.sketches[si].constraints.iter() {
        for id in constraint_point_ids(c) {
            assert!(alive.contains(&id), "a constraint points at the point {id} of a copy that no longer exists");
        }
    }
    p.delete_pattern(si, 0);
    let ties = p.sketches[si].constraints.iter().filter(|c| is_point_on_circle(c)).count();
    assert!(ties == 2, "the source keeps its own two ties and nothing more; {ties} stand");
    let arcs = p.sketches[si].entities.iter().filter(|e| matches!(e.kind, EntityKind::Arc { .. })).count();
    assert!(arcs == 1, "the copies are gone with the pattern; {arcs} arcs stand");
}

// AN INTERNAL TIE KEEPS THE ANGLE IT KEPT: a rotation turns every ray of a tie wholly inside the copied set
// by the same step, so the tie between a copy's own two lines is the source's tie unchanged - the value is
// carried, not turned.
#[test]
fn an_internal_tie_keeps_the_angle_it_kept() {
    let (mut p, si) = new_sketch();
    let one = p.add_line_entity(si, 10.0, 0.0, 20.0, 0.0, Purpose::Real);
    let two = p.add_line_entity(si, 30.0, 0.0, 38.660254, 5.0, Purpose::Real); // at 30 degrees off the first
    let (a1, b1) = line_ends(&p, si, one);
    let (a2, b2) = line_ends(&p, si, two);
    p.sketches[si].constraints.push(Constraint::AngleLines { a: a1, b: b1, c: a2, d: b2, deg: 30.0, expr: String::new(), driven: false, off: 0.0, at: None });
    p.solve_sketch(si);
    let from = p.sketches[si].constraints.len();
    p.array_circular(si, &[one, two], 0.0, 0.0, 3, 360.0);
    let carried: Vec<f64> = p.sketches[si].constraints[from..]
        .iter()
        .filter_map(|c| match c {
            Constraint::AngleLines { deg, .. } => Some(*deg),
            _ => None,
        })
        .collect();
    assert_eq!(carried.iter().filter(|d| (**d - 30.0).abs() < 5e-3).count(), 2, "both copies carry the tie at the source's own 30 degrees; {:?}", carried);
}

// A TIE TO AN OUTSIDE LINE IS CARRIED AS THE OPENING THE LAY MAKES: one line of the tie turns with the
// copy, the other stands, and the unsigned row that judges the clone cannot say which way the old opening
// lay - both openings it might have come to are answered, and the one the laid-out copy makes is laid.
// Here a 30-degree tie turned by 200 degrees comes to the opening of 170, not the 130 of the other way
// round.
#[test]
fn a_tie_to_an_outside_line_is_carried_as_the_opening_the_lay_makes() {
    let (mut p, si) = new_sketch();
    let one = p.add_line_entity(si, 10.0, 0.0, 20.0, 0.0, Purpose::Real);
    let outside = p.add_line_entity(si, 30.0, 0.0, 38.660254, 5.0, Purpose::Real); // stands, at 30 degrees
    let (a1, b1) = line_ends(&p, si, one);
    let (a2, b2) = line_ends(&p, si, outside);
    p.sketches[si].constraints.push(Constraint::AngleLines { a: a1, b: b1, c: a2, d: b2, deg: 30.0, expr: String::new(), driven: false, off: 0.0, at: None });
    p.solve_sketch(si);
    let from = p.sketches[si].constraints.len();
    p.array_circular(si, &[one], 0.0, 0.0, 2, 400.0); // one copy, turned by 200 degrees
    let carried: Vec<f64> = p.sketches[si].constraints[from..]
        .iter()
        .filter_map(|c| match c {
            Constraint::AngleLines { c: q, d, deg, .. } if *q == a2 && *d == b2 => Some(*deg),
            _ => None,
        })
        .collect();
    assert_eq!(carried.len(), 1, "the tie to the outside line is carried onto the copy");
    assert!((carried[0] - 170.0).abs() < 5e-3, "at the opening the lay makes, 170 degrees, not the 130 of the other way round; {}", carried[0]);
}

// AN ORIENTATION IS ABSOLUTE AND TURNS WHOLE: its row holds the direction of a line, a thing modulo a half
// turn, so the carried value is the source's turned by the step - 30 becomes 120, 210, 300 - and is never
// folded to an opening.
#[test]
fn an_orientation_is_carried_turned_whole() {
    let (mut p, si) = new_sketch();
    let line = p.add_line_entity(si, 10.0, 0.0, 18.660254, 5.0, Purpose::Real); // drawn at 30 degrees
    let (a, b) = line_ends(&p, si, line);
    p.sketches[si].constraints.push(Constraint::Orientation { a, b, deg: 30.0 });
    p.solve_sketch(si);
    let from = p.sketches[si].constraints.len();
    p.array_circular(si, &[line], 0.0, 0.0, 4, 360.0);
    let carried: Vec<f64> = p.sketches[si].constraints[from..]
        .iter()
        .filter_map(|c| match c {
            Constraint::Orientation { deg, .. } => Some(*deg),
            _ => None,
        })
        .collect();
    assert_eq!(carried.len(), 3, "every copy carries the orientation; {:?}", carried);
    for want in [120.0, 210.0, 300.0] {
        assert!(carried.iter().any(|d| (*d - want).abs() < 5e-3), "the orientation turned to {want} is missing; {:?}", carried);
    }
}
