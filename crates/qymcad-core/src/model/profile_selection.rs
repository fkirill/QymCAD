//! Profile selections follow exact covers of material faces, including their holes.

use super::{point_in_poly, round, seg_seg_t, Id, Project};
use crate::feature::FreshRegion;
use crate::geom::Point2;
use cavalier_contours::polyline::{BooleanOp, PlineSource, PlineSourceMut, Polyline};
use std::collections::{HashMap, HashSet};

const COVER_TOL: f64 = 1e-6;

struct RegionSnapshot {
    id: Id,
    boundary: Vec<Point2>,
    holes: Vec<Vec<Point2>>,
    parent: Id,
    area: f64,
    bounds: [f64; 4],
}

struct FeatureSelectionSnapshot {
    node: Id,
    regions: Vec<Id>,
}

pub(super) struct HeldSelections {
    regions: HashMap<Id, RegionSnapshot>,
    features: Vec<FeatureSelectionSnapshot>,
    boundary_entities: HashSet<Id>,
}

impl Project {
    pub(super) fn region_selections_referenced(&self, old: &[Id]) -> HeldSelections {
        let replaced: HashSet<Id> = old.iter().copied().collect();
        let mut held = HeldSelections { regions: HashMap::new(), features: Vec::new(), boundary_entities: HashSet::new() };
        for node in &self.timeline {
            let Some(selection) = node.kind.region_selection() else { continue };
            let mut regions: Vec<Id> = selection.profiles.iter().chain(selection.fill).copied().filter(|id| replaced.contains(id)).collect();
            regions.sort_unstable();
            regions.dedup();
            if regions.is_empty() {
                continue;
            }
            held.features.push(FeatureSelectionSnapshot { node: node.id, regions });
        }
        if held.features.is_empty() {
            return held;
        }
        for &id in old {
            held.boundary_entities.extend(self.contours.ents_of(id).into_iter().flatten().copied());
        }
        let children = self.contour_children();
        for feature in &held.features {
            for &id in &feature.regions {
                if let std::collections::hash_map::Entry::Vacant(entry) = held.regions.entry(id) {
                    if let Some(region) = self.region_snapshot(id, &children, &HashSet::new()) {
                        entry.insert(region);
                    }
                }
            }
        }
        held
    }

    fn contour_children(&self) -> HashMap<Id, Vec<Id>> {
        let mut children: HashMap<Id, Vec<Id>> = HashMap::new();
        for &id in self.contours.ids() {
            if let Some(parent) = self.contours.parent_of(id).filter(|&parent| parent != 0) {
                children.entry(parent).or_default().push(id);
            }
        }
        children
    }

    fn region_snapshot(&self, id: Id, children: &HashMap<Id, Vec<Id>>, filled: &HashSet<Id>) -> Option<RegionSnapshot> {
        let contour = &self.contours[self.contour_index(id)?];
        if !contour.closed || contour.points.len() < 3 {
            return None;
        }
        let mut pending = children.get(&id).cloned().unwrap_or_default();
        let mut holes = Vec::new();
        while let Some(child) = pending.pop() {
            if filled.contains(&child) {
                // Joining a fresh region's material preserves its existing unselected descendants as holes.
                pending.extend(children.get(&child).into_iter().flatten().copied());
            } else if let Some(ci) = self.contour_index(child) {
                holes.push(self.contours[ci].points.clone());
            }
        }
        let area = polygon_area(&contour.points) - holes.iter().map(|hole| polygon_area(hole)).sum::<f64>();
        Some(RegionSnapshot { id, boundary: contour.points.clone(), holes, parent: self.contours.parent_of(id).unwrap_or(0), area, bounds: polygon_bounds(&contour.points) })
    }

    pub(super) fn adapt_region_selections(&mut self, held: HeldSelections, standing: &[Id], old: &HashSet<Id>, made: &[Id]) {
        if held.features.is_empty() {
            return;
        }
        let fresh: Vec<FreshRegion> = made
            .iter()
            .copied()
            .filter(|id| {
                // Matching can give a new loop an old tile's id. Its entirely new boundary entities
                // still identify new geometry; a moved existing loop keeps its boundary entities.
                !old.contains(id) || self.contours.ents_of(*id).is_some_and(|entities| !entities.is_empty() && entities.iter().all(|entity| !held.boundary_entities.contains(entity)))
            })
            .filter_map(|child| {
                let contour = &self.contours[self.contour_index(child)?];
                let parent = self.contours.parent_of(child).unwrap_or(0);
                (contour.closed && contour.points.len() >= 3 && parent != 0).then_some(FreshRegion { child, parent })
            })
            .collect();
        let children = self.contour_children();
        let unfilled = HashSet::new();
        let base_tiles: Vec<RegionSnapshot> = standing.iter().filter_map(|&id| self.region_snapshot(id, &children, &unfilled)).collect();
        for selection in held.features {
            let regions: Vec<&RegionSnapshot> = selection.regions.iter().filter_map(|id| held.regions.get(id)).collect();
            // A fresh id can also be a piece of an old unselected hole. Only fresh material within this
            // feature's old selected faces is filled, including its fresh descendants.
            let fresh: Vec<FreshRegion> = fresh.iter().filter(|fresh| base_tiles.iter().find(|tile| tile.id == fresh.child).is_some_and(|tile| material_covered_by(tile, &regions))).copied().collect();
            let filled: HashSet<Id> = fresh.iter().map(|region| region.child).collect();
            let augmented;
            let tiles = if filled.is_empty() {
                &base_tiles
            } else {
                augmented = standing.iter().filter_map(|&id| self.region_snapshot(id, &children, &filled)).collect();
                &augmented
            };
            let cover = tiles_covering(&regions, tiles);
            if let Some(node) = self.timeline.iter_mut().find(|node| node.id == selection.node) {
                node.kind.follow_the_tiles(&cover, &fresh);
            }
        }
    }
}

fn tiles_covering(held: &[&RegionSnapshot], tiles: &[RegionSnapshot]) -> HashMap<Id, Vec<Id>> {
    let mut cover = HashMap::new();
    for old in held.iter().filter(|old| old.area > 1e-9) {
        let mut candidates: Vec<&RegionSnapshot> = tiles.iter().filter(|tile| tile.material_inside(old)).collect();
        let nested: HashSet<Id> = candidates.iter().filter(|tile| candidates.iter().any(|parent| tile.parent == parent.id)).map(|tile| tile.id).collect();
        candidates.retain(|tile| !nested.contains(&tile.id));
        let area: f64 = candidates.iter().map(|tile| tile.area).sum();
        if (area - old.area).abs() <= COVER_TOL * old.area {
            let ids: Vec<Id> = candidates.iter().map(|tile| tile.id).collect();
            if ids != [old.id] {
                cover.insert(old.id, ids);
            }
        }
    }
    // Each merge is gated by this feature's selected faces. Other features cannot complete its cover.
    for tile in tiles.iter().filter(|tile| tile.area > 1e-9) {
        let inside: Vec<&RegionSnapshot> = held.iter().copied().filter(|old| old.material_inside(tile)).collect();
        let area: f64 = inside.iter().map(|old| old.area).sum();
        if !inside.is_empty() && (area - tile.area).abs() <= COVER_TOL * tile.area {
            for old in inside {
                if old.id != tile.id {
                    cover.insert(old.id, vec![tile.id]);
                }
            }
        }
    }
    cover
}

impl RegionSnapshot {
    /// Whether this region's material lies wholly in `outer`, holes respected.
    fn material_inside(&self, outer: &Self) -> bool {
        if self.area <= 1e-9 || !round::meet(&self.bounds, &outer.bounds) || !region_inside([&self.boundary, &outer.boundary]) {
            return false;
        }
        outer.holes.iter().all(|hole| {
            let overlap = polygon_intersection_area([&self.boundary, hole]) - self.holes.iter().map(|excluded| polygon_intersection_area([excluded, hole])).sum::<f64>();
            overlap <= COVER_TOL * self.area
        })
    }

    /// The area of material this region and `other` share, holes subtracted on both sides.
    fn material_intersection_area(&self, other: &Self) -> f64 {
        if !round::meet(&self.bounds, &other.bounds) {
            return 0.0;
        }
        let mut area = polygon_intersection_area([&self.boundary, &other.boundary]);
        for hole in &self.holes {
            area -= polygon_intersection_area([hole, &other.boundary]);
        }
        for hole in &other.holes {
            area -= polygon_intersection_area([&self.boundary, hole]);
            for shared in &self.holes {
                area += polygon_intersection_area([shared, hole]);
            }
        }
        area.max(0.0)
    }
}

fn material_covered_by(region: &RegionSnapshot, selected: &[&RegionSnapshot]) -> bool {
    if selected.iter().any(|old| region.material_inside(old)) {
        return true;
    }
    let covered: f64 = selected.iter().map(|old| region.material_intersection_area(old)).sum();
    region.area > 1e-9 && (covered - region.area).abs() <= COVER_TOL * region.area
}

fn polygon_intersection_area(polys: [&[Point2]; 2]) -> f64 {
    let [first, second] = polys;
    if !round::meet(&polygon_bounds(first), &polygon_bounds(second)) {
        return 0.0;
    }
    let polyline = |points: &[Point2]| {
        let mut polyline = Polyline::new_closed();
        for point in points {
            polyline.add(point.x, point.y, 0.0);
        }
        polyline
    };
    let intersection = polyline(first).boolean(&polyline(second), BooleanOp::And);
    intersection.pos_plines.iter().map(|result| result.pline.area().abs()).sum::<f64>() - intersection.neg_plines.iter().map(|result| result.pline.area().abs()).sum::<f64>()
}

fn polygon_area(points: &[Point2]) -> f64 {
    let mut area = 0.0;
    for i in 0..points.len() {
        let first = points[i];
        let second = points[(i + 1) % points.len()];
        area += first.x * second.y - second.x * first.y;
    }
    (0.5 * area).abs()
}

fn polygon_bounds(points: &[Point2]) -> [f64; 4] {
    points.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |bounds, point| [bounds[0].min(point.x), bounds[1].min(point.y), bounds[2].max(point.x), bounds[3].max(point.y)])
}

fn region_inside(boundaries: [&[Point2]; 2]) -> bool {
    let [inner, outer] = boundaries;
    if inner.len() < 3 || outer.len() < 3 {
        return false;
    }
    let bounds = polygon_bounds(outer);
    let tolerance = ((bounds[2] - bounds[0]).hypot(bounds[3] - bounds[1]) * 1e-6).max(1e-9);
    if !inner.iter().all(|point| point_inside_or_on(outer, *point, tolerance)) {
        return false;
    }
    // Vertices can lie inside a concave polygon while an edge crosses its notch. Test every interval
    // between boundary crossings; the mean of all vertices need not lie inside either polygon.
    for i in 0..inner.len() {
        let start = inner[i];
        let end = inner[(i + 1) % inner.len()];
        let mut cuts = vec![0.0, 1.0];
        for j in 0..outer.len() {
            let first = outer[j];
            let second = outer[(j + 1) % outer.len()];
            if let Some(along) = seg_seg_t([start.x, start.y], [end.x, end.y], [first.x, first.y], [second.x, second.y]) {
                cuts.push(along);
            }
        }
        cuts.sort_unstable_by(f64::total_cmp);
        if !cuts.windows(2).all(|interval| {
            let along = 0.5 * (interval[0] + interval[1]);
            point_inside_or_on(outer, Point2::new(start.x + along * (end.x - start.x), start.y + along * (end.y - start.y)), tolerance)
        }) {
            return false;
        }
    }
    true
}

fn point_inside_or_on(polygon: &[Point2], point: Point2, tolerance: f64) -> bool {
    point_in_poly(polygon, point)
        || (0..polygon.len()).any(|i| {
            let start = polygon[i];
            let end = polygon[(i + 1) % polygon.len()];
            let (dx, dy) = (end.x - start.x, end.y - start.y);
            let length_squared = dx * dx + dy * dy;
            let along = if length_squared > 1e-18 { (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0.0, 1.0) } else { 0.0 };
            (point.x - (start.x + along * dx)).hypot(point.y - (start.y + along * dy)) <= tolerance
        })
}
