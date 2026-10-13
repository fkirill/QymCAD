## Purpose

What a feature's selected sketch area is worth when the sketch is edited: the selection names an
area of the sketch, and it is the area - not the particular regions that tiled it at the moment of
selection - that survives an edit which re-tiles the sketch.

## ADDED Requirements

### Requirement: The selection follows a subdivision of a selected region

When an edit to a sketch divides a region a feature has selected into several regions, the
feature's stored selection SHALL replace the reference to the divided region with references to
every new region that together make up the old one: each of the new regions lies inside the old
one, and their combined area equals the old region's area within tolerance. The built body SHALL
keep its cross-sectional shape and volume through the edit.

#### Scenario: A line across an extruded rectangle

- **WHEN** a rectangle is extruded and a line is then drawn in the sketch from one side of the
  rectangle to the opposite side, dividing it into two regions
- **THEN** the extrude holds both regions in its selection and the body is the same box as before
  the edit

#### Scenario: A circle drawn inside the selected region

- **WHEN** a circle is drawn wholly inside a selected region
- **THEN** the region divides into the ring around the circle and the disc inside it, both enter
  the selection, and the body stays solid - the new circle does not become a hole

#### Scenario: One cut through several selected regions

- **WHEN** a single line crosses two regions of one feature's selection
- **THEN** each divided region is followed by its own cover and the selection holds every
  resulting region

### Requirement: The selection follows a merge of selected regions

When an edit merges regions and the feature's selection holds those regions together, the
references to them SHALL become one reference to the region that now covers exactly their combined
area.

#### Scenario: Deleting the dividing line

- **WHEN** the line that divided a selected region is deleted and the parts merge back into one
  region
- **THEN** the selection collapses to that one region and the body is unchanged

#### Scenario: A selection that held only part of the merge

- **WHEN** regions merge and the feature's selection held only some of them, so no exact cover of
  the merged region exists
- **THEN** the references are not rewritten and the loss is reported as a lost profile

### Requirement: An exact cover is the only trigger

The rewrite SHALL happen only when the new regions cover the old selected area exactly. A partial
cover SHALL NOT rewrite the selection, and geometry that lies outside the selected area SHALL NOT
enter it. An edit that genuinely changes the selected region's own shape - a moved boundary - SHALL
be followed by the feature as it is today, changing the body with it.

#### Scenario: New geometry beside the selection stays out

- **WHEN** geometry is added that creates new regions outside the selected area
- **THEN** the selection and the body are unchanged

#### Scenario: A moved boundary changes the body

- **WHEN** a boundary of the selected region is moved so that the region's area changes
- **THEN** no rewrite happens, the feature follows its region, and the body changes with it

#### Scenario: A partial cover does not fire

- **WHEN** an edit leaves the new regions inside the old one covering less than the whole of it,
  because part of the old area was taken out of the region
- **THEN** the rewrite does not fire and the feature keeps following its region

### Requirement: New geometry inside the area does not become a hole

A closed region newly created inside the selected area SHALL join the selection as filled, so the
body keeps its cross-sectional area. A region that existed before the edit and was left unselected
- a deliberate hole - SHALL stay a hole. A region that only moved into the selected area during the
edit keeps today's behaviour.

#### Scenario: An ellipse inside the selected region

- **WHEN** an ellipse is drawn wholly inside a selected region, where it nests inside the region
  rather than dividing it
- **THEN** it is marked filled on the feature and the body stays solid

#### Scenario: A deliberate hole stays a hole

- **WHEN** the selection was made over a region with a nested region deliberately left unselected
  and an unrelated edit happens to the sketch
- **THEN** the nested region stays out of the selection and the body keeps its hole

### Requirement: The adaptation is written into the feature, once, for every kind

The rewrite SHALL be a change to the feature's own stored parameters, made once when the sketch
edit happens, for every feature whose selection holds any number of regions of that sketch - an
extrude and its boolean form against an existing body, with their filled contours, a revolve, a
sweep. The rewritten selection SHALL be what the document saves and what the feature's edit command
shows when the node is re-opened. Undoing the edit SHALL take the rewrite back with it. A loft
section holds a single contour, which a re-tiled area cannot be folded into: the covers do not
rewrite loft sections, and a lost section is reported as a lost profile.

#### Scenario: Re-editing shows the widened selection

- **WHEN** the node is re-opened for editing after a region it selected was divided
- **THEN** the selection offered is the divided parts, not only one of them

#### Scenario: Undo takes the adaptation back

- **WHEN** the subdividing edit is undone
- **THEN** the feature's selection is the original single region again

#### Scenario: A revolve follows its region

- **WHEN** a region selected by a revolve is divided by a new line
- **THEN** the revolve follows the parts the same way an extrude does

#### Scenario: A sweep follows its region

- **WHEN** a region selected as a sweep's profile is divided by a new line
- **THEN** the sweep holds the parts in its selection, as an extrude does

#### Scenario: The extrude's boolean form follows its region

- **WHEN** a cut or boss made by the extrude command against an existing body has its selected
  region divided
- **THEN** the tool follows the parts, and the cut or boss keeps its shape

### Requirement: A lost profile is reported

When a stored selection entry resolves to no region after an edit and no cover was possible, the
rebuild SHALL keep today's outcome - the fallback selection where the kind has one (the sketch's
first closed region), the refusal where it does not - and SHALL record a warning on the feature's
node, shown where node state is shown. The warning SHALL clear once the selection resolves again.

#### Scenario: The region's boundary is deleted

- **WHEN** entities are deleted so that a selected region opens and no longer exists
- **THEN** the node carries a lost-profile warning and the body keeps today's fallback behaviour

#### Scenario: A lost loft section is reported

- **WHEN** a loft section's contour no longer resolves after an edit
- **THEN** the node reports the lost profile beside the refusal it already gives

#### Scenario: The warning clears on repair

- **WHEN** the feature is re-edited and a live region is selected
- **THEN** the warning is gone
