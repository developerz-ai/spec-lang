//! The `game` library (`libs/game`) and `foreign` bodies resolve, and their
//! wrong forms are refused with a code.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_ir::*;

fn load(src: &str) -> Result<Program, Vec<IrError>> {
    load_sources(vec![("m".into(), "m.slg".into(), src.to_string())]).map_err(|f| f.errors)
}

fn program(src: &str) -> Program {
    match load(src) {
        Ok(p) => p,
        Err(e) => panic!("{e:?}"),
    }
}

fn first_code(src: &str) -> &'static str {
    match load(src) {
        Ok(_) => panic!("loaded: {src}"),
        Err(e) => e[0].code,
    }
}

const GAME: &str = "use game;\n\
    type Pad(jump: Bool = false);\n\
    state n: Int = 0;\n\
    rule start(level: Int) = { n = level; out.line(\"start\"); }\n\
    @on(tick(60Hz))\nrule step(pad: Pad) = { n = n + 1; }\n\
    @uses(keys.read)\nfn key_down(key: Int) -> Bool = foreign c \"raylib#IsKeyDown\";\n\
    @uses(keys.read)\nfn read_pad() -> Pad = Pad(jump: key_down(32));\n\
    @uses(screen.write)\nrule draw(x: Int) = foreign c \"raylib#DrawText\";\n\
    rule present() = { draw(n); }\n\
    fn dump() -> List[Text] = [\"{n}\"];\n\
    fn summary() -> Text = \"end {n}\";\n";

const APP: &str = "@target(rust)\n@bin(\"g\")\n@window(960, 540, \"G\")\n\
    app G(start: start, tick: step, input: read_pad, frame: present, dump: dump, report: summary);\n";

#[test]
fn a_game_app_names_its_six_entries_and_its_window() {
    let p = program(&format!("{GAME}{APP}"));
    let app = &p.apps[0];
    assert_eq!(app.shape.as_deref(), Some("game"));
    let name = |field: &str| p.info(app.field(field).unwrap()).name.clone();
    let names: Vec<String> = ["start", "tick", "input", "frame", "dump", "report"].map(name).to_vec();
    assert_eq!(names, ["start", "step", "read_pad", "present", "dump", "summary"]);
    assert!(matches!(app.field("input"), Some(DeclRef::Fn(_))));
    assert_eq!(app.note("window").unwrap().to_string(), "window(960, 540, \"G\")");
    assert_eq!(app.note("window").unwrap().arg("height"), Some(&NoteValue::Int(540)));
    assert_eq!(app.note("bin").unwrap().text("name"), Some("g"));
    assert!(app.rules.is_empty());
    assert_eq!(app.named().len(), 6);
    let Some(DeclRef::Rule(tick)) = app.field("tick") else { panic!("tick is a rule") };
    let t = &p.rule(tick).triggers;
    assert_eq!((t[0].library.as_str(), t[0].to_string()), ("game", "tick(60Hz)".to_string()));
}

#[test]
fn foreign_bodies_keep_their_language_and_symbol() {
    let p = program(GAME);
    let f = p.fns.iter().find(|f| f.info.name == "key_down").unwrap();
    let foreign = f.foreign.as_ref().unwrap();
    assert_eq!((foreign.lang.as_str(), foreign.symbol.as_str()), ("c", "raylib#IsKeyDown"));
    assert_eq!(p.snippet(foreign.span), "c \"raylib#IsKeyDown\"");
    assert!(matches!(&f.body.kind, ExprKind::Hole(h) if h == "foreign"));
    assert_eq!(f.uses, ["keys.read"]);
    let r = p.rules.iter().find(|r| r.info.name == "draw").unwrap();
    assert_eq!(r.foreign.as_ref().unwrap().to_string(), "foreign c \"raylib#DrawText\"");
    assert!(p.fns.iter().find(|f| f.info.name == "read_pad").unwrap().foreign.is_none());
}

#[test]
fn a_rate_is_a_whole_number_of_hz() {
    let rule = |trigger: &str| format!("use game;\nstate n: Int = 0;\n@on({trigger})\nrule step() = {{ n = 1; }}\n");
    let p = program(&rule("tick(30Hz)"));
    assert_eq!(p.rules[0].triggers[0].arg("rate"), Some(&NoteValue::Num { value: 30, unit: "Hz".into() }));
    assert_eq!(first_code(&rule("tick(5s)")), "E-NOTE-ARG");
    assert_eq!(first_code(&rule("tick(60)")), "E-NOTE-ARG");
    assert_eq!(first_code(&rule("tick(0Hz)")), "E-NOTE-ARG");
    assert_eq!(first_code(&rule("tick(\"x\")")), "E-NOTE-ARG");
    // Without `use game;`, `tick` is no trigger.
    assert_eq!(first_code(&rule("tick(60Hz)").replace("use game;\n", "")), "E-UNSUPPORTED");
}

#[test]
fn a_game_app_refuses_a_wrong_kind_a_missing_entry_and_a_tick_without_trigger() {
    let swap = |from: &str, to: &str| format!("{GAME}{}", APP.replace(from, to));
    assert_eq!(first_code(&swap("input: read_pad", "input: present")), "E-NAME-KIND");
    assert_eq!(first_code(&swap("frame: present", "frame: dump")), "E-NAME-KIND");
    assert_eq!(first_code(&swap(", report: summary", "")), "E-TYPE-ARITY");
    assert_eq!(first_code(&swap("report: summary", "rules: [start]")), "E-TYPE-ARITY");
    assert_eq!(first_code(&swap("dump: dump", "dump: nothing")), "E-NAME-UNKNOWN");
    assert_eq!(first_code(&swap("tick: step", "tick: present")), "E-APP-TRIGGER");
    assert_eq!(first_code(&swap("@window(960, 540, \"G\")", "@window(960, \"G\")")), "E-NOTE-ARG");
    assert_eq!(first_code(&swap("@window(960, 540, \"G\")", "@store(file(\"x\"))")), "E-NOTE-UNKNOWN");
}

#[test]
fn world_effects_and_window_come_from_use_game() {
    // `game` is loaded by another module, but this one does not use it.
    let two = vec![
        ("a".into(), "a.slg".into(), "use game;\n".into()),
        ("m".into(), "m.slg".into(), "@uses(keys.read)\nfn k() -> Bool = foreign c \"raylib#IsKeyDown\";\n".into()),
    ];
    assert_eq!(load_sources(two).unwrap_err().errors[0].code, "E-NAME-UNKNOWN");
    // Nothing loads `game`: `keys.read` is no effect the program knows; the check stage refuses it.
    assert!(load("@uses(keys.read)\nfn k() -> Bool = foreign c \"raylib#IsKeyDown\";\n").is_ok());
    assert_eq!(first_code("use cli;\n@window(1, 1, \"x\")\napp A(rules: []);\n"), "E-NOTE-UNKNOWN");
    // With both, the app has game's shape, and `@bin` names two notes.
    assert_eq!(first_code("use cli;\nuse game;\napp A(rules: []);\n"), "E-TYPE-ARITY");
    assert_eq!(first_code(&format!("use cli;\n{GAME}{APP}")), "E-NOTE-AMBIG");
    assert_eq!(first_code("fn f() -> Int = foreign c r\"raylib#X\";\n"), "E-UNSUPPORTED");
}
