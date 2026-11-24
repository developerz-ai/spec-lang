//! Layout: the printer decides what is written, Biome (the generated project's own formatter, here as a library)
//! decides how it is laid out, with the options of the project's `biome.json`.

use biome_formatter::{IndentStyle, IndentWidth, LineWidth};
use biome_js_formatter::context::JsFormatOptions;
use biome_js_parser::{JsParserOptions, parse};
use biome_languages::JsFileSource;

/// A TypeScript module, laid out; why not, when it is not one.
pub fn typescript(text: &str) -> Result<String, String> {
    let source = JsFileSource::ts();
    let parsed = parse(text, source, JsParserOptions::default());
    if let Some(d) = parsed.diagnostics().first() {
        return Err(format!("{:?}", d));
    }
    let options = JsFormatOptions::new(source)
        .with_indent_style(IndentStyle::Space)
        .with_indent_width(IndentWidth::try_from(2).map_err(|e| e.to_string())?)
        .with_line_width(LineWidth::try_from(120).map_err(|e| e.to_string())?);
    let formatted =
        biome_js_formatter::format_node(options, &parsed.syntax(), Vec::new()).map_err(|e| e.to_string())?;
    Ok(formatted.print().map_err(|e| e.to_string())?.into_code())
}
