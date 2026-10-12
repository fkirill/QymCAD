//! THE UI-DRIVER ESCAPE HATCH: the person's whole vocabulary over the agent's session - pressing a
//! button by the word its plate shows, clicking a place of the sketch or of the space of the model,
//! filling a field, walking a menu, pressing a key, typing. Whatever the program offers but no named
//! operation of the registry serves yet is still reachable through these, by the same gestures a
//! person makes; the set is exhaustive by construction, not by enumeration.
//!
//! The words the calls carry are the program's own, as the window shows them: a button by the word
//! its plate shows while the pointer rests on it, a field by its caption, a menu by the words of its
//! items. A gesture that finds nothing is refused in words naming what was looked for, and the
//! document stands as it stood.
use crate::general::General;
use crate::registry::{the_fields_on_screen, the_names_of, the_words_of, Answered, Take};

/// THE PERSON'S GESTURES, as the agent calls them by name: the escape hatch over the whole
/// interface. Each is named by the word of the dictionary and answered by the article about the
/// agent, where its vocabulary is told of.
pub(crate) const THE_PERSON_S_GESTURES: &[General] = &[
    General { code: "ui.button", article: "general/15-agent", name_key: "mcp-ui-button", synonyms: &["toolbar", "plate"], run: press_a_button, takes: &[Take { name: "hint", kind: "text" }] },
    General { code: "ui.click", article: "general/15-agent", name_key: "mcp-ui-click", synonyms: &["point", "tap"], run: click_a_place, takes: &[Take { name: "at", kind: "place" }] },
    General {
        code: "ui.field",
        article: "general/15-agent",
        name_key: "mcp-ui-field",
        synonyms: &["input", "value"],
        run: fill_a_field,
        takes: &[Take { name: "label", kind: "text" }, Take { name: "text", kind: "text" }],
    },
    General { code: "ui.menu", article: "general/15-agent", name_key: "mcp-ui-menu", synonyms: &["bar", "item"], run: walk_a_menu, takes: &[Take { name: "path", kind: "words" }] },
    General { code: "ui.key", article: "general/15-agent", name_key: "mcp-ui-key", synonyms: &["enter", "escape", "shortcut"], run: press_a_key, takes: &[Take { name: "key", kind: "text" }] },
    General { code: "ui.type", article: "general/15-agent", name_key: "mcp-ui-type", synonyms: &["keyboard"], run: type_words, takes: &[Take { name: "text", kind: "text" }] },
];

/// PRESS A BUTTON by the word its own plate shows while the pointer rests on it - the toolbar's and
/// the panel's buttons, as a person takes them. The word is the program's own, in the language it
/// speaks; a word no button shows is refused in words, and nothing is pressed.
fn press_a_button(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(hint) = the_words_of(given, "hint") else { return Answered::Refused("the press needs the word the button's own plate shows while the pointer rests on it".into()) };
    // THE WORD AS THE WINDOW SHOWS IT: a word that names a key of the dictionary is said as the
    // window says it, so a client that knows the key reaches the button all the same
    let shown = if core.find_hint(&hint).is_some() { hint.clone() } else { core.word(&hint) };
    let Some(at) = core.find_hint(&shown) else {
        return Answered::Refused(format!("{hint}: no button of the window shows this word on its plate; the words are the program's own, as the window shows them"));
    };
    core.click(at);
    Answered::Ran(format!("the button of {shown} pressed"))
}

/// ONE PLACE A CLICK CAN GO: a pair of the sheet of the open sketch, or a triple of the space of
/// the model.
pub(crate) enum Place {
    /// A place of the sheet of the open sketch, in the sketch's own coordinates.
    Sheet(f64, f64),
    /// A place in the space of the model, naming what stands there.
    Space([f64; 3]),
}

/// THE PLACE A VALUE HOLDS: a pair [x, y] of the sheet or a triple [x, y, z] of the space; `None`
/// at anything else.
pub(crate) fn the_place_of(value: &serde_json::Value) -> Option<Place> {
    let numbers: Vec<f64> = value.as_array()?.iter().map(|v| v.as_f64()).collect::<Option<_>>()?;
    match numbers.as_slice() {
        [x, y] => Some(Place::Sheet(*x, *y)),
        [x, y, z] => Some(Place::Space([*x, *y, *z])),
        _ => None,
    }
}

/// CLICK A PLACE - a pair [x, y] of the sheet of the open sketch, or a triple [x, y, z] of the
/// space of the model, each naming what stands there. A pair with no sketch open is refused in
/// words: the sheet belongs to an open sketch, and the space of the model is the other form.
fn click_a_place(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(at) = given.get("at").and_then(the_place_of) else {
        return Answered::Refused("the click needs its place: a pair [x, y] of the sheet of the open sketch, or a triple [x, y, z] of the space of the model".into());
    };
    match at {
        Place::Sheet(x, y) => {
            if core.document().editing.is_none() {
                return Answered::Refused("no sketch is open - a place of the sheet needs one; a place of the space of the model goes as [x, y, z]".into());
            }
            core.click_on_sketch(x, y);
            Answered::Ran(format!("the place ({}, {}) of the sheet clicked", x, y))
        }
        Place::Space(p) => {
            let at = core.in_space(p);
            core.click(at);
            Answered::Ran(format!("the place ({}, {}, {}) of the space clicked", p[0], p[1], p[2]))
        }
    }
}

/// FILL A FIELD by its own caption - the words painted beside it or the grey words it shows while
/// empty - with the text the call carries, as a person types into it. A caption no field reads by is
/// refused in words naming the fields that stand.
fn fill_a_field(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(label) = the_words_of(given, "label") else { return Answered::Refused("the fill needs the caption the field reads by".into()) };
    let Some(text) = the_words_of(given, "text") else { return Answered::Refused("the fill needs the text it fills with".into()) };
    let filled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        core.fill(&label, &text);
    }));
    match filled {
        Ok(()) => Answered::Ran(format!("{} filled with {:?}", label, text)),
        Err(_) => {
            let standing: Vec<String> = the_fields_on_screen(core).into_iter().map(|f| format!("{} ({})", f.name, f.kind)).collect();
            let standing = if standing.is_empty() { "none".to_string() } else { standing.join(", ") };
            Answered::Refused(format!("{label}: no field of the window reads so; the fields that stand: {standing}"))
        }
    }
}

/// WALK A MENU by the words of its items, in the order they are walked - the menu bar and the menus
/// under it, as a person walks them. An item that names nothing is refused in words naming the item
/// and how far the walk came, and the document stands as it stood.
fn walk_a_menu(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(path) = the_names_of(given.get("path")) else { return Answered::Refused("the walk needs the words of the items, in the order they are walked".into()) };
    if path.is_empty() {
        return Answered::Refused("the walk needs at least one word of an item".into());
    }
    let mut near = qymcad::pos2(0.0, 0.0);
    for (walked, item) in path.iter().enumerate() {
        // an item that names what it acts on reads "Undo: Extrusion" - it is still the item "Undo"
        let lead = format!("{item}:");
        let found = core
            .find(item, near)
            .or_else(|| core.words_at().into_iter().filter(|(t, _)| t.starts_with(&lead)).map(|(_, r)| r).min_by(|a, b| a.center().distance(near).total_cmp(&b.center().distance(near))));
        let Some(rect) = found else {
            return Answered::Refused(format!("{item}: no item of the walk says so; the path walked {} of {} words", walked, path.len()));
        };
        core.click(rect.center());
        near = rect.center();
    }
    Answered::Ran(format!("the menu walked by its {} words", path.len()))
}

/// THE KEYS A NAME MAY CARRY, said as the call carries them: the name of the key itself - enter,
/// escape, delete, a letter, a digit, an arrow, an F-key - with the modifiers it is held with, in
/// the person's own spelling: `ctrl+enter`, `shift+a`, `ctrl+shift+s`.
fn the_key_and_its_modifiers(said: &str) -> Option<(qymcad::Modifiers, qymcad::Key)> {
    let mut modifiers = qymcad::Modifiers::default();
    let mut name = said.trim();
    while let Some((held, rest)) = name.split_once('+') {
        match held.trim().to_lowercase().as_str() {
            "ctrl" | "control" | "cmd" | "command" => modifiers |= qymcad::Modifiers::COMMAND,
            "shift" => modifiers |= qymcad::Modifiers::SHIFT,
            "alt" | "option" => modifiers |= qymcad::Modifiers::ALT,
            _ => return None,
        }
        name = rest;
    }
    Some((modifiers, the_key_named(name.trim())?))
}

/// THE KEY A LOWER-CASE NAME CARRIES, as a person says it: the commands (enter, escape, the
/// arrows), the letters, the digits, the F-keys. The spelling is the person's own lower case; the
/// keyboard's own names are matched here and nowhere else.
fn the_key_named(said: &str) -> Option<qymcad::Key> {
    use qymcad::Key;
    let name = said.trim().to_lowercase();
    let letters = || "abcdefghijklmnopqrstuvwxyz".chars().position(|c| Some(c) == name.chars().next());
    let digits = || "0123456789".find(&name);
    let f_keys = || name.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()).filter(|n| (1..=12).contains(n));
    Some(match name.as_str() {
        "enter" | "return" => Key::Enter,
        "escape" | "esc" => Key::Escape,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "insert" => Key::Insert,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "space" => Key::Space,
        "up" | "arrow-up" => Key::ArrowUp,
        "down" | "arrow-down" => Key::ArrowDown,
        "left" | "arrow-left" => Key::ArrowLeft,
        "right" | "arrow-right" => Key::ArrowRight,
        "minus" | "-" => Key::Minus,
        "plus" => Key::Plus,
        "equals" | "=" => Key::Equals,
        "comma" | "," => Key::Comma,
        "period" | "." => Key::Period,
        "slash" | "/" => Key::Slash,
        _ if name.len() == 1 && letters().is_some() => [
            Key::A,
            Key::B,
            Key::C,
            Key::D,
            Key::E,
            Key::F,
            Key::G,
            Key::H,
            Key::I,
            Key::J,
            Key::K,
            Key::L,
            Key::M,
            Key::N,
            Key::O,
            Key::P,
            Key::Q,
            Key::R,
            Key::S,
            Key::T,
            Key::U,
            Key::V,
            Key::W,
            Key::X,
            Key::Y,
            Key::Z,
        ][letters()?],
        _ if digits().is_some() => [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9][digits()?],
        _ if f_keys().is_some() => [Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6, Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12][(f_keys()? - 1) as usize],
        _ => return None,
    })
}

/// PRESS A KEY, as the person's own fingers do - alone or held with its modifiers, said in the
/// person's own spelling: `enter`, `escape`, `ctrl+enter`, `ctrl+k`. What the key does is what a
/// person gets; a name no key of the keyboard carries is refused in words.
fn press_a_key(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(said) = the_words_of(given, "key") else {
        return Answered::Refused("the press needs the key it presses: enter, escape, delete, a letter, a digit, an arrow - alone, or held with ctrl, shift, alt as ctrl+enter".into());
    };
    let Some((modifiers, key)) = the_key_and_its_modifiers(&said) else {
        return Answered::Refused(format!("{said}: no key of the keyboard is named so; the keys are enter, escape, tab, backspace, delete, space, the arrows, the letters, the digits and the F-keys, held with ctrl, shift or alt as ctrl+enter"));
    };
    core.chord(modifiers, key);
    Answered::Ran(format!("the key {said} pressed"))
}

/// TYPE the words the call carries, into whatever holds the keyboard - the field a person is typing
/// into, the search a person is searching by.
fn type_words(core: &mut qymcad::AgentSession, _code: &str, given: &serde_json::Value) -> Answered {
    let Some(text) = the_words_of(given, "text") else { return Answered::Refused("the typing needs the words it types".into()) };
    core.type_text(&text);
    Answered::Ran(format!("{:?} typed", text))
}

#[cfg(test)]
mod tests {
    use crate::checks::take_your_turn;
    use crate::registry::{the_fields_on_screen, TheCall};

    /// ONE CALL, answered.
    fn call_it(program: &mut qymcad::LiveLoop, code: &str, given: serde_json::Value) -> TheCall {
        let code = code.to_string();
        crate::checks::ask_the_core(program, move |core| crate::registry::call(core, &code, &given)).expect("the call answered")
    }

    /// ONE CALL THAT MUST RUN, its outcome said back.
    fn run_it(program: &mut qymcad::LiveLoop, code: &str, given: serde_json::Value) -> String {
        match call_it(program, code, given) {
            TheCall::Answered { outcome, .. } => outcome,
            refused => panic!("{code} was refused: {refused:?}"),
        }
    }

    /// WHAT THE WINDOW SHOWS, between the calls: the word of a button, the caption of a field - read
    /// as the window shows it, in the language it speaks. The reading changes nothing; only the
    /// calls do the work.
    fn ask_the_window<T: Send + 'static>(program: &mut qymcad::LiveLoop, read: impl FnOnce(&mut qymcad::AgentSession) -> T + Send + 'static) -> T {
        crate::checks::ask_the_core(program, read).expect("the window was read")
    }

    /// THE WHOLE OF A SMALL PART - a rectangle drawn, finished, extruded - made with NO named entry:
    /// only the person's own gestures over the window. The start screen goes with Esc, the menu is
    /// walked by its words, the buttons are pressed by the words of their plates, the places are
    /// clicked by where they lie, the command is taken through the search a person types into, the
    /// field is filled by its caption, and Enter applies. What stands after it is a body of the
    /// rectangle's own 40 x 30 x 10 - counted, not assumed.
    #[test]
    fn a_part_is_made_with_no_named_entry_through_the_gestures_alone() {
        let _turn = take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        run_it(&mut program, "ui.key", serde_json::json!({ "key": "escape" }));
        let (view, fit) = ask_the_window(&mut program, |core| (core.word("menu-view"), core.word("menu-fit-view")));
        run_it(&mut program, "ui.menu", serde_json::json!({ "path": [view, fit] }));
        let new_part = ask_the_window(&mut program, |core| core.word("tb-new-part-hint"));
        run_it(&mut program, "ui.button", serde_json::json!({ "hint": new_part }));
        let pencil = ask_the_window(&mut program, |core| core.word("g-sketch-pick-hint"));
        run_it(&mut program, "ui.button", serde_json::json!({ "hint": pencil }));
        run_it(&mut program, "ui.click", serde_json::json!({ "at": [0.0, 0.0, 0.0] }));
        let rect = ask_the_window(&mut program, |core| core.word("tb-rect-hint"));
        run_it(&mut program, "ui.button", serde_json::json!({ "hint": rect }));
        run_it(&mut program, "ui.click", serde_json::json!({ "at": [0.0, 0.0] }));
        run_it(&mut program, "ui.click", serde_json::json!({ "at": [40.0, 30.0] }));
        run_it(&mut program, "ui.key", serde_json::json!({ "key": "escape" }));
        run_it(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+enter" }));
        // THE COMMAND TAKEN THROUGH THE SEARCH a person types into: Ctrl+K, the name, Enter. The
        // fields the take put up - and not the standing bar - are read by what stood before it
        let before = ask_the_window(&mut program, |core| the_fields_on_screen(core).into_iter().map(|f| (f.name, f.kind)).collect::<Vec<_>>());
        let extrude = ask_the_window(&mut program, |_core| qymcad::COMMANDS.iter().find(|c| c.code == "part.extrude").expect("the extrude is in the catalogue").name());
        run_it(&mut program, "ui.key", serde_json::json!({ "key": "ctrl+k" }));
        run_it(&mut program, "ui.type", serde_json::json!({ "text": extrude }));
        run_it(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        let label = ask_the_window(&mut program, move |core| {
            let put_up: Vec<String> = the_fields_on_screen(core).into_iter().filter(|f| !before.iter().any(|b| b == &(f.name.clone(), f.kind.clone()))).map(|f| f.name).collect();
            put_up.first().expect("the extrude's bar put a field of its own up").clone()
        });
        run_it(&mut program, "ui.field", serde_json::json!({ "label": label, "text": "10" }));
        run_it(&mut program, "ui.key", serde_json::json!({ "key": "enter" }));
        let (volume, problems) = ask_the_window(&mut program, |core| {
            let document = core.document();
            (document.bodies.iter().map(|b| b.volume).fold(0.0, f64::max), core.the_standing_problems())
        });
        assert!((volume - 12000.0).abs() < 1.0, "the part made by the gestures alone holds {volume}, not the 12000 of a 40 x 30 x 10 block");
        assert!(problems.is_empty(), "the part made by the gestures alone left problems: {problems:?}");
    }

    /// EVERY GESTURE REFUSES WHAT IS NOT THERE, in words naming what was looked for, and the
    /// document stands as it stood: a button no plate shows, a place of the sheet with no sketch
    /// open, a caption no field reads by, a menu item no menu says, a key no keyboard carries.
    #[test]
    fn the_gestures_refuse_what_is_not_there_in_words() {
        let _turn = take_your_turn();
        let mut program = qymcad::LiveLoop::start();
        run_it(&mut program, "ui.key", serde_json::json!({ "key": "escape" }));
        let stood = ask_the_window(&mut program, |core| core.document());
        for (code, given, named) in [
            ("ui.button", serde_json::json!({ "hint": "a word no plate shows" }), "a word no plate shows"),
            ("ui.click", serde_json::json!({ "at": [5.0, 5.0] }), "no sketch is open"),
            ("ui.field", serde_json::json!({ "label": "A caption of nothing", "text": "1" }), "A caption of nothing"),
            ("ui.menu", serde_json::json!({ "path": ["A menu of nothing"] }), "A menu of nothing"),
            ("ui.key", serde_json::json!({ "key": "not-a-key" }), "not-a-key"),
        ] {
            match call_it(&mut program, code, given) {
                TheCall::Refused(words) => assert!(words.contains(named), "{code} did not name what was looked for: {words}"),
                answered => panic!("{code} was not refused: {answered:?}"),
            }
            let now = ask_the_window(&mut program, |core| core.document());
            assert_eq!(now, stood, "the refused {code} changed the document");
        }
    }
}
