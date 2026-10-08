//! clarc: cloud architecture diagrams (drawio) from a service graph, laid out by c43-layout.
pub mod catalog;
pub mod compose;
pub mod input;
pub mod render;

use catalog::Theme;
use input::Input;

#[derive(Debug)]
pub struct Rendered {
    pub xml: String,
    /// dropped or unsatisfied things and layout rule violations, one line each
    pub warnings: Vec<String>,
}

pub fn parse(text: &str) -> Result<Input, String> {
    serde_json::from_str(text).map_err(|e| format!("invalid input: {e}"))
}

pub fn lay_out(c: &compose::Composed) -> Result<c43_layout::model::LayoutResult, String> {
    c43_layout::layout(&c.graph, &c.hints, &Default::default())
}

pub fn render(input: &Input, theme: Theme) -> Result<Rendered, String> {
    let c = compose::compose(input);
    let mut warnings = c.warnings.clone();
    let r = lay_out(&c)?;
    warnings.extend(r.metrics.violations.iter().map(|v| format!("{}: {}", v.rule, v.detail)));
    warnings.extend(r.metrics.ignored_hints.iter().map(|h| format!("placement hint not satisfied: {h}")));
    let xml = c43::drawio::render_styled(&r.layout, c.graph.title.as_deref(), c.graph.description.as_deref(), render::styler(theme, &c, r.layout.s * c43::drawio::text::UNIT_PX, r.layout.nodes.iter().map(|n| (n.id.clone(), (n.col, n.row))).collect()));
    Ok(Rendered { xml, warnings })
}
