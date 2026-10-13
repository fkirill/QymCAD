//! THE CONTOURS OF A SKETCH: found again after an edit, its origin, frame and axes, its points merged.

use super::*;

impl Project {
    /// THE LOOPS OF A SKETCH REPLACED: the loops `old` (contour ids) give way to `pairs` (each a loop and the entities
    /// it is made of), each new loop taking the id of the old one it is - by the entities of its boundary first, by
    /// its place and size after - so a feature keeps its loop through an edit. Answers the ids of the new loops, in
    /// their order. `kept` - the loops of the sketch that stay as they stood, nested with the new ones.
    pub(crate) fn replace_contours(&mut self, old: Vec<Id>, pairs: Vec<(Contour, Vec<Id>)>, kept: &[Id]) -> Vec<Id> {
        let entity_cids = old;
        // THE REGIONS THE FEATURES NAME, before the loops give way: the full point polygon of every old
        // contour a selection holds, not the signature summary. The cover below is computed against what
        // each region WAS, and the slots of `Contours` are overwritten in place by the replacement.
        let held = self.region_polygons_referenced(&entity_cids);
        // Stable contour ids: the new loops are matched against the old ones by a geometric signature
        // (closedness, centroid, area) rather than by position in the list. Positional reuse breaks
        // associativity: adding or removing a loop shifts a contour id onto a different physical loop, and an
        // extrude or an operation on the selected contour attaches to the wrong one. The signature survives
        // reordering and a changed number of loops; when the loop itself was edited, the nearest one of the same
        // kind by centroid is taken, as the best available "this is the same loop" heuristic.
        let (new_contours, new_prov): (Vec<Contour>, Vec<Vec<Id>>) = pairs.into_iter().unzip();
        let sig = |pts: &[Point2]| -> (Point2, f64) {
            let n = pts.len().max(1) as f64;
            let (sx, sy) = pts.iter().fold((0.0, 0.0), |(ax, ay), p| (ax + p.x, ay + p.y));
            let mut area = 0.0;
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                area += a.x * b.y - b.x * a.y;
            }
            (crate::geom::Point2::new(sx / n, sy / n), (0.5 * area).abs())
        };
        let old: Vec<(Id, Point2, f64, bool)> = entity_cids
            .iter()
            .filter_map(|&cid| {
                let ci = self.contour_index(cid)?;
                let c = &self.contours[ci];
                let (ctr, ar) = sig(&c.points);
                Some((cid, ctr, ar, c.closed))
            })
            .collect();
        let n_new = new_contours.len();
        let new_sig: Vec<(Point2, f64, bool)> = new_contours
            .iter()
            .map(|c| {
                let (ctr, ar) = sig(&c.points);
                (ctr, ar, c.closed)
            })
            .collect();
        let mut assign: Vec<Option<Id>> = vec![None; n_new];
        let mut used_old = vec![false; old.len()];
        // Phase one: match by provenance, that is, by the set of boundary entities. A loop is its entities
        // wherever it ends up after an edit, so loops that swapped places do not swap ids and a feature follows
        // its own geometry. Several regions sharing one set (a circle cut by a line) are separated within the
        // group by geometric proximity.
        {
            let old_prov: Vec<Vec<Id>> = old.iter().map(|o| self.contours.ents_of(o.0).cloned().unwrap_or_default()).collect();
            let mut groups: std::collections::HashMap<&Vec<Id>, (Vec<usize>, Vec<usize>)> = std::collections::HashMap::new();
            for (ni, pv) in new_prov.iter().enumerate() {
                if !pv.is_empty() {
                    groups.entry(pv).or_default().0.push(ni);
                }
            }
            for (oi, pv) in old_prov.iter().enumerate() {
                if !pv.is_empty() {
                    if let Some(g) = groups.get_mut(pv) {
                        g.1.push(oi);
                    }
                }
            }
            for (nis, ois) in groups.into_values() {
                let mut inner: Vec<(f64, usize, usize)> = Vec::new();
                for &ni in &nis {
                    for &oi in &ois {
                        if old[oi].3 == new_sig[ni].2 {
                            inner.push((new_sig[ni].0.dist(old[oi].1) + (new_sig[ni].1 - old[oi].2).abs().sqrt(), ni, oi));
                        }
                    }
                }
                inner.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                for (_, ni, oi) in inner {
                    if assign[ni].is_none() && !used_old[oi] {
                        assign[ni] = Some(old[oi].0);
                        used_old[oi] = true;
                    }
                }
            }
        }
        // Phase two: the rest are matched greedily by geometry (centroid and area), best matches first, or the
        // first loop would take an id belonging to another.
        //
        // A loop that stands as it stood matches at no distance, and such matches come first, each new loop taking the
        // first old one free: they are made by the exact signature, without a pair of every new loop with every old.
        // A drag frame among 10 000 lines is 10 000 open chains matched to 10 000 - 1e8 pairs on every frame.
        {
            let exact = |ctr: Point2, ar: f64, cl: bool| [(ctr.x + 0.0).to_bits(), (ctr.y + 0.0).to_bits(), (ar + 0.0).to_bits(), cl as u64];
            let mut free_old: std::collections::HashMap<[u64; 4], std::collections::VecDeque<usize>> = std::collections::HashMap::new();
            for (oi, o) in old.iter().enumerate().filter(|(oi, o)| !used_old[*oi] && o.1.x.is_finite() && o.1.y.is_finite() && o.2.is_finite()) {
                free_old.entry(exact(o.1, o.2, o.3)).or_default().push_back(oi);
            }
            for (ni, (ctr, ar, cl)) in new_sig.iter().enumerate() {
                if assign[ni].is_some() {
                    continue;
                }
                if let Some(oi) = free_old.get_mut(&exact(*ctr, *ar, *cl)).and_then(|q| q.pop_front()) {
                    assign[ni] = Some(old[oi].0);
                    used_old[oi] = true;
                }
            }
        }
        let mut cands: Vec<(usize, usize, f64)> = Vec::new();
        for (ni, (ctr, ar, cl)) in new_sig.iter().enumerate().filter(|(ni, _)| assign[*ni].is_none()) {
            for (oi, o) in old.iter().enumerate().filter(|(oi, _)| !used_old[*oi]) {
                if o.3 == *cl {
                    cands.push((ni, oi, ctr.dist(o.1) + (ar - o.2).abs().sqrt()));
                }
            }
        }
        cands.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        for (ni, oi, _) in cands {
            if assign[ni].is_none() && !used_old[oi] {
                assign[ni] = Some(old[oi].0);
                used_old[oi] = true;
            }
        }
        let mut new_entity_cids: Vec<Id> = Vec::with_capacity(n_new);
        for (ni, mut c) in new_contours.into_iter().enumerate() {
            c.canonicalize(); // The exact edges are the single source; the polyline and the traversal come from
                              // them.
            match assign[ni] {
                Some(cid) => {
                    if let Some(ci) = self.contour_index(cid) {
                        if let Some(slot) = self.contours.get_mut(ci) {
                            *slot = c;
                        }
                    }
                    new_entity_cids.push(cid);
                }
                None => new_entity_cids.push(self.add_contour(c)),
            }
        }
        // the loops of the sketch that stand now: the made and the kept together, for the nesting and for
        // the selections that follow them
        let standing: Vec<Id> = new_entity_cids.iter().chain(kept).copied().collect();
        // the nesting among the new loops and `kept`, the loops of the sketch left as they stood
        self.rebuild_contour_nesting(&standing);
        // Old contours not reused by any new loop are deleted, the id, the provenance and the nesting with each.
        let gone: std::collections::HashSet<Id> = old.iter().zip(&used_old).filter(|(_, used)| !**used).map(|(o, _)| o.0).collect();
        self.contours.remove_ids(&gone);
        // Provenance is persisted: contour id to the entities of its boundary, for matching on the next
        // edit.
        for (ni, cid) in new_entity_cids.iter().enumerate() {
            if new_prov[ni].is_empty() {
                self.contours.clear_ents(*cid);
            } else {
                self.contours.set_ents(*cid, new_prov[ni].clone());
            }
        }
        // A FEATURE'S SELECTED AREA FOLLOWS THE TILES - the cover, the merge and the fill - once the new
        // loops, their nesting and their provenance stand. It is written into the feature's own
        // parameters, here inside the one door every re-derivation passes through, so the document saves
        // it, the edit command re-offers it, and undo takes it back together with the edit.
        let old_ids: std::collections::HashSet<Id> = entity_cids.iter().copied().collect();
        let cover = self.tiles_covering(&held, &standing);
        let fresh = self.fresh_nested_regions(&new_entity_cids, &old_ids);
        for n in &mut self.timeline {
            n.kind.follow_the_tiles(&cover, &fresh);
        }
        new_entity_cids
    }

    /// THE REGIONS THE TIMELINE NAMES, before the loops give way (see `replace_contours`): the full point
    /// polygon of every old contour a feature's selection holds. Only regions being replaced are captured
    /// - a loop that stands as it stood needs no cover.
    fn region_polygons_referenced(&self, old: &[Id]) -> Vec<(Id, Vec<Point2>)> {
        let mut held: Vec<(Id, Vec<Point2>)> = Vec::new();
        for n in &self.timeline {
            let Some((profiles, fill)) = n.kind.region_selection() else { continue };
            for cid in profiles.iter().chain(fill.into_iter().flatten()) {
                if held.iter().any(|(id, _)| id == cid) || !old.contains(cid) {
                    continue;
                }
                if let Some(pts) = self.contour_index(*cid).map(|ci| self.contours[ci].points.clone()) {
                    held.push((*cid, pts));
                }
            }
        }
        held
    }

    /// THE COVER AND THE MERGE OF A RE-TILING: which old regions give way to which new ones. `held` - the
    /// point polygons of the regions the timeline named, as they were before the replacement; `standing` -
    /// the loops of the sketch after it. The answer maps an old id to the ids that replace it.
    ///
    /// Exactness gates both directions. The arrangement's minimal faces are pairwise interior-disjoint, so
    /// the tiles lying inside an old region sum to the area of their union: the sum meeting the old area,
    /// every tile inside it, IS an exact cover. Read from the other side, the same predicate is the merge:
    /// a new region covered exactly by the old regions lying inside it takes over their references, so
    /// deleting a dividing line collapses the selection back to the one region. A partial cover, or a
    /// boundary genuinely moved, does not fire: the feature follows its region as it does today.
    fn tiles_covering(&self, held: &[(Id, Vec<Point2>)], standing: &[Id]) -> std::collections::HashMap<Id, Vec<Id>> {
        // How close the tiles' total area must come to the old region's for a cover to fire, as a fraction
        // of the old area. The tiles come from the same tessellation machinery as the region they replace,
        // so an exact cover agrees far inside this: measured on the rectangle cut by a line, the circle
        // drawn inside a region, and the disc cut by its diameter line (whose halves carry arc
        // tessellation), the widest disagreement of an exact cover was 2e-16 relative.
        const COVER_TOL: f64 = 1e-6;
        let mut cover = std::collections::HashMap::new();
        if held.is_empty() {
            return cover;
        }
        // the closed regions standing now - the candidate tiles - with their boxes and areas
        let tiles: Vec<(Id, Vec<Point2>)> = standing
            .iter()
            .copied()
            .filter_map(|cid| self.contour_index(cid).and_then(|ci| (self.contours[ci].closed && self.contours[ci].points.len() >= 3).then(|| (cid, self.contours[ci].points.clone()))))
            .collect();
        let tile_boxes: Vec<[f64; 4]> = tiles.iter().map(|(_, pts)| poly_box(pts)).collect();
        let tile_areas: Vec<f64> = tiles.iter().map(|(_, pts)| poly_area(pts)).collect();
        let held_boxes: Vec<[f64; 4]> = held.iter().map(|(_, pts)| poly_box(pts)).collect();
        // THE COVER: an old region exactly met by the tiles lying inside it
        for (k, (oid, opts)) in held.iter().enumerate() {
            let oarea = poly_area(opts);
            if oarea < 1e-9 {
                continue;
            }
            let mut cands: Vec<usize> = (0..tiles.len()).filter(|&t| round::meet(&tile_boxes[t], &held_boxes[k])).collect();
            cands.retain(|&t| region_inside(&tiles[t].1, opts));
            // a hole of the old region lies inside it too, but it is no tile of the cover: the cover is
            // the candidates nested in no other candidate
            let nested: std::collections::HashSet<usize> = cands.iter().filter(|&&t| cands.iter().any(|&u| u != t && self.contours.parent_of(tiles[t].0) == Some(tiles[u].0))).copied().collect();
            cands.retain(|&t| !nested.contains(&t));
            let covered: f64 = cands.iter().map(|&t| tile_areas[t]).sum();
            if (covered - oarea).abs() <= COVER_TOL * oarea {
                let ids: Vec<Id> = cands.iter().map(|&t| tiles[t].0).collect();
                if ids != vec![*oid] {
                    cover.insert(*oid, ids);
                }
            }
        }
        // THE MERGE: a new region exactly met by the old regions lying inside it
        for (t, (nid, npts)) in tiles.iter().enumerate() {
            let narea = tile_areas[t];
            let mut inside: Vec<Id> = Vec::new();
            let mut covered = 0.0;
            for (k, (oid, opts)) in held.iter().enumerate() {
                if round::meet(&tile_boxes[t], &held_boxes[k]) && region_inside(opts, npts) {
                    inside.push(*oid);
                    covered += poly_area(opts);
                }
            }
            if !inside.is_empty() && inside != vec![*nid] && (covered - narea).abs() <= COVER_TOL * narea {
                for oid in inside {
                    cover.insert(oid, vec![*nid]);
                }
            }
        }
        cover
    }

    /// The closed contours the edit minted, each paired with the region it nests directly inside (0 at
    /// the root): the candidates of the fill rule, taken from the loops the edit MADE only - a loop that
    /// stood as it stood is never fresh, however far the loops being replaced reached. Fresh ids only - a
    /// contour that survived by its id and merely moved into the selected area keeps today's hole
    /// semantics, which is a shape change the person can see, while geometry newly drawn inside the area
    /// is the case the fill is for.
    fn fresh_nested_regions(&self, made: &[Id], old: &std::collections::HashSet<Id>) -> Vec<(Id, Id)> {
        made.iter()
            .copied()
            .filter(|cid| !old.contains(cid))
            .filter_map(|cid| {
                let ci = self.contour_index(cid)?;
                (self.contours[ci].closed && self.contours[ci].points.len() >= 3).then(|| (cid, self.contours.parent_of(cid).unwrap_or(0)))
            })
            .collect()
    }
    /// The origin point (0,0, fixed) of a sketch, created lazily. Returns its id. It belongs to no entity and
    /// therefore never reaches a profile or a contour.
    pub fn ensure_origin(&mut self, si: usize) -> Id {
        if let Some(s) = self.sketches.get(si) {
            if s.origin != 0 && s.points.iter().any(|p| p.id == s.origin) {
                return s.origin;
            }
        } else {
            return 0;
        }
        let id = self.alloc_id();
        let s = &mut self.sketches[si];
        s.points.push(SketchPoint { id, x: 0.0, y: 0.0 });
        s.constraints.push(Constraint::Fixed { p: id });
        s.origin = id;
        id
    }
    /// The two points defining the infinite line of a coordinate axis (`which` is 0 for X, 1 for Y): the origin
    /// (0,0) and a fixed guide point at (1,0) or (0,1). Created lazily, for constraints against the axes.
    /// GEOMETRY GLUED INTO THE ORIGIN IS GIVEN A POINT OF ITS OWN, and the origin goes back to zero.
    ///
    /// Reported behaviour: "where has the centre of the circle gone?" - and, put more sharply, "the
    /// circle is not at the origin, but its CENTRE is the origin". Both describe one state: the origin
    /// point had been dragged to (-27, 36) and was the circle's centre, while the origin's MARK is drawn
    /// where the axes cross. Two different places on screen claiming to be the same thing.
    ///
    /// How it got there: `merge_close_points` glues whatever lands at zero INTO the origin, keeping the
    /// origin's id, so the origin becomes somebody's geometry and travels with it.
    ///
    /// The repair keeps the drawing exactly where it is - the entity gets a fresh point at the position the
    /// origin stands at - and leaves the origin to the frame. Returns whether anything was separated.
    ///
    /// IT RUNS WHETHER OR NOT THE ORIGIN HAS WANDERED. It used to give up when the origin sat at zero,
    /// on the reading that a point at zero is home and nothing is wrong. But welded geometry at zero is the
    /// worse half of the same defect: the origin is a system point, `immovable_points` holds those, so the
    /// corner cannot be dragged and moving the shape tears it off - and since `pin_frame` now keeps the
    /// origin at zero, that reading meant the repair could never fire at all.
    pub fn detach_geometry_from_origin(&mut self, si: usize) -> bool {
        let Some(s) = self.sketches.get(si) else { return false };
        let origin = s.origin;
        if origin == 0 {
            return false;
        }
        let Some((ox, oy)) = s.points.iter().find(|p| p.id == origin).map(|p| (p.x, p.y)) else { return false };
        let used = s.entities.iter().any(|e| entity_points(e).contains(&origin)) || s.splines.iter().any(|sp| sp.points.contains(&origin));
        if !used {
            return false; // nobody's geometry - nothing to separate
        }
        let fresh = self.alloc_id();
        let s = &mut self.sketches[si];
        s.points.push(SketchPoint { id: fresh, x: ox, y: oy });
        let (guides, frame) = (s.axis_pts, s.frame);
        for e in &mut s.entities {
            remap_entity_point(e, origin, fresh);
        }
        for sp in &mut s.splines {
            for id in &mut sp.points {
                if *id == origin {
                    *id = fresh;
                }
            }
        }
        for c in &mut s.constraints {
            // `Fixed` on the origin stays on the origin; so does anything naming the frame or an axis
            // guide - those speak about the reference frame, not about the drawing.
            if matches!(c, Constraint::Fixed { p } if *p == origin) {
                continue;
            }
            let pts = c.points();
            if pts.iter().any(|p| guides.contains(p) || *p == frame) {
                continue;
            }
            remap_constraint_point_pub(c, origin, fresh);
        }
        true
    }

    /// The anchor of the frame of reference: a point at (0,0) that is nobody's geometry - see
    /// `Sketch::frame`. Created lazily, like the origin.
    pub fn ensure_frame(&mut self, si: usize) -> Id {
        if let Some(s) = self.sketches.get(si) {
            if s.frame != 0 && s.points.iter().any(|p| p.id == s.frame) {
                return s.frame;
            }
        } else {
            return 0;
        }
        let id = self.alloc_id();
        let s = &mut self.sketches[si];
        s.points.push(SketchPoint { id, x: 0.0, y: 0.0 });
        s.constraints.push(Constraint::Fixed { p: id });
        s.frame = id;
        id
    }

    pub fn ensure_axis(&mut self, si: usize, which: usize) -> (Id, Id) {
        // THE ANCHOR, NOT THE ORIGIN. An axis is the infinite line through these two points, and the
        // origin is a point geometry gets glued to - so an axis hung on it tilts the moment that geometry
        // moves. See `Sketch::frame`.
        let o = self.ensure_frame(si);
        let w = which.min(1);
        let existing = self.sketches.get(si).map(|s| s.axis_pts[w]).unwrap_or(0);
        if existing != 0 && self.sketches.get(si).is_some_and(|s| s.points.iter().any(|p| p.id == existing)) {
            return (o, existing);
        }
        let id = self.alloc_id();
        let (x, y) = if w == 0 { (1.0, 0.0) } else { (0.0, 1.0) };
        let s = &mut self.sketches[si];
        s.points.push(SketchPoint { id, x, y });
        s.constraints.push(Constraint::Fixed { p: id });
        s.axis_pts[w] = id;
        (o, id)
    }
    /// Stitch nearby sketch points (closer than `tol`): duplicates are merged into one and every entity,
    /// constraint and spline is re-pointed at it.
    ///
    /// This cures a corner that falls apart — line endpoints that never merged into a shared point, so a
    /// dimension or a constraint tears the shape open. Returns how many points were merged.
    pub fn merge_close_points(&mut self, si: usize, tol: f64) -> usize {
        let Some(s) = self.sketches.get_mut(si) else { return 0 };
        // Is the point the centre of a circle or an arc, and therefore the carrier of a solver radius variable?
        // Two such centres must not be stitched into one node, or their radii collapse and concentric circles
        // drawn from one coordinate break. The check runs against the current state of the entities, so it
        // accounts for the remappings already done in this loop.
        let is_center = |ents: &[SketchEntity], pid: Id| ents.iter().any(|e| matches!(e.kind, EntityKind::Circle { center, .. } | EntityKind::Arc { center, .. } if center == pid));
        // System points (the origin and the axis endpoints) must not be merged: they lie between 1 and sqrt(2)
        // apart while the tolerance of the line tool reaches 2.0, so merging would consume the axes and tear the
        // dimensions measured to them.
        let sys: Vec<Id> = s.system_ids();
        let mut merged = 0usize;
        // NOTHING IS EVER GLUED ONTO THE ORIGIN. Geometry landing at zero used to be given the origin's id,
        // so that a profile corner drawn there would be one node and the contour would close. The cost was
        // paid by the person: the corner then WAS a system point, and `immovable_points` holds those - the
        // shape could not be dragged, and moving it tore the corner off at zero.
        //
        // Reported behaviour: "I cannot move a circle away from the origin with the move tool". A circle
        // survived by accident (a radius centre is never glued); a line drawn from zero did not - measured,
        // its end came back as the origin's own id.
        //
        // The contour still closes: the ordinary stitching below merges two GEOMETRY points standing in one
        // place into one geometry point, which is what closing needs. The frame of reference stays out of it.
        let mut i = 0;
        while i < s.points.len() {
            let (ix, iy, keep) = (s.points[i].x, s.points[i].y, s.points[i].id);
            let mut j = i + 1;
            while j < s.points.len() {
                let dup = s.points[j].id;
                let d2 = (ix - s.points[j].x).powi(2) + (iy - s.points[j].y).powi(2);
                // Not merged: two radius-curve centres (concentricity is a constraint, not a shared node) and
                // any system point (the reference frame of the origin and the axes is left alone).
                if dup != keep && d2 < tol * tol && !(is_center(&s.entities, keep) && is_center(&s.entities, dup)) && !sys.contains(&keep) && !sys.contains(&dup) {
                    remap_point_id(s, dup, keep);
                    s.points.remove(j);
                    merged += 1;
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
        if merged > 0 {
            // Drop degenerate zero-length lines and the constraints they orphan.
            s.entities.retain(|e| !matches!(e.kind, EntityKind::Line { a, b } if a == b));
            self.regen_sketch(si);
        }
        merged
    }
}
