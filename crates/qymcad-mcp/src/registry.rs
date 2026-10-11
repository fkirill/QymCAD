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
/// same interface - told its own code and the parameters the call carried. The answer is the operation's
/// outcome in words.
pub(crate) type Run = fn(&mut qymcad::AgentSession, &str, &serde_json::Value) -> String;

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

/// THE REGISTRY, derived from the command catalogue: every command of the program, in the panel's order.
pub(crate) fn the_registry() -> &'static [Entry] {
    static THE_REGISTRY: std::sync::OnceLock<Vec<Entry>> = std::sync::OnceLock::new();
    THE_REGISTRY.get_or_init(|| {
        qymcad::COMMANDS.iter().map(|command| Entry { command, synonyms: SYNONYMS.iter().find(|(code, _)| *code == command.code).map(|(_, words)| *words).unwrap_or(&[]), run: None }).collect()
    })
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
    put_up
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
}
