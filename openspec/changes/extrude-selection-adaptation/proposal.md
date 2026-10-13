## Why

An extrude holds the ids of the sketch regions it extrudes. When the sketch is later edited,
`replace_contours` keeps a region's id on the one loop that best continues it and mints fresh ids
for the rest: a line drawn across an extruded region leaves the extrude holding one of the two
halves - the body is silently cut in half - and a circle drawn inside the region becomes a hole
nobody asked for. The person selected an area; what is stored is the particular tiles that covered
that area at the time, and a re-tiling of the same area breaks the selection.

## What Changes

- A feature's profile selection follows the tiles to preserve the selected area. When an edit
  subdivides a referenced region, the reference becomes references to the set of new regions that
  together cover the old one exactly. When an edit merges regions that the feature selected, the
  references become the one region that covers them exactly. Deleting a subdividing line collapses
  the selection back to the single region.
- A closed region newly created inside the selected area joins the selection as filled, so new
  geometry inside the area does not silently punch a hole in the body. This covers the curves that
  live outside the arrangement - an ellipse, a closed spline loop - which nest inside a region
  instead of subdividing it.
- The adaptation is a write-back into the feature's own parameters, done once at the edit: what the
  document saves, what the edit command shows when the node is re-opened, and what undo takes back
  together with the edit that caused it. It covers every kind whose selection holds any number of
  regions - the extrude (`Extrude.profiles` and `Extrude.fill`), its boolean form against an
  existing body (`Combine`, the same two lists, which is what the extrude command creates when the
  part is not empty), the revolve and the sweep (`profiles`) - each with its own checks.
- Exactness gates everything: no exact cover, no rewrite. Geometry outside the selected area never
  enters the selection, and a genuine shape change - a moved boundary, a deleted side - is not
  overridden; the feature follows its region as it does today.
- When a stored profile no longer resolves and no cover fired, the rebuild keeps today's fallback
  (the first closed region) but reports the loss as a warning on the node, the way a partly lost
  edge reference is reported today.
- A loft section is a single contour, and a re-tiled area cannot be folded back into one contour
  reference: the loft keeps following its section contour, and a section that no longer resolves is
  reported as a lost profile. Stitching a covered area back into one section wire is a follow-up of
  its own, not a quiet shortcut here.
- The rule is one for every kind it covers; the checks name the extrude first, then the revolve,
  the sweep and the extrude's boolean form, each in their own right.

This is a best-effort feature by design: where the original area cannot be reconstructed from the
new tiles, the selection is left alone and the loss is said, rather than guessed at.

## Capabilities

### New Capabilities

- `profile-selection-continuity`: how a feature's selected sketch area survives edits to the
  sketch - the subdivision and merge covers, the fill for new nested regions, the exactness gate,
  the write-back into the feature's parameters, and the reported loss when no cover was possible.

### Modified Capabilities

None - the repository has no existing specs for features or sketches.

## Impact

- `crates/qymcad-core`: the cover, merge and fill rules land in `src/model/sketch.rs` beside
  `replace_contours`, which stays the one door every re-derivation of a sketch's regions passes
  through; the lost-profile warning lands in the rebuild path (`src/model/regen.rs`) where
  `profile_faces` drops unresolvable ids today; a `CoreError` variant with its text key.
- `i18n/`: one catalogue entry for the warning.
- Checks: red-first core checks in `qymcad-core/tests` (the `regions` and `feature` style), one
  per covered kind - extrude, combine, revolve, sweep - plus the loft boundary; an associativity
  check in `qymcad-testkit/tests` (the body's volume unchanged through the re-tile, under the real
  OCCT kernel, through an extrude and a revolve), an acceptance step through the extrude contract's
  scenario, and a `user_case` step with `check_all` and a look at the picture.
- No document-format change: `profiles` and `fill` already persist; the rewrite only grows and
  shrinks them. Old files get the behaviour going forward, with no migration.
