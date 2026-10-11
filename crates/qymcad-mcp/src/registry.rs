//! THE REGISTRY OF OPERATIONS the agent calls by name: every command of the program's catalogue, derived
//! from it - never written beside it. A command the panel offers cannot be missing from the agent's reach,
//! and a row the catalogue never held cannot appear in it; the guard in this module's checks holds that
//! link in both directions.
//!
//! Each entry carries what a client needs to find and call an operation: its code, its names in each
//! language the program speaks, a one-line synopsis from its help article, its workbench, the words it is
//! also found by, and its run - the gestures a person would make through the same interface. The names and
//! the synopsis are read through the catalogue and the help articles as they stand, so they cannot drift
//! from what the window shows; only the synonyms are the registry's own words, for neither the catalogue
//! nor the help holds any.

/// HOW AN OPERATION RUNS: its gestures over the agent's session - the strokes a person makes through the
/// same interface - told its own code and the parameters the call carried.
pub(crate) type Run = fn(&mut qymcad::AgentSession, &str, &serde_json::Value) -> Answered;

/// WHAT A RUN ANSWERS: its outcome in words, or the program's own refusal - nothing was changed by a
/// refusal.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Answered {
    /// The operation ran; these are its words.
    Ran(String),
    /// The operation was refused, in the words the program itself says; the document stands as it did.
    Refused(String),
}

/// ONE CALL INTO AN OPERATION, from its validation to its answer: the status line as the program reports
/// it once the call has run and come to rest, the problems the post-call checks found, and the
/// operation's own outcome. A refusal carries its words instead, and nothing was changed by it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TheCall {
    /// The call ran and was answered from rest.
    Answered { status: String, problems: Vec<String>, outcome: String },
    /// The call was refused - the name unknown to the registry, a parameter the operation does not take,
    /// a value of the wrong kind, or the program's own refusal - in these words.
    Refused(String),
}

/// CALL AN OPERATION by its name, through the frame every call runs in: the parameters the call carried
/// are checked against the operation's own schema - read from the live tool, the reading leaving the
/// document as it stood - then the entry's run works its gestures over the agent's session, the program
/// is waited to rest, and the answer is the status line, the problems that stand after the call (the
/// program's own invariants, in the words the acceptance checks use), and the outcome. Every refusal is
/// said in words and leaves the document as it stood.
pub(crate) fn call(core: &mut qymcad::AgentSession, name: &str, given: &serde_json::Value) -> TheCall {
    let Some(entry) = by_code(name) else {
        return TheCall::Refused(format!("{name}: no operation of the program is named so; search-tool finds what the program offers"));
    };
    let fields = parameters(core, entry.command.code);
    if let Err(words) = checked_against(given, &fields, entry.takes) {
        return TheCall::Refused(words);
    }
    let Some(the_run) = entry.run else {
        return TheCall::Refused(format!(
            "{} ({}) is a command of the program, and the agent serves no operation for it; read-tool-help tells what it does, search-tool finds what the program offers",
            entry.command.name(),
            entry.command.code
        ));
    };
    let answered = the_run(core, entry.command.code, given);
    let status = core.status();
    match answered {
        Answered::Ran(outcome) => TheCall::Answered { status, problems: core.the_standing_problems(), outcome },
        Answered::Refused(words) => TheCall::Refused(words),
    }
}

/// THE PARAMETERS A CALL CARRIES, checked against the operation's own: every name must be one of the
/// fields its bar puts up or the run takes, every value of the kind its parameter takes - a number field
/// a number or an expression written in words, a text field words, places on the sheet as [x, y] pairs -
/// and every parameter the run takes must be carried: a drawing without its places is refused before a
/// gesture is made, the document left as it stood. What was expected is named in the refusal.
fn checked_against(given: &serde_json::Value, fields: &[Parameter], takes: &[Take]) -> Result<(), String> {
    let Some(given) = given.as_object() else {
        return Err("the parameters must be given as named fields of the operation".into());
    };
    let expected = || {
        let mut names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        names.extend(takes.iter().map(|t| t.name));
        if names.is_empty() {
            "no parameter at all".to_string()
        } else {
            names.join(", ")
        }
    };
    for (name, value) in given {
        if let Some(field) = fields.iter().find(|f| &f.name == name) {
            let wrong = match field.kind.as_str() {
                "number" => !matches!(value, serde_json::Value::Number(_) | serde_json::Value::String(_)),
                "text" => !value.is_string(),
                _ => false,
            };
            if wrong {
                return Err(format!("{name} takes a {}, and the call carried {value}", field.kind));
            }
            continue;
        }
        let Some(take) = takes.iter().find(|t| t.name == name) else {
            return Err(format!("{name}: the operation takes no parameter so; it takes {}", expected()));
        };
        if take.kind == "points" && !the_pairs(value).is_some_and(|p| p.len() >= 2) {
            return Err(format!("{name} takes places on the sheet as [x, y] pairs, two at least, and the call carried {value}"));
        }
    }
    for take in takes {
        if !given.contains_key(take.name) {
            return Err(format!("the operation takes {}, and the call carried none: it takes {}", take.name, expected()));
        }
    }
    Ok(())
}

/// THE SYNONYMS OF THE REGISTRY'S OWN, by command code: words a client looks for an operation by that
/// neither its code nor its names in any language carry (`fillet` by `round`). The registry is derived;
/// this is the one supplement it takes, and every word here must name its command's code.
const SYNONYMS: &[(&str, &[&str])] = &[("part.fillet", &["round", "rounding"]), ("part.chamfer", &["bevel"]), ("part.shell", &["hollow"]), ("part.hole", &["drill", "bore"])];

/// ONE OPERATION OF THE PROGRAM, as the agent calls it by name.
pub(crate) struct Entry {
    /// THE CATALOGUE ROW the entry is derived from: the code, the workbench, the launch - the one source
    /// the command search and the panel read.
    pub(crate) command: &'static qymcad::Command,
    /// The words the operation is also found by.
    pub(crate) synonyms: &'static [&'static str],
    /// ITS RUN over the agent's session; `None` where none is wired yet, and a call of the entry answers
    /// that no operation is served for its command.
    pub(crate) run: Option<Run>,
    /// The parameters the run takes beside the bar's own fields; empty where the run is the take.
    pub(crate) takes: &'static [Take],
}

impl Entry {
    /// THE NAMES OF THE OPERATION, one in each language the program speaks, as the catalogue takes them:
    /// the word of its own a command has one, the title of its help article otherwise.
    pub(crate) fn names(&self) -> std::collections::BTreeMap<String, String> {
        qymcad_help::languages().into_iter().map(|l| (l.clone(), self.command.name_in(&l))).collect()
    }

    /// THE ONE-LINE SYNOPSIS: the caption of the first picture of the help article, which says in one
    /// sentence what the operation is; the command's own name where the article carries no such picture.
    pub(crate) fn synopsis(&self) -> String {
        let caption = self.command.help_article().and_then(qymcad_help::article).and_then(|md| {
            qymcad_help::parse(md).into_iter().find_map(|b| match b {
                qymcad_help::Block::Image { alt, .. } => Some(alt),
                _ => None,
            })
        });
        match caption {
            Some(alt) if !alt.is_empty() => alt.iter().map(|s| s.text.as_str()).collect(),
            _ => self.command.name(),
        }
    }
}

/// ONE PARAMETER A RUN TAKES, beside the fields the command's bar puts up: a drawing takes its places,
/// a move its from and to. Named here, the parameter is required of every call and answered by
/// get-tool-schema; the run reads it already checked.
pub(crate) struct Take {
    /// The parameter's name, as the call carries it.
    pub(crate) name: &'static str,
    /// What kind of value it takes: `points` - places on the sheet of the open sketch, each an [x, y]
    /// pair.
    pub(crate) kind: &'static str,
}

/// ONE RUN OF RICHER SHAPE, wired to its command: the run that works the command through its picks and
/// values, over the take every command already has.
pub(crate) struct Wired {
    /// The command the run is wired to, by its code.
    pub(crate) code: &'static str,
    /// The run itself.
    pub(crate) run: Run,
    /// The parameters the run takes, beside the bar's own fields.
    pub(crate) takes: &'static [Take],
}

/// THE REGISTRY, derived from the command catalogue: every command of the program, in the panel's order.
/// Every entry is wired to a run - the take itself where no richer run is - so no command of the
/// catalogue can stand outside the agent's reach; the guard below fails the day one does.
pub(crate) fn the_registry() -> &'static [Entry] {
    static THE_REGISTRY: std::sync::OnceLock<Vec<Entry>> = std::sync::OnceLock::new();
    THE_REGISTRY.get_or_init(|| {
        qymcad::COMMANDS
            .iter()
            .map(|command| {
                let wired = RICHER_RUNS.iter().find(|w| w.code == command.code);
                Entry {
                    command,
                    synonyms: SYNONYMS.iter().find(|(code, _)| *code == command.code).map(|(_, words)| *words).unwrap_or(&[]),
                    run: wired.map(|w| w.run).or(Some(the_take)),
                    takes: wired.map(|w| w.takes).unwrap_or(&[]),
                }
            })
            .collect()
    })
}

/// THE RUNS OF RICHER SHAPE: the sketch operations that work by their own gestures - the drawing tools
/// each taking its places on the sheet, the dimensions measured and held at their value, the click
/// operations (trim, extend, break) working on what the places name. The rest of the operations keep
/// the take until their own runs arrive.
const RICHER_RUNS: &[Wired] = &[
    Wired { code: "sketch.line", run: draw_a_chain, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.circle", run: draw_a_circle, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.rect", run: draw_a_rectangle, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.point", run: draw_by_clicks, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.arc", run: draw_by_clicks, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.circle3", run: draw_by_clicks, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.polygon", run: draw_by_clicks, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.slot", run: draw_by_clicks, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.ellipse", run: draw_by_clicks, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.spline", run: draw_a_spline, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.text", run: write_a_text, takes: &[Take { name: "points", kind: "points" }, Take { name: "text", kind: "text" }] },
    Wired { code: "sketch.dim", run: place_a_dimension, takes: &[Take { name: "points", kind: "points" }, Take { name: "value", kind: "text" }] },
    Wired { code: "sketch.dim-angle", run: place_a_dimension, takes: &[Take { name: "points", kind: "points" }, Take { name: "value", kind: "text" }] },
    Wired { code: "sketch.dim-radius", run: place_a_dimension, takes: &[Take { name: "points", kind: "points" }, Take { name: "value", kind: "text" }] },
    Wired { code: "sketch.trim", run: trim_the_segments, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.extend", run: extend_a_line, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.break", run: break_a_line, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.project", run: project_the_edges, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.corner", run: cut_the_corner, takes: &[Take { name: "points", kind: "points" }, Take { name: "radius", kind: "text" }] },
    Wired { code: "sketch.corner-chamfer", run: cut_the_corner, takes: &[Take { name: "points", kind: "points" }, Take { name: "size", kind: "text" }] },
    Wired { code: "sketch.delete", run: delete_the_selection, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.mirror", run: reflect_the_selection, takes: &[Take { name: "points", kind: "points" }] },
    Wired { code: "sketch.offset", run: lay_an_offset, takes: &[Take { name: "points", kind: "points" }, Take { name: "distance", kind: "text" }] },
    Wired { code: "part.box", run: make_a_primitive, takes: &[] },
    Wired { code: "part.cylinder", run: make_a_primitive, takes: &[] },
    Wired { code: "part.sphere", run: make_a_primitive, takes: &[] },
    Wired { code: "part.cone", run: make_a_primitive, takes: &[] },
    Wired { code: "part.torus", run: make_a_primitive, takes: &[] },
    Wired { code: "part.prism", run: make_a_primitive, takes: &[] },
];

/// DRAW A CHAIN OF LINES: the tool taken, every place of the sheet clicked in turn, the chain ended as a
/// person ends it, with Esc. Two places make one line; more make a chain of them. The run answers only
/// for a drawing that came to be - the sketch holds more lines after it than before.
fn draw_a_chain(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the line needs its places on the sheet, two at least".into()) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to draw in - open one and draw again".to_string()) };
    take_the_command(core, entry.command);
    for (x, y) in &places {
        core.click_on_sketch(*x, *y);
    }
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && after.lines > sketch.lines => {
            Answered::Ran(format!("a chain of {} drawn through {} places of {}", places.len().saturating_sub(1), places.len(), sketch.name))
        }
        _ => Answered::Refused(format!("nothing was drawn: {}", core.status())),
    }
}

/// DRAW A CIRCLE: the tool taken, the centre clicked, the rim clicked where the circle is to reach; the
/// sketch holds one circle more after it than before.
fn draw_a_circle(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the circle needs its places on the sheet: the centre, then the rim".into()) };
    let [centre, rim, ..] = places.as_slice() else { return Answered::Refused("the circle needs two places: the centre, then the rim".into()) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to draw in - open one and draw again".to_string()) };
    take_the_command(core, entry.command);
    core.click_on_sketch(centre.0, centre.1);
    core.click_on_sketch(rim.0, rim.1);
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && after.circles > sketch.circles => {
            Answered::Ran(format!("a circle drawn about ({}, {}) through ({}, {}) of {}", centre.0, centre.1, rim.0, rim.1, sketch.name))
        }
        _ => Answered::Refused(format!("nothing was drawn: {}", core.status())),
    }
}

/// DRAW A RECTANGLE from its anchor to its corner, as the person clicks it into place; four lines more
/// in the sketch after it than before.
fn draw_a_rectangle(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the rectangle needs its places on the sheet: the anchor, then the corner".into()) };
    let [anchor, corner, ..] = places.as_slice() else { return Answered::Refused("the rectangle needs two places: the anchor, then the corner".into()) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to draw in - open one and draw again".to_string()) };
    take_the_command(core, entry.command);
    core.click_on_sketch(anchor.0, anchor.1);
    core.click_on_sketch(corner.0, corner.1);
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && after.lines >= sketch.lines + 4 => {
            Answered::Ran(format!("a rectangle drawn from ({}, {}) to ({}, {}) of {}", anchor.0, anchor.1, corner.0, corner.1, sketch.name))
        }
        _ => Answered::Refused(format!("nothing was drawn: {}", core.status())),
    }
}

/// THE SKETCH OPEN FOR EDITING, with its counts of what a drawing adds to; `None` when none is open.
fn the_open_sketch(core: &mut qymcad::AgentSession) -> Option<qymcad::SketchInfo> {
    let document = core.document();
    let name = document.editing.clone()?;
    document.sketches.iter().find(|s| s.name == name).cloned()
}

/// EVERYTHING A DRAWING ADDS TO, counted together: the points, lines, arcs, circles, ellipses,
/// splines and texts of the sheet.
fn things(s: &qymcad::SketchInfo) -> usize {
    s.points + s.lines + s.arcs + s.circles + s.ellipses + s.splines + s.texts
}

/// DRAW WITH THE TOOL OF THE CODE, its places clicked in turn: the tool taken, the fields the call
/// carried filled into its bar, every place of the sheet clicked, the drawing ended as the person ends
/// it, with Esc. Answered only for a drawing that came to be - the open sketch holds more after the
/// call than before.
fn draw_by_clicks(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused(format!("the {} needs its places on the sheet", entry.command.name())) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to draw in - open one and draw again".to_string()) };
    take_the_command(core, entry.command);
    fill_the_fields(core, given);
    for (x, y) in &places {
        core.click_on_sketch(*x, *y);
    }
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && things(&after) > things(&sketch) => Answered::Ran(format!("{} drawn of {} places of {}", entry.command.name(), places.len(), sketch.name)),
        _ => Answered::Refused(format!("nothing was drawn: {}", core.status())),
    }
}

/// DRAW A SPLINE: its nodes clicked one by one, the last one double-clicked, as the tool itself ends a
/// spline of as many nodes as a person wants.
fn draw_a_spline(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the spline needs its places on the sheet, two at least".into()) };
    let Some((last, nodes)) = places.split_last() else { return Answered::Refused("the spline needs its places on the sheet, two at least".into()) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to draw in - open one and draw again".to_string()) };
    take_the_command(core, entry.command);
    for (x, y) in nodes {
        core.click_on_sketch(*x, *y);
    }
    let at = core.on_sketch(last.0, last.1);
    core.double_click(at);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && after.splines > sketch.splines => Answered::Ran(format!("a spline drawn through {} places of {}", places.len(), sketch.name)),
        _ => Answered::Refused(format!("nothing was drawn: {}", core.status())),
    }
}

/// WRITE A TEXT: the words and the height in the bar above, then a click places it - as geometry, or
/// as a note. The words go into the field the tool's own title stands beside, the height as any field
/// of the bar; the sketch holds a text more after the call than before.
fn write_a_text(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the text needs its place on the sheet".into()) };
    let Some((at, _)) = places.split_first() else { return Answered::Refused("the text needs its place on the sheet".into()) };
    let words = given.get("text").and_then(|t| t.as_str()).unwrap_or_default().to_string();
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to draw in - open one and draw again".to_string()) };
    take_the_command(core, entry.command);
    fill_the_fields(core, given);
    let caption = core.word("tool-text");
    let writing = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        core.fill(&caption, &words);
    }));
    core.click_on_sketch(at.0, at.1);
    core.key(qymcad::Key::Escape);
    match (the_open_sketch(core), writing.is_ok()) {
        (Some(after), true) if after.name == sketch.name && after.texts + after.notes > sketch.texts + sketch.notes => {
            Answered::Ran(format!("the text {words:?} written at ({}, {}) of {}", at.0, at.1, sketch.name))
        }
        _ => Answered::Refused(format!("nothing was written: {}", core.status())),
    }
}

/// PLACE A DIMENSION and hold what it measures at the value the call carried: the tool taken, the
/// places that name what is measured clicked in turn - a line for its length, two lines for the angle
/// between them, a circle for its diameter - and the last place clicked where the dimension is to stand
/// (a diameter needs no placing click; its field is up as the circle is read). The placing focuses the
/// value's field, as it does for a person: the value typed, Enter, the tool put down. The sketch holds
/// a constraint of the dimension's own kind after it - a Distance, an AngleLines, a Diameter.
fn place_a_dimension(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else {
        return Answered::Refused("the dimension needs its places on the sheet: what it measures, then where it stands".into());
    };
    let Some(value) = the_words_of(given, "value") else { return Answered::Refused("the dimension needs the value it is to hold".into()) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to measure in - open one and measure again".to_string()) };
    let kind = match code {
        "sketch.dim-angle" => "AngleLines",
        "sketch.dim-radius" => "Diameter",
        _ => "Distance",
    };
    take_the_command(core, entry.command);
    for (x, y) in &places {
        core.click_on_sketch(*x, *y);
    }
    core.type_text(&value);
    core.key(qymcad::Key::Enter);
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && after.constraints > sketch.constraints && after.constraint_kinds.iter().any(|k| k == kind) => {
            Answered::Ran(format!("a {} of {value} placed over {} place(s) of {}", kind, places.len(), sketch.name))
        }
        _ => Answered::Refused(format!("nothing was measured: {}", core.status())),
    }
}

/// TRIM THE SEGMENTS under the carried places: the tool taken, every place clicked - the segment under
/// each is removed up to the intersections it crosses, its end pulled back to the crossing - the tool
/// put down with Esc. The run answers only for a trim that bit: the sheet is not what it was after it;
/// a segment that crosses nothing is refused in the program's own words.
fn trim_the_segments(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the trim needs its places on the sheet: a place on each segment to cut".into()) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to trim in - open one and trim again".to_string()) };
    take_the_command(core, entry.command);
    for (x, y) in &places {
        core.click_on_sketch(*x, *y);
    }
    core.key(qymcad::Key::Escape);
    let bit = |s: &qymcad::SketchInfo| {
        let mut places = s.places.to_vec();
        places.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
        (things(s), places)
    };
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && bit(&after) != bit(&sketch) => Answered::Ran(format!("{} place(s) trimmed of {}", places.len(), sketch.name)),
        _ => Answered::Refused(format!("nothing was trimmed: {}", core.status())),
    }
}

/// EXTEND A LINE to what it must reach: the line selected as a person selects it - the arrow of the
/// panel, a click on the line - the selection standing while the command is taken through the search,
/// the pointer held at what the line must reach, Enter, the tool put down. The run answers only for an
/// extension that reached: a point of the sketch stands at the target after it that stood at no such
/// place before.
fn extend_a_line(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else {
        return Answered::Refused("the extend needs its places on the sheet: the line, then what it must reach".into());
    };
    let [on, to, ..] = places.as_slice() else {
        return Answered::Refused("the extend needs two places: the line, then what it must reach".into());
    };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to extend in - open one and extend again".to_string()) };
    let arrow = core.word("tb-select-hint");
    core.press_hint(&arrow);
    core.click_on_sketch(on.0, on.1);
    take_the_command(core, entry.command);
    let at = core.on_sketch(to.0, to.1);
    core.move_to(at);
    core.key(qymcad::Key::Enter);
    core.key(qymcad::Key::Escape);
    let reached = |s: &qymcad::SketchInfo| s.places.iter().any(|p| (p[0] - to.0).abs() < 1e-6 && (p[1] - to.1).abs() < 1e-6);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && reached(&after) && !reached(&sketch) => {
            Answered::Ran(format!("the line at ({}, {}) extended to ({}, {}) of {}", on.0, on.1, to.0, to.1, sketch.name))
        }
        _ => Answered::Refused(format!("nothing was extended: {}", core.status())),
    }
}

/// BREAK A LINE at the carried place: the tool taken, the place clicked where the line is to split, the
/// tool put down. One line has become two - the sketch holds a line more after it than before.
fn break_a_line(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the break needs its place on the sheet: where the line is to split".into()) };
    let [at, ..] = places.as_slice() else { return Answered::Refused("the break needs its place on the sheet: where the line is to split".into()) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to break in - open one and break again".to_string()) };
    take_the_command(core, entry.command);
    core.click_on_sketch(at.0, at.1);
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && after.lines > sketch.lines => Answered::Ran(format!("the line at ({}, {}) broken in two of {}", at.0, at.1, sketch.name)),
        _ => Answered::Refused(format!("nothing was broken: {}", core.status())),
    }
}

/// PROJECT THE EDGES under the carried places onto the sheet as driven geometry: the tool taken, every
/// place clicked - each names an edge of the body the sketch looks at - the tool put down. What is
/// projected follows the part and cannot be dragged; the sketch holds more after it than before. A
/// sketch on a world plane has no body to look at, and the run is refused in the program's own words.
fn project_the_edges(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the projection needs its places on the sheet: a place on each edge to take".into()) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to project into - open one and project again".to_string()) };
    take_the_command(core, entry.command);
    for (x, y) in &places {
        core.click_on_sketch(*x, *y);
    }
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && things(&after) > things(&sketch) => Answered::Ran(format!("{} place(s) projected into {}", places.len(), sketch.name)),
        _ => Answered::Refused(format!("nothing was projected: {}", core.status())),
    }
}

/// CUT THE CORNER where the carried places' lines meet - rounded for the corner of the code, bevelled
/// for its chamfer: the tool taken, every place clicked into its set with Shift, the size typed into
/// the field that stands up at the corner, Enter. A rounded corner leaves an arc that was not there;
/// a bevelled one a line that was not.
fn cut_the_corner(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the corner needs its places on the sheet: the lines that meet at it".into()) };
    let (field, bevel) = if code == "sketch.corner-chamfer" { ("size", code == "sketch.corner-chamfer") } else { ("radius", false) };
    let Some(size) = the_words_of(given, field) else { return Answered::Refused(format!("the corner needs the {field} it is cut to")) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to cut a corner in - open one and cut again".to_string()) };
    take_the_command(core, entry.command);
    for (x, y) in &places {
        let at = core.on_sketch(*x, *y);
        core.click_with(at, qymcad::PointerButton::Primary, qymcad::Modifiers::SHIFT);
    }
    core.type_text(&size);
    core.key(qymcad::Key::Enter);
    core.key(qymcad::Key::Escape);
    let cut = |s: &qymcad::SketchInfo| if bevel { s.lines > sketch.lines } else { s.arcs > sketch.arcs };
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && cut(&after) => Answered::Ran(format!("a corner of {} place(s) {} of {}", places.len(), if bevel { "bevelled" } else { "rounded" }, sketch.name)),
        _ => Answered::Refused(format!("no corner was cut: {}", core.status())),
    }
}

/// DELETE WHAT THE PLACES NAME: the arrow of the panel, every place clicked - the first picked, the
/// rest added with Shift - and the command taken, which deletes what stands selected. The sketch holds
/// less after it than before; places that name nothing delete nothing, and the run is refused.
fn delete_the_selection(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the delete needs its places on the sheet: a place on each thing to go".into()) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to delete in - open one and delete again".to_string()) };
    take_the_arrow(core);
    pick_the_places(core, &places);
    take_the_command(core, entry.command);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && things(&after) < things(&sketch) => Answered::Ran(format!("{} place(s) deleted of {}", places.len(), sketch.name)),
        _ => Answered::Refused(format!("nothing was deleted: {}", core.status())),
    }
}

/// REFLECT WHAT THE FIRST PLACES NAME about the line the last place names: the arrow of the panel, the
/// what picked - the first place picked, the rest added with Shift - the command taken (a mirror with
/// its what standing asks for its axis, as the person's second press does), the axis clicked. A
/// reflection of what stood is in the sketch after it that was not before.
fn reflect_the_selection(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else {
        return Answered::Refused("the mirror needs its places on the sheet: what to reflect, then the axis it reflects about".into());
    };
    let Some((axis, what)) = places.split_last() else {
        return Answered::Refused("the mirror needs the axis it reflects about: what to reflect, then a place on the axis".into());
    };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to mirror in - open one and mirror again".to_string()) };
    take_the_arrow(core);
    pick_the_places(core, what);
    take_the_command(core, entry.command);
    core.click_on_sketch(axis.0, axis.1);
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && things(&after) > things(&sketch) => Answered::Ran(format!("{} place(s) reflected about ({}, {}) of {}", what.len(), axis.0, axis.1, sketch.name)),
        _ => Answered::Refused(format!("nothing was reflected: {}", core.status())),
    }
}

/// LAY A COPY of what the first places name at the distance the call carries, on the side the last
/// place names: the arrow of the panel, the chain picked - the first place picked, the rest added with
/// Shift - the command taken, the side clicked to fix where the copy falls, the distance typed, Enter.
/// The copy is in the sketch after it that was not before.
fn lay_an_offset(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else {
        return Answered::Refused("the offset needs its places on the sheet: the chain to copy, then the side the copy falls on".into());
    };
    let Some(distance) = the_words_of(given, "distance") else { return Answered::Refused("the offset needs the distance the copy stands at".into()) };
    let Some((side, chain)) = places.split_last() else {
        return Answered::Refused("the offset needs the side the copy falls on: the chain to copy, then a place on that side".into());
    };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to offset in - open one and offset again".to_string()) };
    take_the_arrow(core);
    pick_the_places(core, chain);
    take_the_command(core, entry.command);
    core.click_on_sketch(side.0, side.1);
    core.type_text(&distance);
    core.key(qymcad::Key::Enter);
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && things(&after) > things(&sketch) => Answered::Ran(format!("{} place(s) offset by {distance} of {}", chain.len(), sketch.name)),
        _ => Answered::Refused(format!("no copy was laid: {}", core.status())),
    }
}

/// MAKE A PRIMITIVE of the code's own shape - a box, a cylinder, a sphere, a cone, a torus, a prism:
/// the command taken, the sizes the call carried filled into the fields that stand at the geometry,
/// Enter. The placement a person clicks is the origin by the tool's own default; the body of the
/// primitive is in the part after it, its node built without error. A size the tool will not take - no
/// radius, a length below nothing - is refused in the program's own words and no body comes to be.
fn make_a_primitive(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let kind = match code {
        "part.cylinder" => "Cylinder",
        "part.sphere" => "Sphere",
        "part.cone" => "Cone",
        "part.torus" => "Torus",
        "part.prism" => "Prism",
        _ => "Box3",
    };
    let before = core.document();
    let bodies_before = before.bodies.len();
    let nodes_before = before.features.iter().filter(|f| f.kind == kind).count();
    take_the_command(core, entry.command);
    fill_the_fields(core, given);
    core.key(qymcad::Key::Enter);
    let document = core.document();
    let made = document.features.iter().filter(|f| f.kind == kind && !f.suppressed).rfind(|f| f.error.is_none());
    match (document.bodies.len() > bodies_before || document.features.iter().filter(|f| f.kind == kind).count() > nodes_before, made) {
        (true, Some(node)) if !document.bodies.is_empty() && document.bodies.iter().any(|b| b.volume > 0.0) => {
            Answered::Ran(format!("a {} made ({}), its body in the part", entry.command.name(), node.name))
        }
        (_, Some(node)) => Answered::Refused(format!("the {} would not build: {}", entry.command.name(), node.error.clone().unwrap_or_default())),
        _ => Answered::Refused(format!("nothing was made: {}", core.status())),
    }
}

/// THE ARROW OF THE PANEL, put up as a person puts it before picking - whatever was in hand down, the
/// select tool up.
fn take_the_arrow(core: &mut qymcad::AgentSession) {
    let arrow = core.word("tb-select-hint");
    core.press_hint(&arrow);
}

/// PICK WHAT THE PLACES NAME: the first place clicked, the rest clicked with Shift - a person building
/// a set of many things.
fn pick_the_places(core: &mut qymcad::AgentSession, places: &[(f64, f64)]) {
    for (i, (x, y)) in places.iter().enumerate() {
        if i == 0 {
            core.click_on_sketch(*x, *y);
        } else {
            let at = core.on_sketch(*x, *y);
            core.click_with(at, qymcad::PointerButton::Primary, qymcad::Modifiers::SHIFT);
        }
    }
}

/// THE WORDS A PARAMETER CARRIES, read whether the call gave them as words or as a number: a size or an
/// angle arrives either way, and the field it is typed into takes its digits all the same.
fn the_words_of(given: &serde_json::Value, name: &str) -> Option<String> {
    match given.get(name)? {
        serde_json::Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

/// THE FIELDS THE CALL CARRIED, filled into the bar the take put up: each by its own caption - the grey
/// words of an empty field where the caption stands inside it - a number as its digits, words as they
/// are. A field the call did not carry keeps what it came with; a value the program refuses to take is
/// its own words in the status line, carried by the call's answer.
fn fill_the_fields(core: &mut qymcad::AgentSession, given: &serde_json::Value) {
    let Some(given) = given.as_object() else { return };
    let fields = the_fields_on_screen(core);
    for (name, value) in given {
        if !fields.iter().any(|f| &f.name == name) {
            continue; // not a field of the bar: the places of the drawing, or a name the call will be refused for
        }
        let as_words = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        let filling = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            core.fill(name, &as_words);
        }));
        if filling.is_err() {
            let hinted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                core.fill_first_hinted(name, &as_words);
            }));
            let _ = hinted;
        }
    }
}

/// THE PLACES A CALL CARRIES, read as pairs of the sheet; `None` when the call carried none or a
/// malformed list - the validation has said so in words already, and the run refuses what is left.
fn the_places(given: &serde_json::Value) -> Option<Vec<(f64, f64)>> {
    the_pairs(given.get("points")?)
}

/// THE PAIRS A VALUE HOLDS: places of the sheet, each an [x, y] pair of numbers; `None` at the first
/// thing that is not one.
fn the_pairs(value: &serde_json::Value) -> Option<Vec<(f64, f64)>> {
    value
        .as_array()?
        .iter()
        .map(|p| {
            let pair = p.as_array()?;
            Some((pair.first()?.as_f64()?, pair.get(1)?.as_f64()?))
        })
        .collect()
}

/// THE TAKE ITSELF, the run every command has: the command is taken into the hand through the command
/// search, exactly as a person takes it. Where the command lands - its bar of options standing in the
/// window, Enter applying it and Esc putting it down, as the person's keys do - the take has run; where
/// the program takes nothing (nothing to work on), it has refused in its own words, and nothing changed.
fn the_take(core: &mut qymcad::AgentSession, code: &str, _parameters: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let held = core.in_hand();
    take_the_command(core, entry.command);
    let said = core.status();
    if core.in_hand().iter().eq(held.iter()) {
        return Answered::Refused(said);
    }
    Answered::Ran(format!("{} is taken; its bar stands in the window - fill its fields and press Enter to apply, Esc puts it down", entry.command.name()))
}

/// THE ENTRY OF A CODE, as `call-tool` names an operation.
pub(crate) fn by_code(code: &str) -> Option<&'static Entry> {
    the_registry().iter().find(|e| e.command.code == code)
}

/// THE HITS OF A SEARCH, by the command search's own rules: an operation is found by its code, by its
/// name in any language the program speaks, by the title of its help article in any of them, or by a
/// word it is also known by; a match at the start of a word ranks above one inside it; the workbench the
/// program stands in (`now_in`) comes first, the rest behind it; a workbench named in the call (`only`)
/// keeps every other out. The whole set is answered - a client reads what a person would have to squint
/// at.
pub(crate) fn the_hits(query: &str, only: Option<&str>, now_in: &str) -> Vec<&'static Entry> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut mine: Vec<(usize, &Entry)> = Vec::new();
    let mut rest: Vec<(usize, &Entry)> = Vec::new();
    for entry in the_registry() {
        if let Some(want) = only {
            if entry.command.workbench != want {
                continue;
            }
        }
        let Some(rank) = the_words_it_is_found_by(entry).iter().filter_map(|w| w.to_lowercase().find(&q)).min() else { continue };
        if entry.command.workbench == now_in {
            mine.push((rank, entry));
        } else {
            rest.push((rank, entry));
        }
    }
    let by_rank_then_code = |a: &(usize, &Entry), b: &(usize, &Entry)| (a.0, a.1.command.code).cmp(&(b.0, b.1.command.code));
    mine.sort_by(by_rank_then_code);
    rest.sort_by(by_rank_then_code);
    mine.into_iter().chain(rest).map(|(_, e)| e).collect()
}

/// THE WORDS AN OPERATION IS FOUND BY: its code, its names in each language, the title of its help
/// article in each of them, and the words it is also known by.
fn the_words_it_is_found_by(entry: &Entry) -> Vec<String> {
    let mut words = vec![entry.command.code.to_string()];
    words.extend(entry.names().into_values());
    if let Some(article) = entry.command.help_article() {
        words.extend(qymcad_help::languages().into_iter().filter_map(|l| qymcad_help::title_in(&l, article)));
    }
    words.extend(entry.synonyms.iter().map(|s| s.to_string()));
    words
}

/// ONE PARAMETER OF AN OPERATION: a field its bar puts up when the command is taken.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(crate) struct Parameter {
    /// The words a person reads it by: its caption, or the grey words it shows while empty.
    pub(crate) name: String,
    /// What kind of thing it is: `number`, `text`, `choice` or `slider`.
    pub(crate) kind: String,
    /// What it holds as the command opens - the value it comes with, before anything is typed.
    pub(crate) says: String,
}

/// TAKE THE COMMAND into the hand, the one way a person takes a command: the command search, the
/// command's own name typed whole (a person never types a code - codes are not words, and the search is
/// a person's surface), Enter. The command's bar of options stands in the window as any take leaves it.
pub(crate) fn take_the_command(core: &mut qymcad::AgentSession, command: &qymcad::Command) {
    let name = command.name();
    core.chord(qymcad::Modifiers::COMMAND, qymcad::Key::K).type_text(&name).key(qymcad::Key::Enter);
}

/// THE INPUT FIELDS ON SCREEN, in reading order: what a person can type into, each read as a parameter -
/// its caption or grey words, its kind, and what it holds. A field with no caption of its own is read
/// by the words painted beside it on the canvas (the sizes of a primitive stand at the geometry, their
/// captions drawn beside them and not in the field) - the same pairing a fill makes.
fn the_fields_on_screen(core: &mut qymcad::AgentSession) -> Vec<Parameter> {
    let painted = core.painted();
    core.widgets()
        .into_iter()
        .filter(|w| matches!(w.kind, qymcad::Kind::TextField | qymcad::Kind::Number | qymcad::Kind::ComboBox | qymcad::Kind::Slider))
        .map(|w| {
            // the words painted beside it name it first: a field at the geometry carries only the grey
            // words of any number, its caption drawn beside it
            let painted_beside = the_caption_painted_beside(&painted, &w);
            let name = if painted_beside.is_empty() { the_words_it_reads_by(&w) } else { painted_beside };
            Parameter { name, kind: the_kind_word(w.kind).to_string(), says: w.value }
        })
        .collect()
}

/// THE WORDS PAINTED BESIDE A FIELD: the caption nearest it on the same row after it or below it - how
/// the canvas labels the fields that stand at the geometry.
fn the_caption_painted_beside(painted: &[(qymcad::Rect, String)], w: &qymcad::Widget) -> String {
    painted
        .iter()
        .filter(|(r, text)| {
            !text.trim().is_empty()
                && ((r.min.y < w.rect.max.y && r.max.y > w.rect.min.y && r.max.x <= w.rect.min.x + 1.0) || (r.max.y <= w.rect.min.y + 1.0 && r.min.x < w.rect.max.x && r.min.x >= w.rect.min.x - 40.0))
        })
        .min_by(|a, b| {
            let d = |r: &qymcad::Rect| if r.max.y < w.rect.min.y { 1000.0 + w.rect.min.y - r.max.y } else { w.rect.min.x - r.max.x };
            d(&a.0).total_cmp(&d(&b.0))
        })
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_default()
}

/// THE WORDS A FIELD READS BY: its caption, or the grey words it shows while empty.
fn the_words_it_reads_by(w: &qymcad::Widget) -> String {
    if w.label.is_empty() {
        w.placeholder.clone()
    } else {
        w.label.clone()
    }
}

/// THE KIND OF A FIELD, in one word.
fn the_kind_word(kind: qymcad::Kind) -> &'static str {
    match kind {
        qymcad::Kind::TextField => "text",
        qymcad::Kind::Number => "number",
        qymcad::Kind::ComboBox => "choice",
        qymcad::Kind::Slider => "slider",
        _ => "other",
    }
}

/// THE PARAMETERS OF THE COMMAND of `code`, read from the live tool: the command is taken as a person
/// takes it, and the fields the take put up - and nothing that stood on screen before it - are its
/// parameters, in the bar's own order. Where the take landed a tool it is put down again; the document is
/// left as it stood. Where the command does not open, nothing to work on, no field comes up and none is
/// answered.
pub(crate) fn parameters(core: &mut qymcad::AgentSession, code: &str) -> Vec<Parameter> {
    let Some(entry) = by_code(code) else { return Vec::new() };
    let before = the_fields_on_screen(core);
    let held = core.in_hand();
    take_the_command(core, entry.command);
    let mut put_up = the_fields_on_screen(core);
    put_up.retain(|f| !before.iter().any(|b| b.name == f.name && b.kind == f.kind));
    put_down_what_the_take_landed(core, &held);
    let mut all = put_up;
    all.extend(entry.takes.iter().map(|t| Parameter {
        name: t.name.to_string(),
        kind: t.kind.to_string(),
        says: if t.kind == "points" { "places on the sheet of the open sketch, each an [x, y] pair".to_string() } else { String::new() },
    }));
    all
}

/// PUT DOWN WHAT THE TAKE LANDED, as a person puts a tool down: Esc, as many as the cancel ladder needs -
/// a field of the command holds the keyboard and takes one, the command itself takes the next - until the
/// bars stand as they did before the take, or the ladder's own three steps are spent. Nothing is pressed
/// where the take landed nothing.
fn put_down_what_the_take_landed(core: &mut qymcad::AgentSession, held: &[String]) {
    for _ in 0..3 {
        if core.in_hand().iter().eq(held.iter()) {
            break;
        }
        core.key(qymcad::Key::Escape);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE REGISTRY IS THE CATALOGUE, derived whole: every command of the catalogue is an entry, each
    /// entry carries its command's code and workbench, its names in each language the program speaks -
    /// and nothing stands in the registry the catalogue does not hold.
    #[test]
    fn the_registry_is_derived_from_the_catalogue_whole() {
        for command in qymcad::COMMANDS {
            let Some(entry) = by_code(command.code) else { panic!("{} is in the catalogue and not in the registry", command.code) };
            assert_eq!(entry.command.code, command.code, "the entry of {} answers another code", command.code);
            assert_eq!(entry.command.workbench, command.workbench, "{} carries the wrong workbench", command.code);
            for (language, name) in entry.names() {
                assert!(!name.is_empty(), "{} is nameless in {language}", command.code);
            }
        }
        assert_eq!(the_registry().len(), qymcad::COMMANDS.len(), "the registry holds entries the catalogue does not");
    }

    /// THE PARAMETERS OF EVERY FEATURE COMMAND come from the live tool: taking the command puts its bar
    /// up, and the fields the take put up are the parameter list. The check takes each command itself,
    /// over a document where a finished rectangle stands selected (so the commands that work from a
    /// sketch open their bars), and reads the screen beside the registry's own read; the read puts the
    /// tool down again, and the document stands as it did. The primitives and the commands that take the
    /// sketch open their bars on this fixture - the count keeps the check from passing on empty air.
    #[test]
    fn the_parameters_of_every_feature_command_come_from_the_live_tool() {
        let _turn = crate::checks::take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut program, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            let rect = core.word("tb-rect-hint");
            core.press_hint(&rect);
            core.click_on_sketch(0.0, 0.0).click_on_sketch(40.0, 30.0);
            let finish = core.word("wb-finish");
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
        })
        .expect("the fixture was built");
        /// THE SCREEN'S OWN ANSWER, written beside the registry's read: what stands on screen one may
        /// type into, as (name, what it holds) pairs - the diff against before is the check's own.
        fn the_screen(core: &mut qymcad::AgentSession) -> Vec<(String, String)> {
            let painted = core.painted();
            core.widgets()
                .into_iter()
                .filter(|w| matches!(w.kind, qymcad::Kind::TextField | qymcad::Kind::Number | qymcad::Kind::ComboBox | qymcad::Kind::Slider))
                .map(|w| {
                    let painted_beside = the_caption_painted_beside(&painted, &w);
                    let name = if painted_beside.is_empty() { the_words_it_reads_by(&w) } else { painted_beside };
                    (name, w.value)
                })
                .collect()
        }
        let mut bars_opened = 0;
        for entry in the_registry().iter().filter(|e| matches!(e.command.launch, qymcad::Launch::Feat(_) | qymcad::Launch::Prim(_))) {
            let code = entry.command.code;
            let own = crate::checks::ask_the_core(&mut program, move |core| {
                let before = the_screen(core);
                let held = core.in_hand();
                take_the_command(core, by_code(code).expect("the code names an entry").command);
                let after = the_screen(core);
                let put_up: Vec<(String, String)> = after.into_iter().filter(|(name, _)| !before.iter().any(|(n, _)| n == name)).collect();
                for _ in 0..3 {
                    if core.in_hand().iter().eq(held.iter()) {
                        break;
                    }
                    core.key(qymcad::Key::Escape);
                }
                put_up
            })
            .expect("the screen was read");
            let answered = crate::checks::ask_the_core(&mut program, move |core| parameters(core, code)).expect("the parameters were read");
            let as_pairs: Vec<(String, String)> = answered.iter().map(|p| (p.name.clone(), p.says.clone())).collect();
            assert_eq!(as_pairs, own, "{code}: the registry's parameters are not the fields its bar puts up");
            if !answered.is_empty() {
                bars_opened += 1;
            }
        }
        assert!(bars_opened >= 8, "only {bars_opened} feature commands put a bar up on the fixture; the reads have gone blind");
    }

    /// SEARCH FINDS AN OPERATION BY ANY OF ITS NAMES: its code, its name in each language the program
    /// speaks, the title of its help article, the words it is also known by. A workbench named in the
    /// call keeps every other workbench out. The workbench the program stands in comes first, as the
    /// person's own search ranks it - a command of the standing workbench that matches deep inside its
    /// code stands above one of another workbench that matches at the start of its name. A miss finds
    /// nothing.
    #[test]
    fn search_finds_an_operation_by_any_of_its_names() {
        let finds = |needle: &str, code: &str| the_hits(needle, None, "part").iter().any(|e| e.command.code == code);
        let line = by_code("sketch.line").expect("the line is in the registry");
        assert!(finds("sketch.line", "sketch.line"), "a command's own code found it not");
        for (language, name) in line.names() {
            assert!(finds(&name, "sketch.line"), "the {language} name {name:?} found the line not");
        }
        let the_box = by_code("part.box").expect("the box is in the registry");
        let article = the_box.command.help_article().expect("the box points at an article");
        for language in qymcad_help::languages() {
            let Some(title) = qymcad_help::title_in(&language, article) else { continue };
            assert!(finds(&title, "part.box"), "the article title {title:?} of {language} found the box not");
        }
        assert!(finds("round", "part.fillet"), "the word the fillet is also known by found it not");
        assert!(the_hits("fillet", Some("sketch"), "part").iter().all(|e| e.command.workbench == "sketch"), "the workbench filter let another workbench through");
        let hits = the_hits("line", None, "part");
        assert_eq!(
            hits.first().map(|e| e.command.workbench),
            Some("part"),
            "a command of the standing workbench did not come first: {:?}",
            the_hits("line", None, "part").iter().map(|e| e.command.code).collect::<Vec<_>>()
        );
        assert!(hits.iter().any(|e| e.command.code == "sketch.line"), "the line was not found at all");
        assert!(the_hits("qqqqzzzz", None, "part").is_empty(), "a miss found something");
    }

    /// THE GUARD HOLDS THE LINK BOTH WAYS: every command of the catalogue is discoverable through the
    /// search's matching - by its code, by its name in each language, by each of its synonyms - and every
    /// entry of the registry executes: its run is wired, and the run of each entry runs over a live
    /// program, command by command, each answered in words. An entry left unwired fails the guard where
    /// it stands; the guard was red before a run was wired to every entry, and stays green only while the
    /// wiring does.
    #[test]
    fn the_guard_holds_the_link_between_the_catalogue_and_the_registry() {
        for entry in the_registry() {
            let mut needles = vec![entry.command.code.to_string()];
            needles.extend(entry.names().into_values());
            needles.extend(entry.synonyms.iter().map(|s| s.to_string()));
            for needle in needles {
                assert!(the_hits(&needle, None, "part").iter().any(|e| e.command.code == entry.command.code), "{} is not found by {needle:?}", entry.command.code);
            }
            assert!(entry.run.is_some(), "{} is wired to no run", entry.command.code);
        }
        let _turn = crate::checks::take_your_turn();
        // ON A BARE START every run answers in words - the take lands the commands that need nothing,
        // and a run that needs what is not there (a drawing with no sketch open) refuses in the
        // program's own words
        let mut program = qymcad::LiveLoop::start();
        let mut landed = 0;
        for entry in the_registry().iter().filter(|e| e.takes.is_empty()) {
            let code = entry.command.code;
            let answered = crate::checks::ask_the_core(&mut program, move |core| {
                let the_run = by_code(code).expect("the code names an entry").run.expect("the entry is wired");
                let answered = the_run(core, code, &serde_json::json!({}));
                // the take leaves the command in the hand, as a person's take does; the check puts it
                // down, so the next take starts from bars as they stood
                let held: Vec<String> = Vec::new();
                put_down_what_the_take_landed(core, &held);
                answered
            })
            .expect("the run answered");
            let said = match &answered {
                Answered::Ran(outcome) => {
                    landed += 1;
                    outcome.clone()
                }
                Answered::Refused(words) => words.clone(),
            };
            assert!(!said.is_empty(), "{} said nothing when it ran", entry.command.code);
        }
        assert!(landed >= 15, "only {landed} of the take-only commands landed their tool on a bare start; the runs have gone mute");
        // OVER AN OPEN SKETCH every run that takes parameters runs with what its own work needs: the
        // drawing tools with places of their own aside of the ones before, the operations that work on
        // what stands with a fixture the registry's own calls draw for them beside those places
        let mut sketching = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut sketching, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
        })
        .expect("the fixture was built");
        /// WHAT THE RUN OF `code` IS GIVEN over the open sketch, and what it needs drawn first: a
        /// fixture drawn by the registry's own drawing calls and the parameters the run takes. The
        /// fixtures stand in a grid of 60-squares - `(x, y)` is the square's corner - so the sheet the
        /// whole loop spans stays narrow enough for a view that does not zoom out: at a wide zoom the
        /// snap reach of a click grows past the ends of a line and the glyph of its constraint past its
        /// middle, and no place on a short line picks the line at all.
        fn the_sample_for(core: &mut qymcad::AgentSession, code: &str, x: f64, y: f64) -> serde_json::Value {
            let line = |core: &mut qymcad::AgentSession, a: (f64, f64), b: (f64, f64)| {
                call(core, "sketch.line", &serde_json::json!({ "points": [[a.0, a.1], [b.0, b.1]] }));
            };
            match code {
                // THE DIMENSIONS: a line to measure for its length, two at a free angle between them, a
                // circle for its diameter - each held at the value it already measures, so the solve
                // keeps what the fixture drew, and placed to stand clear (y - 50) of all geometry
                "sketch.dim" => {
                    line(core, (x, y), (x + 40.0, y));
                    serde_json::json!({ "points": [[x + 20.0, y], [x + 20.0, y - 50.0]], "value": "40" })
                }
                "sketch.dim-angle" => {
                    line(core, (x, y), (x + 30.0, y));
                    line(core, (x, y), (x + 10.0, y + 20.0));
                    serde_json::json!({ "points": [[x + 15.0, y], [x + 5.0, y + 10.0], [x + 5.0, y - 50.0]], "value": "63" })
                }
                "sketch.dim-radius" => {
                    call(core, "sketch.circle", &serde_json::json!({ "points": [[x, y + 10.0], [x + 10.0, y + 10.0]] }));
                    serde_json::json!({ "points": [[x + 10.0, y + 10.0]], "value": "20" })
                }
                // THE CLICK OPERATIONS: a crossing for the trim to cut back to, a short line with a wall
                // across it for the extend to reach, a line for the break to split
                "sketch.trim" => {
                    line(core, (x, y), (x + 40.0, y + 40.0));
                    line(core, (x, y + 40.0), (x + 40.0, y));
                    serde_json::json!({ "points": [[x + 30.0, y + 10.0]] })
                }
                "sketch.extend" => {
                    line(core, (x, y), (x + 20.0, y));
                    line(core, (x + 30.0, y - 5.0), (x + 30.0, y + 15.0));
                    serde_json::json!({ "points": [[x + 10.0, y], [x + 30.0, y]] })
                }
                "sketch.break" => {
                    line(core, (x, y), (x + 40.0, y));
                    serde_json::json!({ "points": [[x + 20.0, y]] })
                }
                // THE CORNERS AND THE MODIFY TOOLS: an L to round or bevel, a chain to offset with its
                // side named, a shape to reflect with its axis beside it, a line to delete
                "sketch.corner" => {
                    line(core, (x, y), (x + 40.0, y));
                    line(core, (x + 40.0, y), (x + 40.0, y + 30.0));
                    serde_json::json!({ "points": [[x + 20.0, y], [x + 40.0, y + 15.0]], "radius": "5" })
                }
                "sketch.corner-chamfer" => {
                    line(core, (x, y), (x + 40.0, y));
                    line(core, (x + 40.0, y), (x + 40.0, y + 30.0));
                    serde_json::json!({ "points": [[x + 20.0, y], [x + 40.0, y + 15.0]], "size": "4" })
                }
                "sketch.offset" => {
                    line(core, (x, y), (x + 30.0, y));
                    line(core, (x + 30.0, y), (x + 30.0, y + 20.0));
                    serde_json::json!({ "points": [[x + 10.0, y], [x + 30.0, y + 5.0], [x + 25.0, y - 5.0]], "distance": "3" })
                }
                "sketch.mirror" => {
                    // the what short and the axis beside it, so the reflection lands inside this
                    // fixture's own square - a reflection reaching the next square would stand where
                    // its own draw is about to click
                    line(core, (x, y), (x + 20.0, y));
                    line(core, (x + 25.0, y - 10.0), (x + 25.0, y + 10.0));
                    serde_json::json!({ "points": [[x + 10.0, y], [x + 25.0, y]] })
                }
                "sketch.delete" => {
                    line(core, (x, y), (x + 40.0, y));
                    serde_json::json!({ "points": [[x + 10.0, y]] })
                }
                // EVERY DRAWING TOOL: the places of its own the sample has always carried
                _ => serde_json::json!({ "points": [[x + 5.0, y], [x + 15.0, y], [x + 15.0, y + 10.0]], "text": "a note" }),
            }
        }
        // (the projection is held for its own phase below: its work needs a body under the sheet, and
        // the open sketch of this one has none)
        let mut square = 0;
        for entry in the_registry().iter().filter(|e| !e.takes.is_empty() && e.command.code != "sketch.project") {
            let code = entry.command.code;
            // the squares run right four and then up a row, the span of the whole loop kept narrow
            square += 1;
            let (x, y) = (60.0 + 60.0 * ((square - 1) % 4) as f64, 100.0 + 60.0 * ((square - 1) / 4) as f64);
            let answered = crate::checks::ask_the_core(&mut sketching, move |core| {
                let the_run = by_code(code).expect("the code names an entry").run.expect("the entry is wired");
                let given = the_sample_for(core, code, x, y);
                let answered = the_run(core, code, &given);
                let held: Vec<String> = Vec::new();
                put_down_what_the_take_landed(core, &held);
                answered
            })
            .expect("the run answered");
            assert!(matches!(answered, Answered::Ran(_)), "{} worked nothing over the open sketch: {answered:?}", entry.command.code);
        }
        // OVER A SKETCH SEATED ON THE FACE OF A BODY the projection runs: a block through the registry's
        // own gestures, the pencil of the panel, the top face clicked, the projection of its edges
        let mut on_a_face = qymcad::LiveLoop::start();
        let answered = crate::checks::ask_the_core(&mut on_a_face, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            let rect = core.word("tb-rect-hint");
            core.press_hint(&rect);
            core.click_on_sketch(0.0, 0.0).click_on_sketch(40.0, 30.0);
            let finish = core.word("wb-finish");
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
            let extrude = core.word("tb-extrude-hint");
            core.press_hint(&extrude);
            core.key(qymcad::Key::Enter);
            let pencil = core.word("g-sketch-pick-hint");
            core.press_hint(&pencil);
            let at = core.face_at([20.0, 15.0, 10.0]);
            core.click(at);
            call(core, "sketch.project", &serde_json::json!({ "points": [[20.0, 0.0]] }))
        })
        .expect("the projection answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "sketch.project projected nothing over the face of the block: {answered:?}");
    }

    /// THE FRAME OF A CALL, checked at each of its refusals and at its answer: a name no operation
    /// carries, a parameter the operation does not take, a value of the wrong kind, an operation the
    /// program itself refuses (a command that cannot work on what stands lands nothing) - each answered
    /// in words naming what was expected, each leaving the document as it stood. A call that runs is
    /// answered from rest with the status line, the problems found, and the operation's own outcome.
    #[test]
    fn a_call_is_validated_run_and_answered_with_the_program_s_own_words() {
        let _turn = crate::checks::take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut program, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            let rect = core.word("tb-rect-hint");
            core.press_hint(&rect);
            core.click_on_sketch(0.0, 0.0).click_on_sketch(40.0, 30.0);
            let finish = core.word("wb-finish");
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
        })
        .expect("the fixture was built");
        let the_document_as_it_stood = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        // A NAME NO OPERATION CARRIES
        let refused = crate::checks::ask_the_core(&mut program, |core| call(core, "no.such-thing", &serde_json::json!({}))).expect("the call answered");
        let TheCall::Refused(words) = &refused else { panic!("an unknown name was not refused: {refused:?}") };
        assert!(words.contains("no.such-thing") && words.contains("search-tool"), "the refusal did not name what was asked and where to look: {words}");
        // A PARAMETER THE OPERATION DOES NOT TAKE: the extrude's own fields are read from the live tool,
        // and a name among them is not
        let fields = crate::checks::ask_the_core(&mut program, |core| parameters(core, "part.extrude")).expect("the schema was read");
        assert!(!fields.is_empty(), "the extrude put up no field on the finished sketch");
        let carried: Vec<(String, serde_json::Value)> = vec![("a-parameter-it-takes-not".to_string(), serde_json::json!(1)), (fields[0].name.clone(), serde_json::json!({ "not": "a value" }))];
        for (name, value) in carried {
            let mut given = serde_json::Map::new();
            given.insert(name.clone(), value);
            let given = serde_json::Value::Object(given);
            let refused = crate::checks::ask_the_core(&mut program, move |core| call(core, "part.extrude", &given)).expect("the call answered");
            let TheCall::Refused(words) = &refused else { panic!("the parameter {name:?} was not refused: {refused:?}") };
            assert!(words.contains(&name), "the refusal did not name the parameter: {words}");
            assert!(words.contains("takes"), "the refusal did not say what the operation takes: {words}");
        }
        // AN OPERATION THE PROGRAM ITSELF REFUSES: a command that works only on what stands selected -
        // over a bare program nothing is - lands nothing, and the program's own words say why
        let mut bare = qymcad::LiveLoop::start();
        let bare_as_it_stood = crate::checks::ask_the_core(&mut bare, |core| core.document()).expect("the document was read");
        let refused = crate::checks::ask_the_core(&mut bare, |core| call(core, "part.piece-to-part", &serde_json::json!({}))).expect("the call answered");
        let TheCall::Refused(words) = &refused else { panic!("an operation with nothing selected to work on was not refused: {refused:?}") };
        assert!(!words.trim().is_empty(), "the program's refusal said nothing");
        let after = crate::checks::ask_the_core(&mut bare, |core| core.document()).expect("the document was read");
        assert_eq!(after, bare_as_it_stood, "a refused call changed the document");
        drop(bare);
        // THE DOCUMENT STOOD AS IT DID through every refusal, and a call that runs answers from rest
        let after = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        assert_eq!(after, the_document_as_it_stood, "a refused call changed the document");
        let answered = crate::checks::ask_the_core(&mut program, |core| {
            let answered = call(core, "part.extrude", &serde_json::json!({}));
            // the take leaves the command in the hand; the check puts it down, the document as it stood
            let held: Vec<String> = Vec::new();
            put_down_what_the_take_landed(core, &held);
            answered
        })
        .expect("the call answered");
        let TheCall::Answered { status, problems, outcome } = &answered else { panic!("the take of the extrude over its own sketch did not run: {answered:?}") };
        assert!(!status.trim().is_empty() && !outcome.trim().is_empty(), "the answer carried no words: {answered:?}");
        assert!(problems.is_empty(), "the take itself must leave no problem: {answered:?}");
    }

    /// THE DRAWING TOOLS OF THE SKETCH, run as calls: the places on the sheet carried as the call's
    /// parameters, the tool worked through its own gestures (a chain clicked place by place, a circle by
    /// its centre and rim, a rectangle by its anchor and corner), and the drawing in the document after
    /// it - counted, not assumed. A call that carries no places is refused in words before a gesture is
    /// made, the document as it stood; so is a drawing with no sketch open.
    #[test]
    fn the_drawing_tools_draw_what_a_call_carries() {
        let _turn = crate::checks::take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        // A FIRST PART, A SKETCH ON XY, left open and empty
        crate::checks::ask_the_core(&mut program, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
        })
        .expect("the fixture was built");
        let empty = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        assert!(empty.editing.is_some(), "no sketch stands open on the fixture");
        // A CALL WITHOUT ITS PLACES is refused in words naming what was expected, the document as it stood
        let refused = crate::checks::ask_the_core(&mut program, |core| call(core, "sketch.line", &serde_json::json!({}))).expect("the call answered");
        let TheCall::Refused(words) = &refused else { panic!("a drawing without its places was not refused: {refused:?}") };
        assert!(words.contains("points") && words.contains("takes"), "the refusal did not name what was expected: {words}");
        let after = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        assert_eq!(after, empty, "a refused drawing changed the document");
        // THE LINE: two places, one line more
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "sketch.line", &serde_json::json!({ "points": [[0.0, 0.0], [40.0, 0.0]] }))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the line did not run: {answered:?}");
        let one_line = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        fn of(doc: &qymcad::Document) -> &qymcad::SketchInfo {
            doc.sketches.iter().find(|s| Some(&s.name) == doc.editing.as_ref()).expect("the open sketch")
        }
        assert_eq!(of(&one_line).lines, of(&empty).lines + 1, "the line did not land in the sketch");
        assert_eq!(of(&one_line).circles, of(&empty).circles, "the line drew a circle");
        // THE CIRCLE: a centre and a rim, one circle more
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "sketch.circle", &serde_json::json!({ "points": [[0.0, 0.0], [10.0, 0.0]] }))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the circle did not run: {answered:?}");
        let one_circle = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        assert_eq!(of(&one_circle).circles, of(&one_line).circles + 1, "the circle did not land in the sketch");
        // THE RECTANGLE: an anchor and a corner, four lines more
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "sketch.rect", &serde_json::json!({ "points": [[0.0, 0.0], [40.0, 30.0]] }))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the rectangle did not run: {answered:?}");
        let boxed = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        assert!(of(&boxed).lines >= of(&one_circle).lines + 4, "the rectangle did not land in the sketch: {} lines", of(&boxed).lines);
        // A DRAWING WITH NO SKETCH OPEN is refused in words, the document as it stood
        let mut bare = qymcad::LiveLoop::start();
        let bare_as_it_stood = crate::checks::ask_the_core(&mut bare, |core| core.document()).expect("the document was read");
        let refused = crate::checks::ask_the_core(&mut bare, |core| call(core, "sketch.line", &serde_json::json!({ "points": [[0.0, 0.0], [40.0, 0.0]] }))).expect("the call answered");
        let TheCall::Refused(words) = &refused else { panic!("a drawing with no sketch open was not refused: {refused:?}") };
        assert!(words.contains("sketch"), "the refusal did not say what was missing: {words}");
        let after = crate::checks::ask_the_core(&mut bare, |core| core.document()).expect("the document was read");
        assert_eq!(after, bare_as_it_stood, "a refused drawing changed the document");
    }

    /// THE DIMENSIONS AND THE CLICK OPERATIONS OF THE SKETCH, run as calls: a dimension measured by the
    /// places the call names and held at the value it carries - the geometry itself moved to obey it, a
    /// length pulled to its number, an angle turned to its own, a diameter held where it was set; a
    /// segment trimmed back to the crossing it went past, a line extended to the wall it was pointed at,
    /// a line broken in two where the call said. Each is checked by what the sheet holds after it, not
    /// by the words the call answered; and each refusal - a trim that crosses nothing - is the program's
    /// own words over a sheet that stands as it did.
    #[test]
    fn the_dimensions_and_click_operations_work_their_contracts() {
        let _turn = crate::checks::take_your_turn();
        /// AN OPEN, EMPTY SKETCH on the XY plane of a first part, the auto constraints off - what a check
        /// of one tool sees is what that tool did.
        fn an_empty_sketch() -> qymcad::LiveLoop {
            let mut program = qymcad::LiveLoop::start();
            crate::checks::ask_the_core(&mut program, |core| {
                let new_part = core.word("start-new-part");
                core.press_word(&new_part);
                let xy = core.word("plane-xy-table");
                core.press_word(&xy);
                let auto = core.word("wb-auto-constraints");
                let switch = core.widgets().into_iter().find(|w| w.label.ends_with(&auto)).expect("the switch of the auto constraints");
                if switch.checked == Some(true) {
                    core.click(switch.rect.center());
                }
            })
            .expect("the fixture was built");
            program
        }
        /// ONE DRAWING CALL, answered.
        fn draw(program: &mut qymcad::LiveLoop, code: &str, points: &[[f64; 2]]) -> TheCall {
            let code = code.to_string();
            let points = points.to_vec();
            crate::checks::ask_the_core(program, move |core| call(core, &code, &serde_json::json!({ "points": points }))).expect("the call answered")
        }
        /// THE OPEN SKETCH, read whole.
        fn the_open(core: &mut qymcad::AgentSession) -> qymcad::SketchInfo {
            let document = core.document();
            document.sketches.iter().find(|s| Some(&s.name) == document.editing.as_ref()).expect("the open sketch").clone()
        }
        // A LINEAR DIMENSION PULLS ITS LINE TO THE VALUE: 40 measured, held at 30, the line 30 long
        let mut program = an_empty_sketch();
        draw(&mut program, "sketch.line", &[[0.0, 0.0], [40.0, 0.0]]);
        let answered =
            crate::checks::ask_the_core(&mut program, |core| call(core, "sketch.dim", &serde_json::json!({ "points": [[20.0, 0.0], [20.0, -8.0]], "value": "30" }))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the linear dimension did not run: {answered:?}");
        let length = crate::checks::ask_the_core(&mut program, |core| match core.sketch_under(20.0, 0.0) {
            Some(qymcad::SketchPick::Line { from, to }) => (to.0 - from.0).hypot(to.1 - from.1),
            other => panic!("no line lies at (20, 0): {other:?}"),
        })
        .expect("the line was read");
        assert!((length - 30.0).abs() < 1e-6, "the dimension held at 30 left the line {length} long");
        // AN ANGULAR DIMENSION TURNS ITS LINES TO ITS VALUE: a free 63 deg held at 60, the lines 60 apart
        draw(&mut program, "sketch.line", &[[100.0, 0.0], [130.0, 0.0]]);
        draw(&mut program, "sketch.line", &[[100.0, 0.0], [110.0, 20.0]]);
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "sketch.dim-angle", &serde_json::json!({ "points": [[115.0, 0.0], [105.0, 10.0], [110.0, 5.0]], "value": "60" })))
            .expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the angular dimension did not run: {answered:?}");
        let angle = crate::checks::ask_the_core(&mut program, |core| {
            // the two lines, each as its ray out of the vertex they share at (100, 0)
            let mut ray = |at: (f64, f64)| match core.sketch_under(at.0, at.1) {
                Some(qymcad::SketchPick::Line { from, to }) => {
                    let (a, b) = ((from.0 - 100.0).hypot(from.1), (to.0 - 100.0).hypot(to.1));
                    let (near, far) = if a < b { (from, to) } else { (to, from) };
                    (far.0 - near.0, far.1 - near.1)
                }
                other => panic!("no line lies at {at:?}: {other:?}"),
            };
            let (ax, ay) = ray((115.0, 0.0));
            let (bx, by) = ray((105.0, 10.0));
            let cosine = (ax * bx + ay * by) / (ax.hypot(ay) * bx.hypot(by));
            cosine.clamp(-1.0, 1.0).acos().to_degrees()
        })
        .expect("the lines were read");
        assert!((angle - 60.0).abs() < 1e-6, "the dimension held at 60 left the lines at {angle}");
        // A DIAMETER HELD AT ITS VALUE: a circle of 20 across held at 12, the radius 6
        draw(&mut program, "sketch.circle", &[[200.0, 0.0], [210.0, 0.0]]);
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "sketch.dim-radius", &serde_json::json!({ "points": [[210.0, 0.0]], "value": "12" }))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the diameter did not run: {answered:?}");
        let radius = crate::checks::ask_the_core(&mut program, |core| match core.sketch_under(206.0, 0.0) {
            Some(qymcad::SketchPick::Circle { radius, .. }) => radius,
            other => panic!("no circle lies at (206, 0): {other:?}"),
        })
        .expect("the circle was read");
        assert!((radius - 6.0).abs() < 1e-6, "the diameter held at 12 left the radius at {radius}");
        // A TRIM PULLS THE CLICKED ARM BACK TO ITS CROSSING
        let mut program = an_empty_sketch();
        draw(&mut program, "sketch.line", &[[0.0, 0.0], [40.0, 40.0]]);
        draw(&mut program, "sketch.line", &[[0.0, 40.0], [40.0, 0.0]]);
        let before = crate::checks::ask_the_core(&mut program, |core| the_open(core).places).expect("the places were read");
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "sketch.trim", &serde_json::json!({ "points": [[30.0, 10.0]] }))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the trim did not run: {answered:?}");
        let after = crate::checks::ask_the_core(&mut program, |core| the_open(core).places).expect("the places were read");
        assert!(
            after.iter().any(|p| (p[0] - 20.0).abs() < 1e-6 && (p[1] - 20.0).abs() < 1e-6) && !before.iter().any(|p| (p[0] - 20.0).abs() < 1e-6 && (p[1] - 20.0).abs() < 1e-6),
            "the arm trimmed did not come back to the crossing at (20, 20): {after:?}"
        );
        // A SEGMENT THAT CROSSES NOTHING IS TRIMMED WHOLE - the program's own rule: removed up to the
        // intersections, and with none the whole segment is the piece
        let mut bare = an_empty_sketch();
        draw(&mut bare, "sketch.line", &[[0.0, 10.0], [40.0, 10.0]]);
        let answered = crate::checks::ask_the_core(&mut bare, |core| call(core, "sketch.trim", &serde_json::json!({ "points": [[20.0, 10.0]] }))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the lone-line trim did not run: {answered:?}");
        let lines = crate::checks::ask_the_core(&mut bare, |core| the_open(core).lines).expect("the lines were read");
        assert_eq!(lines, 0, "a segment that crosses nothing was not trimmed whole");
        // A CLICK THAT HITS NO SEGMENT is refused in the program's words, the sheet as it stood
        let stood = crate::checks::ask_the_core(&mut bare, |core| core.document()).expect("the document was read");
        let refused = crate::checks::ask_the_core(&mut bare, |core| call(core, "sketch.trim", &serde_json::json!({ "points": [[200.0, 200.0]] }))).expect("the call answered");
        let TheCall::Refused(words) = &refused else { panic!("a trim over nothing at all was not refused: {refused:?}") };
        assert!(!words.trim().is_empty(), "the refused trim said nothing");
        let after = crate::checks::ask_the_core(&mut bare, |core| core.document()).expect("the document was read");
        assert_eq!(after, stood, "a refused trim changed the document");
        // AN EXTENSION REACHES THE WALL IT IS POINTED AT
        let mut program = an_empty_sketch();
        draw(&mut program, "sketch.line", &[[0.0, 0.0], [20.0, 0.0]]);
        draw(&mut program, "sketch.line", &[[30.0, -10.0], [30.0, 10.0]]);
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "sketch.extend", &serde_json::json!({ "points": [[10.0, 0.0], [30.0, 0.0]] }))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the extend did not run: {answered:?}");
        let reached = crate::checks::ask_the_core(&mut program, |core| the_open(core).places.iter().any(|p| (p[0] - 30.0).abs() < 1e-6 && p[1].abs() < 1e-6)).expect("the places were read");
        assert!(reached, "the line did not reach the wall at (30, 0)");
        // A BREAK SPLITS ITS LINE IN TWO
        let mut program = an_empty_sketch();
        draw(&mut program, "sketch.line", &[[0.0, 0.0], [40.0, 0.0]]);
        let one = crate::checks::ask_the_core(&mut program, |core| the_open(core).lines).expect("the lines were read");
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "sketch.break", &serde_json::json!({ "points": [[20.0, 0.0]] }))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the break did not run: {answered:?}");
        let two = crate::checks::ask_the_core(&mut program, |core| the_open(core).lines).expect("the lines were read");
        assert_eq!(two, one + 1, "the break left {} lines where 1 stood", two - one);
    }

    /// THE CORNERS, THE OFFSET, THE MIRROR AND THE DELETION, run as calls, each checked by what the
    /// sheet holds after it: a rounded corner leaves the arc that was not there, a bevelled one a line;
    /// an offset lays its copy beside the chain; a mirror leaves the reflection standing about its axis;
    /// a deletion takes what the places named away. The projection takes the edges of the body the
    /// sketch looks at into the sheet as driven geometry - and is refused, the sheet as it stood, over a
    /// sketch that looks at no body. Each refusal names what went wrong in the program's own words.
    #[test]
    fn the_corners_offsets_mirrors_and_deletions_work_their_contracts() {
        let _turn = crate::checks::take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut program, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
        })
        .expect("the fixture was built");
        /// ONE CALL, answered.
        fn call_at(program: &mut qymcad::LiveLoop, code: &str, given: serde_json::Value) -> TheCall {
            let code = code.to_string();
            crate::checks::ask_the_core(program, move |core| call(core, &code, &given)).expect("the call answered")
        }
        /// THE OPEN SKETCH, read whole.
        fn the_open(core: &mut qymcad::AgentSession) -> qymcad::SketchInfo {
            let document = core.document();
            document.sketches.iter().find(|s| Some(&s.name) == document.editing.as_ref()).expect("the open sketch").clone()
        }
        /// TWO LINES MEETING AT A CORNER, drawn beside everything that stood before.
        fn an_l(program: &mut qymcad::LiveLoop, at: f64) {
            call_at(program, "sketch.line", serde_json::json!({ "points": [[at, 0.0], [at + 40.0, 0.0]] }));
            call_at(program, "sketch.line", serde_json::json!({ "points": [[at + 40.0, 0.0], [at + 40.0, 30.0]] }));
        }
        // A CORNER ROUNDED: an arc that was not there
        an_l(&mut program, 0.0);
        let answered = call_at(&mut program, "sketch.corner", serde_json::json!({ "points": [[20.0, 0.0], [40.0, 15.0]], "radius": "5" }));
        assert!(matches!(answered, TheCall::Answered { .. }), "the corner did not run: {answered:?}");
        let arcs = crate::checks::ask_the_core(&mut program, |core| the_open(core).arcs).expect("the arcs were read");
        assert_eq!(arcs, 1, "the rounded corner left {arcs} arcs");
        // A CORNER BEVELLED: a line that was not there, and no arc
        an_l(&mut program, 100.0);
        let answered = call_at(&mut program, "sketch.corner-chamfer", serde_json::json!({ "points": [[120.0, 0.0], [140.0, 15.0]], "size": "4" }));
        assert!(matches!(answered, TheCall::Answered { .. }), "the chamfer did not run: {answered:?}");
        let (lines, arcs) = crate::checks::ask_the_core(&mut program, |core| (the_open(core).lines, the_open(core).arcs)).expect("the sheet was read");
        assert_eq!((lines, arcs), (5, 1), "the bevel left {lines} lines and {arcs} arcs (4 lines and the one round corner stood)");
        // AN OFFSET: the chain copied beside itself
        call_at(&mut program, "sketch.line", serde_json::json!({ "points": [[0.0, 100.0], [30.0, 100.0]] }));
        call_at(&mut program, "sketch.line", serde_json::json!({ "points": [[30.0, 100.0], [30.0, 120.0]] }));
        let before = crate::checks::ask_the_core(&mut program, |core| the_open(core).lines).expect("the lines were read");
        let answered = call_at(&mut program, "sketch.offset", serde_json::json!({ "points": [[10.0, 100.0], [30.0, 105.0], [25.0, 95.0]], "distance": "3" }));
        assert!(matches!(answered, TheCall::Answered { .. }), "the offset did not run: {answered:?}");
        let after = crate::checks::ask_the_core(&mut program, |core| the_open(core).lines).expect("the lines were read");
        assert_eq!(after, before + 2, "the offset left {after} lines where {before} stood and 2 more were owed");
        // A MIRROR: the reflection standing about its axis
        call_at(&mut program, "sketch.line", serde_json::json!({ "points": [[100.0, 100.0], [140.0, 100.0]] }));
        call_at(&mut program, "sketch.line", serde_json::json!({ "points": [[150.0, 90.0], [150.0, 110.0]] }));
        let answered = call_at(&mut program, "sketch.mirror", serde_json::json!({ "points": [[115.0, 100.0], [150.0, 100.0]] }));
        assert!(matches!(answered, TheCall::Answered { .. }), "the mirror did not run: {answered:?}");
        let reflected =
            crate::checks::ask_the_core(&mut program, |core| the_open(core).places.iter().any(|p| (p[0] - 160.0).abs() < 1e-6 && (p[1] - 100.0).abs() < 1e-6)).expect("the places were read");
        assert!(reflected, "the reflection of the line about x 150 did not stand from 160");
        // A DELETION: what the places named, gone
        call_at(&mut program, "sketch.line", serde_json::json!({ "points": [[0.0, 200.0], [40.0, 200.0]] }));
        let stood = crate::checks::ask_the_core(&mut program, |core| the_open(core).lines).expect("the lines were read");
        let answered = call_at(&mut program, "sketch.delete", serde_json::json!({ "points": [[15.0, 200.0]] }));
        assert!(matches!(answered, TheCall::Answered { .. }), "the deletion did not run: {answered:?}");
        let after = crate::checks::ask_the_core(&mut program, |core| the_open(core).lines).expect("the lines were read");
        assert_eq!(after, stood - 1, "the deletion left the line standing");
        // A DELETION THAT NAMES NOTHING is refused in the program's words, the sheet as it stood
        let the_document_as_it_stood = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        let refused = call_at(&mut program, "sketch.delete", serde_json::json!({ "points": [[500.0, 500.0]] }));
        let TheCall::Refused(words) = &refused else { panic!("a deletion over nothing at all was not refused: {refused:?}") };
        assert!(!words.trim().is_empty(), "the refused deletion said nothing");
        let after = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        assert_eq!(after, the_document_as_it_stood, "a refused deletion changed the document");
        // THE PROJECTION: the edges of the body the sketch looks at, driven; over a plane sketch that
        // looks at no body, refused in words, the document as it stood
        let refused = call_at(&mut program, "sketch.project", serde_json::json!({ "points": [[20.0, 0.0]] }));
        let TheCall::Refused(words) = &refused else { panic!("a projection over a sketch that looks at no body was not refused: {refused:?}") };
        assert!(!words.trim().is_empty(), "the refused projection said nothing");
        let after = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        assert_eq!(after, the_document_as_it_stood, "a refused projection changed the document");
        let mut on_a_face = qymcad::LiveLoop::start();
        let answered = crate::checks::ask_the_core(&mut on_a_face, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            let rect = core.word("tb-rect-hint");
            core.press_hint(&rect);
            core.click_on_sketch(0.0, 0.0).click_on_sketch(40.0, 30.0);
            let finish = core.word("wb-finish");
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
            let extrude = core.word("tb-extrude-hint");
            core.press_hint(&extrude);
            core.key(qymcad::Key::Enter);
            let pencil = core.word("g-sketch-pick-hint");
            core.press_hint(&pencil);
            let at = core.face_at([20.0, 15.0, 10.0]);
            core.click(at);
            call(core, "sketch.project", &serde_json::json!({ "points": [[20.0, 0.0]] }))
        })
        .expect("the projection answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the projection over the face of the block did not run: {answered:?}");
        let driven = crate::checks::ask_the_core(&mut on_a_face, |core| {
            let open = the_open(core);
            (open.lines, open.points)
        })
        .expect("the sketch was read");
        assert_eq!(driven, (1, 2), "the projected edge did not land as a line of two driven points: {driven:?}");
    }

    /// THE TWELVE CONSTRAINTS OF A SKETCH, tied through the person's vocabulary over the agent's own
    /// session - the same words and clicks a person makes, the ones the escape hatch will carry: the
    /// geometry drawn, the pair picked (a click, Shift for the second), the button of the constraint
    /// pressed by its hint. Every constraint lands in the sketch as its own kind, takes the degrees of
    /// freedom it owes, and the program says its own word for what was added; a pair that cannot take
    /// a constraint is told so and nothing is tied.
    #[test]
    fn the_twelve_constraints_tie_what_the_person_s_vocabulary_picks() {
        let _turn = crate::checks::take_your_turn();
        /// AN OPEN, EMPTY SKETCH with the auto constraints off, over the agent's own loop - what a check
        /// of one constraint sees is what that constraint tied and nothing the program added while
        /// drawing.
        fn an_empty_sketch() -> qymcad::LiveLoop {
            let mut program = qymcad::LiveLoop::start();
            crate::checks::ask_the_core(&mut program, |core| {
                let new_part = core.word("start-new-part");
                core.press_word(&new_part);
                let xy = core.word("plane-xy-table");
                core.press_word(&xy);
                let auto = core.word("wb-auto-constraints");
                let switch = core.widgets().into_iter().find(|w| w.label.ends_with(&auto)).expect("the switch of the auto constraints");
                if switch.checked == Some(true) {
                    core.click(switch.rect.center());
                }
            })
            .expect("the fixture was built");
            program
        }
        /// DRAW with the tool of the hint, clicking the places in turn, the tool put down and the
        /// arrow taken back - what the next pick clicks with.
        fn draw(core: &mut qymcad::AgentSession, hint: &str, places: &[(f64, f64)]) {
            let word = core.word(hint);
            core.press_hint(&word);
            for (x, y) in places {
                core.click_on_sketch(*x, *y);
            }
            core.key(qymcad::Key::Escape);
            take_the_arrow(core);
        }
        /// PICK what lies at the place, adding to the set when `shift`.
        fn pick(core: &mut qymcad::AgentSession, x: f64, y: f64, shift: bool) {
            if shift {
                let at = core.on_sketch(x, y);
                core.click_with(at, qymcad::PointerButton::Primary, qymcad::Modifiers::SHIFT);
            } else {
                core.click_on_sketch(x, y);
            }
        }
        /// THE PAIR EACH CONSTRAINT IS TIED ON, drawn and picked: `(hint, kind, degrees of freedom it takes)`.
        fn the_pair_of(core: &mut qymcad::AgentSession, hint: &str) -> (&'static str, i32) {
            match hint {
                "con-coincident-hint" => {
                    draw(core, "tb-line-hint", &[(0.0, 0.0), (20.0, 0.0)]);
                    draw(core, "tb-line-hint", &[(30.0, 10.0), (50.0, 10.0)]);
                    pick(core, 20.0, 0.0, false);
                    pick(core, 30.0, 10.0, true);
                    ("Coincident", 2)
                }
                "con-horizontal-hint" => {
                    draw(core, "tb-line-hint", &[(0.0, 0.0), (20.0, 20.0)]);
                    pick(core, 10.0, 10.0, false);
                    ("Horizontal", 1)
                }
                "con-vertical-hint" => {
                    draw(core, "tb-line-hint", &[(0.0, 0.0), (20.0, 20.0)]);
                    pick(core, 10.0, 10.0, false);
                    ("Vertical", 1)
                }
                "con-parallel-hint" => {
                    draw(core, "tb-line-hint", &[(0.0, 0.0), (20.0, 0.0)]);
                    draw(core, "tb-line-hint", &[(0.0, 10.0), (20.0, 30.0)]);
                    pick(core, 10.0, 0.0, false);
                    pick(core, 10.0, 20.0, true);
                    ("Parallel", 1)
                }
                "con-perpendicular-hint" => {
                    draw(core, "tb-line-hint", &[(0.0, 0.0), (20.0, 0.0)]);
                    draw(core, "tb-line-hint", &[(0.0, 10.0), (20.0, 30.0)]);
                    pick(core, 10.0, 0.0, false);
                    pick(core, 10.0, 20.0, true);
                    ("Perpendicular", 1)
                }
                "con-equal" => {
                    draw(core, "tb-line-hint", &[(0.0, 0.0), (20.0, 0.0)]);
                    draw(core, "tb-line-hint", &[(0.0, 10.0), (50.0, 10.0)]);
                    pick(core, 10.0, 0.0, false);
                    pick(core, 25.0, 10.0, true);
                    ("Equal", 1)
                }
                "con-fix" => {
                    draw(core, "tb-line-hint", &[(0.0, 0.0), (20.0, 0.0)]);
                    pick(core, 20.0, 0.0, false);
                    ("Fixed", 2)
                }
                "con-collinear-hint" => {
                    draw(core, "tb-line-hint", &[(0.0, 0.0), (20.0, 0.0)]);
                    draw(core, "tb-line-hint", &[(30.0, 10.0), (50.0, 10.0)]);
                    // the first line held by both ends, so the second is what has to come onto it
                    pick(core, 0.0, 0.0, false);
                    pick(core, 20.0, 0.0, true);
                    let fix = core.word("con-fix");
                    core.press_hint(&fix);
                    pick(core, 10.0, 0.0, false);
                    pick(core, 40.0, 10.0, true);
                    ("Collinear", 2)
                }
                "con-concentric-hint" => {
                    draw(core, "tb-circle-hint", &[(0.0, 0.0), (5.0, 0.0)]);
                    draw(core, "tb-circle-hint", &[(30.0, 0.0), (38.0, 0.0)]);
                    pick(core, 5.0, 0.0, false);
                    pick(core, 38.0, 0.0, true);
                    ("Concentric", 2)
                }
                "con-tangent-hint" => {
                    draw(core, "tb-circle-hint", &[(0.0, 0.0), (10.0, 0.0)]);
                    draw(core, "tb-line-hint", &[(20.0, -5.0), (20.0, 5.0)]);
                    pick(core, 10.0, 0.0, false);
                    pick(core, 20.0, 0.0, true);
                    ("Tangent", 1)
                }
                "con-symmetric-hint" => {
                    draw(core, "tb-point-hint", &[(5.0, 10.0)]);
                    draw(core, "tb-point-hint", &[(-25.0, -30.0)]);
                    pick(core, 5.0, 10.0, false);
                    pick(core, -25.0, -30.0, true);
                    pick(core, 0.0, 20.0, true); // the Y axis of the sheet
                    ("Symmetric", 2)
                }
                _ => {
                    draw(core, "tb-line-hint", &[(0.0, 0.0), (20.0, 0.0)]);
                    draw(core, "tb-point-hint", &[(10.0, 15.0)]);
                    // the line held by both ends, so the point is what has to move
                    pick(core, 0.0, 0.0, false);
                    pick(core, 20.0, 0.0, true);
                    let fix = core.word("con-fix");
                    core.press_hint(&fix);
                    pick(core, 10.0, 15.0, false);
                    pick(core, 10.0, 0.0, true);
                    ("Midpoint", 2)
                }
            }
        }
        for hint in [
            "con-coincident-hint",
            "con-horizontal-hint",
            "con-vertical-hint",
            "con-parallel-hint",
            "con-perpendicular-hint",
            "con-equal",
            "con-fix",
            "con-collinear-hint",
            "con-concentric-hint",
            "con-tangent-hint",
            "con-symmetric-hint",
            "con-midpoint-hint",
        ] {
            let mut program = an_empty_sketch();
            let answered = crate::checks::ask_the_core(&mut program, |core| {
                let (kind, takes) = the_pair_of(core, hint);
                let dof = the_open_sketch(core).map(|s| s.dof).unwrap_or(0);
                let button = core.word(hint);
                core.press_hint(&button);
                (kind, takes, dof, core.status())
            })
            .expect("the pair was drawn");
            let (kind, takes, dof, said) = answered;
            let after = crate::checks::ask_the_core(&mut program, |core| {
                let open = the_open_sketch(core).expect("the sketch stands open");
                (open.constraint_kinds.clone(), open.dof, core.word("sk-constraint-added"))
            })
            .expect("the sketch was read");
            assert!(after.0.iter().any(|k| k == kind), "{hint}: the sketch holds no {kind}: {:?}", after.0);
            assert_eq!(after.1, dof - takes, "{hint}: the {kind} had to take {takes} of {dof}, and {} are left", after.1);
            assert_eq!(said, after.2, "{hint}: the program said {said:?} instead of its word for what was added");
        }
    }

    /// MOVING, COPYING AND ROTATING through the person's vocabulary over the agent's own session - the
    /// three clicks of the move tools (what, the base point, the target) and the popup the rotation
    /// types its angle into - the same gestures the escape hatch will carry, checked by where the
    /// geometry stands after each.
    #[test]
    fn moving_copying_and_rotating_go_through_the_person_s_vocabulary() {
        let _turn = crate::checks::take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut program, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            // the auto constraints off: a line they have laid Horizontal cannot be turned by a
            // rotation - the solver holds it and says so
            let auto = core.word("wb-auto-constraints");
            let switch = core.widgets().into_iter().find(|w| w.label.ends_with(&auto)).expect("the switch of the auto constraints");
            if switch.checked == Some(true) {
                core.click(switch.rect.center());
            }
        })
        .expect("the fixture was built");
        /// THE LINE UNDER A PLACE OF THE SHEET, as its two ends.
        fn the_line(core: &mut qymcad::AgentSession, at: (f64, f64)) -> ((f64, f64), (f64, f64)) {
            match core.sketch_under(at.0, at.1) {
                Some(qymcad::SketchPick::Line { from, to }) => (from, to),
                other => panic!("no line lies at {at:?}: {other:?}"),
            }
        }
        /// A LINE OF 20 from `at`, drawn with the line tool and left put down.
        fn draw_a_line(core: &mut qymcad::AgentSession, at: (f64, f64)) {
            let word = core.word("tb-line-hint");
            core.press_hint(&word);
            core.click_on_sketch(at.0, at.1);
            core.click_on_sketch(at.0 + 20.0, at.1);
            core.key(qymcad::Key::Escape);
        }
        // A MOVE: the tool, the line clicked, the base point, the target - the line stands 30 further on
        let moved = crate::checks::ask_the_core(&mut program, |core| {
            draw_a_line(core, (0.0, 0.0));
            let word = core.word("tb-move-hint");
            core.press_hint(&word);
            core.click_on_sketch(10.0, 0.0);
            core.click_on_sketch(0.0, 0.0);
            core.click_on_sketch(30.0, 0.0);
            the_line(core, (40.0, 0.0))
        })
        .expect("the move ran");
        assert!((moved.0 .0 - 30.0).abs() < 1e-6 && (moved.1 .0 - 50.0).abs() < 1e-6, "the moved line stands at {moved:?}, not 30 further on");
        // A COPY: the line picked where the click grips it, the target clicked where the copy goes - a
        // second line stands twenty above the first
        let copied = crate::checks::ask_the_core(&mut program, |core| {
            let word = core.word("tb-copy-hint");
            core.press_hint(&word);
            core.click_on_sketch(35.0, 0.0);
            core.click_on_sketch(35.0, 20.0);
            let open = the_open_sketch(core).expect("the sketch stands open");
            (open.lines, open.places.iter().any(|p| (p[0] - 30.0).abs() < 1e-6 && (p[1] - 20.0).abs() < 1e-6))
        })
        .expect("the copy ran");
        assert!(copied.0 >= 2 && copied.1, "the copy left {} lines and no point at (30, 20): {:?}", copied.0, copied.1);
        // A ROTATION: the line clicked, the centre, the angle typed into the popup - the line stands
        // 30 deg from where it did
        let turned = crate::checks::ask_the_core(&mut program, |core| {
            draw_a_line(core, (100.0, 0.0));
            let word = core.word("tb-rotate-hint");
            core.press_hint(&word);
            core.click_on_sketch(110.0, 0.0);
            core.click_on_sketch(100.0, 0.0);
            core.settle(); // the popup's field takes the focus a frame after the centre is set
            core.type_text("30");
            core.key(qymcad::Key::Enter);
            // the middle of the turned line: 10 out along 30 deg from its held end at (100, 0)
            the_line(core, (100.0 + 10.0 * 30f64.to_radians().cos(), 10.0 * 30f64.to_radians().sin()))
        })
        .expect("the rotation ran");
        let angle = (turned.1 .1 - turned.0 .1).atan2(turned.1 .0 - turned.0 .0).to_degrees();
        assert!((angle - 30.0).abs() < 1e-6, "the rotated line stands at {angle} deg, not 30");
    }

    /// THE PRIMITIVES, run as calls: the sizes carried by the call filled into the fields that stand at
    /// the geometry, Enter, and the body of the shape in the part after it - counted by the node the
    /// timeline gains, weighed by the volume the body holds. A size the shape will not take (no radius)
    /// is the program's own refusal and the part holds no body it did not hold.
    #[test]
    fn the_primitives_make_their_bodies() {
        let _turn = crate::checks::take_your_turn();
        /// A FRESH PART, stood in, no sketch, no body.
        fn a_fresh_part() -> qymcad::LiveLoop {
            let mut program = qymcad::LiveLoop::start();
            crate::checks::ask_the_core(&mut program, |core| {
                core.key(qymcad::Key::Escape);
                let new_part = core.word("tb-new-part-hint");
                core.press_hint(&new_part);
            })
            .expect("the fixture was built");
            program
        }
        /// THE SIZES A PRIMITIVE TAKES, read from the live tool by their painted captions, every number
        /// field set to `size`.
        fn the_sizes(program: &mut qymcad::LiveLoop, code: &str, size: f64) -> serde_json::Value {
            let code = code.to_string();
            crate::checks::ask_the_core(program, move |core| {
                // every field of a primitive is one of its sizes, and each shape gets sizes it can
                // build: a cone of equal radii is nothing, a tube as thick as its ring is too
                let of = |name: &str| {
                    let n = name.to_lowercase();
                    if n.contains("top") {
                        5.0
                    } else if n.contains("bottom") {
                        10.0
                    } else if n.contains("ring") {
                        12.0
                    } else if n.contains("tube") {
                        4.0
                    } else if n.contains("radius") {
                        8.0
                    } else {
                        size
                    }
                };
                let mut given = serde_json::Map::new();
                for p in parameters(core, &code) {
                    given.insert(p.name.clone(), serde_json::json!(of(&p.name).to_string()));
                }
                serde_json::Value::Object(given)
            })
            .expect("the schema was read")
        }
        for (code, kind) in [("part.box", "Box3"), ("part.cylinder", "Cylinder"), ("part.sphere", "Sphere"), ("part.cone", "Cone"), ("part.torus", "Torus"), ("part.prism", "Prism")] {
            let mut program = a_fresh_part();
            let given = the_sizes(&mut program, code, 15.0);
            assert!(!given.as_object().expect("the sizes are an object").is_empty(), "{code} puts up no size field");
            let answered = crate::checks::ask_the_core(&mut program, {
                let given = given.clone();
                let code = code.to_string();
                move |core| call(core, &code, &given)
            })
            .expect("the call answered");
            assert!(matches!(answered, TheCall::Answered { .. }), "{code} did not run: {answered:?}");
            let made = crate::checks::ask_the_core(&mut program, move |core| {
                let document = core.document();
                let node = document.features.iter().rfind(|f| f.kind == kind);
                (node.map(|n| (n.error.is_none(), n.suppressed)), document.bodies.iter().map(|b| b.volume).fold(0.0, f64::max))
            })
            .expect("the document was read");
            assert_eq!(made.0, Some((true, false)), "{code} left its node unbuilt or suppressed");
            assert!(made.1 > 0.0, "{code} left no body of any volume");
        }
        // A SPHERE OF NO RADIUS is refused in the program's words, the part as it stood
        let mut program = a_fresh_part();
        let stood = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "part.sphere", &serde_json::json!({ "Radius": "0" }))).expect("the call answered");
        let TheCall::Refused(words) = &answered else { panic!("a sphere of no radius was not refused: {answered:?}") };
        assert!(!words.trim().is_empty(), "the refused sphere said nothing");
        let after = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        assert_eq!(after, stood, "a refused primitive changed the document");
    }

    /// THE PROBLEMS THAT STAND AFTER A CALL are carried in its answer, in the words the acceptance
    /// checks use: nothing waiting for a click with no bar saying so, no bar standing with nothing in
    /// hand behind it, no key of the catalogue on screen instead of words, no letter the font cannot
    /// draw. The check reads what stands beside what the call answered, and the answer carries exactly
    /// that. A call that leaves the program in a state a person would be warned about cannot be made
    /// here by any means - the program puts a bar behind every picker it arms and clears a stray one in
    /// the frame the read settles into (probed: every take of the catalogue, over a bare program, a
    /// block, an open sketch, and each followed by Undo, stands clean) - so the words of the problems
    /// are the oracle's own, and the carrying is what is checked.
    #[test]
    fn the_problems_that_stand_after_a_call_are_carried_in_its_answer() {
        let _turn = crate::checks::take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        let answered = crate::checks::ask_the_core(&mut program, |core| {
            let answered = call(core, "part.box", &serde_json::json!({}));
            let standing = core.the_standing_problems();
            let held: Vec<String> = Vec::new();
            put_down_what_the_take_landed(core, &held);
            (answered, standing)
        })
        .expect("the call answered");
        let (TheCall::Answered { problems, .. }, standing) = &answered else { panic!("the box did not run: {answered:?}") };
        assert_eq!(problems, standing, "the answer does not carry the problems that stand");
        assert!(problems.is_empty(), "a healthy take left a problem: {problems:?}");
    }

    /// THE TOOLS DRAWN BY CLICKS - a point, an arc by its centre and ends, a circle through three
    /// points, a polygon of a named number of sides, a slot, an ellipse, a text of given words, and a
    /// spline ended by a double click - each run as a call over one open sketch, each checked by what
    /// landed in the document, its own kind counted. The polygon's sides and the text's words are
    /// carried as the call's parameters, filled into the bar the take put up.
    #[test]
    fn the_clicked_tools_draw_their_own_kinds() {
        let _turn = crate::checks::take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut program, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
        })
        .expect("the fixture was built");
        /// THE OPEN SKETCH, read whole.
        fn the_open(core: &mut qymcad::AgentSession) -> qymcad::SketchInfo {
            let document = core.document();
            document.sketches.iter().find(|s| Some(&s.name) == document.editing.as_ref()).expect("the open sketch").clone()
        }
        /// THE THINGS OF A SKETCH, counted together.
        fn things(s: &qymcad::SketchInfo) -> usize {
            s.points + s.lines + s.arcs + s.circles + s.ellipses + s.splines + s.texts
        }
        /// ONE DRAWING CALL, answered; the field the tool's bar puts up, read from the live schema,
        /// carried with the value the check names.
        fn draw(program: &mut qymcad::LiveLoop, code: &str, places: &[[f64; 2]], field: Option<(String, String)>) -> TheCall {
            let code = code.to_string();
            let places = places.to_vec();
            let field = field.map(|(n, v)| (n.to_string(), v));
            crate::checks::ask_the_core(program, move |core| {
                let schema = parameters(core, &code);
                let mut given = serde_json::Map::new();
                given.insert("points".into(), serde_json::json!(places));
                if let Some((name, value)) = &field {
                    assert!(schema.iter().any(|p| p.name == *name), "{code} puts up no field {name:?}: {schema:?}");
                    given.insert(name.clone(), serde_json::json!(value));
                }
                call(core, &code, &serde_json::Value::Object(given))
            })
            .expect("the call answered")
        }
        /// THE FIELD A TOOL'S BAR PUTS UP, first of its schema beside the places, read from the live
        /// tool.
        fn the_first_field(program: &mut qymcad::LiveLoop, code: &str) -> Option<String> {
            let code = code.to_string();
            crate::checks::ask_the_core(program, move |core| parameters(core, &code).into_iter().find(|p| p.name != "points").map(|p| p.name)).expect("the schema was read")
        }
        let before = crate::checks::ask_the_core(&mut program, the_open).expect("the sketch was read");
        // A POINT
        let answered = draw(&mut program, "sketch.point", &[[5.0, 5.0]], None);
        assert!(matches!(answered, TheCall::Answered { .. }), "the point did not run: {answered:?}");
        let after = crate::checks::ask_the_core(&mut program, the_open).expect("the sketch was read");
        assert_eq!(after.points, before.points + 1, "the point did not land");
        // AN ARC by its centre and two ends
        let answered = draw(&mut program, "sketch.arc", &[[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]], None);
        assert!(matches!(answered, TheCall::Answered { .. }), "the arc did not run: {answered:?}");
        let arcs = crate::checks::ask_the_core(&mut program, the_open).expect("the sketch was read");
        assert_eq!(arcs.arcs, after.arcs + 1, "the arc did not land");
        // A CIRCLE THROUGH THREE POINTS
        let answered = draw(&mut program, "sketch.circle3", &[[10.0, 0.0], [0.0, 10.0], [-10.0, 0.0]], None);
        assert!(matches!(answered, TheCall::Answered { .. }), "the three-point circle did not run: {answered:?}");
        let circles = crate::checks::ask_the_core(&mut program, the_open).expect("the sketch was read");
        assert_eq!(circles.circles, arcs.circles + 1, "the three-point circle did not land");
        // A POLYGON of six sides: its sides carried as the call's parameter, filled into the bar
        let sides = the_first_field(&mut program, "sketch.polygon");
        let answered = draw(&mut program, "sketch.polygon", &[[0.0, 0.0], [15.0, 0.0]], sides.map(|n| (n, "6".to_string())));
        assert!(matches!(answered, TheCall::Answered { .. }), "the polygon did not run: {answered:?}");
        let polygon = crate::checks::ask_the_core(&mut program, the_open).expect("the sketch was read");
        assert_eq!(polygon.lines, circles.lines + 6, "the polygon's six sides did not land: {} lines", polygon.lines);
        // A SLOT
        let answered = draw(&mut program, "sketch.slot", &[[0.0, 0.0], [30.0, 0.0], [30.0, 5.0]], None);
        assert!(matches!(answered, TheCall::Answered { .. }), "the slot did not run: {answered:?}");
        let slot = crate::checks::ask_the_core(&mut program, the_open).expect("the sketch was read");
        assert!(things(&slot) > things(&polygon), "the slot did not land");
        // AN ELLIPSE by its centre and the ends of its axes
        let answered = draw(&mut program, "sketch.ellipse", &[[0.0, 0.0], [20.0, 0.0], [0.0, 10.0]], None);
        assert!(matches!(answered, TheCall::Answered { .. }), "the ellipse did not run: {answered:?}");
        let ellipse = crate::checks::ask_the_core(&mut program, the_open).expect("the sketch was read");
        assert_eq!(ellipse.ellipses, slot.ellipses + 1, "the ellipse did not land");
        // A TEXT of given words, placed by a click
        let answered = draw(&mut program, "sketch.text", &[[0.0, 0.0]], Some(("text".to_string(), "a note".to_string())));
        assert!(matches!(answered, TheCall::Answered { .. }), "the text did not run: {answered:?}");
        let text = crate::checks::ask_the_core(&mut program, the_open).expect("the sketch was read");
        assert!(text.texts + text.notes > ellipse.texts + ellipse.notes, "the text did not land");
        // A SPLINE, ended by a double click on its last node
        let answered = draw(&mut program, "sketch.spline", &[[0.0, 0.0], [10.0, 10.0], [20.0, 0.0]], None);
        assert!(matches!(answered, TheCall::Answered { .. }), "the spline did not run: {answered:?}");
        let spline = crate::checks::ask_the_core(&mut program, the_open).expect("the sketch was read");
        assert_eq!(spline.splines, ellipse.splines + 1, "the spline did not land");
    }
}
