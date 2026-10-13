## 1. Red checks first (core)

- [x] 1.1 In `qymcad-core/tests` (the `regions`/`feature` style): extrude a rectangle, then draw a
  line across it through the model's sketch-edit door, and assert the extrude's `profiles` hold
  BOTH halves and the resolved selection's area equals the rectangle's. Red before the fix (today
  it holds one half).
- [x] 1.2 The same check for a circle drawn wholly inside the selected region: the selection holds
  the ring and the disc, the resolved area is unchanged, and the circle is not a hole. Red before
  the fix.
- [x] 1.3 The gate: a boundary of the region moved so its area changes - no rewrite, the feature
  follows its region; geometry added outside the selected area enters no selection. Red where it
  applies (the moved boundary is today's behaviour, the assertion pins it).
- [x] 1.4 The merge: after the subdivision of 1.1, delete the dividing line and assert the
  selection collapses back to the one region. Red before the fix.
- [x] 1.5 The fill rule: an ellipse drawn inside the selected region is marked in `fill` (and in
  `profiles` for a revolve), keeping the resolved area; an ellipse nested-but-preexisting stays a
  hole. Red before the fix.
- [x] 1.6 The write-back: re-editing the node after a subdivision shows the widened selection in
  the command; undoing the subdividing edit restores the single original reference.
- [x] 1.7 Each covered kind in its own check: a `Revolve` and a `Sweep` over a subdivided region
  follow the cover; a `Combine` cut (the extrude command against an existing body) follows it the
  same. Red before the fix.
- [x] 1.8 The loft boundary, pinned: a subdivided loft section keeps following its one contour
  (today's behaviour, asserted so it is a decision, not an accident); a loft section whose contour
  no longer resolves is reported as a lost profile beside the loft's refusal.

## 2. The cover rewrite (core)

- [x] 2.1 Capture, at entry to `replace_contours` (`model/sketch.rs`), the full point polygons of
  the old contours referenced by timeline features (the ids, not the signature summaries), so the
  cover is computed against what the region was before the overwrite.
- [x] 2.2 The cover predicate beside it, as its own method: candidates are the new closed regions
  whose polyline vertices all lie in or on the old polygon (box prefilter on `span`, centroid
  next); it fires only when the candidates' total area equals the old region's within the measured
  relative tolerance, pinned with a comment naming the cases it was measured on.
- [x] 2.3 The merge direction in the same method: a new region whose contained dead old regions
  sum to its area exactly takes over their references.
- [x] 2.4 The timeline walk (D4): every `Extrude`/`Combine`/`Revolve`/`Sweep` over the sketch -
  suppressed nodes included - with one helper lending the mutable reference lists of a kind;
  rewritten lists stay sorted and deduped, as `profile_faces` expects.
- [x] 2.5 Call it at the end of `replace_contours`, after `rebuild_contour_nesting` and the
  removal of dead ids, so every re-derivation path (`regen_sketch`, `regen_sketch_whole`,
  `regen_sketch_moved`, `loops_round`) passes through it.

## 3. The fill rule (core)

- [x] 3.1 After the cover rewrite: a closed contour with a fresh id, nested strictly inside a
  selected region and outside its holes (`contour_parent`), joins `Extrude.fill` - `profiles` for
  the kinds without a fill list. Contours that survived by id and only moved in are left alone.

## 4. The lost-profile warning (core, i18n)

- [x] 4.1 `CoreError::ProfilesDropped { asked, dropped }` beside `EdgesDropped`, with its text key
  in the catalogue (`i18n/`), no hard-coded string.
- [x] 4.2 In the feature prep of the rebuild path (`prep_extrude`, `prep_combine`, `prep_revolve`,
  `prep_sweep`, `prep_loft`), compare the stored selection with what resolution returned: every
  stored non-zero id that resolved to nothing records the warning in `regen_warnings`; the
  empty-means-first-contour convention stays silent; the map clears on a clean rebuild as it
  already does. Red check: delete the boundary of a selected region and read the node's warning,
  then re-pick a live region and read it gone; the same for a loft section, beside the loft's own
  refusal.

## 5. Associativity under the real kernel

- [x] 5.1 In `qymcad-testkit/tests` (the `matrix_extrude` style, live OCCT, headless): extrude a
  region, measure the body's volume, subdivide the region with a line and rebuild - the volume is
  unchanged; delete the line - unchanged again; a circle drawn inside changes nothing either. The
  same measurement runs through a revolve and through the extrude command's boolean cut
  (`Combine`). Red before the fix.

## 6. Acceptance and end-to-end

- [x] 6.1 An acceptance step beside the extrude scenario in
  `qymcad-acceptance/tests/acceptance/part_from_sketch.rs`, driven through the person's path
  (`Session`/`Hand`): the sketch is re-entered, a line is laid across the profile, the session
  closes, and the body keeps its shape; the tree re-edit of the extrude shows both regions.
- [x] 6.2 A `user_case` step in the scenario chain: the extruded shape of the chain gets a
  dividing line laid in its sketch, `check_all` after the edit session closes, and the picture is
  looked at through `rasterize_3d`.

## 7. Gate and commit

- [x] 7.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets`,
  `cargo test --workspace` and `cargo check --workspace --all-targets` all clean (exit codes read,
  not piped); `tools/gate.py` passes; the drag-frame cost of the rewrite measured on the
  many-lines sketch and within bounds.
- [x] 7.2 Commit series on `main` in the house style (`feat(part): ...` or
  `feat(associativity): ...`), each commit with its red-turning-green check in the same layer.
