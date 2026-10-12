//! THE REGISTRY OF OPERATIONS the agent calls by name: every command of the program's catalogue,
//! derived from it - never written beside it - and beside them the operations of the whole program
//! (the view, the file, the table of parameters), derived from the help articles that name them. A
//! command the panel offers cannot be missing from the agent's reach, and a row the catalogue never
//! held cannot appear in it; the guard in this module's checks holds that link in both directions.
//!
//! Each entry carries what a client needs to find and call an operation: its code, its names in each
//! language the program speaks, a one-line synopsis from its help article, its workbench, the words it is
//! also found by, and its run - the gestures a person would make through the same interface. The names and
//! the synopsis are read through the catalogue, the dictionary and the help articles as they stand, so
//! they cannot drift from what the window shows; only the synonyms are the registry's own words, for
//! neither the catalogue nor the help holds any.

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
    let fields = parameters(core, entry.code());
    if let Err(words) = checked_against(given, &fields, entry.takes) {
        return TheCall::Refused(words);
    }
    let Some(the_run) = entry.run else {
        return TheCall::Refused(format!(
            "{} ({}) is a command of the program, and the agent serves no operation for it; read-tool-help tells what it does, search-tool finds what the program offers",
            entry.name(),
            entry.code()
        ));
    };
    let answered = the_run(core, entry.code(), given);
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
        if take.kind == "picks" && !the_triples(Some(value)).is_some_and(|p| !p.is_empty()) {
            return Err(format!("{name} takes places in the space of the model as [x, y, z] triples, and the call carried {value}"));
        }
        if take.kind == "sketches" && !the_names_of(Some(value)).is_some_and(|s| s.len() >= 2) {
            return Err(format!("{name} takes the names of finished sketches of the tree, two at least, and the call carried {value}"));
        }
        if take.kind == "joints" && !the_names_of(Some(value)).is_some_and(|s| s.len() >= 2) {
            return Err(format!("{name} takes the names of the joints of the list of mates, two at least, and the call carried {value}"));
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
const SYNONYMS: &[(&str, &[&str])] = &[
    ("part.fillet", &["round", "rounding"]),
    ("part.chamfer", &["bevel"]),
    ("part.shell", &["hollow"]),
    ("part.hole", &["drill", "bore"]),
    ("assembly.joint", &["mate", "mates", "fasten", "hinge"]),
    ("assembly.ground", &["anchor", "fix"]),
    ("assembly.group", &["weld"]),
    ("assembly.tangent", &["touch"]),
    ("assembly.width", &["midway"]),
    ("assembly.relation", &["gear", "rack", "screw", "linear"]),
    ("part.measure", &["distance", "angle", "diameter", "ruler"]),
    ("assembly.section", &["cut", "clip", "half"]),
];

/// ONE OPERATION OF THE PROGRAM, as the agent calls it by name.
pub(crate) struct Entry {
    /// WHAT THE OPERATION ANSWERS BY: the command of the catalogue it is derived from, or - for the
    /// operations of the whole program, which no workbench owns and no command of the catalogue
    /// launches - its own code and the article of the help that names it.
    pub(crate) by: By,
    /// The words the operation is also found by.
    pub(crate) synonyms: &'static [&'static str],
    /// ITS RUN over the agent's session; `None` where none is wired yet, and a call of the entry answers
    /// that no operation is served for its command.
    pub(crate) run: Option<Run>,
    /// The parameters the run takes beside the bar's own fields; empty where the run is the take.
    pub(crate) takes: &'static [Take],
}

/// WHAT AN OPERATION ANSWERS BY.
pub(crate) enum By {
    /// A command of the catalogue: the entry is derived from its row - the code, the workbench, the
    /// names in each language, the article F1 opens.
    Command(&'static qymcad::Command),
    /// AN OPERATION OF THE WHOLE PROGRAM - the view, the file, the table of parameters: called by its
    /// own code (`area.name`, the catalogue's own style), named in each language by the word of the
    /// dictionary its own surface carries (a menu item, a bar button), answered by its help article,
    /// living in no workbench (`general`).
    OfTheProgram { code: &'static str, article: &'static str, name_key: &'static str },
}

impl Entry {
    /// THE CODE the operation is called by: its command's own where there is one.
    pub(crate) fn code(&self) -> &'static str {
        match &self.by {
            By::Command(command) => command.code,
            By::OfTheProgram { code, .. } => code,
        }
    }

    /// THE WORKBENCH the operation lives in, for the search's ranking and its filter: `general` for
    /// the operations of the whole program, which no workbench owns and none of them ranks first.
    pub(crate) fn workbench(&self) -> &'static str {
        match &self.by {
            By::Command(command) => command.workbench,
            By::OfTheProgram { .. } => "general",
        }
    }

    /// THE COMMAND OF THE CATALOGUE the entry answers by, where it is one. The runs that take a
    /// command through the search work over these alone; an operation of the whole program is taken
    /// by no command and works its own gestures.
    pub(crate) fn the_command(&self) -> Option<&'static qymcad::Command> {
        match &self.by {
            By::Command(command) => Some(command),
            By::OfTheProgram { .. } => None,
        }
    }

    /// THE NAME of the operation in the language the program speaks now.
    pub(crate) fn name(&self) -> String {
        match &self.by {
            By::Command(command) => command.name(),
            By::OfTheProgram { name_key, .. } => qymcad_i18n::tr(name_key),
        }
    }

    /// THE NAME IN A PARTICULAR LANGUAGE, as the catalogue takes it.
    pub(crate) fn name_in(&self, lang: &str) -> String {
        match &self.by {
            By::Command(command) => command.name_in(lang),
            By::OfTheProgram { name_key, .. } => qymcad_i18n::tr_in(lang, name_key).unwrap_or_default(),
        }
    }

    /// THE HELP ARTICLE the operation answers by.
    pub(crate) fn help_article(&self) -> Option<&'static str> {
        match &self.by {
            By::Command(command) => command.help_article(),
            By::OfTheProgram { article, .. } => Some(article),
        }
    }

    /// THE NAMES OF THE OPERATION, one in each language the program speaks: the word of its own a
    /// command has one, the title of its article otherwise.
    pub(crate) fn names(&self) -> std::collections::BTreeMap<String, String> {
        qymcad_help::languages().into_iter().map(|l| (l.clone(), self.name_in(&l))).collect()
    }

    /// THE ONE-LINE SYNOPSIS: the caption of the first picture of the help article, which says in one
    /// sentence what the operation is; the operation's own name where the article carries no such
    /// picture.
    pub(crate) fn synopsis(&self) -> String {
        let caption = self.help_article().and_then(qymcad_help::article).and_then(|md| {
            qymcad_help::parse(md).into_iter().find_map(|b| match b {
                qymcad_help::Block::Image { alt, .. } => Some(alt),
                _ => None,
            })
        });
        match caption {
            Some(alt) if !alt.is_empty() => alt.iter().map(|s| s.text.as_str()).collect(),
            _ => self.name(),
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
    /// pair; `picks` - places in the space of the model, each an [x, y, z] triple naming what stands
    /// there (an edge, a face, a body, a mesh); `sketches` - the names of finished sketches of the tree,
    /// each clicked into the tool as a person clicks its row.
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

/// THE REGISTRY, derived from the command catalogue and the help: every command of the program, in the
/// panel's order, and beside them the operations of the whole program - the view, the file, the table
/// of parameters - which no workbench owns. Every entry is wired to a run - the take itself where no
/// richer run is - so nothing the program offers can stand outside the agent's reach; the guard below
/// fails the day something does.
pub(crate) fn the_registry() -> &'static [Entry] {
    static THE_REGISTRY: std::sync::OnceLock<Vec<Entry>> = std::sync::OnceLock::new();
    THE_REGISTRY.get_or_init(|| {
        let of_commands = qymcad::COMMANDS.iter().map(|command| {
            let wired = RICHER_RUNS.iter().find(|w| w.code == command.code);
            Entry {
                by: By::Command(command),
                synonyms: SYNONYMS.iter().find(|(code, _)| *code == command.code).map(|(_, words)| *words).unwrap_or(&[]),
                run: wired.map(|w| w.run).or(Some(the_take)),
                takes: wired.map(|w| w.takes).unwrap_or(&[]),
            }
        });
        let of_the_program = crate::general::THE_OPERATIONS_OF_THE_PROGRAM.iter().map(|o| Entry {
            by: By::OfTheProgram { code: o.code, article: o.article, name_key: o.name_key },
            synonyms: o.synonyms,
            run: Some(o.run),
            takes: o.takes,
        });
        of_commands.chain(of_the_program).collect()
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
    Wired { code: "part.extrude", run: work_a_contour, takes: &[] },
    Wired { code: "part.revolve", run: work_a_contour, takes: &[] },
    Wired { code: "part.sweep", run: work_from_the_sketches, takes: &[Take { name: "sketches", kind: "sketches" }] },
    Wired { code: "part.loft", run: work_from_the_sketches, takes: &[Take { name: "sketches", kind: "sketches" }] },
    Wired { code: "part.fillet", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.chamfer", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.shell", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.hole", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.push-face", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.split-body", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.split-face", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.thicken", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.draft", run: tilt_a_face, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.thread", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.remove-face", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.mirror", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.array-linear", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.array-circular", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.face-copy", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.offset-surface", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.surface-replace", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.patch", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.stitch", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.trim", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.recognise", run: work_on_the_body, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.datum-plane", run: stand_a_datum, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.datum-axis", run: stand_a_datum, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "part.datum-point", run: stand_a_datum, takes: &[] },
    Wired { code: "assembly.joint", run: lay_a_joint, takes: &[Take { name: "kind", kind: "text" }, Take { name: "picks", kind: "picks" }] },
    Wired { code: "assembly.ground", run: ground_a_part, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "assembly.group", run: hold_the_parts_together, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "assembly.tangent", run: rest_a_shaft_on_the_face, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "assembly.width", run: sit_the_part_between_the_walls, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "assembly.relation", run: tie_the_mates_together, takes: &[Take { name: "joints", kind: "joints" }] },
    Wired { code: "part.measure", run: measure_between_the_picks, takes: &[Take { name: "picks", kind: "picks" }] },
    Wired { code: "assembly.section", run: cut_the_view_by_a_plane, takes: &[Take { name: "picks", kind: "picks" }] },
];

/// DRAW A CHAIN OF LINES: the tool taken, every place of the sheet clicked in turn, the chain ended as a
/// person ends it, with Esc. Two places make one line; more make a chain of them. The run answers only
/// for a drawing that came to be - the sketch holds more lines after it than before.
fn draw_a_chain(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(places) = the_places(given) else { return Answered::Refused("the line needs its places on the sheet, two at least".into()) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to draw in - open one and draw again".to_string()) };
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    let Some(places) = the_places(given) else { return Answered::Refused(format!("the {} needs its places on the sheet", entry.name())) };
    let Some(sketch) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to draw in - open one and draw again".to_string()) };
    take_the_command(core, the_command_of(entry));
    fill_the_fields(core, given);
    for (x, y) in &places {
        core.click_on_sketch(*x, *y);
    }
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && things(&after) > things(&sketch) => Answered::Ran(format!("{} drawn of {} places of {}", entry.name(), places.len(), sketch.name)),
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
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
    take_the_command(core, the_command_of(entry));
    core.click_on_sketch(side.0, side.1);
    core.type_text(&distance);
    core.key(qymcad::Key::Enter);
    core.key(qymcad::Key::Escape);
    match the_open_sketch(core) {
        Some(after) if after.name == sketch.name && things(&after) > things(&sketch) => Answered::Ran(format!("{} place(s) offset by {distance} of {}", chain.len(), sketch.name)),
        _ => Answered::Refused(format!("no copy was laid: {}", core.status())),
    }
}

/// WORK A CONTOUR into a body - extruded for the extrude of the code, revolved for its revolve: the
/// command taken over the sketch that stands finished, the sizes the call carried filled into the bar,
/// Enter to set the size, Enter to apply. The body of the feature is in the part after it, its node
/// built without error; what the contour cannot make (nothing drawn, a revolve about no axis) is the
/// program's own refusal.
fn work_a_contour(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let kind = if code == "part.revolve" { "Revolve" } else { "Extrude" };
    let before = core.document();
    let (bodies_before, nodes_before) = (before.bodies.len(), before.features.iter().filter(|f| f.kind == kind).count());
    take_the_command(core, the_command_of(entry));
    fill_the_fields(core, given);
    core.key(qymcad::Key::Enter);
    core.key(qymcad::Key::Enter);
    let document = core.document();
    let node = document.features.iter().filter(|f| f.kind == kind).rfind(|f| f.error.is_none());
    match (document.bodies.len() > bodies_before || document.features.iter().filter(|f| f.kind == kind).count() > nodes_before, node) {
        (true, Some(node)) if document.bodies.iter().any(|b| b.volume > 0.0) => Answered::Ran(format!("a {} made ({}), its body in the part", kind, node.name)),
        (true, Some(node)) => Answered::Refused(format!("the {} would not build: {}", kind, node.error.clone().unwrap_or_default())),
        _ => Answered::Refused(format!("nothing was made: {}", core.status())),
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
    take_the_command(core, the_command_of(entry));
    fill_the_fields(core, given);
    core.key(qymcad::Key::Enter);
    let document = core.document();
    let made = document.features.iter().filter(|f| f.kind == kind && !f.suppressed).rfind(|f| f.error.is_none());
    match (document.bodies.len() > bodies_before || document.features.iter().filter(|f| f.kind == kind).count() > nodes_before, made) {
        (true, Some(node)) if !document.bodies.is_empty() && document.bodies.iter().any(|b| b.volume > 0.0) => Answered::Ran(format!("a {} made ({}), its body in the part", entry.name(), node.name)),
        (_, Some(node)) => Answered::Refused(format!("the {} would not build: {}", entry.name(), node.error.clone().unwrap_or_default())),
        _ => Answered::Refused(format!("nothing was made: {}", core.status())),
    }
}

/// WORK ON THE BODY THAT STANDS - an edge rounded or cut, a face bored, hollowed, pushed, tilted,
/// removed, split, a sheet thickened, a thread sized to its cylinder, a body reflected or patterned, a
/// face taken out as a surface, lifted, replaced, spanned over, stitched, trimmed, a mesh recognised
/// back into a body: the command taken, every place in the space of the model clicked (each names what
/// it stands on - an edge of the body, a face of it, the round side of a shaft, a mesh, a sheet), the
/// values the call carried filled into the fields the take put up - a thread sizes itself to the face
/// it was put on, so its fields are filled after its pick, as a person fills them - Enter. The
/// feature's node is in the timeline after it, built without error, and a body still stands; what the
/// picks and values cannot make (nothing to work on, an edge that is not there) is the program's own
/// refusal.
fn work_on_the_body(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let kind = match code {
        "part.chamfer" => "Chamfer",
        "part.shell" => "Shell",
        "part.hole" => "Hole",
        "part.thread" => "Thread",
        "part.push-face" => "PushFace",
        "part.remove-face" => "RemoveFace",
        "part.split-body" => "SplitBody",
        "part.split-face" => "SplitFace",
        "part.thicken" => "Thicken",
        "part.mirror" => "Mirror",
        "part.array-linear" => "LinearArray",
        "part.array-circular" => "CircularArray",
        "part.face-copy" => "FaceCopy",
        "part.offset-surface" => "OffsetSurface",
        "part.surface-replace" => "SurfaceReplace",
        "part.patch" => "Patch",
        "part.stitch" => "Stitch",
        "part.trim" => "Trim",
        "part.recognise" => "MeshRecognised",
        "part.draft" => "Draft",
        _ => "Fillet",
    };
    let Some(picks) = the_triples(given.get("picks")) else {
        return Answered::Refused(format!("the {} needs its places in the space of the model: a place on each edge or face it works on", entry.name()));
    };
    let before = core.document();
    let nodes_before = before.features.iter().filter(|f| f.kind == kind).count();
    take_the_command(core, the_command_of(entry));
    core.settle(); // the take lands the tool a frame later; the picks click what the tool waits for
    for p in &picks {
        let at = core.in_space(*p);
        core.click(at);
    }
    fill_the_fields(core, given);
    core.key(qymcad::Key::Enter);
    let document = core.document();
    let node = document.features.iter().filter(|f| f.kind == kind && !f.suppressed).rfind(|f| f.error.is_none());
    match (document.features.iter().filter(|f| f.kind == kind).count() > nodes_before, node, document.bodies.iter().any(|b| b.volume > 0.0)) {
        (true, Some(node), true) => Answered::Ran(format!("a {} made ({}), the body still in the part", kind, node.name)),
        (true, Some(node), _) => Answered::Refused(format!("the {} would not build: {}", kind, node.error.clone().unwrap_or_default())),
        _ => Answered::Refused(format!("nothing was made: {}", core.status())),
    }
}

/// TILT A FACE by an angle about the face that holds still - the draft: the command taken, the face to
/// tilt clicked, the word of the bar that says the next click is the neutral one pressed between the
/// picks (as the person's own second press does), the neutral face clicked, the angle filled, Enter.
/// The feature's node is in the timeline after it, built without error, and a body still stands; picks
/// that name no face of a body are the program's own refusal, and nothing changed.
fn tilt_a_face(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(picks) = the_triples(given.get("picks")) else {
        return Answered::Refused(format!("the {} needs its places in the space of the model: the face to tilt, then the neutral face it turns about", entry.name()));
    };
    let Some((neutral, to_tilt)) = picks.split_last() else {
        return Answered::Refused("the draft needs two places: the face to tilt, then the neutral face it turns about".into());
    };
    let before = core.document();
    let nodes_before = before.features.iter().filter(|f| f.kind == "Draft").count();
    take_the_command(core, the_command_of(entry));
    core.settle(); // the take lands the tool a frame later; the picks click what the tool waits for
    for p in to_tilt {
        let at = core.in_space(*p);
        core.click(at);
    }
    let neutral_word = core.word("cmd-neutral-face");
    core.press_word_near(&neutral_word, qymcad::pos2(640.0, 0.0));
    let at = core.in_space(*neutral);
    core.click(at);
    fill_the_fields(core, given);
    core.key(qymcad::Key::Enter);
    let document = core.document();
    let node = document.features.iter().filter(|f| f.kind == "Draft" && !f.suppressed).rfind(|f| f.error.is_none());
    match (document.features.iter().filter(|f| f.kind == "Draft").count() > nodes_before, node, document.bodies.iter().any(|b| b.volume > 0.0)) {
        (true, Some(node), true) => Answered::Ran(format!("a Draft made ({}), the body still in the part", node.name)),
        (true, Some(node), _) => Answered::Refused(format!("the Draft would not build: {}", node.error.clone().unwrap_or_default())),
        _ => Answered::Refused(format!("nothing was made: {}", core.status())),
    }
}

/// WORK FROM THE SKETCHES OF THE TREE - a profile swept along a path, sections lofted from one to the
/// next: the first sketch's row clicked in the tree (as the person selects what the tool is to work
/// on), the command taken - it lands in hand with that selection - the rows of the rest clicked in
/// their turn, Enter. The sketches are spent into the body the tool makes; its node stands in the
/// timeline without error and a body holds volume. Sketches the tree does not hold, or sections the
/// kernel cannot join, are the program's own refusal, and nothing changed.
fn work_from_the_sketches(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let kind = if code == "part.loft" { "Loft" } else { "Sweep" };
    let Some(names) = the_names_of(given.get("sketches")) else {
        return Answered::Refused(format!("the {} needs the names of the sketches it works from: the profile then the path, or each section in its turn", entry.name()));
    };
    let Some((first, rest)) = names.split_first() else {
        return Answered::Refused("the tool needs the names of the sketches it works from, two at least".into());
    };
    let held: Vec<String> = core.document().sketches.iter().map(|s| s.name.clone()).collect();
    let Some(row) = core.find(first, qymcad::pos2(0.0, 300.0)) else {
        return Answered::Refused(format!("no sketch named {first:?} is in the tree; the document holds {held:?}"));
    };
    core.click(row.center());
    let before = core.document();
    let nodes_before = before.features.iter().filter(|f| f.kind == kind).count();
    take_the_command(core, the_command_of(entry));
    core.settle(); // the take lands the tool a frame later; the row clicks land on what it waits for
    for name in rest {
        let Some(row) = core.find(name, qymcad::pos2(0.0, 300.0)) else {
            core.key(qymcad::Key::Escape);
            return Answered::Refused(format!("no sketch named {name:?} is in the tree; the document holds {held:?}"));
        };
        core.click(row.center());
    }
    core.key(qymcad::Key::Enter);
    let document = core.document();
    let node = document.features.iter().filter(|f| f.kind == kind && !f.suppressed).rfind(|f| f.error.is_none());
    match (document.features.iter().filter(|f| f.kind == kind).count() > nodes_before, node, document.bodies.iter().any(|b| b.volume > 0.0)) {
        (true, Some(node), true) => Answered::Ran(format!("a {} made of {} sketches ({}), its body in the part", kind, names.len(), node.name)),
        (true, Some(node), _) => Answered::Refused(format!("the {} would not build: {}", kind, node.error.clone().unwrap_or_default())),
        _ => Answered::Refused(format!("nothing was made: {}", core.status())),
    }
}

/// STAND A DATUM - a plane at an offset from a face or a world plane, an axis along an edge, a point at
/// coordinates: the command taken, the base clicked where the datum takes one (a plane from what it
/// stands on, an axis from the edge it lies along), the values the call carried filled - a point's X,
/// Y and Z, a plane's offset - Enter. The datum is in the tree after it and the timeline holds its
/// node; the body the part held is untouched, its volume what it was. A base that names nothing is the
/// program's own refusal, and nothing changed.
fn stand_a_datum(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let (kind, what) = match code {
        "part.datum-axis" => ("DatumAxis", "axis"),
        "part.datum-point" => ("DatumPoint", "point"),
        _ => ("Plane", "plane"),
    };
    let picks = the_triples(given.get("picks")).unwrap_or_default();
    let before = core.document();
    let volume_before = before.bodies.iter().filter(|b| !b.consumed).map(|b| b.volume).sum::<f64>();
    take_the_command(core, the_command_of(entry));
    core.settle(); // the take lands the tool a frame later; the pick clicks what the tool waits for
    for p in &picks {
        let at = core.in_space(*p);
        core.click(at);
    }
    fill_the_fields(core, given);
    core.key(qymcad::Key::Enter);
    let document = core.document();
    let stands = document.datums.iter().filter(|d| d.kind == what).count() > before.datums.iter().filter(|d| d.kind == what).count();
    let node = document.features.iter().filter(|f| f.kind == kind && !f.suppressed).rfind(|f| f.error.is_none());
    let body_stands = document.bodies.iter().filter(|b| !b.consumed).map(|b| b.volume).sum::<f64>();
    match (stands, node, (body_stands - volume_before).abs() < 1e-6) {
        (true, Some(node), true) => Answered::Ran(format!("a {} standing in the tree ({}), the body untouched", what, node.name)),
        (true, _, _) => Answered::Refused(format!("the {} moved the body it stood beside: {} over {} stood", what, body_stands, volume_before)),
        _ => Answered::Refused(format!("nothing was made: {}", core.status())),
    }
}

/// THE KINDS A JOINT COMES IN, as the bar's own list names them: the key of each kind's word, and the
/// name the document reads the joint by. A call's kind is matched against both, as the program's own
/// names match - case, dashes and spaces aside - so "Pin slot", "pin-slot" and "PinSlot" name one kind.
const JOINT_KINDS: &[(&str, &str)] = &[
    ("joint-kind-rigid", "Rigid"),
    ("joint-kind-revolute", "Revolute"),
    ("joint-kind-slider", "Slider"),
    ("joint-kind-cylindrical", "Cylindrical"),
    ("joint-kind-planar", "Planar"),
    ("joint-kind-ball", "Ball"),
    ("joint-kind-pin-slot", "PinSlot"),
    ("joint-kind-parallel", "Parallel"),
];

/// THE WORDS OF A KIND, matched as the program matches its own names: case, dashes and spaces aside.
pub(crate) fn the_same_kind(a: &str, b: &str) -> bool {
    let squeeze = |w: &str| w.to_lowercase().replace(['-', ' ', '_'], "");
    squeeze(a) == squeeze(b)
}

/// LAY A JOINT OF ANY KIND between two parts: the command taken, the kind chosen in the bar's own
/// list (opened and its word pressed, as a person chooses), an anchor clicked on each part - the
/// program infers the anchor from where the click lands, a face's centre here - Enter keeping the
/// joint the second click laid, Esc putting the tool down. The joint stands in the list afterwards,
/// of the kind asked for and holding; the first part becomes the anchor the second is held against.
/// A kind no joint comes in, or picks that name no face of the parts, is the program's own refusal,
/// and nothing changed.
fn lay_a_joint(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(picks) = the_triples(given.get("picks")) else {
        return Answered::Refused(format!("the {} needs its places in the space of the model: an anchor on each of the two parts", entry.name()));
    };
    let [a, b, ..] = picks.as_slice() else {
        return Answered::Refused("the joint needs two places: an anchor on part A, then on part B".into());
    };
    let Some(asked) = the_words_of(given, "kind") else {
        return Answered::Refused(format!("the joint needs the kind it is to hold: one of {}", JOINT_KINDS.iter().map(|(_, name)| *name).collect::<Vec<_>>().join(", ")));
    };
    let Some((key, _)) = JOINT_KINDS.iter().find(|(key, name)| the_same_kind(name, &asked) || the_same_kind(&core.word(key), &asked)) else {
        return Answered::Refused(format!("{asked}: no joint of the program comes in that kind; the kinds are {}", JOINT_KINDS.iter().map(|(_, name)| *name).collect::<Vec<_>>().join(", ")));
    };
    let wanted = core.word(key);
    let before = core.document().joints.len();
    take_the_command(core, the_command_of(entry));
    core.settle(); // the take lands the tool a frame later; the bar stands before its list is opened
    let caption = core.word("j-kind");
    let list = core.field(&caption);
    core.click(list.rect.center());
    core.press_word_near(&wanted, list.rect.center());
    let at = core.face_at(*a);
    core.click(at);
    let at = core.face_at(*b);
    core.click(at);
    core.key(qymcad::Key::Enter);
    core.key(qymcad::Key::Escape);
    let document = core.document();
    match document.joints.get(before) {
        Some(joint) if the_same_kind(&joint.kind, &asked) && !joint.violated => Answered::Ran(format!("a {} joint made ({}), part B held to part A", joint.kind, joint.name)),
        Some(joint) if joint.violated => Answered::Refused(format!("the {} joint does not hold: {}", joint.kind, core.status())),
        _ => Answered::Refused(format!("no joint was laid: {}", core.status())),
    }
}

/// GROUND A PART where it stands: the command taken, the part clicked - one click fixes it, the
/// anchor the other parts are placed against - the tool put down with Esc (what the click fixed
/// stays fixed). The part stands grounded in the tree afterwards; a click that names no part is the
/// program's own refusal, and nothing changed.
fn ground_a_part(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(picks) = the_triples(given.get("picks")) else {
        return Answered::Refused(format!("the {} needs its place in the space of the model: on the part to fix", entry.name()));
    };
    let [on, ..] = picks.as_slice() else {
        return Answered::Refused("the grounding needs one place: on the part to fix".into());
    };
    let before = core.document().parts.iter().filter(|p| p.grounded).count();
    take_the_command(core, the_command_of(entry));
    core.settle(); // the take lands the tool a frame later; the click lands on what it waits for
    let at = core.face_at(*on);
    core.click(at);
    core.key(qymcad::Key::Escape);
    let document = core.document();
    let grounded = document.parts.iter().filter(|p| p.grounded).count();
    if grounded > before {
        Answered::Ran(format!("the part under ({}, {}, {}) fixed where it stands, the anchor of the assembly", on[0], on[1], on[2]))
    } else {
        Answered::Refused(format!("nothing was fixed: {}", core.status()))
    }
}

/// HOLD THE PARTS TOGETHER where they stand - a group: the command taken, every part clicked in its
/// turn, Enter. The group stands among the mates afterwards, and no joint was laid; picks that name
/// no face of the parts are the program's own refusal, and nothing changed.
fn hold_the_parts_together(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(picks) = the_triples(given.get("picks")) else {
        return Answered::Refused(format!("the {} needs its places in the space of the model: on each part to hold, two at least", entry.name()));
    };
    if picks.len() < 2 {
        return Answered::Refused("the group needs the parts it holds, two at least".into());
    }
    the_mates_of_a_clicking_tool(core, the_command_of(entry), &picks, "Group", "the parts held together where they stand (a Group mate)")
}

/// REST A SHAFT ON A FACE - a tangent: the command taken, the plane clicked, then the round side of
/// the shaft, Enter. The shaft comes to rest on the face, the distance between them its radius, and
/// the tangent stands among the mates; picks that name no plane and no cylinder are the program's
/// own refusal, and nothing changed.
fn rest_a_shaft_on_the_face(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(picks) = the_triples(given.get("picks")) else {
        return Answered::Refused(format!("the {} needs its places in the space of the model: the plane, then the round side of the shaft", entry.name()));
    };
    if picks.len() != 2 {
        return Answered::Refused("the tangent needs two places: the plane, then the round side of the shaft".into());
    }
    the_mates_of_a_clicking_tool(core, the_command_of(entry), &picks, "Tangent", "the shaft rests on the face (a Tangent mate)")
}

/// SIT A PART BETWEEN TWO WALLS - a width: the command taken, the two walls clicked, then the part
/// between them, Enter. The part comes to stand midway, and the width stands among the mates; picks
/// that name no walls are the program's own refusal, and nothing changed.
fn sit_the_part_between_the_walls(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(picks) = the_triples(given.get("picks")) else {
        return Answered::Refused(format!("the {} needs its places in the space of the model: the two walls, then the part between them", entry.name()));
    };
    if picks.len() != 3 {
        return Answered::Refused("the width needs three places: the two walls, then the part between them".into());
    }
    the_mates_of_a_clicking_tool(core, the_command_of(entry), &picks, "Width", "the part sits midway between the walls (a Width mate)")
}

/// THE SHARED BODY OF THE MATES THAT CLICK THEIR PARTS: the command taken, every place clicked in
/// its turn, Enter. The mate of `kind` stands among the document's mates afterwards - said in its
/// own words when it came to be, refused in the program's own words when it did not.
fn the_mates_of_a_clicking_tool(core: &mut qymcad::AgentSession, command: &qymcad::Command, picks: &[[f64; 3]], kind: &str, made: &str) -> Answered {
    let before = core.document().mates.iter().filter(|m| *m == kind).count();
    take_the_command(core, command);
    core.settle(); // the take lands the tool a frame later; the clicks land on what it waits for
    for p in picks {
        let at = core.face_at(*p);
        core.click(at);
    }
    core.key(qymcad::Key::Enter);
    let document = core.document();
    if document.mates.iter().filter(|m| *m == kind).count() > before {
        Answered::Ran(made.to_string())
    } else {
        Answered::Refused(format!("nothing was tied: {}", core.status()))
    }
}

/// TIE THE MATES TOGETHER - a relation: the command taken, the rows of the joints it ties clicked in
/// the list of mates, Enter. A gear relation stands among the mates afterwards, keeping the turns of
/// the two in step; names the list does not hold are refused in words that say what the list holds.
fn tie_the_mates_together(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(names) = the_names_of(given.get("joints")) else {
        return Answered::Refused(format!("the {} needs the names of the joints it ties, two at least", entry.name()));
    };
    if names.len() < 2 {
        return Answered::Refused("the relation needs the joints it ties, two at least".into());
    }
    let held: Vec<String> = core.document().joints.iter().map(|j| j.name.clone()).collect();
    for name in &names {
        if !held.contains(name) {
            return Answered::Refused(format!("no joint named {name:?} is in the list of mates; the list holds {held:?}"));
        }
    }
    let before = core.document().mates.iter().filter(|m| *m == "Gear").count();
    take_the_command(core, the_command_of(entry));
    core.settle(); // the take lands the tool a frame later; the rows click into what it waits for
    for name in &names {
        let Some(row) = core.find(name, qymcad::pos2(1100.0, 600.0)) else {
            core.key(qymcad::Key::Escape);
            return Answered::Refused(format!("the joint named {name:?} has no row in the list of mates; the list holds {held:?}"));
        };
        core.click(row.center());
    }
    core.key(qymcad::Key::Enter);
    let document = core.document();
    if document.mates.iter().filter(|m| *m == "Gear").count() > before {
        Answered::Ran(format!("a Gear relation made, tying {} to {}", names[0], names[1]))
    } else {
        Answered::Refused(format!("nothing was tied: {}", core.status()))
    }
}

/// MEASURE WHAT THE PICKS NAME, as the person's own tool does: the tool taken from its bar button
/// where it does not stand (the button is a toggle, and pressing it over a standing tool would put it
/// down - the tool's own bar on screen says whether it stands), every place clicked in turn - each
/// names a vertex, an edge or a face - and the measured value stands in the status line, in the
/// program's own words. Two places measure between them; one says its own length or diameter. Places
/// that name nothing measure nothing, and the call is refused with what the status line then says.
fn measure_between_the_picks(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(picks) = the_triples(given.get("picks")) else {
        return Answered::Refused(format!("the {} needs its places in the space of the model: the two things it measures", entry.name()));
    };
    let bar = core.word("m3-hint-short");
    if core.find(&bar, qymcad::pos2(640.0, 0.0)).is_none() {
        let measure = core.word("tb-measure3d-hint");
        let Some(button) = core.find_hint(&measure) else {
            return Answered::Refused("the measure of the 3D view is reached from the bar of a workbench in 3D - finish the sketch and measure again".into());
        };
        core.click(button);
        core.settle(); // the take lands the tool a frame later; the picks click what it waits for
    }
    for p in &picks {
        let at = core.in_space(*p);
        core.click(at);
    }
    let said = core.status();
    core.key(qymcad::Key::Escape); // the tool put down, as the person's Esc leaves it; the measured words are said
    if said == core.word("m3-miss") || said == core.word("m3-hint") || said.trim().is_empty() {
        return Answered::Refused(format!("nothing was measured: {said}"));
    }
    Answered::Ran(said)
}

/// CUT THE VIEW BY THE PLANE THE PICK NAMES - the section: the section's bar button taken (a toggle -
/// where a section already stands, the button puts it away first, as a person's press would), the
/// plane clicked where the pick lands - a face of the body, a datum, a world plane - Enter applying
/// what the pick laid. Half the model is hidden and its inside shows; the document itself is
/// untouched, and the answer says the view is cut. Where no bar of a workbench stands - the sheet of
/// an open sketch - the call is refused in words.
fn cut_the_view_by_a_plane(core: &mut qymcad::AgentSession, code: &str, given: &serde_json::Value) -> Answered {
    let Some(entry) = by_code(code) else { return Answered::Refused(format!("{code}: no operation of the program is named so")) };
    let Some(picks) = the_triples(given.get("picks")) else {
        return Answered::Refused(format!("the {} needs its place in the space of the model: a place on the plane or face it cuts by", entry.name()));
    };
    let [on, ..] = picks.as_slice() else { return Answered::Refused("the section needs one place: on the plane or face it cuts by".into()) };
    let section = core.word("sec-btn");
    let engaged = core.find(&section, qymcad::pos2(640.0, 0.0)).is_some();
    let in_the_assembly = core.word("tb-section-hint");
    let in_the_part = core.word("tb-section-hint-bar");
    let button = core.find_hint(&in_the_assembly).or_else(|| core.find_hint(&in_the_part));
    let Some(button) = button else {
        return Answered::Refused("the section is reached from the bar of the Part or the Assembly workbench in 3D - finish the sketch and section again".into());
    };
    if engaged {
        core.click(button);
        core.settle();
    }
    core.click(button);
    core.settle(); // the take lands the pick a frame later; the click lands on what it waits for
    let at = core.in_space(*on);
    core.click(at);
    core.key(qymcad::Key::Enter); // applies what the pick laid, as the bar's own Apply does
    core.settle();
    if core.document().section {
        Answered::Ran(format!("the view cut by the plane through ({}, {}, {}); the document is untouched", on[0], on[1], on[2]))
    } else {
        Answered::Refused(format!("no section stands: {}", core.status()))
    }
}

/// THE NAMES A VALUE HOLDS: finished sketches of the tree, joints of the list of mates - each a
/// string; `None` at the first thing that is not one.
fn the_names_of(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    value?.as_array()?.iter().map(|s| Some(s.as_str()?.to_string())).collect()
}

/// THE TRIPLES A VALUE HOLDS: places in the space of the model, each an [x, y, z] triple of numbers;
/// `None` at the first thing that is not one.
fn the_triples(value: Option<&serde_json::Value>) -> Option<Vec<[f64; 3]>> {
    value?.as_array()?.iter().map(|p| Some([p.as_array()?.first()?.as_f64()?, p.as_array()?.get(1)?.as_f64()?, p.as_array()?.get(2)?.as_f64()?])).collect()
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
pub(crate) fn the_words_of(given: &serde_json::Value, name: &str) -> Option<String> {
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
    take_the_command(core, the_command_of(entry));
    let said = core.status();
    if core.in_hand().iter().eq(held.iter()) {
        return Answered::Refused(said);
    }
    Answered::Ran(format!("{} is taken; its bar stands in the window - fill its fields and press Enter to apply, Esc puts it down", entry.name()))
}

/// THE ENTRY OF A CODE, as `call-tool` names an operation.
pub(crate) fn by_code(code: &str) -> Option<&'static Entry> {
    the_registry().iter().find(|e| e.code() == code)
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
            if entry.workbench() != want {
                continue;
            }
        }
        let Some(rank) = the_words_it_is_found_by(entry).iter().filter_map(|w| w.to_lowercase().find(&q)).min() else { continue };
        if entry.workbench() == now_in {
            mine.push((rank, entry));
        } else {
            rest.push((rank, entry));
        }
    }
    let by_rank_then_code = |a: &(usize, &Entry), b: &(usize, &Entry)| (a.0, a.1.code()).cmp(&(b.0, b.1.code()));
    mine.sort_by(by_rank_then_code);
    rest.sort_by(by_rank_then_code);
    mine.into_iter().chain(rest).map(|(_, e)| e).collect()
}

/// THE WORDS AN OPERATION IS FOUND BY: its code, its names in each language, the title of its help
/// article in each of them, and the words it is also known by.
fn the_words_it_is_found_by(entry: &Entry) -> Vec<String> {
    let mut words = vec![entry.code().to_string()];
    words.extend(entry.names().into_values());
    if let Some(article) = entry.help_article() {
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

/// THE COMMAND A RUN TAKES THROUGH THE SEARCH: the runs of the catalogue's commands work through one.
/// An operation of the whole program is taken by no command and works its own gestures; a run that
/// reaches for its command is wired to the wrong table, and the panic says so where it stands.
fn the_command_of(entry: &Entry) -> &'static qymcad::Command {
    entry.the_command().unwrap_or_else(|| panic!("{} is no command of the catalogue to take", entry.code()))
}

/// DOES THE COMMAND OPEN A BAR OF ITS OWN when it is taken: the numbered launches do; an action arms a
/// mode or opens a window and puts no bar up.
fn opens_a_bar(command: &qymcad::Command) -> bool {
    matches!(command.launch, qymcad::Launch::Feat(_) | qymcad::Launch::Prim(_) | qymcad::Launch::SkTool(_) | qymcad::Launch::Dim(_) | qymcad::Launch::ClickOp(_) | qymcad::Launch::Modify(_))
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

/// THE PARAMETERS OF THE OPERATION of `code`, read from the live tool where it is a command that opens
/// a bar: the command is taken as a person takes it, and the fields the take put up - and nothing that
/// stood on screen before it - are its parameters, in the bar's own order. Where the take landed a tool
/// it is put down again; the document is left as it stood. The actions arm a mode or open a window and
/// put no bar up, and taking one to read it would leave the mode armed (a toggle taken twice is a
/// toggle undone) - their parameters, and those of the operations of the whole program, are the run's
/// takes alone. Where the command does not open, nothing to work on, no field comes up and none is
/// answered.
pub(crate) fn parameters(core: &mut qymcad::AgentSession, code: &str) -> Vec<Parameter> {
    let Some(entry) = by_code(code) else { return Vec::new() };
    let mut all = Vec::new();
    if let Some(command) = entry.the_command().filter(|c| opens_a_bar(c)) {
        let before = the_fields_on_screen(core);
        let held = core.in_hand();
        take_the_command(core, command);
        let mut put_up = the_fields_on_screen(core);
        put_up.retain(|f| !before.iter().any(|b| b.name == f.name && b.kind == f.kind));
        put_down_what_the_take_landed(core, &held);
        all = put_up;
    }
    all.extend(entry.takes.iter().map(|t| Parameter {
        name: t.name.to_string(),
        kind: t.kind.to_string(),
        says: match t.kind {
            "points" => "places on the sheet of the open sketch, each an [x, y] pair".to_string(),
            "picks" => "places in the space of the model, each an [x, y, z] triple naming what stands there".to_string(),
            "sketches" => "the names of finished sketches of the tree, in the order the tool takes them".to_string(),
            "joints" => "the names of the joints of the list of mates, in the order the tool ties them".to_string(),
            _ => String::new(),
        },
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

    /// THE REGISTRY IS THE CATALOGUE AND THE HELP, derived whole: every command of the catalogue is an
    /// entry, each entry carries its command's code and workbench, its names in each language the
    /// program speaks; beside them stand the operations of the whole program, each named by its article
    /// in each language - and nothing else stands in the registry.
    #[test]
    fn the_registry_is_derived_from_the_catalogue_whole() {
        for command in qymcad::COMMANDS {
            let Some(entry) = by_code(command.code) else { panic!("{} is in the catalogue and not in the registry", command.code) };
            assert_eq!(entry.code(), command.code, "the entry of {} answers another code", command.code);
            assert_eq!(entry.workbench(), command.workbench, "{} carries the wrong workbench", command.code);
            for (language, name) in entry.names() {
                assert!(!name.is_empty(), "{} is nameless in {language}", command.code);
            }
        }
        for entry in the_registry().iter().filter(|e| e.the_command().is_none()) {
            for (language, name) in entry.names() {
                assert!(!name.is_empty(), "{} is nameless in {language}: its article names it not", entry.code());
            }
        }
        assert_eq!(
            the_registry().len(),
            qymcad::COMMANDS.len() + crate::general::THE_OPERATIONS_OF_THE_PROGRAM.len(),
            "the registry holds entries neither the catalogue nor the table of the program's own operations carries"
        );
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
        for entry in the_registry().iter().filter(|e| e.the_command().is_some_and(|c| matches!(c.launch, qymcad::Launch::Feat(_) | qymcad::Launch::Prim(_)))) {
            let code = entry.code();
            let own = crate::checks::ask_the_core(&mut program, move |core| {
                let before = the_screen(core);
                let held = core.in_hand();
                take_the_command(core, the_command_of(by_code(code).expect("the code names an entry")));
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
            // the run's own takes (its picks, its places) stand beside the bar's fields, not on it
            let take_names: Vec<String> = by_code(code).expect("the code names an entry").takes.iter().map(|t| t.name.to_string()).collect();
            let as_pairs: Vec<(String, String)> = answered.iter().filter(|p| !take_names.contains(&p.name)).map(|p| (p.name.clone(), p.says.clone())).collect();
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
        let finds = |needle: &str, code: &str| the_hits(needle, None, "part").iter().any(|e| e.code() == code);
        let line = by_code("sketch.line").expect("the line is in the registry");
        assert!(finds("sketch.line", "sketch.line"), "a command's own code found it not");
        for (language, name) in line.names() {
            assert!(finds(&name, "sketch.line"), "the {language} name {name:?} found the line not");
        }
        let the_box = by_code("part.box").expect("the box is in the registry");
        let article = the_box.help_article().expect("the box points at an article");
        for language in qymcad_help::languages() {
            let Some(title) = qymcad_help::title_in(&language, article) else { continue };
            assert!(finds(&title, "part.box"), "the article title {title:?} of {language} found the box not");
        }
        assert!(finds("round", "part.fillet"), "the word the fillet is also known by found it not");
        assert!(the_hits("fillet", Some("sketch"), "part").iter().all(|e| e.workbench() == "sketch"), "the workbench filter let another workbench through");
        let hits = the_hits("line", None, "part");
        assert_eq!(
            hits.first().map(|e| e.workbench()),
            Some("part"),
            "a command of the standing workbench did not come first: {:?}",
            the_hits("line", None, "part").iter().map(|e| e.code()).collect::<Vec<_>>()
        );
        assert!(hits.iter().any(|e| e.code() == "sketch.line"), "the line was not found at all");
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
            let mut needles = vec![entry.code().to_string()];
            needles.extend(entry.names().into_values());
            needles.extend(entry.synonyms.iter().map(|s| s.to_string()));
            for needle in needles {
                assert!(the_hits(&needle, None, "part").iter().any(|e| e.code() == entry.code()), "{} is not found by {needle:?}", entry.code());
            }
            assert!(entry.run.is_some(), "{} is wired to no run", entry.code());
        }
        let _turn = crate::checks::take_your_turn();
        // ON A BARE START every run answers in words - the take lands the commands that need nothing,
        // and a run that needs what is not there (a drawing with no sketch open) refuses in the
        // program's own words. The part features and the assembly mates have left this phase as their
        // runs grew their own takes - their picks and sketches - so the floor stands at what is left:
        // the primitives, the contour features, the actions and the queries
        let mut program = qymcad::LiveLoop::start();
        let mut landed = 0;
        for entry in the_registry().iter().filter(|e| e.takes.is_empty()) {
            let code = entry.code();
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
            assert!(!said.is_empty(), "{} said nothing when it ran", entry.code());
        }
        assert!(landed >= 7, "only {landed} of the take-only commands landed their tool on a bare start; the runs have gone mute");
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
        // the open sketch of this one has none; the part features are held for the block phase - their
        // picks are places in the space of a body)
        let mut square = 0;
        for entry in the_registry().iter().filter(|e| !e.takes.is_empty() && e.workbench() == "sketch" && e.code() != "sketch.project") {
            let code = entry.code();
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
            assert!(matches!(answered, Answered::Ran(_)), "{} worked nothing over the open sketch: {answered:?}", entry.code());
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
        // OVER ITS OWN FIXTURE each part feature that works by its picks runs: a fresh program for each
        // (a feature changes the body it worked on), the picks naming an edge, a face, a shaft or a mesh
        // of what the fixture built - the registry's own calls drawing and building what the feature
        // needs under it - the values carried as the call carries them.
        /// WHAT A ROW OF THE GUARD'S PART PHASE STANDS ON: the body its picks are to name.
        #[derive(Clone, Copy)]
        enum Over {
            /// The block 40 x 30 x 10.
            Block,
            /// The cylinder 20 across and 10 tall - a thread's shaft.
            Cylinder,
            /// The block with a hole 20 across through it - a remove-face's wall, a patch's rim.
            Holed,
            /// The block with two sheets taken from its faces - a stitch's pair, a replace's surface.
            TwoSheets,
            /// The block with a sheet on its top and a boss grown through it - a trim's cut.
            SheetAndBoss,
            /// The block 40 x 20 x 10 standing off the origin, so a circular pattern's copies fall clear.
            OffAxis,
            /// A mesh of a cube 10 on a side, imported from a file - a recognise's pick.
            Mesh,
        }
        /// THE FIXTURE OF A ROW, built through the registry's own calls: a first part, the body it
        /// names, and what stands on it.
        fn the_fixture_of(core: &mut qymcad::AgentSession, over: Over) {
            if let Over::Mesh = over {
                core.key(qymcad::Key::Escape); // the start screen away, the toolbar's own new-part button in - the part entered, where the mesh lands pickable
                let new_part = core.word("tb-new-part-hint");
                core.press_hint(&new_part);
                let path = std::env::temp_dir().join(format!("qymcad-registry-mesh-{}.stl", std::process::id()));
                std::fs::write(&path, a_cube_stl(10.0)).expect("the mesh file was written");
                core.import(&path.to_string_lossy());
                core.settle();
                core.key(qymcad::Key::Enter); // the window of scale takes the mesh in as it stands
                core.settle();
                return;
            }
            if let Over::Cylinder = over {
                core.key(qymcad::Key::Escape); // the start screen away, the toolbar's own new-part button in
                let new_part = core.word("tb-new-part-hint");
                core.press_hint(&new_part);
                call(core, "part.cylinder", &serde_json::json!({ "Radius": "10", "Height": "10" }));
                return;
            }
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            let rect = core.word("tb-rect-hint");
            core.press_hint(&rect);
            let (from, to) = if let Over::OffAxis = over { ((200.0, 0.0), (240.0, 20.0)) } else { ((0.0, 0.0), (40.0, 30.0)) };
            core.click_on_sketch(from.0, from.1).click_on_sketch(to.0, to.1);
            let finish = core.word("wb-finish");
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
            call(core, "part.extrude", &serde_json::json!({ "Length": "10" }));
            match over {
                Over::Holed => {
                    call(core, "part.hole", &serde_json::json!({ "picks": [[20.0, 15.0, 10.0]], "Diameter": "20", "Depth": "12" }));
                }
                Over::TwoSheets => {
                    call(core, "part.face-copy", &serde_json::json!({ "picks": [[20.0, 15.0, 10.0]] }));
                    call(core, "part.face-copy", &serde_json::json!({ "picks": [[20.0, 0.0, 5.0]] }));
                }
                Over::SheetAndBoss => {
                    call(core, "part.face-copy", &serde_json::json!({ "picks": [[20.0, 15.0, 10.0]] }));
                    let xy = core.word("plane-xy-table");
                    core.press_word(&xy);
                    call(core, "sketch.circle", &serde_json::json!({ "points": [[20.0, 15.0], [25.0, 15.0]] }));
                    core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
                    call(core, "part.extrude", &serde_json::json!({ "Length": "20" }));
                }
                _ => {}
            }
        }
        for (code, over, picks, fields) in [
            ("part.fillet", Over::Block, vec![[20.0, 0.0, 10.0]], vec![("radius", "3")]),
            ("part.chamfer", Over::Block, vec![[20.0, 0.0, 10.0]], vec![("leg", "2")]),
            ("part.shell", Over::Block, vec![[20.0, 15.0, 10.0]], vec![("thickness", "2")]),
            ("part.hole", Over::Block, vec![[20.0, 15.0, 10.0]], vec![("diameter", "10"), ("depth", "5")]),
            ("part.push-face", Over::Block, vec![[20.0, 15.0, 10.0]], vec![("offset", "2")]),
            ("part.split-body", Over::Block, vec![[20.0, 15.0, 10.0]], vec![("offset", "-3")]),
            ("part.split-face", Over::Block, vec![[20.0, 15.0, 10.0]], vec![("offset", "-3")]),
            ("part.thicken", Over::Block, vec![[20.0, 15.0, 10.0]], vec![("thickness", "2")]),
            ("part.draft", Over::Block, vec![[20.0, 0.0, 5.0], [20.0, 15.0, 10.0]], vec![("angle", "3")]),
            ("part.thread", Over::Cylinder, vec![[7.071, -7.071, 5.0]], vec![("nominal", "20"), ("length", "10")]),
            ("part.remove-face", Over::Holed, vec![[20.0, 25.0, 5.0]], Vec::<(&str, &str)>::new()),
            ("part.mirror", Over::Block, vec![[20.0, 15.0, 10.0], [40.0, 15.0, 5.0]], Vec::<(&str, &str)>::new()),
            ("part.array-linear", Over::Block, vec![[20.0, 15.0, 10.0]], vec![("pitch", "25"), ("copies", "3")]),
            ("part.array-circular", Over::OffAxis, vec![[220.0, 0.0, 5.0]], vec![("copies", "6")]),
            ("part.face-copy", Over::Block, vec![[20.0, 15.0, 10.0]], Vec::<(&str, &str)>::new()),
            ("part.offset-surface", Over::Block, vec![[30.0, 10.0, 10.0]], vec![("distance", "5")]),
            ("part.surface-replace", Over::TwoSheets, vec![[20.0, 15.0, 10.0], [20.0, 15.0, 10.0]], Vec::<(&str, &str)>::new()),
            ("part.patch", Over::Holed, vec![[30.0, 15.0, 10.0]], Vec::<(&str, &str)>::new()),
            ("part.stitch", Over::TwoSheets, vec![[20.0, 15.0, 10.0], [20.0, 0.0, 5.0]], vec![("tol", "0.01")]),
            ("part.trim", Over::SheetAndBoss, vec![[5.0, 5.0, 10.0], [20.0, 15.0, 20.0]], Vec::<(&str, &str)>::new()),
            ("part.recognise", Over::Mesh, vec![[10.0, 3.0, 5.0]], Vec::<(&str, &str)>::new()),
            ("part.datum-plane", Over::Block, vec![[20.0, 15.0, 10.0]], vec![("offset", "10")]),
            ("part.datum-axis", Over::Block, vec![[20.0, 0.0, 10.0]], Vec::<(&str, &str)>::new()),
            ("part.datum-point", Over::Block, Vec::<[f64; 3]>::new(), vec![("X", "10"), ("Y", "5"), ("Z", "3")]),
        ] {
            let mut program = qymcad::LiveLoop::start();
            let answered = crate::checks::ask_the_core(&mut program, move |core| {
                the_fixture_of(core, over);
                let mut given = serde_json::Map::new();
                if !picks.is_empty() {
                    given.insert("picks".into(), serde_json::json!(picks));
                }
                for (needle, value) in &fields {
                    if let Some(found) = parameters(core, code).into_iter().find(|p| p.name.to_lowercase().contains(&needle.to_lowercase())) {
                        given.insert(found.name.clone(), serde_json::json!(value));
                    }
                }
                call(core, code, &serde_json::Value::Object(given))
            })
            .expect("the call answered");
            assert!(matches!(answered, TheCall::Answered { .. }), "{code} worked nothing over its fixture: {answered:?}");
        }
        // OVER TWO SKETCHES the sweep and the loft run: the profile and the path drawn on their own world
        // planes, the sections of the loft on the table and on a datum the registry's own call stands 30
        // above it - the pencil of the panel clicking that datum open, the face-picking word the person's
        // own hand presses - the sketches named to the call in their order.
        for (code, second_on_a_datum) in [("part.sweep", false), ("part.loft", true)] {
            let mut program = qymcad::LiveLoop::start();
            let answered = crate::checks::ask_the_core(&mut program, move |core| {
                let new_part = core.word("start-new-part");
                core.press_word(&new_part);
                let xy = core.word("plane-xy-table");
                core.press_word(&xy);
                let rect = core.word("tb-rect-hint");
                core.press_hint(&rect);
                core.click_on_sketch(0.0, 0.0).click_on_sketch(10.0, 10.0);
                let finish = core.word("wb-finish");
                core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
                if second_on_a_datum {
                    call(core, "part.datum-plane", &serde_json::json!({ "picks": [[0.0, 0.0, 0.0]], "Offset": "30" }));
                    let pencil = core.word("g-sketch-pick-hint");
                    core.press_hint(&pencil);
                    let on_datum = core.in_space([0.0, 0.0, 30.0]);
                    core.click(on_datum);
                } else {
                    core.key(qymcad::Key::Escape); // the sketch's selection down, the plane buttons of the panel up
                    let xz = core.word("plane-xz-front");
                    core.press_word(&xz);
                }
                if second_on_a_datum {
                    let rect = core.word("tb-rect-hint");
                    core.press_hint(&rect);
                    core.click_on_sketch(-5.0, -5.0).click_on_sketch(15.0, 15.0);
                } else {
                    let line = core.word("tb-line-hint");
                    core.press_hint(&line);
                    core.click_on_sketch(0.0, 0.0).click_on_sketch(0.0, 40.0);
                    core.key(qymcad::Key::Escape);
                }
                core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
                let names: Vec<String> = core.document().sketches.iter().map(|s| s.name.clone()).collect();
                call(core, code, &serde_json::json!({ "sketches": names }))
            })
            .expect("the call answered");
            assert!(matches!(answered, TheCall::Answered { .. }), "{code} worked nothing over its sketches: {answered:?}");
        }
        // OVER THE ASSEMBLY the mates run: fixtures of parts built through the registry's own calls
        // (a part made, a rectangle drawn and grown, the part finished - a second beside it, a shaft,
        // walls with a tab, hinges laid as a person lays them), each mate's own picks or joint names
        // carried as the call carries them.
        for code in ["assembly.joint", "assembly.ground", "assembly.group", "assembly.tangent", "assembly.width", "assembly.relation"] {
            let mut program = qymcad::LiveLoop::start();
            let answered = crate::checks::ask_the_core(&mut program, move |core| {
                the_fixture_of_the_mates(core, code);
                let given = the_sample_of_the_mates(core, code);
                call(core, code, &given)
            })
            .expect("the call answered");
            assert!(matches!(answered, TheCall::Answered { .. }), "{code} tied nothing over its fixture: {answered:?}");
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

    /// THE CONTOUR FEATURES, run as calls over a finished rectangle: the extrude lifts it to the height
    /// the call carries (a block of exactly its area times its height), the revolve turns it about the
    /// axis its bar comes with (a cylinder of exactly its swept volume). Each is weighed by the volume
    /// of the body that lands; over a part with no contour to work, each is refused in the program's
    /// own words and the part holds what it held.
    #[test]
    fn the_contour_features_make_their_bodies() {
        let _turn = crate::checks::take_your_turn();
        /// A FINISHED 40 X 30 RECTANGLE ON XY of a first part: the sketch closed, the part standing in 3D.
        fn a_finished_rectangle() -> qymcad::LiveLoop {
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
            program
        }
        // THE EXTRUDE: the rectangle lifted 10 - a block of 40 x 30 x 10
        let mut program = a_finished_rectangle();
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "part.extrude", &serde_json::json!({ "Length": "10" }))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the extrude did not run: {answered:?}");
        let volume = crate::checks::ask_the_core(&mut program, |core| core.document().bodies.iter().map(|b| b.volume).fold(0.0, f64::max)).expect("the body was read");
        assert!((volume - 12000.0).abs() < 12000.0 * 1e-3, "the extruded rectangle of 40 x 30 lifted 10 holds {volume}, not 12000");
        // THE REVOLVE: the rectangle turned about the axis its bar comes with - a cylinder of the swept
        // volume
        let mut program = a_finished_rectangle();
        let answered = crate::checks::ask_the_core(&mut program, |core| call(core, "part.revolve", &serde_json::json!({}))).expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the revolve did not run: {answered:?}");
        let volume = crate::checks::ask_the_core(&mut program, |core| core.document().bodies.iter().map(|b| b.volume).fold(0.0, f64::max)).expect("the body was read");
        let want = std::f64::consts::PI * 30.0 * 30.0 * 40.0;
        assert!((volume - want).abs() < want * 1e-3, "the revolved rectangle holds {volume}, not the {want} of a cylinder of radius 30 and length 40");
        // OVER A PART WITH NO CONTOUR each is refused in the program's words, the part as it stood
        let mut bare = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut bare, |core| {
            core.key(qymcad::Key::Escape);
            let new_part = core.word("tb-new-part-hint");
            core.press_hint(&new_part);
        })
        .expect("the fixture was built");
        let stood = crate::checks::ask_the_core(&mut bare, |core| core.document()).expect("the document was read");
        for code in ["part.extrude", "part.revolve"] {
            let refused = crate::checks::ask_the_core(&mut bare, |core| call(core, code, &serde_json::json!({}))).expect("the call answered");
            let TheCall::Refused(words) = &refused else { panic!("{code} over no contour was not refused: {refused:?}") };
            assert!(!words.trim().is_empty(), "{code} over no contour said nothing");
        }
        let after = crate::checks::ask_the_core(&mut bare, |core| core.document()).expect("the document was read");
        assert_eq!(after, stood, "a refused contour feature changed the document");
    }

    /// THE BODY FEATURES, run as calls over a block of 40 x 30 x 10: an edge chamfered back by its leg
    /// (the wedge of it gone from the volume), a hole bored to its diameter and depth, the inside
    /// hollowed to a thickness open at the face that was clicked, an edge rounded. Each is weighed by
    /// the volume the body holds after it, in the numbers the acceptance checks weigh the same tools
    /// by; over a part with no body to work on, each is refused in the program's own words and the
    /// part holds what it held.
    #[test]
    fn the_body_features_work_their_contracts() {
        let _turn = crate::checks::take_your_turn();
        /// A BLOCK OF 40 X 30 X 10 in the first part: the rectangle drawn and finished, extruded by
        /// the registry's own call.
        fn a_block() -> qymcad::LiveLoop {
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
                call(core, "part.extrude", &serde_json::json!({ "Length": "10" }));
            })
            .expect("the fixture was built");
            program
        }
        /// THE VOLUME OF THE BODY the tools last worked on: the last one the part holds.
        fn the_volume(program: &mut qymcad::LiveLoop) -> f64 {
            crate::checks::ask_the_core(program, |core| core.document().bodies.last().map(|b| b.volume).unwrap_or(0.0)).expect("the body was read")
        }
        /// ONE FEATURE CALL over a fresh block, its fields read from the live tool and set to `values`
        /// by the words their captions carry (a radius, a leg, a diameter, a depth, a thickness).
        fn work(program: &mut qymcad::LiveLoop, code: &str, picks: &[[f64; 3]], values: &[(&str, &str)]) -> TheCall {
            let code = code.to_string();
            let picks = picks.to_vec();
            let values = values.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>();
            crate::checks::ask_the_core(program, move |core| {
                let mut given = serde_json::Map::new();
                given.insert("picks".into(), serde_json::json!(picks));
                for (needle, value) in &values {
                    let found = parameters(core, &code).into_iter().find(|p| p.name.to_lowercase().contains(needle)).unwrap_or_else(|| panic!("{code} puts up no field carrying {needle:?}"));
                    given.insert(found.name.clone(), serde_json::json!(value));
                }
                call(core, &code, &serde_json::Value::Object(given))
            })
            .expect("the call answered")
        }
        // A CHAMFER of 2 along the front-top edge of 40: the wedge of 80 gone
        let mut program = a_block();
        let answered = work(&mut program, "part.chamfer", &[[20.0, 0.0, 10.0]], &[("leg", "2")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the chamfer did not run: {answered:?}");
        assert!((the_volume(&mut program) - (12000.0 - 80.0)).abs() < 1.0, "the chamfered block does not hold 11920");
        // A HOLE of 10 across and 5 deep in the middle of the top face
        let mut program = a_block();
        let answered = work(&mut program, "part.hole", &[[20.0, 15.0, 10.0]], &[("diameter", "10"), ("depth", "5")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the hole did not run: {answered:?}");
        let want = 12000.0 - std::f64::consts::PI * 25.0 * 5.0;
        assert!((the_volume(&mut program) - want).abs() < 2.0, "the bored block does not hold {want}");
        // A SHELL of 2, open at the top face: the cavity of 36 by 26 by 8 gone
        let mut program = a_block();
        let answered = work(&mut program, "part.shell", &[[20.0, 15.0, 10.0]], &[("thickness", "2")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the shell did not run: {answered:?}");
        let want = 12000.0 - 36.0 * 26.0 * 8.0;
        assert!((the_volume(&mut program) - want).abs() < 1.0, "the hollowed block does not hold {want}");
        // A FILLET of 3 on the front-top edge: material gone, the edge gone
        let mut program = a_block();
        let answered = work(&mut program, "part.fillet", &[[20.0, 0.0, 10.0]], &[("radius", "3")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the fillet did not run: {answered:?}");
        let volume = the_volume(&mut program);
        assert!((12000.0 - 200.0..12000.0).contains(&volume), "the rounded block holds {volume}, no wedge of a fillet of 3 came off");
        // OVER A PART WITH NO BODY each is refused in the program's words, the part as it stood
        let mut bare = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut bare, |core| {
            core.key(qymcad::Key::Escape);
            let new_part = core.word("tb-new-part-hint");
            core.press_hint(&new_part);
        })
        .expect("the fixture was built");
        let stood = crate::checks::ask_the_core(&mut bare, |core| core.document()).expect("the document was read");
        for code in ["part.fillet", "part.chamfer", "part.shell", "part.hole"] {
            let refused = crate::checks::ask_the_core(&mut bare, |core| call(core, code, &serde_json::json!({ "picks": [[0.0, 0.0, 0.0]] }))).expect("the call answered");
            let TheCall::Refused(words) = &refused else { panic!("{code} over no body was not refused: {refused:?}") };
            assert!(!words.trim().is_empty(), "{code} over no body said nothing");
        }
        let after = crate::checks::ask_the_core(&mut bare, |core| core.document()).expect("the document was read");
        assert_eq!(after, stood, "a refused body feature changed the document");
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

    /// A CUBE OF `side` AS AN ASCII STL, the way a program without units writes it: twelve triangles over
    /// the eight corners, the mesh a recognise is checked over.
    fn a_cube_stl(side: f64) -> String {
        let c = |i: usize| [(i & 1) as f64 * side, ((i >> 1) & 1) as f64 * side, ((i >> 2) & 1) as f64 * side];
        let tris = [[0, 2, 1], [1, 2, 3], [4, 5, 6], [5, 7, 6], [0, 1, 4], [1, 5, 4], [2, 6, 3], [3, 6, 7], [0, 4, 2], [2, 4, 6], [1, 3, 5], [3, 7, 5]];
        let mut out = String::from("solid cube\n");
        for t in tris {
            out.push_str("facet normal 0 0 0\nouter loop\n");
            for i in t {
                let p = c(i);
                out.push_str(&format!("vertex {} {} {}\n", p[0], p[1], p[2]));
            }
            out.push_str("endloop\nendfacet\n");
        }
        out + "endsolid cube\n"
    }

    /// THE BLOCK 40 x 30 x 10, alone in a first part of a program of its own - the body the part checks
    /// below work on, built through the registry's own calls.
    fn a_block() -> qymcad::LiveLoop {
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
            call(core, "part.extrude", &serde_json::json!({ "Length": "10" }));
        })
        .expect("the block was built");
        program
    }

    /// ONE CALL OVER THE BODY, its picks and its fields carried as the call carries them: the fields are
    /// named by what the live tool's bar puts up, matched by the words the call spoke.
    fn worked(program: &mut qymcad::LiveLoop, code: &str, picks: &[[f64; 3]], fields: &[(&str, &str)]) -> TheCall {
        let code = code.to_string();
        let picks = picks.to_vec();
        let fields = fields.iter().map(|(needle, value)| (needle.to_string(), value.to_string())).collect::<Vec<_>>();
        crate::checks::ask_the_core(program, move |core| {
            let mut given = serde_json::Map::new();
            if !picks.is_empty() {
                given.insert("picks".into(), serde_json::json!(picks));
            }
            for (needle, value) in &fields {
                if let Some(found) = parameters(core, &code).into_iter().find(|p| p.name.to_lowercase().contains(&needle.to_lowercase())) {
                    given.insert(found.name.clone(), serde_json::json!(value));
                }
            }
            call(core, &code, &serde_json::Value::Object(given))
        })
        .expect("the call answered")
    }

    /// THE BLOCK IN THE FIRST PART, the part finished: the assembly open around it, nothing grounded.
    /// Everything the mates are laid over is built from this, through the registry's own calls.
    fn the_block_in_the_assembly(core: &mut qymcad::AgentSession) {
        the_first_part_of(core, |core| {
            call(core, "sketch.rect", &serde_json::json!({ "points": [[0.0, 0.0], [40.0, 30.0]] }));
        });
    }

    /// THE FIRST PART FROM THE START SCREEN, its drawing drawn by the call the `draw` closure makes
    /// and grown 10 by the extrude call, the part finished into the assembly around it.
    fn the_first_part_of(core: &mut qymcad::AgentSession, draw: impl FnOnce(&mut qymcad::AgentSession)) {
        let new_part = core.word("start-new-part");
        core.press_word(&new_part);
        let xy = core.word("plane-xy-table");
        core.press_word(&xy);
        draw(core);
        let finish = core.word("wb-finish");
        core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
        call(core, "part.extrude", &serde_json::json!({ "Length": "10" }));
        core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
    }

    /// A PART MADE IN THE ASSEMBLY, the toolbar's own button (the start screen is gone by then), its
    /// drawing drawn by the call the `draw` closure makes and grown by the extrude call, the part
    /// finished back into the assembly.
    fn a_part_made_in_the_assembly(core: &mut qymcad::AgentSession, draw: impl FnOnce(&mut qymcad::AgentSession)) {
        let new_part = core.word("tb-new-part-hint");
        core.press_hint(&new_part);
        let xy = core.word("plane-xy-table");
        core.press_word(&xy);
        draw(core);
        let finish = core.word("wb-finish");
        core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
        call(core, "part.extrude", &serde_json::json!({ "Length": "10" }));
        core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
    }

    /// THE LAST PART PLACED at `x` along X, by the numbers of its properties, as a person places a
    /// part: its row clicked in the tree, X filled, Enter.
    fn the_last_part_placed_at(core: &mut qymcad::AgentSession, x: f64) {
        let name = core.document().parts.last().map(|p| p.name.clone()).expect("a part stands to be placed");
        let row = core.find(&name, qymcad::pos2(0.0, 300.0)).unwrap_or_else(|| panic!("the part {name:?} is not in the tree"));
        core.click(row.center());
        core.fill("X", &x.to_string());
        core.key(qymcad::Key::Enter);
    }

    /// A REVOLUTE HINGE LAID AS BUILT between two corners of the parts that stand: the joint list's
    /// start button, the kind chosen in the bar, "keep as it stands" ticked (a hinge laid the other
    /// way pulls the parts onto one another), the two corners clicked, Enter, the tool put down.
    fn a_revolute_hinge_as_built(core: &mut qymcad::AgentSession, a: [f64; 3], b: [f64; 3]) {
        let start = core.word("jp-start-joint");
        core.press_word_near(&start, qymcad::pos2(1100.0, 400.0));
        let caption = core.word("j-kind");
        let list = core.field(&caption);
        core.click(list.rect.center());
        let want = core.word("joint-kind-revolute");
        core.press_word_near(&want, list.rect.center());
        let as_built = core.word("j-as-built");
        core.press_word_near(&as_built, qymcad::pos2(640.0, 24.0));
        let at = core.vertex_at(a);
        core.click(at);
        let at = core.vertex_at(b);
        core.click(at);
        core.key(qymcad::Key::Enter);
        core.key(qymcad::Key::Escape);
    }

    /// THE FIXTURE A MATE STANDS ON, what the mate of `code` needs: one part with a block (the
    /// ground), a second part beside it or a shaft (the joint, the group, the tangent), walls with a
    /// tab between them (the width), or three blocks with two hinges (the relation).
    fn the_fixture_of_the_mates(core: &mut qymcad::AgentSession, code: &str) {
        let block = |core: &mut qymcad::AgentSession| {
            call(core, "sketch.rect", &serde_json::json!({ "points": [[0.0, 0.0], [40.0, 30.0]] }));
        };
        match code {
            "assembly.ground" => the_block_in_the_assembly(core),
            "assembly.tangent" => {
                the_block_in_the_assembly(core);
                a_part_made_in_the_assembly(core, |core| {
                    call(core, "sketch.circle", &serde_json::json!({ "points": [[70.0, 15.0], [75.0, 15.0]] }));
                });
            }
            "assembly.width" => {
                let rect = |from: (f64, f64), to: (f64, f64)| {
                    move |core: &mut qymcad::AgentSession| {
                        call(core, "sketch.rect", &serde_json::json!({ "points": [[from.0, from.1], [to.0, to.1]] }));
                    }
                };
                the_first_part_of(core, rect((0.0, 0.0), (10.0, 30.0))); // the first wall
                a_part_made_in_the_assembly(core, rect((50.0, 0.0), (60.0, 30.0))); // the second wall
                a_part_made_in_the_assembly(core, rect((20.0, 10.0), (30.0, 20.0)));
                // the tab between them
            }
            "assembly.relation" => {
                the_block_in_the_assembly(core);
                a_part_made_in_the_assembly(core, block);
                the_last_part_placed_at(core, 60.0);
                a_part_made_in_the_assembly(core, block);
                the_last_part_placed_at(core, 120.0);
                a_revolute_hinge_as_built(core, [40.0, 0.0, 10.0], [100.0, 0.0, 10.0]);
                a_revolute_hinge_as_built(core, [40.0, 0.0, 0.0], [160.0, 0.0, 10.0]);
            }
            _ => {
                the_block_in_the_assembly(core);
                a_part_made_in_the_assembly(core, block);
                the_last_part_placed_at(core, 60.0);
            }
        }
    }

    /// WHAT THE CALL OF A MATE CARRIES over its fixture: the picks each mate needs, the joint's own
    /// kind, the relation's two joints read off the list that stands.
    fn the_sample_of_the_mates(core: &mut qymcad::AgentSession, code: &str) -> serde_json::Value {
        match code {
            "assembly.joint" => serde_json::json!({ "kind": "Rigid", "picks": [[20.0, 15.0, 10.0], [80.0, 15.0, 10.0]] }),
            "assembly.ground" => serde_json::json!({ "picks": [[20.0, 15.0, 10.0]] }),
            "assembly.group" => serde_json::json!({ "picks": [[20.0, 15.0, 10.0], [80.0, 15.0, 10.0]] }),
            "assembly.tangent" => serde_json::json!({ "picks": [[20.0, 0.0, 5.0], [70.0, 10.0, 5.0]] }),
            "assembly.width" => serde_json::json!({ "picks": [[10.0, 15.0, 5.0], [60.0, 15.0, 5.0], [25.0, 15.0, 10.0]] }),
            "assembly.relation" => {
                let names: Vec<String> = core.document().joints.iter().map(|j| j.name.clone()).collect();
                serde_json::json!({ "joints": names })
            }
            _ => unreachable!("the mates have no other code"),
        }
    }

    /// THE WORKING BODY of a program, as the person sees it: the last solid that is its own - sheets and
    /// the pieces other features have used up do not answer for the body.
    fn the_working_body(program: &mut qymcad::LiveLoop) -> qymcad::Solid {
        crate::checks::ask_the_core(program, |core| {
            let mut bodies = core.document().bodies;
            let body = bodies.drain(..).rfind(|b| !b.consumed && !b.sheet).expect("a body stands");
            body
        })
        .expect("the body was read")
    }

    /// THE SHEETS of a program, the surfaces standing beside its body.
    fn the_sheets(program: &mut qymcad::LiveLoop) -> Vec<qymcad::Solid> {
        crate::checks::ask_the_core(program, |core| core.document().bodies.into_iter().filter(|b| !b.consumed && b.sheet).collect()).expect("the sheets were read")
    }

    /// THE DRAFT, THE THREAD AND THE REMOVED FACE, run as calls and weighed: a draft of 3 degrees about
    /// the top face leans the front face over and the body gains the wedge the lean makes; a thread of
    /// the standard on a shaft 20 across takes its groove off and leaves the node green; removing the
    /// wall of a hole gives the plain block back. Each is weighed in the acceptance suite's own numbers.
    #[test]
    fn the_drafts_threads_and_removed_faces_work_their_contracts() {
        let _turn = crate::checks::take_your_turn();
        // A DRAFT OF 3 DEGREES: 12000 + 200 * 10 * tan(3 deg) = 12104.82
        let mut program = a_block();
        let answered = worked(&mut program, "part.draft", &[[20.0, 0.0, 5.0], [20.0, 15.0, 10.0]], &[("angle", "3")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the draft did not run: {answered:?}");
        let now = the_working_body(&mut program);
        assert!((now.volume - 12104.8155585661).abs() < 0.5, "a draft of 3 degrees leaves the body at {}, and 12104.82 was owed", now.volume);
        // A THREAD OF THE STANDARD on a shaft 20 across and 10 tall
        let mut program = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut program, |core| {
            core.key(qymcad::Key::Escape); // the start screen away, the toolbar's own new-part button in
            let new_part = core.word("tb-new-part-hint");
            core.press_hint(&new_part);
            call(core, "part.cylinder", &serde_json::json!({ "Radius": "10", "Height": "10" }));
        })
        .expect("the shaft was built");
        let before = the_working_body(&mut program).volume;
        let answered = worked(&mut program, "part.thread", &[[7.071, -7.071, 5.0]], &[("nominal", "20"), ("length", "10")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the thread did not run: {answered:?}");
        let (now, node) = crate::checks::ask_the_core(&mut program, |core| {
            let document = core.document();
            (document.bodies.into_iter().rfind(|b| !b.consumed && !b.sheet).expect("the shaft stands"), document.features.into_iter().rfind(|f| f.kind == "Thread").expect("the thread node"))
        })
        .expect("the shaft was read");
        assert!(node.error.is_none(), "the thread node is red: {node:?}");
        assert!(now.volume < before, "the thread took nothing off the shaft: it still holds {}", now.volume);
        let most = std::f64::consts::PI * (100.0 - 8.5 * 8.5) * 10.0;
        let taken = before - now.volume;
        assert!(taken > most * 0.1 && taken < most, "an M20 thread over 10 of length takes between {} and {most} off, and it took {taken}", most * 0.1);
        // A REMOVED FACE: the wall of a hole 20 across taken away, the plain block back
        let mut program = a_block();
        worked(&mut program, "part.hole", &[[20.0, 15.0, 10.0]], &[("diameter", "20"), ("depth", "12")]);
        let holed = the_working_body(&mut program).volume;
        assert!(holed < 12000.0, "the hole was not bored: the body still holds {holed}");
        let answered = worked(&mut program, "part.remove-face", &[[20.0, 25.0, 5.0]], &[]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the removed face did not run: {answered:?}");
        let now = the_working_body(&mut program);
        assert!((now.volume - 12000.0).abs() < 1.0, "taking the hole's wall away leaves the block whole at 12000, and it holds {}", now.volume);
        assert_eq!(now.faces, 6, "the block whole has six faces, and it has {}", now.faces);
    }

    /// THE SWEEP AND THE LOFT, run as calls over the sketches of the tree and weighed: a square of 10
    /// swept along a straight line of 40 is a bar of 4000; a square of 10 lofted to a square of 20
    /// thirty above is a frustum of 30 / 3 * (100 + 400 + sqrt(100 * 400)) = 7000. The second section
    /// of the loft is drawn on a datum plane the registry's own call stands, the sketch opened on it by
    /// the pencil - the person's own way onto a face or a plane.
    #[test]
    fn the_sweeps_and_lofts_work_their_contracts() {
        let _turn = crate::checks::take_your_turn();
        // A SWEEP: the square on the table, the path rising from it on the front plane
        let mut program = qymcad::LiveLoop::start();
        let answered = crate::checks::ask_the_core(&mut program, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            call(core, "sketch.rect", &serde_json::json!({ "points": [[0.0, 0.0], [10.0, 10.0]] }));
            let finish = core.word("wb-finish");
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
            core.key(qymcad::Key::Escape); // the sketch's selection down, the plane buttons of the panel up
            let xz = core.word("plane-xz-front");
            core.press_word(&xz);
            call(core, "sketch.line", &serde_json::json!({ "points": [[0.0, 0.0], [0.0, 40.0]] }));
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
            let names: Vec<String> = core.document().sketches.iter().map(|s| s.name.clone()).collect();
            call(core, "part.sweep", &serde_json::json!({ "sketches": names }))
        })
        .expect("the sweep answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the sweep did not run: {answered:?}");
        let swept = the_working_body(&mut program);
        assert!((swept.volume - 4000.0).abs() < 1.0, "a square of 10 swept along 40 holds 4000, and the body holds {}", swept.volume);
        // A LOFT: the square on the table, the square of 20 on a datum 30 above
        let mut program = qymcad::LiveLoop::start();
        let answered = crate::checks::ask_the_core(&mut program, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            call(core, "sketch.rect", &serde_json::json!({ "points": [[0.0, 0.0], [10.0, 10.0]] }));
            let finish = core.word("wb-finish");
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
            call(core, "part.datum-plane", &serde_json::json!({ "picks": [[0.0, 0.0, 0.0]], "Offset": "30" }));
            let pencil = core.word("g-sketch-pick-hint");
            core.press_hint(&pencil);
            let on_datum = core.in_space([0.0, 0.0, 30.0]);
            core.click(on_datum);
            call(core, "sketch.rect", &serde_json::json!({ "points": [[-5.0, -5.0], [15.0, 15.0]] }));
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
            let names: Vec<String> = core.document().sketches.iter().map(|s| s.name.clone()).collect();
            call(core, "part.loft", &serde_json::json!({ "sketches": names }))
        })
        .expect("the loft answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the loft did not run: {answered:?}");
        let lofted = the_working_body(&mut program);
        assert!((lofted.volume - 7000.0).abs() < 1.0, "a square of 10 lofted to a square of 20 thirty above holds 7000, and the body holds {}", lofted.volume);
    }

    /// THE DATUMS, run as calls and read where they stand: a plane 10 above the top face of the block,
    /// an axis along an edge of it, a point at the coordinates the call carried - each in the tree, each
    /// leaving the body untouched. The pencil that opens a sketch on a face - the person's way onto the
    /// face of a body, no command of the catalogue but the whole of the person's vocabulary - opens one
    /// on the block's top, standing on the extrusion under it.
    #[test]
    fn the_datums_stand_where_the_call_puts_them() {
        let _turn = crate::checks::take_your_turn();
        let mut program = a_block();
        // THE PENCIL ON A FACE: a sketch opens on the top of the block, on the extrusion under it
        let sketch = crate::checks::ask_the_core(&mut program, |core| {
            let pencil = core.word("g-sketch-pick-hint");
            core.press_hint(&pencil);
            let top = core.face_at([20.0, 15.0, 10.0]);
            core.click(top);
            let document = core.document();
            (document.editing.clone(), document.features.iter().filter(|f| f.kind == "Sketch").count())
        })
        .expect("the pencil answered");
        let (open, sketches) = sketch;
        assert!(open.is_some(), "the pencil opened no sketch on the face");
        assert!(sketches >= 2, "the sketch on the face stands on no node of the timeline: {sketches} sketches");
        let finish = crate::checks::ask_the_core(&mut program, |core| core.word("wb-finish")).expect("the word was read");
        crate::checks::ask_the_core(&mut program, move |core| {
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
        })
        .expect("the sketch was left");
        // A PLANE 10 above the top face
        let answered = worked(&mut program, "part.datum-plane", &[[20.0, 15.0, 10.0]], &[("offset", "10")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the datum plane did not run: {answered:?}");
        // AN AXIS along an edge
        let answered = worked(&mut program, "part.datum-axis", &[[20.0, 0.0, 10.0]], &[]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the datum axis did not run: {answered:?}");
        // A POINT at coordinates
        let answered = worked(&mut program, "part.datum-point", &[], &[("X", "10"), ("Y", "5"), ("Z", "3")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the datum point did not run: {answered:?}");
        let (plane, axis, point, kinds, volume) = crate::checks::ask_the_core(&mut program, |core| {
            let document = core.document();
            (
                document.datums.iter().find(|d| d.kind == "plane" && (d.at[2] - 20.0).abs() < 1e-6).cloned(),
                document.datums.iter().find(|d| d.kind == "axis").cloned(),
                document.datums.iter().find(|d| d.kind == "point" && (d.at[0] - 10.0).abs() < 1e-6 && (d.at[1] - 5.0).abs() < 1e-6 && (d.at[2] - 3.0).abs() < 1e-6).cloned(),
                (document.features.iter().any(|f| f.kind == "Plane"), document.features.iter().any(|f| f.kind == "DatumAxis"), document.features.iter().any(|f| f.kind == "DatumPoint")),
                document.bodies.iter().filter(|b| !b.consumed).map(|b| b.volume).sum::<f64>(),
            )
        })
        .expect("the datums were read");
        assert!(plane.is_some(), "no plane stands 10 above the top face (z 20)");
        assert!(axis.is_some(), "no axis stands along the edge clicked");
        assert!(point.is_some(), "no point stands at (10, 5, 3)");
        assert!(kinds.0 && kinds.1 && kinds.2, "the timeline holds no node of each datum: {kinds:?}");
        assert!((volume - 12000.0).abs() < 1e-6, "the datums moved the body: it holds {volume}");
    }

    /// THE PATTERNS AND THE MIRROR, run as calls and weighed: a mirror about the right face of the block
    /// doubles it; a row of three copies 25 apart grows it to 90 long; six copies turned the whole way
    /// about the axis fall apart as six pieces of their own, together six times the body.
    #[test]
    fn the_patterns_and_mirrors_work_their_contracts() {
        let _turn = crate::checks::take_your_turn();
        // A MIRROR about the right face, the original kept: one body of 24000
        let mut program = a_block();
        let answered = worked(&mut program, "part.mirror", &[[20.0, 15.0, 10.0], [40.0, 15.0, 5.0]], &[]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the mirror did not run: {answered:?}");
        let now = the_working_body(&mut program);
        assert!((now.volume - 24000.0).abs() < 1.0, "the block reflected about its right face holds 24000, and it holds {}", now.volume);
        // A ROW of three copies 25 apart: one block 90 long
        let mut program = a_block();
        let answered = worked(&mut program, "part.array-linear", &[[20.0, 15.0, 10.0]], &[("pitch", "25"), ("copies", "3")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the linear array did not run: {answered:?}");
        let now = the_working_body(&mut program);
        assert!((now.volume - 27000.0).abs() < 1.0, "three copies of the block 25 apart overlap into 27000, and the body holds {}", now.volume);
        assert!((now.max[0] - 90.0).abs() < 1e-3, "the row runs to 90 long, and it reaches {}", now.max[0]);
        // A RING of six copies about the axis, the block standing clear of it: six pieces of 8000
        let mut program = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut program, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            let rect = core.word("tb-rect-hint");
            core.press_hint(&rect);
            core.click_on_sketch(200.0, 0.0).click_on_sketch(240.0, 20.0);
            let finish = core.word("wb-finish");
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
            call(core, "part.extrude", &serde_json::json!({ "Length": "10" }));
        })
        .expect("the block was built");
        let answered = worked(&mut program, "part.array-circular", &[[220.0, 0.0, 5.0]], &[("copies", "6")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the circular array did not run: {answered:?}");
        // the copies fill one body of the timeline; the exact body it holds is the six solids of them
        let pieces = crate::checks::ask_the_core(&mut program, |core| {
            let document = core.document();
            let i = document.bodies.iter().rposition(|b| !b.consumed && !b.sheet).expect("a body stands");
            (document.bodies[i].volume, core.inspect(i).map(|is| is.solids))
        })
        .expect("the pieces were read");
        assert_eq!(pieces.1, Some(6), "six copies stand apart as six solids of the body, and {:?} do", pieces.1);
        assert!((pieces.0 - 48000.0).abs() < 1.0, "each copy holds the 8000 of the block and together they hold 48000, and the body holds {}", pieces.0);
    }

    /// THE SURFACES, run as calls and weighed: a face taken out as a sheet of its own; a sheet lifted by
    /// its distance; two sheets stitched into one surface along the edge they meet at; a disc spanned
    /// over the top of a hole; a face replaced by the sheet lying on it; a sheet trimmed by the boss
    /// that grows through it; a mesh recognised back into a body. Each leaves the body it worked beside
    /// whole, and what it made is read off the sheets and the bodies, not the words.
    #[test]
    fn the_surfaces_are_worked_by_their_calls() {
        let _turn = crate::checks::take_your_turn();
        // A COPIED FACE: a sheet of 1200 beside the untouched block
        let mut program = a_block();
        let answered = worked(&mut program, "part.face-copy", &[[20.0, 15.0, 10.0]], &[]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the face copy did not run: {answered:?}");
        let (sheets, solid) = (the_sheets(&mut program), the_working_body(&mut program));
        assert_eq!(sheets.len(), 1, "one sheet stands, and {} do", sheets.len());
        assert!((sheets[0].area - 1200.0).abs() < 1.0, "the surface of the 40 by 30 face is 1200, and it is {}", sheets[0].area);
        assert!((solid.volume - 12000.0).abs() < 1e-6, "taking a copy of the face changed the block: it holds {}", solid.volume);
        // AN OFFSET SURFACE: the sheet lifted 5, standing at z 15
        let mut program = a_block();
        let answered = worked(&mut program, "part.offset-surface", &[[30.0, 10.0, 10.0]], &[("distance", "5")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the offset surface did not run: {answered:?}");
        let sheets = the_sheets(&mut program);
        assert_eq!(sheets.len(), 1, "one sheet stands, and {} do", sheets.len());
        assert!((sheets[0].min[2] - 15.0).abs() < 1e-3 && (sheets[0].max[2] - 15.0).abs() < 1e-3, "the sheet lifted 5 stands at z 15, and it spans {:?}", sheets[0].min);
        // A STITCH: the two sheets closed along the edge they meet at, one surface of 1600
        let mut program = a_block();
        worked(&mut program, "part.face-copy", &[[20.0, 15.0, 10.0]], &[]);
        worked(&mut program, "part.face-copy", &[[20.0, 0.0, 5.0]], &[]);
        assert_eq!(the_sheets(&mut program).len(), 2, "the two sheets do not stand to be stitched");
        let answered = worked(&mut program, "part.stitch", &[[30.0, 15.0, 10.0], [30.0, 0.0, 5.0]], &[("tol", "0.01")]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the stitch did not run: {answered:?}");
        let sheets = the_sheets(&mut program);
        assert_eq!(sheets.len(), 1, "the two sheets became not one surface: {} stand", sheets.len());
        assert!((sheets[0].area - 1600.0).abs() < 1.0, "the top and the front stitched hold 1600, and they hold {}", sheets[0].area);
        // A PATCH: a disc spanned over the top of a hole 20 across
        let mut program = a_block();
        worked(&mut program, "part.hole", &[[20.0, 15.0, 10.0]], &[("diameter", "20"), ("depth", "12")]);
        let plain = the_working_body(&mut program).volume;
        let answered = worked(&mut program, "part.patch", &[[30.0, 15.0, 10.0]], &[]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the patch did not run: {answered:?}");
        let (sheets, solid) = (the_sheets(&mut program), the_working_body(&mut program));
        assert_eq!(sheets.len(), 1, "the patch spanned no surface of its own: {} stand", sheets.len());
        assert!((sheets[0].area - std::f64::consts::PI * 100.0).abs() < 1.5, "the disc over the hole of 20 across is 314.16 to within the mesh's deflection, and it is {}", sheets[0].area);
        assert!((solid.volume - plain).abs() < 1e-6, "the patch changed the body: it holds {} over {}", solid.volume, plain);
        // A REPLACED FACE: the top face takes the sheet lying on it, the sheet handed back is used up
        let mut program = a_block();
        worked(&mut program, "part.face-copy", &[[20.0, 15.0, 10.0]], &[]);
        worked(&mut program, "part.face-copy", &[[20.0, 0.0, 5.0]], &[]);
        let answered = worked(&mut program, "part.surface-replace", &[[30.0, 15.0, 10.0], [30.0, 15.0, 10.0]], &[]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the replaced face did not run: {answered:?}");
        let (sheets, solid) = (the_sheets(&mut program), the_working_body(&mut program));
        assert_eq!(sheets.len(), 1, "the sheet handed to the face is used up and the other stays: {} stand", sheets.len());
        assert!((solid.volume - 12000.0).abs() < 1.0, "the face took the shape of its own copy and the block is what it was, and it holds {}", solid.volume);
        // A TRIMMED SHEET: the disc under the boss cut out of the sheet
        let mut program = a_block();
        worked(&mut program, "part.face-copy", &[[20.0, 15.0, 10.0]], &[]);
        crate::checks::ask_the_core(&mut program, |core| {
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            call(core, "sketch.circle", &serde_json::json!({ "points": [[20.0, 15.0], [25.0, 15.0]] }));
            let finish = core.word("wb-finish");
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
            call(core, "part.extrude", &serde_json::json!({ "Length": "20" }));
        })
        .expect("the boss was grown");
        let answered = worked(&mut program, "part.trim", &[[5.0, 5.0, 10.0], [20.0, 15.0, 20.0]], &[]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the trim did not run: {answered:?}");
        let (sheets, solid) = (the_sheets(&mut program), the_working_body(&mut program));
        assert_eq!(sheets.len(), 1, "the trimmed sheet stands beside the body: {} stand", sheets.len());
        assert!((sheets[0].area - (1200.0 - std::f64::consts::PI * 25.0)).abs() < 1.0, "the sheet loses the disc of 5 under the boss, and it holds {}", sheets[0].area);
        assert!((solid.volume - (12000.0 + std::f64::consts::PI * 250.0)).abs() < 1.0, "the boss grown through the block holds 12785.4 with it, and the body holds {}", solid.volume);
        // A RECOGNISED MESH: a cube of 10 imported from a file, turned back into a body
        let mut program = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut program, |core| {
            core.key(qymcad::Key::Escape); // the start screen away, the toolbar's own new-part button in - the part entered, where the mesh lands pickable
            let new_part = core.word("tb-new-part-hint");
            core.press_hint(&new_part);
            let path = std::env::temp_dir().join(format!("qymcad-registry-recognise-{}.stl", std::process::id()));
            std::fs::write(&path, a_cube_stl(10.0)).expect("the mesh file was written");
            core.import(&path.to_string_lossy());
            core.settle();
            core.key(qymcad::Key::Enter); // the window of scale takes the mesh in as it stands
            core.settle();
        })
        .expect("the mesh was brought in");
        let answered = worked(&mut program, "part.recognise", &[[10.0, 3.0, 5.0]], &[]);
        assert!(matches!(answered, TheCall::Answered { .. }), "the recognise did not run: {answered:?}");
        let (body, node) = crate::checks::ask_the_core(&mut program, |core| {
            let document = core.document();
            let node = document.features.iter().rfind(|f| f.kind == "MeshRecognised").cloned().expect("the recognised node");
            (document.bodies.into_iter().rfind(|b| !b.consumed && !b.sheet).expect("a body stands"), node)
        })
        .expect("the body was read");
        assert!(node.error.is_none(), "the recognised node is red: {node:?}");
        assert!((body.volume - 1000.0).abs() < 1.0, "the cube of 10 recognised holds 1000, and the body holds {}", body.volume);
    }

    /// THE JOINTS OF EVERY KIND, run as calls between the top faces of two blocks and weighed: each
    /// kind asked for is the kind that stands in the list, each holds (nothing violated), and the
    /// rigid one brings the second block onto the first wherever it stood - the acceptance suite's
    /// own numbers. A kind no joint comes in is refused in words that name the kinds, and nothing
    /// changed by it.
    #[test]
    fn the_joints_of_every_kind_hold_what_they_promise() {
        let _turn = crate::checks::take_your_turn();
        for (_, kind) in JOINT_KINDS {
            let kind = kind.to_string();
            let (for_the_call, the_kind) = (kind.clone(), kind.clone());
            let mut program = qymcad::LiveLoop::start();
            let answered = crate::checks::ask_the_core(&mut program, move |core| {
                the_block_in_the_assembly(core);
                a_part_made_in_the_assembly(core, |core| {
                    call(core, "sketch.rect", &serde_json::json!({ "points": [[0.0, 0.0], [40.0, 30.0]] }));
                });
                the_last_part_placed_at(core, 60.0);
                call(core, "assembly.joint", &serde_json::json!({ "kind": for_the_call, "picks": [[20.0, 15.0, 10.0], [80.0, 15.0, 10.0]] }))
            })
            .expect("the call answered");
            assert!(matches!(answered, TheCall::Answered { .. }), "a {kind} joint was not laid: {answered:?}");
            let (stands, holds, second_at) = crate::checks::ask_the_core(&mut program, move |core| {
                let document = core.document();
                let joint = document.joints.last().cloned().expect("a joint stands");
                (the_same_kind(&joint.kind, &the_kind), !joint.violated, document.parts.last().map(|p| p.at))
            })
            .expect("the joint was read");
            assert!(stands, "a {kind} joint was asked for and another kind stands");
            assert!(holds, "the {kind} joint does not hold");
            if kind == "Rigid" {
                let at = second_at.expect("the second part stands");
                assert!(at.iter().all(|v| v.abs() < 1e-3), "the rigid joint did not bring the second block onto the first: it stands at {at:?}");
            }
        }
        // A KIND NO JOINT COMES IN: refused in words, the document as it stood
        let mut program = qymcad::LiveLoop::start();
        let (answered, joints) = crate::checks::ask_the_core(&mut program, |core| {
            the_block_in_the_assembly(core);
            a_part_made_in_the_assembly(core, |core| {
                call(core, "sketch.rect", &serde_json::json!({ "points": [[0.0, 0.0], [40.0, 30.0]] }));
            });
            the_last_part_placed_at(core, 60.0);
            let answered = call(core, "assembly.joint", &serde_json::json!({ "kind": "Wobbly", "picks": [[20.0, 15.0, 10.0], [80.0, 15.0, 10.0]] }));
            (answered, core.document().joints.len())
        })
        .expect("the call answered");
        match answered {
            TheCall::Refused(words) => assert!(words.contains("Wobbly") && words.contains("Rigid"), "the refusal does not name what was asked and what there is: {words}"),
            other => panic!("a joint of no kind was laid: {other:?}"),
        }
        assert_eq!(joints, 0, "the refused call left a joint in the list");
    }

    /// THE GROUND AND THE GROUP, run as calls and read where they stand: the ground fixes the part
    /// under its click - the anchor of the assembly, the tool staying in hand until Esc puts it down;
    /// the group holds the two parts together where they stand, a mate of the kind "Group" and no
    /// joint laid between them.
    #[test]
    fn the_ground_and_the_group_hold_what_they_promise() {
        let _turn = crate::checks::take_your_turn();
        // THE GROUND: one part, fixed by the click
        let mut program = qymcad::LiveLoop::start();
        let answered = crate::checks::ask_the_core(&mut program, |core| {
            the_block_in_the_assembly(core);
            call(core, "assembly.ground", &serde_json::json!({ "picks": [[20.0, 15.0, 10.0]] }))
        })
        .expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the ground did not run: {answered:?}");
        let (grounded, in_hand) = crate::checks::ask_the_core(&mut program, |core| {
            let grounded = core.document().parts.iter().filter(|p| p.grounded).count();
            (grounded, core.in_hand())
        })
        .expect("the parts were read");
        assert_eq!(grounded, 1, "one part was clicked and {grounded} stand fixed");
        assert!(in_hand.is_empty(), "the tool stayed in hand after Esc: {in_hand:?}");
        // THE GROUP: two parts, held together where they stand
        let mut program = qymcad::LiveLoop::start();
        let answered = crate::checks::ask_the_core(&mut program, |core| {
            the_block_in_the_assembly(core);
            a_part_made_in_the_assembly(core, |core| {
                call(core, "sketch.rect", &serde_json::json!({ "points": [[0.0, 0.0], [40.0, 30.0]] }));
            });
            the_last_part_placed_at(core, 60.0);
            call(core, "assembly.group", &serde_json::json!({ "picks": [[20.0, 15.0, 10.0], [80.0, 15.0, 10.0]] }))
        })
        .expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the group did not run: {answered:?}");
        let (mates, joints, at) = crate::checks::ask_the_core(&mut program, |core| {
            let document = core.document();
            (document.mates.clone(), document.joints.len(), document.parts.last().map(|p| p.at))
        })
        .expect("the mates were read");
        assert_eq!(mates, vec!["Group".to_string()], "the group is not the one mate standing: {mates:?}");
        assert_eq!(joints, 0, "a group laid a joint, and it is no joint");
        let at = at.expect("the second part stands");
        assert!((at[0] - 60.0).abs() < 1e-3, "the parts did not stay where they stood: the second is at {at:?}");
    }

    /// THE TANGENT AND THE WIDTH, run as calls and read where they stand: the shaft comes to rest on
    /// the face of the block, the distance between them its radius; the tab comes to stand midway
    /// between the walls. Each leaves its mate of its own kind in the list.
    #[test]
    fn the_tangent_and_the_width_rest_and_sit_their_parts() {
        let _turn = crate::checks::take_your_turn();
        // THE TANGENT: a shaft 10 across resting its side on the front face of the block
        let mut program = qymcad::LiveLoop::start();
        let answered = crate::checks::ask_the_core(&mut program, |core| {
            the_fixture_of_the_mates(core, "assembly.tangent");
            let given = the_sample_of_the_mates(core, "assembly.tangent");
            call(core, "assembly.tangent", &given)
        })
        .expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the tangent did not run: {answered:?}");
        let (mates, at) = crate::checks::ask_the_core(&mut program, |core| {
            let document = core.document();
            (document.mates.clone(), document.parts.last().map(|p| p.at))
        })
        .expect("the mates were read");
        assert_eq!(mates, vec!["Tangent".to_string()], "the tangent is not the one mate standing: {mates:?}");
        let at = at.expect("the shaft's part stands");
        let off = (15.0 + at[1]).abs(); // the shaft was drawn about y 15: its centre stands 15 + at off the face, on either side of it
        assert!((off - 5.0).abs() < 1e-3, "the shaft of radius 5 does not rest its centre 5 off the face: the part is at {at:?}, its centre {off} off");
        // THE WIDTH: the tab set midway between the walls ten and forty apart
        let mut program = qymcad::LiveLoop::start();
        let answered = crate::checks::ask_the_core(&mut program, |core| {
            the_fixture_of_the_mates(core, "assembly.width");
            let given = the_sample_of_the_mates(core, "assembly.width");
            call(core, "assembly.width", &given)
        })
        .expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the width did not run: {answered:?}");
        let (mates, at) = crate::checks::ask_the_core(&mut program, |core| {
            let document = core.document();
            (document.mates.clone(), document.parts.last().map(|p| p.at))
        })
        .expect("the mates were read");
        assert_eq!(mates, vec!["Width".to_string()], "the width is not the one mate standing: {mates:?}");
        let at = at.expect("the tab's part stands");
        assert!((at[0] - 10.0).abs() < 1e-3, "the tab drawn at 20..30 does not sit midway between the faces picked, at 10 and at 60 (35 its centre, 10 over where it stood): the part is at {at:?}");
    }

    /// THE RELATION AND THE DRAG, run over two hinges the person's own gestures laid: the relation
    /// ties the turns of the two hinges in step, a Gear mate beside the joints; the drag - the
    /// person's own gesture of taking a part and pulling - turns the hanging part about its hinge,
    /// the one degree of freedom it has left.
    #[test]
    fn the_relations_tie_two_hinges_and_the_drag_turns_them() {
        let _turn = crate::checks::take_your_turn();
        // THE RELATION: a gear over the two hinges
        let mut program = qymcad::LiveLoop::start();
        let answered = crate::checks::ask_the_core(&mut program, |core| {
            the_fixture_of_the_mates(core, "assembly.relation");
            let given = the_sample_of_the_mates(core, "assembly.relation");
            call(core, "assembly.relation", &given)
        })
        .expect("the call answered");
        assert!(matches!(answered, TheCall::Answered { .. }), "the relation did not run: {answered:?}");
        let mates = crate::checks::ask_the_core(&mut program, |core| core.document().mates.clone()).expect("the mates were read");
        assert_eq!(mates, vec!["Gear".to_string()], "the gear is not the one mate standing: {mates:?}");
        // THE DRAG: the part that hangs on a hinge follows the pull, turning about the hinge
        let mut program = qymcad::LiveLoop::start();
        let turned = crate::checks::ask_the_core(&mut program, |core| {
            the_block_in_the_assembly(core);
            a_part_made_in_the_assembly(core, |core| {
                call(core, "sketch.rect", &serde_json::json!({ "points": [[0.0, 0.0], [40.0, 30.0]] }));
            });
            the_last_part_placed_at(core, 60.0);
            a_revolute_hinge_as_built(core, [40.0, 0.0, 10.0], [60.0, 0.0, 10.0]);
            let before = core.document().parts.last().map(|p| (p.at, p.axes)).expect("the second part stands");
            let from = core.face_at([60.0, 15.0, 10.0]); // the part on the hinge, not the grounded block
            let to = from + qymcad::vec2(0.0, -120.0);
            core.drag(from, to, qymcad::PointerButton::Primary, qymcad::Modifiers::SHIFT);
            let now = core.document().parts.last().map(|p| (p.at, p.axes)).expect("the second part stands");
            (before, now)
        })
        .expect("the drag ran");
        let ((at_before, axes_before), (at_now, axes_now)) = turned;
        assert!(at_now != at_before || axes_now != axes_before, "the part on the hinge did not follow the pull: it stands at {at_now:?} looking {axes_now:?}, as before");
    }
}
