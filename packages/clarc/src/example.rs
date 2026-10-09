//! `clarc --agentic-help`: a small input document for an LLM (or a person) to take inspiration from.
use crate::catalog::{self, Theme};

pub fn example_json(theme: Theme) -> String {
    let (client, cdn, vm, dns) = match theme {
        Theme::Aws => ("client", "cloudfront", "ec2", "route_53"),
        Theme::Azure => ("general/Browser", "networking/Front_Doors", "compute/Virtual_Machine", "networking/DNS_Zones"),
    };
    format!(
        r#"{{
  "title": "Web app behind a CDN",
  "description": "Viewers reach an app on a VM in a private network through a CDN; DNS points the domain at the CDN",
  "groups": [
    {{ "id": "vpc", "label": "VPC", "service": "network" }}
  ],
  "nodes": [
    {{ "id": "viewer", "label": "Viewer (browser)", "service": "{client}" }},
    {{ "id": "cdn", "label": "CDN", "service": "{cdn}" }},
    {{ "id": "app", "label": "App server", "service": "{vm}", "group": "vpc" }},
    {{ "id": "dns", "label": "DNS zone", "service": "{dns}" }}
  ],
  "edges": [
    {{ "from": "viewer", "to": "cdn", "label": "HTTPS" }},
    {{ "from": "cdn", "to": "app", "label": "origin fetch" }},
    {{ "from": "dns", "to": "cdn", "label": "DNS", "kind": "nf" }}
  ],
  "hints": {{
    "viewer": {{ "start": true }},
    "dns": {{ "below": "cdn" }}
  }}
}}"#
    )
}

/// every placement directive of a node's `hints` entry: (name, meaning)
pub const HINT_DIRECTIVES: &[(&str, &str)] = &[
    ("start", "true = this node is the left-most one; at most one node may have it"),
    ("leftOf", "<id> = this node is placed left of the node <id>"),
    ("rightOf", "<id> = this node is placed right of the node <id>"),
    ("above", "<id> = this node is placed above the node <id>"),
    ("below", "<id> = this node is placed below the node <id>"),
    ("sameRow", "<id> = this node is placed on the row of the node <id>"),
    ("sameCol", "<id> = this node is placed in the column of the node <id>"),
];

/// what each group frame kind (`service` of a group) is for
pub const GROUP_PURPOSES: &[(&str, &str)] = &[
    ("network", "private network (VPC / VNet); members get `\"group\": \"<group id>\"`"),
    ("region", "cloud region"),
    ("cloud", "the whole cloud account / provider boundary"),
    ("group", "generic grey frame (default)"),
];

fn names(key: &str, aliases: &[&str]) -> String {
    match aliases {
        [] => key.to_string(),
        a => format!("{key} (also {})", a.join(", ")),
    }
}

fn purpose(table: &[(&'static str, &'static str)], key: &str) -> &'static str {
    table.iter().find(|(k, _)| *k == key).map_or("", |(_, p)| *p)
}

/// the example for the theme followed by what hints, glyphs, group frames and edges mean
pub fn example_text(theme: Theme) -> String {
    let lines = |items: Vec<String>| items.join("\n");
    let directives = lines(HINT_DIRECTIVES.iter().map(|(n, d)| format!("  {n}: {d}")).collect());
    let glyphs = lines(catalog::common(theme).iter().map(|(n, p)| format!("  {n}: {p}")).collect());
    let groups = lines(catalog::GROUPS.iter().map(|g| format!("  {}: {}", names(g.key, g.aliases), purpose(GROUP_PURPOSES, g.key))).collect());
    let (aws_url, azure_url) = (catalog::glyph_list_url(Theme::Aws), catalog::glyph_list_url(Theme::Azure));
    format!(
        "{json}\n\n\
Hint directives (keys of an entry in \"hints\", keyed by the node they are about; suggestions that are honoured when possible):\n{directives}\n\n\
Node \"service\" is the name of a drawio glyph of the {theme} provider. Case, `-` and `_` do not matter; an unknown name is an error with suggestions. Without a service the node is a text box, as is \"note\".\n\
Full lists of glyph names (aws, azure):\n  {aws_url}\n  {azure_url}\n\
Common ones:\n{glyphs}\n\n\
Group frames (\"service\" of a group; \"group\" on a group or node puts it inside another group):\n{groups}\n\n\
Edges: an edge is a data flow by default and flows left to right. \"kind\": \"nf\" marks a non-functional link (DNS, monitoring, identity, configuration, image pull); those flow top to bottom and are dashed. The same service can have both: fargate -> s3 is data, fargate -> cloudwatch is nf. \"style\": \"solid\"|\"dashed\" overrides the line.\n",
        json = example_json(theme)
    )
}
