## Context

The features that select sketch regions store the ids of the sketch's contours
(`crates/qymcad-core/src/feature.rs`): `Extrude` holds `profiles` and `fill`, and so does
`Combine` - the extrude command's boolean form against an existing body, created when the part is
not empty; `Revolve` and `Sweep` hold `profiles` and resolve through the same
`encode_profiles_role` -> `profile_faces` path (with their own face-name roles `Revolved` and
`Swept`). `Loft` holds one contour id per section (`contours[i]` of `sketches[i]`, resolved by
`loft_section_contour` to a single closed loop). Contours are re-derived on every sketch edit by
the planar arrangement (`arrangement_regions_prov`, `model/tess.rs`) and matched to the old ones by
`Project::replace_contours` (`model/sketch.rs`): provenance first, exact signature after, nearest
centroid last - one old id lands on ONE surviving loop, and every other tile of a re-tiled area
gets a fresh id. A line drawn across a selected region therefore leaves the feature holding one
half; the other half is new and unreferenced. At resolution time `Project::profile_faces` silently
drops ids that no longer resolve and falls back to the sketch's first closed contour.

Two further facts shape the design. Construction entities never enter the arrangement
(`tess.rs` skips `e.construction`), so whatever subdivides a region is ordinary geometry. Ellipses,
spline loops and text glyphs are contours that live OUTSIDE the arrangement graph: a new one
inside a selected region does not subdivide it but nests inside it, and nesting children are read
as holes.

Precedents to stay inside: `rebind_lost_face_refs` repairs a lost reference by writing the found id
back once, during the pass that detects the loss, and reports it in `RegenReport`; `regen_warnings`
(`Map<Id, CoreError>`, persisted, cleared on a clean rebuild) already carries
`CoreError::EdgesDropped { asked, dropped }`.

## Goals / Non-Goals

**Goals:**

- The feature's selected AREA survives a re-tiling of the sketch: subdivision, merge, and new
  nested regions inside the area.
- One rule for every profile-holding feature kind; the write-back is transactional with the edit
  (undo, save).
- The whole computation stays in `qymcad-core` plane arithmetic - no kernel calls, no new file
  format.

**Non-Goals:**

- No change to the empty-selection convention (empty or zero means the first closed contour) and
  no change to the fallback body that today's silent drop produces.
- No healing of other reference kinds (sketch base faces and live edges have their own machinery),
  none for single-curve references (a sweep's `path`, a revolve's `axis_line`), and none that folds
  a covered loft section back into one wire - the follow-up named in D4.
- No approximate or partial covers: where the area cannot be reconstructed exactly, nothing is
  rewritten and the loss is only reported.
- No new interface beyond the warning string: no new buttons, panels or fields.

## Decisions

### D1: Adapt at edit time, inside the one door that re-derives regions

The cover can only be computed where both sides of the edit are known: the old region's polygon
before it is overwritten, and the new tiles after. That place is `replace_contours`, which every
re-derivation passes through (`regen_sketch`, `regen_sketch_whole`, `regen_sketch_moved`,
`loops_round`). A new method beside it - called at its end, after nesting is rebuilt and dead ids
removed - walks the timeline for features over that sketch holding region references, computes the
covers against polygons captured at entry, and writes the result into the feature's own vectors.
`replace_contours` itself gains only the capture and the call; the matching logic stays out of it.

Alternative considered: heal lazily at rebuild time in the `prep_node` arms. Rejected - by then the
old geometry is gone, so the feature would have to carry the dead polygon itself (a format change,
and stale the moment a second edit happens), and the guess would re-run on every rebuild. The
`rebind_lost_face_refs` lesson applies: repair once, where the loss is first seen, and write it
back.

The undo audit rule ("undo is bound to an operation, not to a frame") makes the rewrite
transactional for free: it happens inside the same core call as the entity edit, within the same
snapshot.

### D2: The cover predicate is geometric, with an exactness gate

For a referenced old region O that did not survive identically, the candidate tiles are the new
closed regions whose polyline vertices all lie in or on O's polygon (bounding-box prefilter on the
contours' `span` boxes, centroid-inside next). The rewrite fires only when the candidates' total
area equals O's area within a relative tolerance. The arrangement's minimal faces are pairwise
interior-disjoint, so the sum of the tile areas is the area of their union, and area equality plus
containment is an exact cover. The merge direction is the same predicate seen from the other side:
for a new region N, the dead old regions contained in it fire when their total area equals N's.
The tolerance is measured on the ring-and-line cases during implementation and pinned with a
comment (expected order `1e-6` relative - the tiles come from the same tessellation machinery - but
arc tessellation density differs between passes, so the number is measured, not assumed).

Alternative considered: matching by provenance subsets (boundary entity sets). Rejected: an
existing line extended across the region changes the boundary sets in ways that do not compose,
while the geometry still tiles exactly; geometry is the ground truth for "the same area". Also
rejected: point-set booleans or kernel face comparisons - plane polygon arithmetic in the sketch's
own 2D coordinates is enough and keeps the work out of OCCT and its lock.

Performance: only regions referenced by features enter the computation (a handful), each scanning
the sketch's closed faces with a box prefilter - the same class of cost as one `contour_under_2d`
pick. During drags `loops_round` re-derives only around the change, and the rewrite is idempotent:
once the references point at the current tiles, nothing fires.

### D3: Non-arrangement curves inside the area are filled, not holed

After the cover rewrite, for each region in a feature's selection: a closed contour that received
a FRESH id this edit (it did not exist before), whose centroid lies strictly inside the region and
outside its holes (the `contour_parent` nesting map), joins the selection - into `Extrude.fill`,
into `profiles` for the kinds without a fill list. Reading it as one more selected region gives
exactly the old area back: the parent contributes its ring, the nested loop its interior, and the
kernel fuses them (the documented nesting-compose rule of `profile_faces`).

Fresh ids only, deliberately: a contour that survived by its id and merely moved into the area
keeps today's hole semantics. Moved geometry is an intentional shape change; filling it would
override a decision the person can see. Drawing new geometry inside is the case the request names,
and there "preserve the area" is the wanted default.

### D4: Every multi-region kind, suppressed ones included - and why the loft is out

The timeline walk matches `Extrude { sketch, profiles, fill, .. }`, `Combine { sketch, profiles,
fill, .. }`, `Revolve { sketch, profiles, .. }` and `Sweep { sketch, profiles, .. }`; one small
helper that lends the mutable reference lists of a kind keeps the arms uniform. Suppressed features
heal too: their parameters are live document state, and unsuppressing later should not resurrect a
stale selection.

`Loft` is deliberately absent from the walk. A loft section is ONE contour -
`loft_section_contour` resolves `contours[i]` to a single closed loop, and the kernel lofts one
wire per section - so a cover's several tiles cannot be written into the reference. Expressing the
covered area as one section would take a union contour stitched from the tiles' outer edges (a
derived contour in the pool, re-derived by the healing at every edit) plus an encoder change; that
is a follow-up of its own, not a quiet shortcut here. What the loft does get in this change is the
loss report: a section id that no longer resolves is a lost profile like any other (D5). The same
boundary applies to the sweep's `path` and the revolve's `axis_line` - single-curve references, not
area selections, and a lost path or axis line already refuses visibly (`SweepPathMissing` and the
axis diagnostics).

### D5: The loss warning is emitted at rebuild time

Where ids are dropped silently today is where the warning belongs: the rebuild's feature prep
compares the stored selection with what `profile_faces` resolved and records
`CoreError::ProfilesDropped { asked, dropped }` in `regen_warnings` - the `EdgesDropped` pattern
in the same file, text key through the `i18n/` catalogue. The arms are `prep_extrude`,
`prep_combine`, `prep_revolve` and `prep_sweep`, which all resolve through the `profile_faces`
family, and `prep_loft`, where a section id that `loft_section_contour` cannot resolve is the
lost entry (the loft's existing refusal stands beside it). The warning clears on any later clean
rebuild of the node, which is the existing behaviour of the map. It fires only for stored non-zero
ids that resolved to nothing; the empty-means-first-contour convention is not a loss and stays
silent.

Edit time (D1) and rebuild time (D5) divide the work this way on purpose: the cover needs both
geometries, which exist only at the edit; the report needs the feature's own resolution, which
happens only at the rebuild.

### D6: No format change, no migration

`profiles` and `fill` already serialize in the feature node; contour provenance already persists.
The rewrite only grows and shrinks those vectors, so old files gain the behaviour going forward
with no migration (the house rule). Rollback is reverting the commits.

## Risks / Trade-offs

- [Old and new areas agree only to tessellation error] -> the gate is relative and the number is
  measured on the ring, rectangle and arc cases before landing, then pinned with a comment.
- [A subdivision the person wanted as a notch is preserved instead] -> the default the request
  asks for; the escape is re-editing the feature and dropping the unwanted tiles, which the
  widened selection makes visible (D1 writes into the feature's parameters).
- [Warning noise from documents that lived on the silent fallback] -> the fallback body is
  unchanged, and the warning names counts (`asked`/`dropped`) rather than failing anything.
- [`replace_contours` runs per frame during drags] -> referenced-regions-only inputs, box
  prefilter, idempotent rewrite; measured in the drag-frame cost check before landing.
- [Two features sharing one sketch cross-heal] -> each feature's selection heals independently
  against the same tile set; sharing a sketch does not couple the selections, same as today.

## Migration Plan

None beyond the code: no format change, no data migration, no interface change. Rollback is the
revert of the commit series.

## Open Questions

- Where the lost-profile warning is shown besides the node's state in the tree (status line too,
  as `EdgesDropped` does) - a placement choice that does not affect the core rule and can follow
  the existing warning plumbing wherever it already surfaces.
