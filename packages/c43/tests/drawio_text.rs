use c43::drawio::text::{label_side, text_width, wrap, LABEL_PX};

#[test]
fn width_grows_with_length_and_font_size() {
    assert!(text_width("dispatcher", 12.0, false) < text_width("dispatcher-x", 12.0, false));
    assert_eq!(text_width("ab", 24.0, false), 2.0 * text_width("ab", 12.0, false));
    assert!(text_width("WWW", 12.0, false) > text_width("iii", 12.0, false));
    assert!(text_width("ab", 12.0, true) > text_width("ab", 12.0, false));
}

fn lines(xs: &[&str]) -> Option<Vec<String>> {
    Some(xs.iter().map(|s| s.to_string()).collect())
}

#[test]
fn wrap_breaks_at_spaces_and_after_separators() {
    let w = text_width("frontend/", LABEL_PX, false);
    assert_eq!(wrap("@bob/frontend", w, LABEL_PX, false), lines(&["@bob/", "frontend"]));
    assert_eq!(wrap("API Gateway", text_width("Gateway", LABEL_PX, false), LABEL_PX, false), lines(&["API", "Gateway"]));
    assert_eq!(wrap("a_b.c", text_width("a_", LABEL_PX, false), LABEL_PX, false), lines(&["a_", "b.", "c"]));
    assert_eq!(wrap("sub-bob-manager", 1000.0, LABEL_PX, false), lines(&["sub-bob-manager"]));
}

#[test]
fn wrap_packs_pieces_greedily_onto_a_line() {
    let w = text_width("@bob/sub-bob-", LABEL_PX, false);
    assert_eq!(wrap("@bob/sub-bob-bootstrap", w, LABEL_PX, false), lines(&["@bob/sub-bob-", "bootstrap"]));
}

#[test]
fn wrap_returns_none_when_one_piece_cannot_fit() {
    assert_eq!(wrap("dispatcher", text_width("dispatche", LABEL_PX, false), LABEL_PX, false), None);
}

#[test]
fn label_side_smallest_square_holding_the_wrapped_label() {
    let u = 20.0;
    let s = label_side(&["dispatcher"], u);
    assert!(s * u >= text_width("dispatcher", LABEL_PX, false) + LABEL_PX);
    assert!(label_side(&["@bob/frontend"], u) >= label_side(&["frontend"], u));
    assert_eq!(label_side(&["a", "dispatcher"], u), s);
}
