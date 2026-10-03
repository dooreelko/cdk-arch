mod common;
use c43_layout::geom::{cell_box, EndInfo};
use c43_layout::model::{Box as GBox, Cell, End, Graph, Kind, Ports, Side, SIDES};
use c43_layout::place::place;
use c43_layout::ports::{assign_ports, choose_sides, end_info, side_count};
use c43_layout::skeleton::skeleton;
use common::{case_graph, internal, placement_of};
use serde_json::json;

fn c(col: i32, row: i32) -> GBox {
    cell_box(&Cell { col, row })
}

#[test]
fn choose_sides_cases() {
    use Side::*;
    assert_eq!(choose_sides(Kind::Data, &c(0, 0), &c(1, 0)), (Right, Left));
    assert_eq!(choose_sides(Kind::Data, &c(0, 0), &c(1, 1)), (Right, Left)); // AGW→LC
    assert_eq!(choose_sides(Kind::Data, &c(0, 0), &c(0, 1)), (Bottom, Top));
    assert_eq!(choose_sides(Kind::Data, &c(1, 0), &c(0, 1)), (Bottom, Top));
    assert_eq!(choose_sides(Kind::Nf, &c(0, 0), &c(2, 1)), (Bottom, Left)); // nf below-right: leave down, enter facing source
    assert_eq!(choose_sides(Kind::Nf, &c(1, 0), &c(1, 2)), (Bottom, Top)); // nf directly below: short link
    assert_eq!(choose_sides(Kind::Nf, &c(1, 0), &c(0, 2)), (Bottom, Top)); // nf below-left: top faces the source
    assert_eq!(choose_sides(Kind::Nf, &c(1, 2), &c(0, 0)), (Right, Left)); // CW→SES
    assert_eq!(choose_sides(Kind::Data, &c(1, 0), &c(0, 0)), (Right, Left));
}

fn ids(g: &Graph, ports: &Ports, node: &str, side: Side) -> Vec<String> {
    ports[node].get(side).iter().map(|r| format!("{}>{}", g.edges[r.edge].from, g.edges[r.edge].to)).collect()
}

fn setup(name: &str) -> (Graph, Ports) {
    let g = case_graph(name);
    let p = place(&g, &skeleton(&g));
    let ports = assign_ports(&g, &p);
    (g, ports)
}

#[test]
fn aws_ports() {
    let (g, ports) = setup("aws");
    assert_eq!(ids(&g, &ports, "AGW", Side::Right), ["AGW>LA", "AGW>LB", "AGW>LC"]);
    assert_eq!(ids(&g, &ports, "S3", Side::Left), ["LA>S3", "LB>S3", "ATH>S3"]);
    assert_eq!(ids(&g, &ports, "SES", Side::Left), ["LA>SES", "CW>SES"]);
    assert_eq!(ids(&g, &ports, "LA", Side::Right), ["LA>SES", "LA>S3"]);
    assert_eq!(ids(&g, &ports, "CF", Side::Bottom), ["CF>R53"]);
    assert_eq!(side_count(&ports, None), 3);
    let agw_lb = g.edges.iter().find(|e| e.from == "AGW" && e.to == "LB").unwrap().id;
    assert_eq!(end_info(&ports, "AGW", agw_lb, End::Src), EndInfo { side: Side::Right, index: 1, count: 3, inner: false });
}

fn ports_for(edges: &[(&str, &str)], cells: &[(&str, (i32, i32))]) -> (Graph, Ports) {
    let g = internal(json!({
        "nodes": cells.iter().map(|(id, _)| json!({"id": id})).collect::<Vec<_>>(),
        "edges": edges.iter().map(|(f, t)| json!({"from": f, "to": t})).collect::<Vec<_>>()
    }));
    let ports = assign_ports(&g, &placement_of(cells));
    (g, ports)
}

#[test]
fn four_data_outs_three_stay_right_lowest_target_leaves_from_the_bottom() {
    let (g, p) = ports_for(&[("A", "B"), ("A", "C"), ("A", "D"), ("A", "E")],
        &[("A", (0, 1)), ("B", (1, 0)), ("C", (1, 1)), ("D", (1, 2)), ("E", (1, 3))]);
    assert_eq!(ids(&g, &p, "A", Side::Right), ["A>B", "A>C", "A>D"]);
    assert_eq!(ids(&g, &p, "A", Side::Bottom), ["A>E"]);
    assert!(ids(&g, &p, "A", Side::Top).is_empty());
}

#[test]
fn overflow_comes_from_the_end_farther_from_the_source_row() {
    let (g, p) = ports_for(&[("A", "B"), ("A", "C"), ("A", "D"), ("A", "E")],
        &[("A", (0, 2)), ("B", (1, 0)), ("C", (1, 1)), ("D", (1, 2)), ("E", (2, 2))]);
    assert_eq!(ids(&g, &p, "A", Side::Top), ["A>B"]);
    assert_eq!(ids(&g, &p, "A", Side::Right), ["A>C", "A>E", "A>D"]);
    assert!(ids(&g, &p, "A", Side::Bottom).is_empty());
}

#[test]
fn star16_seven_data_outs() {
    let (g, ports) = setup("star16");
    assert_eq!(ids(&g, &ports, "N2", Side::Right), ["N2>N3", "N2>N8", "N2>N4"]);
    assert_eq!(ids(&g, &ports, "N2", Side::Top), ["N2>N7"]);
    assert_eq!(ids(&g, &ports, "N2", Side::Bottom),
        ["N2>N15", "N2>N11", "N2>N10", "N2>N13", "N2>N6", "N2>N5", "N2>N16", "N2>N12", "N2>N9", "N2>N14"]);
    assert_eq!(ids(&g, &ports, "N2", Side::Left), ["N1>N2"]);
    assert_eq!(side_count(&ports, None), 10);
    for n in &g.nodes {
        for side in SIDES {
            for r in ports[&n.id].get(side) {
                if r.end == End::Src { assert_ne!(side, Side::Left); }
                if r.end == End::Dst { assert_ne!(side, Side::Right); }
            }
        }
    }
}
