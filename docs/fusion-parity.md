# Reproducing a Fusion assembly in QymCAD over the agent's server

Working notes from reproducing the "Air Washer V3" assembly (127 occurrences, 21 unique
components, 51 sketches, 77 extrudes) from the Fusion document's own edit history, driven
wholly through the MCP server. Three lists: what Fusion has that QymCAD serves differently
or not at all (with the alternative each case used), the pre-existing problems the run
found and fixed, and the fixes and operations this session added.

## Fusion operations with no direct QymCAD equivalent

| # | Fusion side | QymCAD side | Alternative used |
| --- | --- | --- | --- |
| 1 | Construction sketch curves (`isConstruction`) | supported in the window ("Construction geometry" toolbar toggle, key X), but no `call-tool` sketch operation carries a construction flag | press the toggle by its word through `ui.button`, draw, press again; or skip the curve — a construction curve bounds no profile, so the solid is identical (the chosen route) |
| 2 | Extrude extent "to entity" (up to a face) | extrude serves To a length / Symmetric / Two sides / Through all | the distance measured off the feature's final bounding box in the source document; Through all where the cut goes through |
| 3 | One component placed many times at irregular places (the connector pads: mixed z offsets) | component patterns cover regular grids only; no duplicate-or-place of an existing part | same-shaped separate parts drawn at each place; a pattern plus a gizmo move is the person's route, the gizmo is not reachable over the server |
| 4 | Move/rotate features on bodies (the gears' `MoveFeature`) | parts move by gizmo or Shift-drag; no named operation moves a body | draw the geometry at its final place |
| 5 | Combine across components (cut/keep tool bodies from other components) | booleans work within a part; a combine that consumes another component's bodies has no path | the final bodies rebuilt directly from the sketches that produced them |
| 6 | Exact geometry by construction | geometry drawn by clicks carries pixel-level slop | exact sizes typed into the value fields at the geometry (`ui.field`), or laid as dimensions; "Snap" turned off by its word before drawing decimal coordinates |
| 7 | Thread specs read off the feature (`threadInfo`) | threads size themselves to the picked cylinder | the cylinder's own diameter is the spec |
| 8 | Per-sketch user parameters driving curves | QymCAD parameters drive dimensions | the source document held none; numbers were baked into the drawn geometry |

Gesture flows the run established for a client (no app change needed):

- the extrude ladder: take (`part.extrude`) -> click the contours (a pair `[x, y]` on the
  flat sheet, a triple `[x, y, z]` in the 3D view) -> Enter sets the size -> `ui.field`
  Length -> Enter applies;
- the crash notice ("The last run ended in an error") closes by its "Close" word through
  `ui.button` — refused means nothing stands; detection and dismissal in one gesture;
- an occluded window starves the frames (macOS suspends painting): calls then answer
  "frames are not running" and are dropped; bring the window frontmost and re-issue.

## Pre-existing problems found by the run

Fixed in this session (commit `6998752` and the one beside it):

- a click of the model's space off the 3D view — greeting screen, a sketch's flat sheet —
  brought the program down from a server call: the place machinery raises (the canvas, a
  cover, a hidden point) and the raise unwound through the pump into the frame. The
  `ui.click` gesture now takes the raise and answers it in words;
- a call the asking side gave up waiting for (150 s, the frames never came) stayed in the
  queue and ran when a frame finally came — a gesture out of the context it was called
  into (the crash above was one such, landing minutes later). `hand_over` hands back the
  word of abandonment; a timed-out call is dropped unread;
- the flat sheet with no open sketch — the extrude's contour picking — was unreachable:
  a pair was refused ("no sketch is open"), a triple too ("the canvas is not the 3D
  view"). `AgentSession::click_on_sheet` clicks wherever the flat sheet stands.

Known, routed around, not yet fixed:

- `bring_into_view` (session.rs) panics when its wheel-and-drag heuristic cannot converge
  — reached when the window's frames are starved mid-gesture. Keeping the window
  frontmost avoids it; the heuristic itself is future work;
- the take of a bar command answers as a refusal ("nothing was made: Not yet ...") while
  the bar stays in hand — the words are right, the shape is a design question for the
  registry.

## Geometry by value - the one capability the reproduction still waits on

The whole chain is proven end to end on real parts: sketches drawn with retries and
stepped-back-into, curves trimmed at their true intersections, the extrude ladder (take ->
click the contours -> Enter sets the size -> Length -> Enter), layered extrusions over one
sketch through `ui.open`, the Flip side, per-part volume and bounding-box checks against the
source document's own numbers. A three-layer ring built this way came out exact: volumes in
the ratio 1:2:3 and the bounding box at -1..2 mm along the axis, as the source has it.

What still fails: profiles made of many curves that must meet EXACTLY - the finned disk's 30
flank arcs among three rings. Curves drawn by clicks carry pixel-level slop; snapping rounds to
the grid instead of onto the curve; trimming heals crossings exactly, but a trim click names its
piece by position, so it needs the drawn geometry to be where the call says it is. The missing
piece is entry of sketch geometry BY VALUE - a circle by its centre and radius, an arc by its
centre, radius and two angles, as the numbers the source document already holds - or a read-back
of the drawn curves so a client can aim its trims at what actually stands. Either one unblocks
the disk and every part like it; the simple rectilinear parts (boxes, connectors, pads, legs)
do not need it - their lines and circles meet at shared endpoints that the vertex snap closes.

## Fixes and operations added this session

- `fix(mcp): a gesture that cannot land is refused, an untaken call is dropped unread`
  (6998752): `ui.click` panic -> refusal; abandoned calls dropped unread. Tests:
  `a_click_of_the_space_off_the_3d_view_is_refused_not_fatal`,
  `a_call_abandoned_while_it_waited_never_runs`.
- the flat-sheet click (beside it): `AgentSession::click_on_sheet`, the `ui.click` sheet
  branch reaches any flat sheet and refuses in words when the canvas is the 3D view. Test:
  `a_click_of_the_flat_sheet_needs_no_open_sketch` (drives the whole contour-picking
  ladder). The refusal wording of the sheet branch changed; the wording test updated with
  it.
- no new registry operations were added; everything reached through the existing
  catalogue plus the person's gestures.
