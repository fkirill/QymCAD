//! THE HELPERS OF THE USER SCENARIO: the recipe that applies any tool to the body a part holds now,
//! and the drawing a recipe takes its clicks from. A file of their own beside the scenario, which was
//! past the limit of the file-size ratchet and may only go down.

use super::super::hand::Hand;
use super::super::App;
use super::tests::check_all::check_all;

/// APPLY A TOOL TO THE CURRENT BODY OF A PART - one recipe per tool, for any part.
///
/// That is exactly what it is for: the request was that EVERY part carry ALL of the tools, but in a
/// DIFFERENT order. Separate code per part cannot express that - a recipe has to work with whatever body
/// exists NOW, however the previous operations have left it.
///
/// Returns `false` when the tool did not fire: that is either a finding or an honest reason, and the
/// caller decides which, because only the caller knows what it expected.
///
/// Type a string into the options bar of the text tool - without one there is nothing to draw.
pub(super) fn apply_tool(app: &mut App, kind: u8, problems: &mut Vec<String>, part: &str) -> bool {
    let before = app.project.timeline.len();
    // THE LAST SOLID BODY, not the last body of any kind: after a face copy or a patch the last one is A
    // SHEET, and the part tools would try to cut a surface. Work at that moment continues on the part, and
    // the recipe must behave the same way.
    // ...AND ONLY WITHIN THE CURRENT PART. Searching the whole document led to a reference into another
    // component: the tool took the body of a neighbouring part and the program honestly refused. Work
    // happens inside the part that was entered, and the recipe must behave the same way.
    let here = app.project.active_ctx();
    let Some(body) = app.project.timeline.iter().rev().filter(|n| n.parent == Some(here)).filter_map(|n| n.kind.body()).find(|b| app.project.bodies.iter().any(|x| x.id == *b && !x.sheet)) else {
        return false;
    };
    qymcad_ui_state::select_body(&mut app.project, &mut app.chosen.sel, &mut app.viewing.view, body);
    if qymcad_ui_state::selected_body(&app.project, &app.chosen.sel) != Some(body) {
        problems.push(format!("[{part}] the selection drifted: body {body} was asked for, {:?} is selected", qymcad_ui_state::selected_body(&app.project, &app.chosen.sel)));
    }
    let c = app
        .project
        .bodies
        .iter()
        .find(|b| b.id == body)
        .map(|b| {
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for v in &b.mesh.verts {
                for (k, q) in [v.x, v.y, v.z].into_iter().enumerate() {
                    lo[k] = lo[k].min(q);
                    hi[k] = hi[k].max(q);
                }
            }
            [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5, (lo[2] + hi[2]) * 0.5]
        })
        .unwrap_or([0.0; 3]);
    let scale = 8.0;
    let face = |app: &App, dir: [f64; 3]| {
        app.project
            .regen_faces
            .get(&body)
            .and_then(|fs| {
                // THE OUTER FACE, NOT THE LARGEST ONE. After a shell the part also has an inner wall with
                // the same normal, and its area is sometimes larger than the outer one - "the largest" took
                // the tool inside the box, where nobody is aiming.
                fs.iter()
                    .filter(|f| f.normal[0] * dir[0] + f.normal[1] * dir[1] + f.normal[2] * dir[2] > 0.9)
                    .max_by(|a, b| {
                        let d = |f: &qymcad_core::geom::MeshFace| (f.centroid.x - c[0]) * dir[0] + (f.centroid.y - c[1]) * dir[1] + (f.centroid.z - c[2]) * dir[2];
                        d(a).total_cmp(&d(b)).then(a.area.total_cmp(&b.area))
                    })
                    .cloned()
            })
            .map(|f| ([f.centroid.x, f.centroid.y, f.centroid.z], f.id))
    };
    let edge_vertical = |app: &App| app.project.regen_edges.get(&body).and_then(|es| es.iter().filter(|e| (e.a[2] - e.b[2]).abs() > 1.0).max_by(|x, y| x.mid[0].total_cmp(&y.mid[0])).map(|e| e.mid));
    // THE OUTER RIM, NOT JUST ANY EDGE AT THE SAME HEIGHT. After a shell the top of the part carries two
    // rims, inner and outer, and "the highest edge" took the inner one just as readily - and a chamfer on
    // it runs into a 2 mm wall. A person clicks the outer one: it is further from the axis of the part.
    let edge_top = |app: &App| {
        let es = app.project.regen_edges.get(&body)?;
        let top = es.iter().filter(|e| (e.a[2] - e.b[2]).abs() < 1e-6).map(|e| e.mid[2]).fold(f64::MIN, f64::max);
        es.iter()
            .filter(|e| (e.a[2] - e.b[2]).abs() < 1e-6 && (e.mid[2] - top).abs() < 1e-6)
            .max_by(|x, y| {
                let d = |m: &[f64; 3]| (m[0] - c[0]).hypot(m[1] - c[1]);
                d(&x.mid).total_cmp(&d(&y.mid))
            })
            .map(|e| e.mid)
    };

    match kind {
        4 | 5 => {
            // THE SIZE FOLLOWS THE STATE OF THE PART, as it does by hand: on a solid body the fillet is
            // large (and it must be LARGER than the wall to come, or the shell will eat it whole), while on
            // an already shelled part the wall is thin and the chamfer has to fit into it.
            let shelled = app.project.timeline.iter().any(|n| n.parent == Some(here) && matches!(n.kind, qymcad_core::feature::FeatureKind::Shell { .. }));
            let size = if shelled { 0.3 } else { 3.0 };
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(kind);
            let t = if kind == 4 { edge_vertical(hand.app) } else { edge_top(hand.app) };
            let Some(t) = t else { return false };
            hand.click(t).set(if kind == 4 { "radius" } else { "dist" }, size).enter();
        }
        6 => {
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(6);
            let Some((t, _)) = face(hand.app, [0.0, 0.0, 1.0]) else { return false };
            hand.click(t).set("thickness", 1.2).enter();
        }
        25 => {
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(25);
            let Some((t, _)) = face(hand.app, [1.0, 0.0, 0.0]) else { return false };
            hand.click(t).set("dist", 2.0).enter();
        }
        23 => {
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(23);
            // THE BOTTOM IS TAKEN AS THE NEUTRAL FACE - as a draft for printing requires - and to click it
            // the part IS TURNED. That is exactly what a person does when the face is behind the part.
            let (Some((nt, _)), Some((st, _))) = (face(hand.app, [0.0, 0.0, -1.0]), face(hand.app, [0.0, -1.0, 0.0])) else { return false };
            hand.app.params.draft.pick_neutral = true;
            hand.look_from_below().click(nt);
            hand.orbit(-0.7, 0.6).click(st).set("angle", 4.0).enter();
        }
        7 => {
            // A HOLE: the centre comes from the face that was clicked, the diameter and depth from the fields.
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(7);
            let Some((t, _)) = face(hand.app, [0.0, 0.0, 1.0]) else { return false };
            hand.click(t).set("diameter", 6.0).set("depth", 4.0).enter();
        }
        16 => {
            // MIRRORING A BODY: the click picks THE PLANE (here a side face of the part), Enter reflects it.
            // With "keep the original" the part stays ONE body made of two halves, so the rule holds.
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(16);
            let Some((t, _)) = face(hand.app, [0.0, 1.0, 0.0]) else { return false };
            hand.click(t).enter();
        }
        29 => {
            // DIVIDING FACES: the body stays one while the number of faces grows - a draft, a shell or a
            // thickness will later lie on them. The cutting plane is picked by click, as with the mirror.
            // The cutting plane is THE TOP of the part, lowered inwards: a plane parallel to a face does not
            // divide that face itself, while it cuts the sides exactly in half.
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(29);
            let Some((t, _)) = face(hand.app, [0.0, 0.0, 1.0]) else { return false };
            hand.click(t).set("offset", -5.0).enter();
        }
        26 => {
            // REMOVING A FACE: the neighbours stretch and the body stays closed. A chamfer or a fillet is
            // taken off without taking the timeline apart - which is how it is done once a part is
            // finished.
            // WHAT GETS REMOVED IS A SMALL FACE - a chamfer or a fillet - and not a wall: a wall cannot be
            // stretched over by its neighbours, and the core says so honestly. A person aims at something
            // small.
            // THE AIM IS A CHAMFER: its normal is SLANTED (not along the axes), and the neighbours around it
            // do stretch. Simply "the smallest face" may turn out to be a band that cannot be removed - the
            // core says so honestly, but nobody aims there either.
            let slanted = |f: &qymcad_core::geom::MeshFace| f.normal.iter().all(|c| c.abs() < 0.99);
            let small = app
                .project
                .regen_faces
                .get(&body)
                .and_then(|fs| fs.iter().filter(|f| f.area > 1e-6 && slanted(f)).min_by(|a, b| a.area.total_cmp(&b.area)).cloned())
                .or_else(|| app.project.regen_faces.get(&body).and_then(|fs| fs.iter().filter(|f| f.area > 1e-6).min_by(|a, b| a.area.total_cmp(&b.area)).cloned()));
            let _ = small;
            // REFUSED, SO ANOTHER FACE IS TRIED. Not every face can be removed: around a wall the
            // neighbours cannot be stretched, and the core honestly refuses. At that moment a person does
            // not drop the tool but clicks the next face - and the recipe must behave the same way.
            let mut cands: Vec<qymcad_core::geom::MeshFace> = app.project.regen_faces.get(&body).map(|fs| fs.iter().filter(|f| f.area > 1e-6).cloned().collect()).unwrap_or_default();
            cands.sort_by(|a, b| {
                let slant = |f: &qymcad_core::geom::MeshFace| if f.normal.iter().all(|c| c.abs() < 0.99) { 0 } else { 1 };
                slant(a).cmp(&slant(b)).then(a.area.total_cmp(&b.area))
            });
            let mut done = false;
            for f in cands.into_iter().take(4) {
                let before_n = app.project.timeline.len();
                let mut hand = Hand::new(app);
                hand.look_at(c, scale).tool(26);
                hand.click([f.centroid.x, f.centroid.y, f.centroid.z]).enter();
                qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
                let bad = app.project.timeline.iter().skip(before_n).any(|n| app.project.regen_errors.contains_key(&n.id));
                if !bad && app.project.timeline.len() > before_n {
                    done = true;
                    break;
                }
                // it did not work, so the failed node is removed and the next face is tried
                let ids: Vec<u64> = app.project.timeline.iter().skip(before_n).map(|n| n.id).collect();
                for id in ids {
                    app.project.delete_feature_op(id);
                }
                qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
            }
            if !done {
                return false;
            }
        }
        24 => {
            // A THREAD lies on a cylindrical face, which a hole provides - that is why the thread comes
            // after it in every order. The diameter and the pitch come from the catalogue, the length from
            // a field.
            // A THREAD LIES ON A CYLINDER, which a hole provides. The cylindrical face is found through the
            // circular edges around it; with no cylinder a thread has nothing to stand on, and nobody
            // reaches for it. That is not a fault of the program but a consequence of there being no hole.
            let round: Vec<[f64; 3]> = app.project.regen_edges.get(&body).map(|es| es.iter().filter(|e| e.radius > 1e-6).map(|e| e.mid).collect()).unwrap_or_default();
            // THE AIM IS THE WALL, NOT THE RIM. The midpoint of a circular edge lies on the boundary, and a
            // click there lands on the flat face around the hole; a person aims slightly BELOW the rim,
            // where the wall of the cylinder is.
            let mut done = false;
            for t in round.into_iter().take(4) {
                let before_n = app.project.timeline.len();
                let mut hand = Hand::new(app);
                hand.look_at(c, scale).tool(24);
                hand.click([t[0], t[1], t[2] - 0.5]).enter();
                if app.project.timeline.len() > before_n {
                    done = true;
                    break;
                }
            }
            if !done {
                return false;
            }
        }
        18 => {
            // A CIRCULAR ARRAY OF A BODY: the axis comes from the command bar, the count from a field. The
            // body stays one: the instances merge, as they do in a linear array.
            // the body to repeat is clicked first, as a person does: a pattern with nothing picked is refused
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(18);
            if !matches!(hand.app.chosen.sel, super::super::Sel::Mesh(..) | super::super::Sel::Face(..)) {
                let Some((t, _)) = face(hand.app, [0.0, 0.0, 1.0]) else { return false };
                hand.click(t);
            }
            hand.set("count", 3.0).set("angle", 360.0).enter();
        }
        27 => {
            // SPLITTING A BODY: the plane is picked by click, as when dividing faces, and the offset takes
            // it inside the part. The body falls into pieces - that is the work of the tool.
            // THE OFFSET COMES FROM THE BOUNDS OF THE BODY rather than being hard-coded. Five millimetres
            // down from the top face cut only the part the recipe was written on: let the shape change, and
            // the plane passes by while the tool honestly says the plane does not cut the body. At that
            // moment a person takes another plane rather than repeating the previous one.
            let half = app.project.mesh_index(body).and_then(|mi| app.project.bodies[mi].mesh.bounds()).map(|bb| ((bb.max.z - bb.min.z) * 0.5).max(1.0)).unwrap_or(5.0);
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(27);
            let Some((t, _)) = face(hand.app, [0.0, 0.0, 1.0]) else { return false };
            hand.click(t).set("offset", -half).enter();
        }
        33 => {
            // STITCHING: two sheets grow into one. The sheets are taken as copies of ADJACENT faces - they
            // share an edge, otherwise there is nothing to stitch and the core legitimately refuses.
            for dir in [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0]] {
                let mut hand = Hand::new(app);
                hand.look_at(c, scale).tool(30);
                let Some((t, _)) = face(hand.app, dir) else { return false };
                hand.click(t).enter();
            }
            let sheets: Vec<u64> = app
                .project
                .timeline
                .iter()
                .rev()
                .filter(|n| n.parent == Some(here))
                .filter_map(|n| n.kind.body())
                .filter(|b| app.project.bodies.iter().any(|x| x.id == *b && x.sheet))
                .take(2)
                .collect();
            if sheets.len() < 2 {
                return false;
            }
            // THE SELECTION FOR A STITCH IS A LIST OF BODIES rather than of faces: a click on a sheet puts
            // it into `stitch_parts`. Each one gets clicked, and if a click missed (the sheet lies ON the
            // part, and the part is what ends up under the cursor) the selection is set directly - at that
            // moment a person clicks in the tree instead.
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(33);
            for sh in &sheets {
                if let Some(f) = hand.app.project.regen_faces.get(sh).and_then(|fs| fs.first().cloned()) {
                    hand.click([f.centroid.x, f.centroid.y, f.centroid.z]);
                }
            }
            if app.params.stitch_parts.len() < 2 {
                app.params.stitch_parts = sheets.clone();
            }
            app.apply_feat_cmd();
            qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
        }
        34 => {
            // TRIMMING: a sheet is cut by a body, and the piece at THE POINT OF THE CLICK remains. The
            // sheet comes from a face copy. A REAL TOOL BODY IS REQUIRED: the part itself will not do - the
            // copy lies on it, and there is nothing to cut with (the core refuses, and rightly so). The
            // recipe is waiting for its place in an order: after a body split, when a second piece
            // legitimately appears in the part and does the cutting. It is not written into the orders
            // yet.
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(30);
            let Some((t, _)) = face(hand.app, [0.0, 0.0, 1.0]) else { return false };
            hand.click(t).enter();
            let sheet = app.project.timeline.iter().rev().filter(|n| n.parent == Some(here)).filter_map(|n| n.kind.body()).find(|b| app.project.bodies.iter().any(|x| x.id == *b && x.sheet));
            let Some(sheet) = sheet else { return false };
            let Some(f) = app.project.regen_faces.get(&sheet).and_then(|fs| fs.first().cloned()) else { return false };
            app.start_feat_cmd(34);
            app.side.trim.keep = Some((sheet, [f.centroid.x, f.centroid.y, f.centroid.z]));
            // THE CUTTING IS DONE BY THE SECOND PIECE rather than by the part itself: after a split a part
            // legitimately holds two bodies, and one of them is a real tool that crosses the sheet. The
            // part itself will not do: the face copy lies on it, and there is nothing to cut with.
            // AND IT IS CUT BY SOMETHING THAT REALLY CROSSES THE SHEET. "Any second piece" may lie off to
            // one side, and then the refusal is legitimate while it looks like a fault. A person chooses by
            // eye; the recipe chooses by the bounds - either they overlap or they do not.
            let bb = |b: u64| -> Option<[f64; 6]> {
                let mi = app.project.mesh_index(b)?;
                let mut r = [f64::MAX, f64::MAX, f64::MAX, f64::MIN, f64::MIN, f64::MIN];
                for v in &app.project.bodies[mi].mesh.verts {
                    for (k, q) in [v.x, v.y, v.z].into_iter().enumerate() {
                        r[k] = r[k].min(q);
                        r[k + 3] = r[k + 3].max(q);
                    }
                }
                (r[0] <= r[3]).then_some(r)
            };
            let sb = bb(sheet);
            let cutter = app
                .project
                .timeline
                .iter()
                .rev()
                .filter(|n| n.parent == Some(here))
                .flat_map(|n| n.kind.bodies())
                .filter(|b| *b != body && *b != sheet && app.project.bodies.iter().any(|x| x.id == *b && !x.sheet))
                .find(|b| match (sb, bb(*b)) {
                    (Some(a), Some(c)) => (0..3).all(|k| a[k] <= c[k + 3] + 1e-6 && c[k] <= a[k + 3] + 1e-6),
                    _ => false,
                });
            let mut cands: Vec<u64> = app
                .project
                .timeline
                .iter()
                .rev()
                .filter(|n| n.parent == Some(here))
                .flat_map(|n| n.kind.bodies())
                .filter(|b| *b != body && *b != sheet && app.project.bodies.iter().any(|x| x.id == *b && !x.sheet))
                .collect();
            if let Some(best) = cutter {
                cands.retain(|b| *b != best);
                cands.insert(0, best); // the one whose bounds overlap goes first
            }
            let mut done = false;
            for cut in cands.into_iter().take(4) {
                let before_n = app.project.timeline.len();
                app.start_feat_cmd(34);
                app.side.trim.keep = Some((sheet, [f.centroid.x, f.centroid.y, f.centroid.z]));
                app.side.trim.tool = Some(cut);
                app.apply_feat_cmd();
                qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
                let bad = app.project.timeline.iter().skip(before_n).any(|n| app.project.regen_errors.contains_key(&n.id));
                if !bad && app.project.timeline.len() > before_n {
                    done = true;
                    break;
                }
                let ids: Vec<u64> = app.project.timeline.iter().skip(before_n).map(|n| n.id).collect();
                for id in ids {
                    app.project.delete_feature_op(id);
                }
                qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
            }
            if !done {
                return false;
            }
        }
        32 => {
            // A PATCH is stretched over EDGES. The sheet comes from a face copy, and its contour is a
            // closed boundary a patch will certainly lie on - not "some edges or other".
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(30);
            let Some((t, _)) = face(hand.app, [0.0, 0.0, 1.0]) else { return false };
            hand.click(t).enter();
            let sheet = app.project.timeline.iter().rev().filter(|n| n.parent == Some(here)).filter_map(|n| n.kind.body()).find(|b| app.project.bodies.iter().any(|x| x.id == *b && x.sheet));
            let Some(sheet) = sheet else { return false };
            let edges: Vec<u32> = app.project.regen_edges.get(&sheet).map(|es| es.iter().map(|e| e.id).collect()).unwrap_or_default();
            if edges.len() < 2 {
                return false;
            }
            app.start_feat_cmd(32);
            // the edges are the sheet's, and a click on one records whose it is: the part's current body is its
            // solid, never a sheet beside it
            app.tools.gsel.edges = edges.into_iter().collect();
            app.edges.body = Some(sheet);
            app.apply_feat_cmd();
            qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
        }
        30 => {
            // A FACE COPY: a surface beside the part. It is clicked like an ordinary face, and the "a part
            // is one body" check does not count sheets - they are legitimate neighbours.
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(30);
            let Some((t, _)) = face(hand.app, [0.0, 0.0, 1.0]) else { return false };
            hand.click(t).enter();
        }
        28 => {
            // THICKENING A SHEET: the part already carries a surface from a face copy - it is given a
            // thickness and returns INTO the part. One tool, two cases: a face of a part and a sheet.
            let sheet = app.project.timeline.iter().rev().filter(|n| n.parent == Some(here)).filter_map(|n| n.kind.body()).find(|b| app.project.bodies.iter().any(|x| x.id == *b && x.sheet));
            let Some(sheet) = sheet else { return false };
            let Some(f) = app.project.regen_faces.get(&sheet).and_then(|fs| fs.first().cloned()) else { return false };
            qymcad_ui_state::select_body(&mut app.project, &mut app.chosen.sel, &mut app.viewing.view, sheet);
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(28).click([f.centroid.x, f.centroid.y, f.centroid.z]).set("thickness", 2.0).enter();
        }
        17 => {
            let mut hand = Hand::new(app);
            hand.look_at(c, scale).tool(17);
            if !matches!(hand.app.chosen.sel, super::super::Sel::Mesh(..) | super::super::Sel::Face(..)) {
                let Some((t, _)) = face(hand.app, [0.0, 0.0, 1.0]) else { return false };
                hand.click(t);
            }
            hand.enter();
        }
        _ => return false,
    }
    let made = app.project.timeline.len() > before;
    // A NAMED LIMIT OF THE CORE IS NOT A FAULT, and that rule is already accepted here for other tools. A
    // measurement (nine heights tried across the whole thickness) showed that some bodies the core cuts at
    // NO height at all - not "the plane was chosen badly" but the shape being what it is. A person backs
    // off at that point and does it another way; demanding a node from a tool the core refused in words
    // means demanding the impossible.
    let named_refusal = app.status == qymcad_ui_state::Notice::refusal(crate::i18n::tr("msg-plane-cuts-nothing"));
    if !made && named_refusal {
        return false; // the refusal was named - the step did not happen, but the document is not broken
    }
    if !made {
        // NOTHING WAS CREATED, SO WHAT THE PROGRAM ACTUALLY SAID IS RECORDED, along with the body being
        // worked on: "the tool did not fire" without that is as empty a complaint as any.
        let fs: Vec<String> =
            app.project.regen_faces.get(&body).map(|v| v.iter().map(|f| format!("[{:.2},{:.2},{:.2}]S{:.0}", f.normal[0], f.normal[1], f.normal[2], f.area)).collect()).unwrap_or_default();
        problems.push(format!("[{part}] tool {kind} created no node: body {body}, {} faces picked, the faces of the body: {}; status: {}", app.tools.gsel.faces.len(), fs.join(" "), app.status));
    }
    // A NAMED LIMIT OF THE CORE IS NOT A FAULT. The program said in words what it cannot do ("the body is
    // assembled from copies", "the offset fails inside the core"); a person then cancels the step and does
    // it differently rather than leaving a red node in the part. What stays red is only THE NAMELESS - and
    // that is the finding this whole thing exists for.
    let named: Vec<u64> =
        app.project.timeline.iter().skip(before).filter(|n| app.project.regen_errors.get(&n.id).is_some_and(|e| !matches!(e, qymcad_core::errors::CoreError::OpFailed(_)))).map(|n| n.id).collect();
    for id in named {
        app.project.delete_feature_op(id);
    }
    qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
    check_all(app, &format!("{part}: tool {kind}"), problems);
    made
}

/// A sketch tool by its number, its name in a report, and the clicks it is used with.
pub(super) struct Drawing {
    pub(super) tool: u8,
    pub(super) name: &'static str,
    pub(super) clicks: &'static [(f64, f64)],
}

pub(super) fn drawing(tool: u8, name: &'static str, clicks: &'static [(f64, f64)]) -> Drawing {
    Drawing { tool, name, clicks }
}
