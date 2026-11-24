//! Small programs through the whole backend: what a declaration becomes, and
//! what the backend refuses with a diagnostic instead of guessing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use spec_lang_ir::load_sources;

const APP: &str = r#"
use cli;
use shop.core;

@target(rust)
@bin("shop")
@store(file(".shop.json"))
app Shop(rules: [core.stock, core.take, core.count]);
"#;

fn emit(core: &str) -> Result<Vec<spec_lang_emit_rust::OutFile>, Vec<spec_lang_diag::Diag>> {
    let prog = load_sources(vec![
        ("shop.core".into(), "src/shop/core.slg".into(), core.into()),
        ("apps.rust".into(), "src/apps/rust.slg".into(), APP.into()),
    ])
    .unwrap_or_else(|f| panic!("{}", f.render()));
    let app = prog.apps.iter().find(|a| a.info.name == "Shop").unwrap();
    spec_lang_emit_rust::emit(&prog, app)
}

fn file<'a>(files: &'a [spec_lang_emit_rust::OutFile], path: &str) -> &'a str {
    let f = files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>()));
    std::str::from_utf8(&f.bytes).unwrap()
}

const CORE: &str = r#"
use cli;

type Sku = Int where self >= 1;
type Item(sku: Sku, left: Int);

state items: Map[Sku, Item];

@on(cli("stock"))
@uses(items.write)
pub rule stock(sku: Sku, n: Int) = {
  items[sku] = Item(sku, left: n);
}

@on(cli("take"))
@uses(items.write, out.write)
@guard(sku in items else NoItem, items[sku].left >= 1 else Empty)
pub rule take(sku: Sku) = {
  items[sku].left = items[sku].left - 1;
  out.line("took {sku}");
}

@on(cli("count"))
@uses(items.read, out.write)
pub rule count() = {
  out.line("{items.size} items");
}
"#;

#[test]
fn a_guarded_rule_reads_its_entry_once() {
    let files = emit(CORE).unwrap_or_else(|ds| panic!("{ds:?}"));
    let take = file(&files, "src/core/take.rs");
    assert!(take.contains("let item = world.items.get_mut(&sku).ok_or(Refusal::NoItem)?;"), "{take}");
    assert!(take.contains("if item.left < 1 {\n        return Err(Refusal::Empty);"), "{take}");
    assert!(take.contains("// slg: src/shop/core.slg:17 guard[1]"), "{take}");
    assert!(take.contains("item.left -= 1;"), "{take}");
    let count = file(&files, "src/core/count.rs");
    assert!(count.contains("pub fn count(world: &CountView, out: &mut Out)"), "{count}");
    assert!(file(&files, "src/store/codec.rs").contains("pub fn decode_count("));
}

#[test]
fn the_store_schema_follows_the_states() {
    let files = emit(CORE).unwrap_or_else(|ds| panic!("{ds:?}"));
    let codec = file(&files, "src/store/codec.rs");
    // `items` is keyed by `Item.sku`: stored as the array of its values.
    assert!(codec.contains("match map.insert(value.sku, value)"), "{codec}");
    assert!(codec.contains("Sku::new(cursor.integer()?).ok_or(Unreadable::Claim(\"Sku\"))"), "{codec}");
}

#[test]
fn the_help_comes_from_the_signatures() {
    let files = emit(CORE).unwrap_or_else(|ds| panic!("{ds:?}"));
    let table = file(&files, "src/cli/table.rs");
    assert!(
        table.contains("usage: shop <command> [args]\\n\\ncommands:\\n  stock <sku> <n>\\n  take <sku>\\n  count\\n"),
        "{table}"
    );
    let main = file(&files, "src/main.rs");
    assert!(main.contains("env::var_os(\"SHOP_STORE\"), \".shop.json\""), "{main}");
}

#[test]
fn an_unshown_claim_is_refused_not_guessed() {
    // `n` is any Int; `Sku` needs >= 1. Nothing shows it, and a rule has no failure for it.
    let core =
        CORE.replace("items[sku] = Item(sku, left: n);", "items[sku] = Item(sku, left: n);\n  items[sku].sku = n;");
    let ds = emit(&core).expect_err("refused");
    assert!(ds.iter().any(|d| d.code == "E-UNSUPPORTED" && d.decl.as_deref() == Some("shop.core.stock")), "{ds:?}");
}

#[test]
fn an_undeclared_write_is_an_error() {
    let core = CORE.replace("@uses(items.write)\npub rule stock", "@uses(out.write)\npub rule stock");
    let ds = emit(&core).expect_err("refused");
    let d = ds.iter().find(|d| d.code == "E-EFFECT-UNDECLARED").unwrap_or_else(|| panic!("{ds:?}"));
    assert_eq!(d.decl.as_deref(), Some("shop.core.stock"));
    assert_eq!(d.found.as_deref(), Some("@uses(out.write)"));
}

#[test]
fn a_map_read_without_a_guard_has_no_strategy() {
    let core = CORE.replace("out.line(\"{items.size} items\");", "out.line(\"{items[1].left} left\");");
    let ds = emit(&core).expect_err("refused");
    assert!(
        ds.iter().any(|d| d.code == "E-UNSUPPORTED" && d.message.contains("no strategy keeps `Missing`")),
        "{ds:?}"
    );
}
