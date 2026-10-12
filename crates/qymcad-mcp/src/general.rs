//! THE OPERATIONS OF THE WHOLE PROGRAM - the view, the file, the table of parameters: what no workbench
//! owns and no command of the catalogue launches, offered to the agent by name beside the catalogue's
//! commands. Each is named by the word of the dictionary its own surface carries (a menu item, a bar
//! button) and answered by its help article; each runs by the person's own gestures - a menu walked, the
//! viewcube clicked, the table of parameters filled row by row, the file doors a chooser's answer runs
//! through. The agent's file operations carry their path and put no chooser up, as the spec of the
//! server says.
use crate::registry::{the_open_sketch, the_places, the_same_kind, the_words_of, Answered, Run, Take};

/// ONE OPERATION OF THE WHOLE PROGRAM, as the agent calls it by name - and one gesture of the
/// person's own vocabulary, which the UI-driver escape hatch serves in the same shape.
pub(crate) struct General {
    /// The code the operation is called by: `area.name`, the catalogue's own style.
    pub(crate) code: &'static str,
    /// The help article the operation answers by.
    pub(crate) article: &'static str,
    /// The key of the dictionary word that names it: a menu item of its own, a bar button - read
    /// through the same dictionary the window reads, so the agent's name for it cannot drift from the
    /// person's.
    pub(crate) name_key: &'static str,
    /// The words the operation is also found by.
    pub(crate) synonyms: &'static [&'static str],
    /// ITS RUN over the agent's session.
    pub(crate) run: Run,
    /// The parameters the run takes.
    pub(crate) takes: &'static [Take],
}

/// THE OPERATIONS OF THE WHOLE PROGRAM: the view and its fit, the file at both its doors, the table of
/// parameters. The formats the program writes are words the export entry is found by, so a client that
/// searches for a format finds the door that writes it.
pub(crate) const THE_OPERATIONS_OF_THE_PROGRAM: &[General] = &[
    General { code: "view.fit", article: "general/09-viewport", name_key: "menu-fit-view", synonyms: &["zoom", "frame"], run: fit_the_view, takes: &[] },
    General { code: "view.orbit", article: "general/09-viewport", name_key: "menu-orbit3d", synonyms: &["turn", "cube"], run: turn_the_view, takes: &[Take { name: "side", kind: "text" }] },
    General {
        code: "parameters.set",
        article: "general/05-parameters",
        name_key: "wb-params",
        synonyms: &["formula", "variable"],
        run: hold_a_parameter,
        takes: &[Take { name: "name", kind: "text" }, Take { name: "expression", kind: "text" }],
    },
    General { code: "file.import", article: "general/06-import-export", name_key: "file-import", synonyms: &["load", "read"], run: bring_the_file_in, takes: &[Take { name: "path", kind: "text" }] },
    General {
        code: "file.export",
        article: "general/06-import-export",
        name_key: "file-export",
        synonyms: &["step", "iges", "stl", "obj", "ply", "glb", "gltf", "3mf", "amf", "write"],
        run: write_the_model_out,
        takes: &[Take { name: "format", kind: "text" }, Take { name: "path", kind: "text" }],
    },
    General {
        code: "sketch.constrain",
        article: "general/15-agent",
        name_key: "mcp-constrain",
        synonyms: &["tie", "join", "constrain"],
        run: lay_a_constraint,
        takes: &[Take { name: "kind", kind: "text" }, Take { name: "points", kind: "points" }],
    },
    General {
        code: "file.export-sketch",
        article: "general/06-import-export",
        name_key: "act-export",
        synonyms: &["svg", "dxf", "drawing"],
        run: write_the_sketch_out,
        takes: &[Take { name: "sketch", kind: "text" }, Take { name: "format", kind: "text" }, Take { name: "path", kind: "text" }],
    },
];

/// FIT THE VIEW to what stands, as the person's own menu item does: the View menu opened by its word,
/// its item pressed. Where no menu bar stands - a screen the menus are not drawn over - the call is
/// refused in words rather than guessed at.
fn fit_the_view(core: &mut qymcad::AgentSession, _code: &str, _given: &serde_json::Value) -> Answered {
    let (view, fit) = (core.word("menu-view"), core.word("menu-fit-view"));
    if core.find(&view, qymcad::pos2(0.0, 0.0)).is_none() {
        return Answered::Refused("the menu bar does not stand on this screen - the view cannot be fitted from here".into());
    }
    core.menu(&[&view, &fit]);
    core.settle();
    Answered::Ran("the view fitted to what stands".into())
}

/// THE SIDES THE VIEW TURNS TO, as the viewcube's own faces name them: the word a call may carry, the
/// key of the face's own word, and the direction the face looks from.
const SIDES: &[(&str, &str, [i8; 3])] = &[
    ("front", "view-front", [0, -1, 0]),
    ("back", "view-back", [0, 1, 0]),
    ("top", "view-top", [0, 0, 1]),
    ("bottom", "view-bottom", [0, 0, -1]),
    ("left", "view-left", [-1, 0, 0]),
    ("right", "view-right", [1, 0, 0]),
];

/// TURN THE VIEW to the side the call names, as the person's own viewcube does: the face's zone
/// clicked, the turn waited out to its rest. The side is matched against the program's own word for
/// the face in the language it speaks, as the joint kinds are matched; a side the viewcube carries no
/// face for is refused in words that name the sides, and so is a view the cube does not stand over -
/// the sheet of an open sketch.
fn turn_the_view(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(side) = the_words_of(given, "side") else {
        return Answered::Refused(format!("the turn needs the side it looks from: one of {}", SIDES.iter().map(|(en, _, _)| *en).collect::<Vec<_>>().join(", ")));
    };
    let named = SIDES.iter().find(|(en, key, _)| the_same_kind(en, &side) || the_same_kind(&core.word(key), &side));
    let Some((_, key, dir)) = named else {
        return Answered::Refused(format!("{side}: the viewcube carries no face so; the sides are {}", SIDES.iter().map(|(en, _, _)| *en).collect::<Vec<_>>().join(", ")));
    };
    // A FACE SEEN EDGE-ON has the centre of its zone covered by the face the view stands on: the cube
    // is rolled over the edge zone between the two first, as a person rolls it, and the face's own
    // zone then stands clear
    if core.cube_toward(*dir).is_none() {
        let Some(axis) = the_axis_looked_along(core) else {
            return Answered::Refused("the viewcube does not stand over this view - it is the sheet of an open sketch; finish the sketch and turn again".into());
        };
        let mut rolled = false;
        for sign in [1_i8, -1] {
            let mut hop = *dir;
            hop[axis] += sign;
            if let Some(at) = core.cube_toward(hop) {
                core.click(at);
                core.settle();
                rolled = true;
                break;
            }
        }
        if !rolled {
            return Answered::Refused(format!("the viewcube offers no way to the {} from where it stands", core.word(key)));
        }
    }
    let Some(at) = core.cube_toward(*dir) else {
        return Answered::Refused("the viewcube does not stand over this view - it is the sheet of an open sketch; finish the sketch and turn again".into());
    };
    core.click(at);
    core.settle();
    Answered::Ran(format!("the view turned to look from the {}", core.word(key)))
}

/// THE AXIS THE VIEW LOOKS ALONG: the world axis whose far points project to one place, `None` when
/// the view looks along none of them.
fn the_axis_looked_along(core: &mut qymcad::AgentSession) -> Option<usize> {
    (0..3).find(|&axis| {
        let mut a = [0.0_f64; 3];
        a[axis] = 1000.0;
        let mut b = a;
        b[axis] = -1000.0;
        core.projected(a).distance(core.projected(b)) < 1.0
    })
}

/// HOLD A PARAMETER at the expression the call carries, as a person holds one in the table of
/// parameters: the table opened from its bar button, and either a row added and filled - the name into
/// the field that waits for it, the expression beside it - or the field of the row that already reads
/// that name typed over; Enter applies, the table closed again. A parameter of another follows it
/// (w = 50, d = w/2, then w = 60 carries d to 30); a name or a formula the table will not take is the
/// program's own words - the row's error stands under the table where a person reads it - and the
/// answer says so.
fn hold_a_parameter(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(name) = the_words_of(given, "name") else { return Answered::Refused("the parameter needs the name it is held by".into()) };
    let Some(expression) = the_words_of(given, "expression") else {
        return Answered::Refused("the parameter needs the expression it holds - a number, or a formula over the other parameters (w/2, sin(30))".into());
    };
    let params = core.word("wb-params");
    if core.find(&params, qymcad::pos2(0.0, 0.0)).is_none() {
        return Answered::Refused("the table of parameters is reached from the bar of a workbench - step into a part first".into());
    }
    let stands = core.document().parameters.iter().any(|p| p.name == name);
    core.press_word_near(&params, qymcad::pos2(0.0, 0.0));
    if stands {
        // THE ROW THAT READS THE NAME, its expression field typed over, as a person edits a standing
        // parameter: the row is sought beside the table's own button, for the same name can stand
        // elsewhere, in a field that uses it
        let add = core.word("par-add");
        let near = core.find(&add, qymcad::pos2(640.0, 400.0)).map_or(qymcad::pos2(640.0, 400.0), |r| r.center());
        let Some(row) = core.find(&name, near) else {
            let title = core.word("win-params");
            core.close_window(&title);
            return Answered::Refused(format!("the table shows no parameter {name:?}"));
        };
        let field = core
            .widgets()
            .into_iter()
            .filter(|w| w.kind == qymcad::Kind::TextField && w.rect.center().y > row.min.y && w.rect.center().y < row.max.y && w.rect.min.x > row.max.x)
            .min_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x));
        let Some(field) = field else {
            let title = core.word("win-params");
            core.close_window(&title);
            return Answered::Refused(format!("the row of {name:?} has no field to write an expression in"));
        };
        core.click(field.rect.center()).chord(qymcad::Modifiers::COMMAND, qymcad::Key::A).type_text(&expression).key(qymcad::Key::Enter);
    } else {
        let add = core.word("par-add");
        core.press_word(&add);
        let example = core.word("par-example");
        core.fill_empty("w", &name).fill_empty(&example, &expression).key(qymcad::Key::Enter);
    }
    core.settle();
    // THE ROW'S OWN ERROR stands under the table, named by the parameter it is of - the same words a
    // person reads; a parameter that came to stand has none
    let said = core.status();
    let errored = core.painted().into_iter().any(|(_, words)| words.starts_with(&format!("{name}: ")));
    let title = core.word("win-params");
    core.close_window(&title);
    match (core.document().parameters.iter().find(|p| p.name == name), errored) {
        (Some(p), false) if p.expr == expression => Answered::Ran(format!("{name} = {expression} ({}), held in the table of parameters", p.value)),
        (Some(p), _) => Answered::Refused(format!("{name} holds {} = {:?}, not {expression:?}: the table says what it cannot take", p.expr, p.value)),
        (None, _) => Answered::Refused(format!("{name} did not come to stand: {said}")),
    }
}

/// BRING THE FILE AT `path` INTO THE DOCUMENT, as the person's own import does: the door a chooser's
/// answer runs through, the chooser never put up, the path carried by the call itself. What the file
/// holds tells the rest: a solid comes in on its own; a mesh asks its units and scale, and the call
/// answers the window as the person's Enter does - keeping what came in as it stands; a drawing waits
/// for the plane it is laid on, and the call places it by a click where the table lies clear. A file
/// that is not there, or holds nothing the program reads, is the program's own refusal, and nothing
/// came in.
fn bring_the_file_in(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(path) = the_words_of(given, "path") else { return Answered::Refused("the import needs the path of the file to bring in".into()) };
    let drawing = std::path::Path::new(&path).extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_lowercase().as_str(), "dxf" | "svg"));
    let before = core.document();
    let (parts, bodies, sketches) = (before.parts.len(), before.bodies.len(), before.sketches.len());
    core.import(&path);
    core.settle();
    if core.find(&core.word("import-scale-title"), qymcad::pos2(700.0, 450.0)).is_some() {
        core.key(qymcad::Key::Enter);
        core.settle();
    }
    if drawing {
        let Some(table) = core.seen_at([0.0, 0.0, 0.0]) else {
            return Answered::Refused(format!("the drawing waits for the plane it is laid on, and the table's place is covered - turn the view or move what covers it: {}", core.status()));
        };
        core.click(table);
        core.settle();
    }
    let now = core.document();
    if now.parts.len() + now.bodies.len() + now.sketches.len() > parts + bodies + sketches {
        Answered::Ran(format!("{} came in: {}", std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.clone()), core.status()))
    } else {
        Answered::Refused(format!("nothing came in from {path}: {}", core.status()))
    }
}

/// THE FORMATS THE PROGRAM WRITES A MODEL IN, as its own export menu lists them: STEP and IGES carry
/// the exact B-rep, the rest are meshes of it.
enum OutFormat {
    /// An exact format: the B-rep, live from the kernel.
    Exact(qymcad_kernel::ExactFormat),
    /// A mesh format: a tessellation of the bodies.
    Mesh(qymcad_ui_state::MeshFormat),
}

/// THE FORMAT A WORD NAMES, as the export menu's own names do; `None` where the program writes no
/// model format so.
fn the_model_format(word: &str) -> Option<OutFormat> {
    match word.to_lowercase().as_str() {
        "step" => Some(OutFormat::Exact(qymcad_kernel::ExactFormat::Step)),
        "iges" | "igs" => Some(OutFormat::Exact(qymcad_kernel::ExactFormat::Iges)),
        "stl" => Some(OutFormat::Mesh(qymcad_ui_state::MeshFormat::Stl)),
        "obj" => Some(OutFormat::Mesh(qymcad_ui_state::MeshFormat::Obj)),
        "ply" => Some(OutFormat::Mesh(qymcad_ui_state::MeshFormat::Ply)),
        "glb" | "gltf" => Some(OutFormat::Mesh(qymcad_ui_state::MeshFormat::Glb)),
        "3mf" => Some(OutFormat::Mesh(qymcad_ui_state::MeshFormat::ThreeMf)),
        "amf" => Some(OutFormat::Mesh(qymcad_ui_state::MeshFormat::Amf)),
        _ => None,
    }
}

/// WRITE THE WHOLE MODEL OUT at `path`, in the format the call names, as the answer to the export
/// chooser does: the chooser never put up, the path carried by the call itself. A mesh goes out at the
/// menu's own standard detail, a deflection of 0.05 mm. A format the program writes no model in is
/// refused in words that name them (a drawing of a sketch is the other export door); a model with
/// nothing to write is the program's own refusal, and nothing is written.
fn write_the_model_out(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(format) = the_words_of(given, "format") else { return Answered::Refused("the export needs the format it writes: step, iges, stl, obj, ply, glb, 3mf or amf".into()) };
    let Some(path) = the_words_of(given, "path") else { return Answered::Refused("the export needs the path it writes to".into()) };
    let Some(out) = the_model_format(&format) else {
        return Answered::Refused(format!("{format}: the program writes a model in step, iges, stl, obj, ply, glb, 3mf or amf (a drawing of a sketch in svg or dxf, by file.export-sketch)"));
    };
    match out {
        OutFormat::Exact(format) => core.export_exact(format, &path),
        OutFormat::Mesh(format) => core.export_mesh(format, &path, 0.05),
    }
    core.settle();
    match std::fs::metadata(&path) {
        Ok(file) if file.len() > 0 => Answered::Ran(format!("the model written to {path} ({} bytes)", file.len())),
        _ => Answered::Refused(format!("nothing was written to {path}: {}", core.status())),
    }
}

/// WRITE THE SKETCH NAMED `sketch` OUT as a drawing - an SVG or a DXF - at `path`, the exact curves of
/// the sheet and not a tessellation: the same write the tree's export menu makes when its chooser is
/// answered. A format a sketch does not go out in, or a sketch the tree does not hold, is refused in
/// words that name what there is.
fn write_the_sketch_out(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(sketch) = the_words_of(given, "sketch") else { return Answered::Refused("the export needs the name of the sketch it draws out".into()) };
    let Some(format) = the_words_of(given, "format") else { return Answered::Refused("the export needs the format it draws in: svg or dxf".into()) };
    let Some(path) = the_words_of(given, "path") else { return Answered::Refused("the export needs the path it writes to".into()) };
    let dxf = match format.to_lowercase().as_str() {
        "dxf" => true,
        "svg" => false,
        other => return Answered::Refused(format!("{other}: a sketch goes out as svg or dxf")),
    };
    let held: Vec<String> = core.document().sketches.iter().map(|s| s.name.clone()).collect();
    if !held.contains(&sketch) {
        return Answered::Refused(format!("no sketch named {sketch:?} is in the tree; the tree holds {held:?}"));
    }
    core.export_sketch(&sketch, dxf, &path);
    core.settle();
    match std::fs::metadata(&path) {
        Ok(file) if file.len() > 0 => Answered::Ran(format!("the sketch {sketch} drawn out to {path} ({} bytes)", file.len())),
        _ => Answered::Refused(format!("nothing was written to {path}: {}", core.status())),
    }
}

/// LAY A CONSTRAINT of the kind the call names, over the places it lists on the sheet of the open
/// sketch: the first place selects what stands there, each one after is taken with Shift held -
/// gathered into the selection, as a person gathers entities - and the constraint panel's own
/// button of the kind presses it. The sketch holds one constraint more after it and its degrees of
/// freedom stand at least one fewer; what the panel refuses - a kind the selection cannot take -
/// is the program's own refusal. This is how a sketch transferred from another program keeps its
/// own constraint net rather than the luck of drawn clicks.
fn lay_a_constraint(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(kind) = the_words_of(given, "kind") else {
        return Answered::Refused(
            "the constraint needs its kind, by the panel's own word: Coincident, Horizontal, Vertical, Parallel, Perpendicular, Equal, Fix, Collinear, Concentric, Tangent, Symmetric, Midpoint".into(),
        );
    };
    let Some(hint) = the_panel_hint_of(&kind) else {
        return Answered::Refused(format!(
            "{kind}: no constraint of the panel reads so; the kinds are: Coincident, Horizontal, Vertical, Parallel, Perpendicular, Equal, Fix, Collinear, Concentric, Tangent, Symmetric, Midpoint"
        ));
    };
    let Some(places) = the_places(given) else { return Answered::Refused("the constraint needs its places on the sheet, two at least".into()) };
    let Some(before) = the_open_sketch(core) else { return Answered::Refused("no sketch is open to constrain - open one and constrain again".to_string()) };
    let took = places.len();
    let mut places = places.into_iter();
    let Some((x, y)) = places.next() else { return Answered::Refused("the constraint needs its places on the sheet, two at least".into()) };
    // a drawing tool in hand would take the clicks as its own places: put it down first, as a
    // person does before gathering a selection
    core.key(qymcad::Key::Escape);
    core.click_on_sketch(x, y);
    for (x, y) in places {
        core.click_on_sketch_holding_shift(x, y);
    }
    let shown = core.word(hint);
    core.press_hint(&shown);
    let Some(after) = the_open_sketch(core) else { return Answered::Refused("the sketch closed under the constraint".into()) };
    if after.constraints > before.constraints && after.dof < before.dof {
        return Answered::Ran(format!("a {} laid over {} place(s) of {}; the sketch holds {} constraints and {} degrees of freedom", kind, took, after.name, after.constraints, after.dof));
    }
    Answered::Refused(format!("nothing was laid: {}", core.status()))
}

/// THE PANEL'S OWN HINT WORD of a constraint kind, as the person's panel shows it; `None` when no
/// kind of the panel reads so.
fn the_panel_hint_of(kind: &str) -> Option<&'static str> {
    Some(match kind.trim() {
        "Coincident" => "con-coincident-hint",
        "Horizontal" => "con-horizontal-hint",
        "Vertical" => "con-vertical-hint",
        "Parallel" => "con-parallel-hint",
        "Perpendicular" => "con-perpendicular-hint",
        "Equal" => "con-equal",
        "Fix" => "con-fix",
        "Collinear" => "con-collinear-hint",
        "Concentric" => "con-concentric-hint",
        "Tangent" => "con-tangent-hint",
        "Symmetric" => "con-symmetric-hint",
        "Midpoint" => "con-midpoint-hint",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use crate::checks::take_your_turn;
    use crate::registry::TheCall;

    /// ONE CALL, answered.
    fn call_it(program: &mut qymcad::LiveLoop, code: &str, given: serde_json::Value) -> TheCall {
        let code = code.to_string();
        crate::checks::ask_the_core(program, move |core| crate::registry::call(core, &code, &given)).expect("the call answered")
    }

    /// A BLOCK OF 40 X 30 X 10 in the first part (or the rectangle the `from`/`to` pair names, drawn
    /// wherever it is asked to), built through the registry's own calls.
    fn a_block_from(from: (f64, f64), to: (f64, f64)) -> qymcad::LiveLoop {
        let mut program = qymcad::LiveLoop::start();
        let (fx, fy, tx, ty) = (from.0, from.1, to.0, to.1);
        crate::checks::ask_the_core(&mut program, move |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
            crate::registry::call(core, "sketch.rect", &serde_json::json!({ "points": [[fx, fy], [tx, ty]] }));
            let finish = core.word("wb-finish");
            core.press_word_near(&finish, qymcad::pos2(0.0, 0.0));
            crate::registry::call(core, "part.extrude", &serde_json::json!({ "Length": "10" }));
        })
        .expect("the block was built");
        program
    }

    /// A BLOCK AT THE ORIGIN, 40 x 30 x 10.
    fn a_block() -> qymcad::LiveLoop {
        a_block_from((0.0, 0.0), (40.0, 30.0))
    }

    /// A FIRST PART STEPPED INTO, empty and standing in 3D: where a file comes in and a mesh lands
    /// pickable.
    fn an_empty_part() -> qymcad::LiveLoop {
        let mut program = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut program, |core| {
            core.key(qymcad::Key::Escape); // the start screen away, the toolbar's own new-part button in
            let new_part = core.word("tb-new-part-hint");
            core.press_hint(&new_part);
        })
        .expect("the part was entered");
        program
    }

    /// AN OPEN, EMPTY SKETCH on the XY plane of a first part.
    fn an_open_sketch() -> qymcad::LiveLoop {
        let mut program = qymcad::LiveLoop::start();
        crate::checks::ask_the_core(&mut program, |core| {
            let new_part = core.word("start-new-part");
            core.press_word(&new_part);
            let xy = core.word("plane-xy-table");
            core.press_word(&xy);
        })
        .expect("the sketch was opened");
        program
    }

    /// THE BIGGEST VOLUME OF THE BODIES THAT STAND.
    fn the_volume(program: &mut qymcad::LiveLoop) -> f64 {
        crate::checks::ask_the_core(program, |core| core.document().bodies.iter().map(|b| b.volume).fold(0.0, f64::max)).expect("the bodies were read")
    }

    /// A PARAMETER IS HELD at the expression a call carries, one of another follows it, and a dimension
    /// driven by a parameter follows it too: w = 50 stands at 50; d = w/2 comes to 25; a line measured
    /// by a dimension held at w is 50 long; w carried to 60 takes the line with it to 60. A formula
    /// that cannot be counted is the table's own words, and the parameter does not stand at it.
    #[test]
    fn a_parameter_is_held_follows_another_and_drives_a_dimension() {
        let _turn = take_your_turn();
        let mut program = an_open_sketch();
        let answered = call_it(&mut program, "parameters.set", serde_json::json!({ "name": "w", "expression": "50" }));
        assert!(matches!(answered, TheCall::Answered { .. }), "w = 50 was not held: {answered:?}");
        let answered = call_it(&mut program, "parameters.set", serde_json::json!({ "name": "d", "expression": "w/2" }));
        assert!(matches!(answered, TheCall::Answered { .. }), "d = w/2 was not held: {answered:?}");
        let held = crate::checks::ask_the_core(&mut program, |core| core.document().parameters).expect("the parameters were read");
        let of = |name: &str| held.iter().find(|p| p.name == name).cloned().unwrap_or_else(|| panic!("the parameter {name:?} does not stand: {held:?}"));
        assert!((of("w").value - 50.0).abs() < 1e-9, "w holds {}", of("w").value);
        assert!((of("d").value - 25.0).abs() < 1e-9, "d = w/2 holds {} over w = 50", of("d").value);
        // A DIMENSION HELD AT THE PARAMETER'S NAME: the line it measures follows the parameter
        call_it(&mut program, "sketch.line", serde_json::json!({ "points": [[0.0, 0.0], [40.0, 0.0]] }));
        let answered = call_it(&mut program, "sketch.dim", serde_json::json!({ "points": [[20.0, 0.0], [20.0, -8.0]], "value": "w" }));
        assert!(matches!(answered, TheCall::Answered { .. }), "the dimension held at w did not run: {answered:?}");
        let length = |program: &mut qymcad::LiveLoop| {
            crate::checks::ask_the_core(program, |core| match core.sketch_under(20.0, 0.0) {
                Some(qymcad::SketchPick::Line { from, to }) => (to.0 - from.0).hypot(to.1 - from.1),
                other => panic!("no line lies at (20, 0): {other:?}"),
            })
            .expect("the line was read")
        };
        assert!((length(&mut program) - 50.0).abs() < 1e-6, "the dimension held at w = 50 left the line at {}", length(&mut program));
        let answered = call_it(&mut program, "parameters.set", serde_json::json!({ "name": "w", "expression": "60" }));
        assert!(matches!(answered, TheCall::Answered { .. }), "w = 60 was not held: {answered:?}");
        assert!((length(&mut program) - 60.0).abs() < 1e-6, "carrying w to 60 left the line it drives at {}", length(&mut program));
        // A FORMULA THAT CANNOT BE COUNTED: the table keeps its row and says its own words under it -
        // the call is refused with what the table says, and what stood is not disturbed
        let answered = call_it(&mut program, "parameters.set", serde_json::json!({ "name": "bad", "expression": "sin(" }));
        assert!(matches!(answered, TheCall::Refused(_)), "a formula that cannot be counted was answered as held: {answered:?}");
        let still = crate::checks::ask_the_core(&mut program, |core| core.document().parameters).expect("the parameters were read");
        assert!(still.iter().any(|p| p.name == "w" && (p.value - 60.0).abs() < 1e-9), "the uncountable formula disturbed what stood: {still:?}");
    }

    /// THE VIEW OF A CALL: the fit takes the whole model in - a block drawn far off the screen comes to
    /// be seen; the orbit turns the view to the side its word names - from the top, the points of the
    /// Z axis project together, from the front the points of the Y axis do; the section cuts the view
    /// by the face its pick names, and the document is untouched. A side the viewcube carries no face
    /// for is refused in words that name the sides.
    #[test]
    fn the_view_fits_orbits_and_sections_as_a_call_carries() {
        let _turn = take_your_turn();
        // THE FIT: a block far out, the view zoomed in until it is gone, then fitted back
        let mut program = a_block_from((300.0, 0.0), (340.0, 30.0));
        let far = [320.0, 15.0, 10.0]; // on the top face of the block: a point inside a body always has a face in front of it
        let centre = crate::checks::ask_the_core(&mut program, |core| core.canvas().center()).expect("the canvas was read");
        let look = |program: &mut qymcad::LiveLoop, gone: bool| crate::checks::ask_the_core(program, move |core| core.sees(far) != gone).expect("the view was read");
        let mut gone = look(&mut program, true);
        for _ in 0..40 {
            if gone {
                break;
            }
            let at = centre;
            crate::checks::ask_the_core(&mut program, move |core| {
                core.wheel(at, qymcad::vec2(0.0, 100.0), qymcad::Modifiers::default());
            })
            .expect("the wheel turned");
            gone = look(&mut program, true);
        }
        assert!(gone, "zooming in over the origin never took the far block out of the view");
        let answered = call_it(&mut program, "view.fit", serde_json::json!({}));
        assert!(matches!(answered, TheCall::Answered { .. }), "the fit did not run: {answered:?}");
        let seen = look(&mut program, false);
        assert!(seen, "the fitted view does not take the far block in");
        // THE ORBIT: the view turned to the top and to the front by their words; the axis of the side
        // looked along projects to one place
        let mut program = a_block();
        let apart = |program: &mut qymcad::LiveLoop, a: [f64; 3], b: [f64; 3]| {
            crate::checks::ask_the_core(program, move |core| core.projected(a).distance(core.projected(b))).expect("the projections were read")
        };
        let (z_up, y_along) = (apart(&mut program, [0.0, 0.0, 1000.0], [0.0, 0.0, -1000.0]), apart(&mut program, [0.0, 1000.0, 0.0], [0.0, -1000.0, 0.0]));
        assert!(z_up > 1.0 && y_along > 1.0, "the standing view already looks straight down an axis: z {z_up}, y {y_along}");
        let answered = call_it(&mut program, "view.orbit", serde_json::json!({ "side": "top" }));
        assert!(matches!(answered, TheCall::Answered { .. }), "the turn to the top did not run: {answered:?}");
        assert!(apart(&mut program, [0.0, 0.0, 1000.0], [0.0, 0.0, -1000.0]) < 1.0, "from the top, the Z axis does not project to one place");
        let answered = call_it(&mut program, "view.orbit", serde_json::json!({ "side": "front" }));
        assert!(matches!(answered, TheCall::Answered { .. }), "the turn to the front did not run: {answered:?}");
        assert!(apart(&mut program, [0.0, 1000.0, 0.0], [0.0, -1000.0, 0.0]) < 1.0, "from the front, the Y axis does not project to one place");
        let refused = call_it(&mut program, "view.orbit", serde_json::json!({ "side": "diagonal" }));
        let TheCall::Refused(words) = &refused else { panic!("a side the cube carries no face for was not refused: {refused:?}") };
        assert!(words.contains("diagonal") && words.contains("top"), "the refusal does not name what was asked and what there is: {words}");
        // THE SECTION: the view cut by the top face of the block, the document untouched
        let mut program = a_block();
        let stood = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        let answered = call_it(&mut program, "assembly.section", serde_json::json!({ "picks": [[20.0, 15.0, 10.0]] }));
        assert!(matches!(answered, TheCall::Answered { .. }), "the section did not run: {answered:?}");
        let now = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        assert!(now.section, "the view is not cut by a section");
        assert_eq!(now.bodies, stood.bodies, "the section touched the document");
    }

    /// THE MEASURE OF A CALL: the two corners of the block's front top edge are 40 apart, and the
    /// status line says so in the program's own words; picks that name nothing measure nothing, and
    /// the call is refused with what the status line says.
    #[test]
    fn the_measure_reads_what_the_picks_name() {
        let _turn = take_your_turn();
        let mut program = a_block();
        let answered = call_it(&mut program, "part.measure", serde_json::json!({ "picks": [[0.0, 0.0, 10.0], [40.0, 0.0, 10.0]] }));
        let TheCall::Answered { outcome, .. } = &answered else { panic!("the measure did not run: {answered:?}") };
        assert!(outcome.contains("40"), "the measured words do not say 40: {outcome:?}");
        let stood = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        let refused = call_it(&mut program, "part.measure", serde_json::json!({ "picks": [[0.0, 0.0, 10.0], [3000.0, 3000.0, 3000.0]] }));
        let TheCall::Refused(words) = &refused else { panic!("a measure over nothing at all was not refused: {refused:?}") };
        assert!(!words.trim().is_empty(), "the refused measure said nothing");
        let now = crate::checks::ask_the_core(&mut program, |core| core.document()).expect("the document was read");
        assert_eq!(now, stood, "a measure changed the document");
    }

    /// THE MODEL FORMATS OF THE CHECK, each with the extension it writes and the bytes its file must
    /// open with (the STL is binary and carries no magic: what it holds is read by its coming back).
    const MODEL_FILES: &[(&str, &str, &[u8])] = &[
        ("step", "step", b"ISO-10303"),
        ("iges", "igs", b"S"),
        ("stl", "stl", b""),
        ("obj", "obj", b"o body_"),
        ("ply", "ply", b"ply"),
        ("glb", "glb", b"glTF"),
        ("3mf", "3mf", b"PK"),
        ("amf", "amf", b"<?xml"),
    ];

    /// EVERY FORMAT THE PROGRAM OFFERS goes out by a call and comes back by one: the eight model
    /// formats written from a block, each opening as what it is, and each read back into a fresh part
    /// as a body of the block's own 12000; the drawing formats written from the block's own sketch and
    /// read back as a sketch of the rectangle it holds, four lines of 40 by 30. A format the program
    /// writes no model in is refused in words that name them, a file that is not there is the
    /// program's own refusal, and nothing is written by either.
    #[test]
    fn every_format_the_program_offers_goes_out_and_comes_back() {
        let _turn = take_your_turn();
        let dir = std::env::temp_dir().join(format!("qymcad-mcp-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("the folder for the files");
        let mut program = a_block();
        for (format, ext, magic) in MODEL_FILES {
            let path = dir.join(format!("by-the-agent.{ext}"));
            let _ = std::fs::remove_file(&path);
            let path_s = path.to_string_lossy().into_owned();
            let answered = call_it(&mut program, "file.export", serde_json::json!({ "format": format, "path": path_s }));
            assert!(matches!(answered, TheCall::Answered { .. }), "{format} did not go out: {answered:?}");
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{format} was not written: {e}"));
            assert!(bytes.len() > 100, "the {format} file holds almost nothing: {} bytes", bytes.len());
            // the IGES header is a padded column: its mark is the word IGES in it, not its first bytes
            let opens = if *format == "iges" { String::from_utf8_lossy(&bytes).contains("IGES") } else { bytes.starts_with(magic) };
            assert!(opens, "the {format} file does not open as {format}: {:?}", String::from_utf8_lossy(&bytes[..bytes.len().min(40)]));
            // AND IT COMES BACK: a fresh part, the import, a body of the block's own volume
            let mut back = an_empty_part();
            let answered = call_it(&mut back, "file.import", serde_json::json!({ "path": path.to_string_lossy() }));
            assert!(matches!(answered, TheCall::Answered { .. }), "{format} did not come back: {answered:?}");
            let volume = the_volume(&mut back);
            assert!((volume - 12000.0).abs() < 1.0, "the block came back from {format} holding {volume}, not 12000");
        }
        // THE DRAWING FORMATS: the block's own sketch drawn out, read back as the rectangle it holds
        let sketch = crate::checks::ask_the_core(&mut program, |core| core.document().sketches[0].name.clone()).expect("the sketch was read");
        for (format, magic) in [("svg", "<svg"), ("dxf", "SECTION")] {
            let path = dir.join(format!("by-the-agent.{format}"));
            let _ = std::fs::remove_file(&path);
            let answered = call_it(&mut program, "file.export-sketch", serde_json::json!({ "sketch": sketch, "format": format, "path": path.to_string_lossy() }));
            assert!(matches!(answered, TheCall::Answered { .. }), "{format} did not go out: {answered:?}");
            let said = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("the {format} drawing was not written: {e}"));
            assert!(said.contains(magic), "the {format} file does not open as {format}: {:?}", &said[..said.len().min(40)]);
            let mut back = an_empty_part();
            let answered = call_it(&mut back, "file.import", serde_json::json!({ "path": path.to_string_lossy() }));
            assert!(matches!(answered, TheCall::Answered { .. }), "{format} did not come back: {answered:?}");
            let drawn = crate::checks::ask_the_core(&mut back, |core| core.document().sketches.last().cloned()).expect("the sketches were read");
            let drawn = drawn.unwrap_or_else(|| panic!("the {format} drawing came in as no sketch"));
            assert_eq!(drawn.lines, 4, "the rectangle came back from {format} as {} lines", drawn.lines);
            let size = [drawn.max[0] - drawn.min[0], drawn.max[1] - drawn.min[1]];
            assert!((size[0] - 40.0).abs() < 0.01 && (size[1] - 30.0).abs() < 0.01, "the rectangle of 40 by 30 came back from {format} as {size:?}");
        }
        // THE REFUSALS: a format the program writes no model in, and a file that is not there
        let nowhere = dir.join("nowhere.step");
        let _ = std::fs::remove_file(&nowhere);
        let refused = call_it(&mut program, "file.export", serde_json::json!({ "format": "docx", "path": dir.join("refused.docx").to_string_lossy() }));
        let TheCall::Refused(words) = &refused else { panic!("a format the program writes no model in was not refused: {refused:?}") };
        assert!(words.contains("docx") && words.contains("step") && words.contains("amf"), "the refusal does not name what was asked and what there is: {words}");
        assert!(!dir.join("refused.docx").exists(), "the refused export wrote a file");
        let refused = call_it(&mut program, "file.import", serde_json::json!({ "path": nowhere.to_string_lossy() }));
        let TheCall::Refused(words) = &refused else { panic!("a file that is not there was not refused: {refused:?}") };
        assert!(!words.trim().is_empty(), "the refused import said nothing");
    }
}
