use std::path::Path;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::Module;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};

pub fn parse_ts_file(path: &Path) -> Option<Module> {
    let source = std::fs::read_to_string(path).ok()?;
    let is_tsx = path
        .extension()
        .map_or(false, |ext| ext == "tsx");
    parse_ts_source(&source, path.to_string_lossy().as_ref(), is_tsx)
}

pub fn parse_ts_source(source: &str, filename: &str, tsx: bool) -> Option<Module> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom(filename.into()).into(), source.to_string());

    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx,
            decorators: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);
    parser.parse_module().ok()
}
