//! A CIRCULAR PATTERN PINS ITS COPIES: the carry holds a copy's shape against its own points, and the pins
//! hold its place about the centre - an angle at the centre for the step, a turn tie between the anchor
//! pairs, an equal length for the radius - so a sketch fully constrained before the pattern is fully
//! constrained after it, through either door. The pins are relations, not numbers baked at apply time: an
//! edit of the source moves the copies with it. A copy that lands on the source is pinned by coincidence;
//! a set of one circle has no turn of its own. The centre itself becomes a point the constraints name: a
//! click that landed on geometry uses it, a click on empty sheet puts a point there.
use qymcad_core::feature::Purpose;
use qymcad_core::model::{constraint_point_ids, Constraint, EntityKind, Id, PatternKind, Project};

fn new_sketch() -> (Project, usize) {
    let mut p = Project::default();
    p.new_document();
    let sid = p.add_sketch("t", vec![], None);
    p.add_sketch_node(sid, "Sketch");
    let si = p.sketch_index(sid).unwrap();
    (p, si)
}

/// The ends of line entity `eid`, in the order the line holds them.
fn line_ends(p: &Project, si: usize, eid: Id) -> (Id, Id) {
    match p.sketches[si].entity(eid).map(|e| e.kind) {
        Some(EntityKind::Line { a, b }) => (a, b),
        _ => panic!("no line {eid}"),
    }
}

/// Where a point of the sketch stands.
fn at(p: &Project, si: usize, id: Id) -> (f64, f64) {
    let q = p.sketches[si].points.iter().find(|q| q.id == id).unwrap_or_else(|| panic!("no point {id}"));
    (q.x, q.y)
}

/// A FULLY CONSTRAINED LINE at (10, 0)-(20, 0), held from the origin: the start's height and radius, the
/// length, the level. Nothing free, no anchor pin. Answered with the line's entity id.
fn line_held_from_the_origin(p: &mut Project, si: usize) -> Id {
    let o = p.ensure_origin(si);
    let line = p.add_line_entity(si, 10.0, 0.0, 20.0, 0.0, Purpose::Real);
    let (a, b) = line_ends(p, si, line);
    p.sketches[si].constraints.push(Constraint::Horizontal { a: o, b: a });
    p.sketches[si].constraints.push(Constraint::Distance { a: o, b: a, d: 10.0, off: 0.0, expr: String::new(), driven: false, axis: 0, at: None });
    p.sketches[si].constraints.push(Constraint::Distance { a, b, d: 10.0, off: 0.0, expr: String::new(), driven: false, axis: 0, at: None });
    p.sketches[si].constraints.push(Constraint::Horizontal { a, b });
    p.solve_sketch(si);
    line
}

/// A FULLY CONSTRAINED LINE at (10, 0)-(20, 0), its start pinned where it stands and its length and level
/// held: the radius of its far end is nowhere held but by the pattern's own equal length. Answered with the
/// line's entity id.
fn line_held_by_a_pin(p: &mut Project, si: usize) -> Id {
    let line = p.add_line_entity(si, 10.0, 0.0, 20.0, 0.0, Purpose::Real);
    let (a, b) = line_ends(p, si, line);
    p.sketches[si].constraints.push(Constraint::Fixed { p: a });
    p.sketches[si].constraints.push(Constraint::Distance { a, b, d: 10.0, off: 0.0, expr: String::new(), driven: false, axis: 0, at: None });
    p.sketches[si].constraints.push(Constraint::Horizontal { a, b });
    p.solve_sketch(si);
    line
}

/// Every line entity but `source`, as its ends' places.
fn other_lines(p: &Project, si: usize, source: Id) -> Vec<(Id, Id)> {
    p.sketches[si]
        .entities
        .iter()
        .filter(|e| e.id != source)
        .filter_map(|e| match e.kind {
            EntityKind::Line { a, b } => Some((a, b)),
            _ => None,
        })
        .collect()
}

fn near(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).hypot(a.1 - b.1) < 5e-3
}

/// The number of points standing at `(x, y)`, to the landing tolerance.
fn points_at(p: &Project, si: usize, x: f64, y: f64) -> usize {
    p.sketches[si].points.iter().filter(|q| near((q.x, q.y), (x, y))).count()
}

fn fully_constrained(p: &Project, si: usize) {
    assert_eq!(p.sketch_dof(si), (0, 0), "the sketch reports {} degrees of freedom and {} redundant", p.sketch_dof(si).0, p.sketch_dof(si).1);
    assert!(p.sketch_conflicts(si).is_empty(), "the sketch solves with no conflict");
    assert!(p.sketch_redundant_constraints(si).is_empty(), "nothing the solver calls redundant stands");
}

// FULLY CONSTRAINED IN, FULLY CONSTRAINED OUT, through the quick edit tool: the copies hold their place
// about the centre by the pins, every one of them where the rotation laid it.
#[test]
fn a_fully_constrained_sketch_stays_fully_constrained_through_the_quick_edit_tool() {
    let (mut p, si) = new_sketch();
    let line = line_held_from_the_origin(&mut p, si);
    assert_eq!(p.sketch_dof(si), (0, 0), "the source is fully constrained before the pattern");
    p.array_circular(si, &[line], 0.0, 0.0, 4, 360.0);
    fully_constrained(&p, si);
    // every copy sits where the rotation laid it: quarter turns of (10, 0) and (20, 0)
    let expected = [[(0.0, 10.0), (0.0, 20.0)], [(-10.0, 0.0), (-20.0, 0.0)], [(0.0, -10.0), (0.0, -20.0)]];
    for (pair, want) in other_lines(&p, si, line).iter().zip(expected.iter()) {
        let places = [at(&p, si, pair.0), at(&p, si, pair.1)];
        let turns = places.iter().any(|q| near(*q, want[0])) && places.iter().any(|q| near(*q, want[1]));
        assert!(turns, "a copy does not sit where the rotation laid it: {:?} against {:?}", places, want);
    }
}

// THE SAME through the pattern bar: both doors give the same result.
#[test]
fn a_fully_constrained_sketch_stays_fully_constrained_through_the_pattern_bar() {
    let (mut p, si) = new_sketch();
    let line = line_held_from_the_origin(&mut p, si);
    let _ = p.add_pattern(si, &[line], PatternKind::Circular { cx: 0.0, cy: 0.0, count: 4, total_deg: 360.0 });
    fully_constrained(&p, si);
}

// THE PINS ARE RELATIONS, NOT NUMBERS: the source's length edited after the apply, every copy's far end
// follows - its radius from the centre was never a number, it rides the source's.
#[test]
fn an_edit_of_the_source_length_moves_the_copies_with_it() {
    let (mut p, si) = new_sketch();
    let line = line_held_by_a_pin(&mut p, si);
    let (_, b) = line_ends(&p, si, line);
    p.array_circular(si, &[line], 0.0, 0.0, 4, 360.0);
    let di = p.sketches[si].constraints.iter().position(|c| matches!(c, Constraint::Distance { b: q, .. } if *q == b)).unwrap();
    if let Constraint::Distance { d, .. } = &mut p.sketches[si].constraints[di] {
        *d = 5.0; // the far end comes to (15, 0)
    }
    p.solve_sketch(si);
    fully_constrained(&p, si);
    for (a, q) in other_lines(&p, si, line) {
        let (pa, pq) = (at(&p, si, a), at(&p, si, q));
        let far = if pa.0.hypot(pa.1) > pq.0.hypot(pq.1) { pa } else { pq };
        let want = [(0.0, 15.0), (-15.0, 0.0), (0.0, -15.0)];
        assert!(want.iter().any(|w| near(far, *w)), "a copy's far end did not follow the edit: {:?}", far);
    }
}

// A CENTRE THAT LANDED ON GEOMETRY names it and adds no point: the pattern about a circle's centre pins
// its copies with an angle at that centre.
#[test]
fn a_centre_landed_on_geometry_names_it_and_adds_no_point() {
    let (mut p, si) = new_sketch();
    let circle = p.add_circle_entity(si, 5.0, 0.0, 3.0, Purpose::Real);
    let cc = match p.sketches[si].entity(circle).map(|e| e.kind) {
        Some(EntityKind::Circle { center, .. }) => center,
        _ => panic!("no circle"),
    };
    let line = line_held_by_a_pin(&mut p, si);
    let before = p.sketches[si].points.len();
    p.array_circular(si, &[line], 5.0, 0.0, 4, 360.0);
    assert_eq!(p.sketches[si].points.len(), before + 6, "the copies add their points and the centre adds none");
    let named = p.sketches[si].constraints.iter().any(|c| matches!(c, Constraint::Angle { b, .. } if *b == cc));
    assert!(named, "no pin names the centre the click landed on");
}

// A CENTRE ON EMPTY SHEET PUTS ONE PINNED POINT: the click's place becomes a point of the sketch, held
// where it stands, and the pins name it.
#[test]
fn a_centre_on_empty_sheet_puts_one_pinned_point() {
    let (mut p, si) = new_sketch();
    let line = line_held_by_a_pin(&mut p, si);
    let before = p.sketches[si].points.len();
    p.array_circular(si, &[line], 0.0, 30.0, 4, 360.0);
    assert_eq!(points_at(&p, si, 0.0, 30.0), 1, "the centre is one point at the click's place");
    assert_eq!(p.sketches[si].points.len(), before + 7, "the copies add theirs and the centre adds one");
    let cid = p.sketches[si].points.iter().find(|q| near((q.x, q.y), (0.0, 30.0))).unwrap().id;
    let pinned = p.sketches[si].constraints.iter().any(|c| matches!(c, Constraint::Fixed { p } if *p == cid));
    assert!(pinned, "the put centre is pinned where it stands");
    let named = p.sketches[si].constraints.iter().any(|c| matches!(c, Constraint::Angle { b, .. } if *b == cid));
    assert!(named, "no pin names the put centre");
}

// THE CENTRE RESOLVED TWICE PUTS THE POINT ONCE: a second pattern about the same empty place finds the
// point the first one put.
#[test]
fn a_centre_resolved_twice_puts_the_point_once() {
    let (mut p, si) = new_sketch();
    let line = line_held_by_a_pin(&mut p, si);
    p.array_circular(si, &[line], 0.0, 30.0, 4, 360.0);
    p.array_circular(si, &[line], 0.0, 30.0, 4, 360.0);
    assert_eq!(points_at(&p, si, 0.0, 30.0), 1, "two patterns about the same place share one centre point");
}

// A WHOLE-TURN COPY IS PINNED BY COINCIDENCE: the copy a sector steps a whole turn onto the source keeps
// its place by coincidences with its source points - what it carries leaves the anchor free, the anchor
// pin being placement and not carried - and no angle at the centre is laid for it.
#[test]
fn a_whole_turn_copy_is_pinned_by_coincidence() {
    let (mut p, si) = new_sketch();
    let line = line_held_by_a_pin(&mut p, si);
    let (a, b) = line_ends(&p, si, line);
    let from = p.sketches[si].constraints.len();
    p.array_circular(si, &[line], 0.0, 0.0, 7, 420.0); // the sixth copy comes round the whole turn onto the source
    fully_constrained(&p, si);
    let coincident = p.sketches[si].constraints[from..].iter().any(|c| match c {
        Constraint::Coincident { a: x, b: y } => (*x == a || *x == b) && *y != *x,
        _ => false,
    });
    assert!(coincident, "the copy that lands on the source is pinned by coincidences with its source points");
}

// A CIRCLE ON ITS OWN CENTRE IS PINNED BY COINCIDENCE ALONE: the rotation leaves every point where it
// stands, and only the centres' coincidence holds the copies - no angle, no turn tie, nothing else is laid
// for placement.
#[test]
fn a_circle_on_its_own_centre_is_pinned_by_coincidence_alone() {
    let (mut p, si) = new_sketch();
    let circle = p.add_circle_entity(si, 0.0, 0.0, 20.0, Purpose::Real);
    let cc = match p.sketches[si].entity(circle).map(|e| e.kind) {
        Some(EntityKind::Circle { center, .. }) => center,
        _ => panic!("no circle"),
    };
    p.sketches[si].constraints.push(Constraint::Fixed { p: cc }); // the anchor pin: placement, not carried
    p.sketches[si].constraints.push(Constraint::Diameter { c: cc, d: 40.0, off: 0.0, expr: String::new(), driven: false, diam: true, at: None });
    p.solve_sketch(si);
    assert_eq!(p.sketch_dof(si), (0, 0), "the source is fully constrained before the pattern");
    let from = p.sketches[si].constraints.len();
    p.array_circular(si, &[circle], 0.0, 0.0, 4, 360.0);
    fully_constrained(&p, si);
    assert!(
        !p.sketches[si].constraints[from..].iter().any(|c| matches!(c, Constraint::Angle { .. } | Constraint::AngleLines { .. } | Constraint::Equal { .. })),
        "a set sitting on the centre is pinned by coincidence alone"
    );
    let centres_tied = p.sketches[si].constraints[from..].iter().any(|c| matches!(c, Constraint::Coincident { a: x, .. } if *x == cc));
    assert!(centres_tied, "the copies' centres are tied to the source's");
}

// A LONE CIRCLE OFF THE CENTRE IS PINNED BY THE ANGLE AND THE EQUAL LENGTH: a circle has no turn of its
// own, so no turn tie is laid - the angle at the centre and the equal radius hold each copy.
#[test]
fn a_lone_circle_off_the_centre_is_pinned_by_the_angle_and_the_equal_length() {
    let (mut p, si) = new_sketch();
    let circle = p.add_circle_entity(si, 10.0, 0.0, 5.0, Purpose::Real);
    let cc = match p.sketches[si].entity(circle).map(|e| e.kind) {
        Some(EntityKind::Circle { center, .. }) => center,
        _ => panic!("no circle"),
    };
    p.sketches[si].constraints.push(Constraint::Fixed { p: cc });
    p.sketches[si].constraints.push(Constraint::Diameter { c: cc, d: 10.0, off: 0.0, expr: String::new(), driven: false, diam: true, at: None });
    p.solve_sketch(si);
    let from = p.sketches[si].constraints.len();
    p.array_circular(si, &[circle], 0.0, 0.0, 4, 360.0);
    fully_constrained(&p, si);
    let count = |want: fn(&Constraint) -> bool| p.sketches[si].constraints[from..].iter().filter(|c| want(c)).count();
    assert_eq!(count(is_angle), 3, "one angle at the centre per copy");
    assert_eq!(count(is_turn_tie), 0, "a lone circle gains no turn tie");
    assert_eq!(count(is_equal), 3, "one equal length per copy");
}

// THE STEP ANGLE WINS ITS PLACE: where the carried constraints already hold the anchor's radius - the
// start's radius is a distance the rotation preserves - the pin that is not laid is the equal length, and
// the angle at the centre is.
#[test]
fn the_step_angle_is_laid_and_the_equal_length_is_not_where_the_carry_holds_the_radius() {
    let (mut p, si) = new_sketch();
    let line = line_held_from_the_origin(&mut p, si);
    let (_, b) = line_ends(&p, si, line);
    p.array_circular(si, &[line], 0.0, 0.0, 4, 360.0);
    fully_constrained(&p, si);
    let equal = p.sketches[si].constraints.iter().any(|c| matches!(c, Constraint::Equal { a, b: q, c, d } if *q == b && *a == *c && *d != *a));
    assert!(!equal, "the equal length is not laid where the carry already holds the radius");
    let step = p.sketches[si].constraints.iter().any(|c| matches!(c, Constraint::Angle { a, c: d, deg, .. } if *a == b && *d != b && (*deg - 90.0).abs() < 5e-3));
    assert!(step, "the step angle at the centre is laid");
}

fn is_angle(c: &Constraint) -> bool {
    matches!(c, Constraint::Angle { .. })
}

fn is_turn_tie(c: &Constraint) -> bool {
    matches!(c, Constraint::AngleLines { .. })
}

fn is_equal(c: &Constraint) -> bool {
    matches!(c, Constraint::Equal { .. })
}

// A CENTRE THE PATTERN PUT GOES WITH THE PATTERN: deleting it takes the point away together with the
// copies and their pins, and the source stands with its own constraints and nothing more.
#[test]
fn deleting_the_pattern_takes_the_centre_it_put() {
    let (mut p, si) = new_sketch();
    let line = line_held_by_a_pin(&mut p, si);
    let had = p.sketches[si].constraints.len();
    let _ = p.add_pattern(si, &[line], PatternKind::Circular { cx: 0.0, cy: 30.0, count: 4, total_deg: 360.0 });
    assert_eq!(points_at(&p, si, 0.0, 30.0), 1, "the apply put the centre");
    p.delete_pattern(si, 0);
    assert_eq!(points_at(&p, si, 0.0, 30.0), 0, "the put centre went with the pattern");
    assert_eq!(p.sketches[si].points.len(), 2, "the source's two points are all that stand");
    assert_eq!(p.sketches[si].constraints.len(), had, "the source keeps its own constraints and nothing more");
}

// A CENTRE PICKED FROM GEOMETRY STAYS: the pattern about a circle's centre is deleted and the circle, its
// centre with it, stands as it was - a point the pattern found is not the pattern's to take away.
#[test]
fn a_centre_picked_from_geometry_stays_behind_the_deleted_pattern() {
    let (mut p, si) = new_sketch();
    let circle = p.add_circle_entity(si, 5.0, 0.0, 3.0, Purpose::Real);
    let cc = match p.sketches[si].entity(circle).map(|e| e.kind) {
        Some(EntityKind::Circle { center, .. }) => center,
        _ => panic!("no circle"),
    };
    let line = line_held_by_a_pin(&mut p, si);
    let _ = p.add_pattern(si, &[line], PatternKind::Circular { cx: 5.0, cy: 0.0, count: 4, total_deg: 360.0 });
    assert_eq!(p.sketches[si].patterns[0].centre, None, "a centre picked from geometry is not recorded");
    p.delete_pattern(si, 0);
    assert!(p.sketches[si].points.iter().any(|q| q.id == cc), "the circle's centre stays");
    assert!(p.sketches[si].entities.iter().any(|e| e.id == circle), "the circle stays");
}

// RE-LAYING KEEPS THE CENTRE, MOVING IT LEAVES NO ORPHAN: a count changed re-derives the pins about the
// same centre point; the centre moved, the point it put at the old place goes and one stands at the new.
#[test]
fn re_laying_keeps_the_centre_and_moving_it_leaves_no_orphan() {
    let (mut p, si) = new_sketch();
    let line = line_held_by_a_pin(&mut p, si);
    let _ = p.add_pattern(si, &[line], PatternKind::Circular { cx: 0.0, cy: 30.0, count: 4, total_deg: 360.0 });
    let put = p.sketches[si].patterns[0].centre;
    assert!(put.is_some(), "the centre on empty sheet was put");
    p.update_pattern(si, 0, PatternKind::Circular { cx: 0.0, cy: 30.0, count: 6, total_deg: 360.0 });
    assert_eq!(p.sketches[si].patterns[0].centre, put, "an unchanged centre is the same point");
    assert_eq!(points_at(&p, si, 0.0, 30.0), 1, "no second centre was put");
    p.update_pattern(si, 0, PatternKind::Circular { cx: 0.0, cy: 40.0, count: 6, total_deg: 360.0 });
    assert_eq!(points_at(&p, si, 0.0, 30.0), 0, "the old centre went when the pattern moved off it");
    assert_eq!(points_at(&p, si, 0.0, 40.0), 1, "the new centre stands at the new place");
}

// THE PUT CENTRE AND ITS PINS TRAVEL THROUGH THE FILE: a saved document opens with the centre the pattern
// put, its pins, and the sketch still fully constrained; the record travels with it, so the pattern
// deleted after the opening still takes its centre away.
#[test]
fn a_put_centre_and_its_pins_travel_through_the_file() {
    let (mut p, si) = new_sketch();
    let line = line_held_by_a_pin(&mut p, si);
    let _ = p.add_pattern(si, &[line], PatternKind::Circular { cx: 0.0, cy: 30.0, count: 4, total_deg: 360.0 });
    let text = qymcad_core::model::to_ron(&p).expect("the document serialises");
    let mut p: Project = qymcad_core::model::from_ron(&text).expect("the document reads back");
    let si = p.sketches.iter().position(|s| !s.patterns.is_empty()).expect("the pattern travels");
    assert_eq!(points_at(&p, si, 0.0, 30.0), 1, "the put centre travels");
    fully_constrained(&p, si);
    p.delete_pattern(si, 0);
    assert_eq!(points_at(&p, si, 0.0, 30.0), 0, "the record travels: the pattern still takes its centre away");
}

// THE QUICK EDIT'S PUT CENTRE DIES WITH ITS COPIES AND NOT BEFORE: the tool lays no record, and the centre
// its copies pin about is held up by their pins - an unrelated delete sweeps no orphan through it - while
// deleting the copies takes the pins and the centre with them.
#[test]
fn the_quick_edit_s_put_centre_dies_with_its_copies_and_not_before() {
    let (mut p, si) = new_sketch();
    let line = line_held_by_a_pin(&mut p, si);
    let other = p.add_line_entity(si, -30.0, 40.0, -20.0, 40.0, Purpose::Real); // a free line, going
    p.array_circular(si, &[line], 0.0, 30.0, 4, 360.0);
    let centre = p.sketches[si].points.iter().find(|q| near((q.x, q.y), (0.0, 30.0))).unwrap().id;
    p.delete_entities(si, &[other]);
    assert_eq!(points_at(&p, si, 0.0, 30.0), 1, "an unrelated delete swept the quick edit's centre away as an orphan");
    let named = p.sketches[si].constraints.iter().any(|c| matches!(c, Constraint::Angle { .. } | Constraint::Equal { .. }) && constraint_point_ids(c).contains(&centre));
    assert!(named, "no pin names the centre after the unrelated delete");
    assert_eq!(p.sketch_dof(si), (0, 0), "the copies keep their place with the free line gone");
    let copies: Vec<Id> = p.sketches[si].entities.iter().filter(|e| e.id != line).map(|e| e.id).collect();
    p.delete_entities(si, &copies);
    assert_eq!(points_at(&p, si, 0.0, 30.0), 0, "the centre went with its copies");
}

// A PUT CENTRE SHARED BY TWO PATTERNS IS HELD BY THE ONES THAT STAND: the second pattern about the same
// place finds the first's centre and records nothing, and deleting the first leaves the centre standing in
// the second's pins; the last pattern to go takes it with it.
#[test]
fn a_put_centre_shared_by_two_patterns_is_held_by_the_patterns_that_stand() {
    let (mut p, si) = new_sketch();
    let line = line_held_by_a_pin(&mut p, si);
    let _ = p.add_pattern(si, &[line], PatternKind::Circular { cx: 0.0, cy: 30.0, count: 4, total_deg: 360.0 });
    let _ = p.add_pattern(si, &[line], PatternKind::Circular { cx: 0.0, cy: 30.0, count: 4, total_deg: 360.0 });
    assert_eq!(p.sketches[si].patterns[1].centre, None, "a centre another pattern put is found, not recorded again");
    let centre = p.sketches[si].patterns[0].centre.unwrap();
    p.delete_pattern(si, 0);
    assert_eq!(points_at(&p, si, 0.0, 30.0), 1, "the centre stands: the second pattern's copies pin about it");
    let named = p.sketches[si].constraints.iter().any(|c| matches!(c, Constraint::Angle { .. } | Constraint::Equal { .. }) && constraint_point_ids(c).contains(&centre));
    assert!(named, "the standing pattern's own pins still name the centre");
    assert_eq!(p.sketch_dof(si), (0, 0), "the standing pattern's copies keep their place");
    p.delete_pattern(si, 0);
    assert_eq!(points_at(&p, si, 0.0, 30.0), 0, "the centre went with the last pattern that held it");
}

// A CIRCULAR PATTERN OF ONE LAYS NO CENTRE: it copies nothing, and nothing stands to pin - the click that
// meant a centre belongs to copies that are not there. Neither door puts a point.
#[test]
fn a_circular_pattern_of_one_lays_no_centre() {
    let (mut p, si) = new_sketch();
    let line = line_held_by_a_pin(&mut p, si);
    let before = p.sketches[si].points.len();
    p.array_circular(si, &[line], 0.0, 30.0, 1, 360.0);
    assert_eq!(p.sketches[si].points.len(), before, "the quick edit of one copy puts no centre");
    let _ = p.add_pattern(si, &[line], PatternKind::Circular { cx: 0.0, cy: 30.0, count: 1, total_deg: 360.0 });
    assert_eq!(p.sketches[si].patterns[0].centre, None, "the bar's pattern of one records no centre");
    assert_eq!(p.sketches[si].points.len(), before, "and puts no point");
}
