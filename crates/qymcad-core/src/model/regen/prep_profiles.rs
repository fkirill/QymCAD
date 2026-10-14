//! THE OPERATIONS ON A PROFILE PREPARED: extrusions, revolutions, sweeps, combinations.

use super::*;

impl Project {
    /// Live edges of a query reference: recorded names are translated, a description is asked again.
    ///
    /// Two steps, both required: translate the old numbers through the rename map of the source body
    /// (`EdgeRenames`), and repair a selection that came loose when the topology changed, using the geometric
    /// snapshot. Spelling that out separately in the fillet and in the chamfer invites a third copy with the
    /// next modifier, and a copy that forgets the second step lets an edge selection drift silently after any
    /// edit higher up the timeline, putting a fillet on the wrong edge.
    ///
    /// The distinction between the two paths is not cosmetic. A recorded edge name goes stale together with the
    /// names of its faces, and translating through the rename map is the only honest way to catch up. A
    /// description has nothing to catch up with: it already refers to today's geometry.
    ///
    /// Fresh edges come from the kernel rather than from `regen_edges`, which is filled by a later pass and at
    /// this point still describes the previous topology.
    /// WHERE A SKETCH'S GEOMETRY LANDS: its plane as a transform for the kernel, the identity for a world
    /// plane.
    pub(crate) fn sketch_place(&self, sketch: Id) -> [f64; 12] {
        self.sketch_frame_by_id(sketch).map(|f| f.matrix12()).unwrap_or(crate::feature::PLACE_IDENTITY)
    }

    /// THE AXIS A REVOLUTION TURNS ABOUT, in the sketch's own frame, with whether it was named (a line or a datum) rather
    /// than the sketch's X or Y. Priority: a sketch centreline (already local), then a datum axis (world, including one
    /// bound to an edge or a face, brought in through the inverse placement), then the sketch X or Y axis. One
    /// computation for the rebuild, its check of a profile crossing the axis and the preview before Enter: copies
    /// would drift, and the preview would turn about an axis the body is not built about.
    pub fn revolve_axis_local(&self, sketch: Id, ax: RevolveAxis) -> ([f64; 3], [f64; 3], bool) {
        let axis_od = (ax.datum != 0).then(|| self.datum_axes.iter().find(|d| d.id == ax.datum).map(|d| (d.origin(), d.dir()))).flatten();
        match (self.revolve_axis_from_line(sketch, ax.line), axis_od) {
            (Some((lo, ld)), _) => (lo, ld, true),
            (None, Some((wo, wd))) => {
                let inv = crate::feature::mat_inv12(&self.sketch_place(sketch));
                (crate::feature::apply12(&inv, wo), crate::feature::apply12_dir(&inv, wd), true)
            }
            _ => ([0.0, 0.0, 0.0], if ax.axis == 0 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] }, false),
        }
    }

    /// Extrude: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_extrude(&mut self, p: &mut Pass, prof: Profile, span: ExtrudeSpan, body: Id) -> crate::feature::KernelJob {
        let (Profile { sketch, profiles }, ExtrudeSpan { height, down, reach, fill }) = (prof, span);
        let mut pl = self.sketch_place(sketch);
        // Parametric dimension expressions, where present, override the stored numbers.
        let height = p.dim("height", height);
        let down = p.dim("down", down);
        // Extent along the normal: both ways, two-sided or one-sided (see `extrude_extent`).
        let (start, total) = crate::feature::extrude_extent(height, down, reach);
        if start.abs() > 1e-9 {
            // Shift the origin by `start` along the normal (N is the column [2,6,10]).
            pl[3] += pl[2] * start;
            pl[7] += pl[6] * start;
            pl[11] += pl[10] * start;
        }
        // Encode every contour and extrude them in one node (`combine_region_multi`; a zero `src`
        // makes a new body).

        let feature = p.node;
        match self.encode_profiles_sourced(sketch, profiles, fill) {
            None => crate::feature::KernelJob::refused(crate::errors::CoreError::ProfileNotFound),
            Some(sourced) => crate::feature::KernelJob::new(Vec::new(), move |k| {
                k.combine_region_multi(
                    BodyOp { src: 0, op: 1, body },
                    &sourced.profiles,
                    crate::feature::ToolSpan { start: 0.0, length: total, ends: crate::feature::ToolEnds::Kept },
                    pl,
                    crate::feature::Sides { feature, sources: &sourced.sources },
                )
            }),
        }
    }

    /// Revolve: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_revolve(&mut self, p: &mut Pass, prof: Profile, ax: RevolveAxis, angle: f64, reach: crate::feature::Reach, t: BodyOp) -> crate::feature::KernelJob {
        let (sketch, profiles) = (prof.sketch, prof.profiles);
        let axis = ax.axis;
        let (src, op, body) = (t.src, t.op, t.body);
        let pl = self.sketch_place(sketch);
        let angle = eval_dim(p.dims, "angle", angle, p.vars);
        let (ax_o, ax_d, named) = self.revolve_axis_local(sketch, ax);
        // Symmetric means a start angle of -angle/2, and a flip means -angle, sweeping the other
        // way. Implemented by pre-rotating the result about the local axis: pl' = pl * Rot(axis, t0).
        // The kernel is untouched — it builds [0, angle] and the rotation carries that to
        // [t0, t0 + angle].
        let theta0 = match reach {
            crate::feature::Reach::BothWays => -angle / 2.0,
            crate::feature::Reach::Backward => -angle,
            crate::feature::Reach::Forward => 0.0,
        };
        let with_start = |o: [f64; 3], d: [f64; 3], pl: [f64; 12]| -> [f64; 12] {
            if theta0.abs() < 1e-12 {
                pl
            } else {
                crate::feature::compose12(&pl, &crate::feature::rot12_axis(o, d, theta0))
            }
        };

        let feature = p.node;
        match self.encode_profiles_sourced(sketch, profiles, &[]) {
            None => crate::feature::KernelJob::refused(crate::errors::CoreError::ProfileNotFound),
            Some(sourced) => (|| {
                // Honest diagnostics before the kernel: a profile crossing the axis produces readable
                // text with a hint rather than a faceless "revolve failed".
                //
                // The axis in sketch local space is resolved once and serves both the check and the
                // kernel call. Writing the priority (centreline, then datum through the inverse
                // placement, then the X or Y fallback) twice — once for diagnostics and once for the
                // call — lets the copies drift, and the "profile crosses the axis" check would then
                // validate an axis other than the one the body is built about, lying silently.
                // The axis check runs over every contour rather than one: the command takes them all,
                // and a second contour crossing the axis fails the operation just as the first would.
                let chk: Vec<Id> = if profiles.is_empty() { self.sketch_index(sketch).map(|si| self.sketches[si].contour_ids.clone()).unwrap_or_default() } else { profiles.to_vec() };
                for cid in chk {
                    if let Some(xy) = self.contour_profile_xy(cid) {
                        if let Some(msg) = self.revolve_profile_crosses_axis(&xy, ax_o, ax_d) {
                            return crate::feature::KernelJob::refused(msg);
                        }
                    }
                }
                let line = named.then_some(crate::feature::AxisLine { origin: ax_o, dir: ax_d });
                let about = crate::feature::RevolveAbout { axis, line };
                let place = with_start(ax_o, ax_d, pl);
                let inputs = if src != 0 { vec![src] } else { Vec::new() };
                crate::feature::KernelJob::new(inputs, move |k| {
                    k.revolve_region_multi(BodyOp { src, op, body }, &sourced.profiles, about, angle, place, crate::feature::Sides { feature, sources: &sourced.sources })
                })
            })(),
        }
    }

    /// Sweep: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_sweep(&mut self, p: &mut Pass, prof: Profile, path_sketch: Id, path: Id, bo: BodyOp) -> crate::feature::KernelJob {
        let (Profile { sketch, profiles }, BodyOp { src, op, body }) = (prof, bo);
        // The profile and the path are different sketches, each with its own placement (the frame
        // of its plane).
        let prof_pl = self.sketch_place(sketch);
        let path_pl = self.sketch_place(path_sketch);
        let pth = self.sweep_path_encoded(path_sketch, path);

        let feature = p.node;
        match (self.encode_profiles_sourced(sketch, profiles, &[]), pth) {
            (Some(sourced), Some(pth)) => {
                let inputs = if src != 0 { vec![src] } else { Vec::new() };
                crate::feature::KernelJob::new(inputs, move |k| {
                    k.sweep_multi(BodyOp { src, op, body }, &sourced.profiles, prof_pl, &pth, path_pl, crate::feature::Sides { feature, sources: &sourced.sources })
                })
            }
            (None, _) => crate::feature::KernelJob::refused(crate::errors::CoreError::SweepProfileMissing),
            (_, None) => crate::feature::KernelJob::refused(crate::errors::CoreError::SweepPathMissing),
        }
    }

    /// Combine: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_combine(&mut self, p: &mut Pass, prof: Profile, span: CombineSpan, t: BodyOp) -> crate::feature::KernelJob {
        let CombineSpan { height, down, extent, from, fill } = span;
        let (sketch, profiles) = (prof.sketch, prof.profiles);
        let (src, op, body) = (t.src, t.op, t.body);
        let pl = self.sketch_place(sketch);
        let h0 = p.dim("height", height).abs();
        let down = p.dim("down", down);
        self.regen_warnings.remove(&p.node); // the faces-met note of an earlier pass does not survive a rebuild
                                             // "Through all" spans everything in the way, so a start face would be ignored, not honored: the
                                             // pair is refused rather than half-kept.
        if extent.stop == crate::feature::Stop::Through && from.is_some() {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::ThroughWithStartFace);
        }
        // Where the sweep starts: the sketch plane, or the picked face moved onto the normal axis (see
        // `reached_face_place`, which also refuses a lost or turned face).
        let d_from = match &from {
            Some(fr) => match self.reached_face_place(fr, crate::feature::FaceEnd::Start, &pl) {
                Ok(at) => at,
                Err(e) => return crate::feature::KernelJob::refused(e),
            },
            None => 0.0,
        };
        // Extent of the tool along the normal: one named computation (see `tool_extent`), with the stop by
        // a face resolved here, because a face can refuse where a number cannot.
        let tool = match &extent.stop {
            crate::feature::Stop::UpToFace(fr) => {
                let d_stop = match self.reached_face_place(fr, crate::feature::FaceEnd::Stop, &pl) {
                    Ok(at) => at,
                    Err(e) => return crate::feature::KernelJob::refused(e),
                };
                // The band is measured along the growth: a cut whose sketch sits on the top of the body
                // grows against the normal, and its stop face lies on the far side of the plane - the
                // distance runs the other way, or every such cut would come out "crossed".
                let forward = extent.reach != crate::feature::Reach::Backward;
                let length = if forward { d_stop - d_from } else { d_from - d_stop };
                if length < -1e-9 {
                    return crate::feature::KernelJob::refused(crate::errors::CoreError::ReachedFacesCrossed);
                }
                if length <= 1e-9 {
                    // The faces met: the tool has no length. A zero-length prism handed to the kernel is a
                    // degenerate answer, so the source is passed through unchanged, the node stays well, and
                    // the note says why nothing was cut.
                    self.regen_warnings.insert(p.node, crate::errors::CoreError::ReachedFacesMet);
                    return crate::feature::KernelJob::new(vec![src], move |k| k.transform_body(body, src, crate::feature::PLACE_IDENTITY));
                }
                // A cut whose end lands on a face of the body meets it as a one-sided cut does (see
                // `ToolEnds::OneSidedCut`); a pad ends as drawn, a union with a coincident face having no
                // cap to leave behind. The sweep runs along the normal, so a backward growth starts at the
                // far end of its band.
                let start = if forward { d_from } else { d_from - length };
                let ends = if op == 0 { crate::feature::ToolEnds::OneSidedCut } else { crate::feature::ToolEnds::Kept };
                crate::feature::ToolSpan { start, length, ends }
            }
            _ => self.tool_extent(src, &pl, super::apply::ToolSizing { height: h0, down, extent: &extent, from_face_at: d_from }, op),
        };
        // Encode every tool contour and apply them with a single boolean
        // (`combine_region_multi`).

        let feature = p.node;
        match self.encode_profiles_sourced(sketch, profiles, fill) {
            None => crate::feature::KernelJob::refused(crate::errors::CoreError::ProfileNotFound),
            Some(sourced) => {
                let inputs = if src != 0 { vec![src] } else { Vec::new() };
                crate::feature::KernelJob::new(inputs, move |k| {
                    k.combine_region_multi(BodyOp { src, op, body }, &sourced.profiles, tool, pl, crate::feature::Sides { feature, sources: &sourced.sources })
                })
            }
        }
    }

    /// WHERE A REACHED FACE STANDS along the normal of the sketch placement `pl`: the signed distance
    /// from the plane to the face, so a face below the plane can serve as a start. The face is resolved
    /// by its recipe against the live rebuild of its body and checked for parallelism within
    /// [`crate::feature::REACHED_FACE_PARALLEL_TOL_DEG`]; anything else is a refusal naming the face
    /// (`end` says which of the two it is).
    pub(crate) fn reached_face_place(&self, fr: &crate::feature::FaceRef, end: crate::feature::FaceEnd, pl: &[f64; 12]) -> Result<f64, crate::errors::CoreError> {
        let what = end.words(crate::feature::EndWords { start: "ref-what-start-face", stop: "ref-what-stop-face" });
        let cand = self.resolve_face_ref(fr.body, &fr.face, what).map_err(|e| match e {
            crate::refs::RefError::Lost { was, .. } => crate::errors::CoreError::ReachedFaceLost { end, was },
            crate::refs::RefError::Ambiguous { .. } => crate::errors::CoreError::ReachedFaceLost { end, was: crate::refs::Fingerprint::default() },
        })?;
        let n = [pl[2], pl[6], pl[10]];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12);
        let m = cand.normal;
        let mlen = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt().max(1e-12);
        let dot = (n[0] * m[0] + n[1] * m[1] + n[2] * m[2]) / (len * mlen);
        if dot.abs() < crate::feature::REACHED_FACE_PARALLEL_TOL_DEG.to_radians().cos() {
            return Err(crate::errors::CoreError::ReachedFaceNotParallel { end });
        }
        let o = [pl[3], pl[7], pl[11]];
        Ok(((cand.centroid[0] - o[0]) * n[0] + (cand.centroid[1] - o[1]) * n[1] + (cand.centroid[2] - o[2]) * n[2]) / len)
    }
}
