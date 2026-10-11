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

/// THE RUNS OF RICHER SHAPE: the drawing tools of the sketch, each taking its places on the sheet. The
/// rest of the operations keep the take until their own runs arrive.
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
/// its caption or grey words, its kind, and what it holds.
fn the_fields_on_screen(core: &mut qymcad::AgentSession) -> Vec<Parameter> {
    core.widgets()
        .into_iter()
        .filter(|w| matches!(w.kind, qymcad::Kind::TextField | qymcad::Kind::Number | qymcad::Kind::ComboBox | qymcad::Kind::Slider))
        .map(|w| Parameter { name: the_words_it_reads_by(&w), kind: the_kind_word(w.kind).to_string(), says: w.value })
        .collect()
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
            core.widgets()
                .into_iter()
                .filter(|w| matches!(w.kind, qymcad::Kind::TextField | qymcad::Kind::Number | qymcad::Kind::ComboBox | qymcad::Kind::Slider))
                .map(|w| (if w.label.is_empty() { w.placeholder } else { w.label }, w.value))
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
        // OVER AN OPEN SKETCH every run that takes parameters runs with them: the drawing tools land
        // their own kinds, their places carried as the call carries them
        let mut sketching = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut sketching, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
        })
        .expect("the fixture was built");
        let mut aside = 0.0;
        for entry in the_registry().iter().filter(|e| !e.takes.is_empty()) {
            let code = entry.command.code;
            // each drawing stands aside of the ones before it, so nothing lands on what exists
            aside += 30.0;
            let (a, b, c) = (aside + 5.0, aside + 15.0, aside + 15.0);
            let answered = crate::checks::ask_the_core(&mut sketching, move |core| {
                let the_run = by_code(code).expect("the code names an entry").run.expect("the entry is wired");
                let mut given = serde_json::Map::new();
                given.insert("points".into(), serde_json::json!([[a, 5.0], [b, 5.0], [c, 15.0]]));
                given.insert("text".into(), serde_json::json!("a note"));
                let answered = the_run(core, code, &serde_json::Value::Object(given));
                let held: Vec<String> = Vec::new();
                put_down_what_the_take_landed(core, &held);
                answered
            })
            .expect("the run answered");
            assert!(matches!(answered, Answered::Ran(_)), "{} drew nothing over the open sketch: {answered:?}", entry.command.code);
        }
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
