use crate::geom::{Contour, Point2, ProfEdge, piece_of};
use super::Id;

#[derive(Clone, Copy)]
struct SourcedEdge {
    source: Id,
    curve: ProfEdge,
}

#[derive(Clone, Copy, PartialEq)]
enum Retention {
    Boundary,
    Seam,
}

struct BoundaryPiece {
    edge: SourcedEdge,
    hole: usize,
    retention: Retention,
}

fn same_point(ends: [Point2; 2]) -> bool {
    let [left, right] = ends;
    left.dist(right) < 1e-7
}

fn start(curve: &ProfEdge) -> Option<Point2> {
    match *curve {
        ProfEdge::Line { a, .. } | ProfEdge::Arc { a, .. } => Some(a),
        ProfEdge::Circle { .. } => None,
    }
}

fn end(curve: &ProfEdge) -> Option<Point2> {
    match *curve {
        ProfEdge::Line { b, .. } | ProfEdge::Arc { b, .. } => Some(b),
        ProfEdge::Circle { .. } => None,
    }
}

fn shared_seam(edges: [&ProfEdge; 2]) -> bool {
    let [left, right] = edges;
    if piece_of(left).0 != piece_of(right).0 {
        return false;
    }
    match *left {
        ProfEdge::Line { a, b } => match *right {
            ProfEdge::Line { a: other_a, b: other_b } => same_point([a, other_b]) && same_point([b, other_a]),
            _ => false,
        },
        ProfEdge::Arc { a, b, center, ccw } => match *right {
            ProfEdge::Arc { a: other_a, b: other_b, center: other_center, ccw: other_ccw } => {
                ccw != other_ccw && same_point([center, other_center]) && same_point([a, other_b]) && same_point([b, other_a])
            }
            _ => false,
        },
        ProfEdge::Circle { .. } => false,
    }
}

fn stitch(mut edges: Vec<SourcedEdge>) -> Option<Vec<Contour>> {
    let mut contours = Vec::new();
    while !edges.is_empty() {
        let first = edges.remove(0);
        let origin = start(&first.curve)?;
        let mut tip = end(&first.curve)?;
        let mut contour = Contour { closed: true, edges: vec![first.curve], edge_src: vec![first.source], ..Default::default() };
        while !same_point([tip, origin]) {
            let next = edges.iter().position(|edge| start(&edge.curve).is_some_and(|at| same_point([at, tip])))?;
            let edge = edges.remove(next);
            tip = end(&edge.curve)?;
            contour.edges.push(edge.curve);
            contour.edge_src.push(edge.source);
        }
        contour.canonicalize();
        contours.push(contour);
    }
    Some(contours)
}

/// Adjacent hole regions become one wire by dropping their shared exact pieces; source ids stay on the surviving curves.
pub(super) fn joined_holes(holes: &[&Contour]) -> Vec<Contour> {
    if holes.len() < 2 {
        return holes.iter().map(|hole| (*hole).clone()).collect();
    }
    let mut pieces = Vec::new();
    for (hole, contour) in holes.iter().enumerate() {
        let mut canonical = (*contour).clone();
        canonical.canonicalize();
        pieces.extend(canonical.sourced_edges().into_iter().map(|(source, curve)| BoundaryPiece { edge: SourcedEdge { source, curve }, hole, retention: Retention::Boundary }));
    }
    let mut groups: Vec<usize> = (0..holes.len()).collect();
    let mut at_key: std::collections::HashMap<[i64; 2], Vec<usize>> = std::collections::HashMap::new();
    for (index, piece) in pieces.iter().enumerate() {
        at_key.entry(piece_of(&piece.edge.curve).0).or_default().push(index);
    }
    for left in 0..pieces.len() {
        if pieces[left].retention == Retention::Seam {
            continue;
        }
        let twin = at_key.get(&piece_of(&pieces[left].edge.curve).0).into_iter().flatten().copied().find(|right| {
            *right > left && pieces[*right].retention == Retention::Boundary && pieces[left].hole != pieces[*right].hole && shared_seam([&pieces[left].edge.curve, &pieces[*right].edge.curve])
        });
        let Some(right) = twin else { continue };
        pieces[left].retention = Retention::Seam;
        pieces[right].retention = Retention::Seam;
        let target = groups[pieces[left].hole];
        let replaced = groups[pieces[right].hole];
        for group in &mut groups {
            if *group == replaced {
                *group = target;
            }
        }
    }
    if !pieces.iter().any(|piece| piece.retention == Retention::Seam) {
        return holes.iter().map(|hole| (*hole).clone()).collect();
    }
    let mut joined = Vec::new();
    for (hole, original) in holes.iter().enumerate() {
        let group = groups[hole];
        if groups[..hole].contains(&group) {
            continue;
        }
        let members: Vec<usize> = (0..holes.len()).filter(|index| groups[*index] == group).collect();
        if members.len() == 1 {
            joined.push((*original).clone());
            continue;
        }
        let edges = pieces.iter().filter(|piece| groups[piece.hole] == group && piece.retention == Retention::Boundary).map(|piece| piece.edge).collect();
        if let Some(contours) = stitch(edges) {
            joined.extend(contours);
        } else {
            // An unclosed boundary must reach the kernel unchanged, where the invalid profile is reported.
            joined.extend(members.into_iter().map(|index| holes[index].clone()));
        }
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contour(edges: Vec<ProfEdge>, sources: Vec<Id>) -> Contour {
        let mut contour = Contour { closed: true, edges, edge_src: sources, ..Default::default() };
        contour.canonicalize();
        contour
    }

    #[test]
    fn two_half_discs_join_with_their_exact_arcs_and_sources() {
        let right = Point2::new(5.0, 0.0);
        let left = Point2::new(-5.0, 0.0);
        let center = Point2::new(0.0, 0.0);
        let upper = contour(vec![ProfEdge::Arc { a: right, b: left, center, ccw: true }, ProfEdge::Line { a: left, b: right }], vec![11, 20]);
        let lower = contour(vec![ProfEdge::Line { a: right, b: left }, ProfEdge::Arc { a: left, b: right, center, ccw: true }], vec![20, 12]);
        let joined = joined_holes(&[&upper, &lower]);
        assert_eq!(joined.len(), 1, "the half discs form one hole");
        assert_eq!(joined[0].edges.len(), 2, "the diameter seam is removed");
        assert!(joined[0].edges.iter().all(|edge| matches!(edge, ProfEdge::Arc { .. })), "both exact semicircles remain");
        assert_eq!(joined[0].edge_src, vec![11, 12], "the arcs keep their sketch sources");
        assert!(joined[0].closed && joined[0].signed_area() > 0.0, "the joined hole is closed and canonical");
        for edge in &joined[0].edges {
            assert!(upper.edges.iter().chain(&lower.edges).any(|original| original == edge), "joining does not alter the exact boundary curves");
        }
    }

    #[test]
    fn disjoint_holes_are_unchanged() {
        let first = contour(vec![ProfEdge::Circle { center: Point2::new(0.0, 0.0), r: 2.0 }], vec![11]);
        let second = contour(vec![ProfEdge::Circle { center: Point2::new(10.0, 0.0), r: 3.0 }], vec![12]);
        assert_eq!(joined_holes(&[&first, &second]), vec![first, second], "disjoint holes keep their complete contours");
    }

    #[test]
    fn only_the_shared_piece_of_a_subdivided_side_is_removed() {
        let left_points = [Point2::new(0.0, 0.0), Point2::new(10.0, 0.0), Point2::new(10.0, 5.0), Point2::new(10.0, 15.0), Point2::new(10.0, 20.0), Point2::new(0.0, 20.0)];
        let left_edges = (0..left_points.len()).map(|index| ProfEdge::Line { a: left_points[index], b: left_points[(index + 1) % left_points.len()] }).collect();
        let left = contour(left_edges, vec![1, 2, 2, 2, 3, 4]);
        let right_points = [Point2::new(10.0, 5.0), Point2::new(20.0, 5.0), Point2::new(20.0, 15.0), Point2::new(10.0, 15.0)];
        let right_edges = (0..right_points.len()).map(|index| ProfEdge::Line { a: right_points[index], b: right_points[(index + 1) % right_points.len()] }).collect();
        let right = contour(right_edges, vec![5, 6, 7, 2]);
        let joined = joined_holes(&[&left, &right]);
        assert_eq!(joined.len(), 1, "the adjacent rectangles form one hole");
        assert_eq!(joined[0].edges.len(), 8, "only the two copies of the shared middle segment are removed");
        assert!((joined[0].area() - 300.0).abs() < 1e-7, "the union keeps both rectangular areas");
        assert_eq!(joined[0].edge_src.iter().filter(|source| **source == 2).count(), 2, "the unshared pieces retain their source");
    }
}
