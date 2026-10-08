//! Cloud services per provider: what a service looks like and whether it is data flow or non-functional (nf).
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Aws,
    Azure,
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Theme::Aws => "aws",
            Theme::Azure => "azure",
        })
    }
}

/// how a node is drawn
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Look {
    /// aws resource icon: stencil name, category colour
    AwsRes(&'static str, &'static str),
    /// aws standalone shape: shape name, colour
    AwsShape(&'static str, &'static str),
    /// drawio's built-in azure2 icon path
    AzureImg(&'static str),
    /// a bordered text box
    Note,
}

#[derive(Debug, Clone, Copy)]
pub struct Service {
    /// provider-neutral key
    pub key: &'static str,
    /// provider-native names that mean the same
    pub aliases: &'static [&'static str],
    /// non-functional: edges of the service default to nf
    pub nf: bool,
    pub aws: Look,
    pub azure: Look,
}

const ORANGE: &str = "#ED7100";
const GREEN: &str = "#7AA116";
const PURPLE: &str = "#8C4FFF";
const RED: &str = "#DD344C";
const PINK: &str = "#E7157B";
const DARK: &str = "#232F3D";

use Look::*;

pub const SERVICES: &[Service] = &[
    Service { key: "client", aliases: &["user", "browser", "viewer"], nf: false, aws: AwsShape("mxgraph.aws4.client", DARK), azure: AzureImg("img/lib/azure2/general/Browser.svg") },
    Service { key: "server", aliases: &["external"], nf: false, aws: AwsShape("mxgraph.aws4.traditional_server", DARK), azure: AzureImg("img/lib/azure2/compute/Virtual_Machine.svg") },
    Service { key: "cdn", aliases: &["cloudfront", "front-door", "frontdoor"], nf: false, aws: AwsRes("mxgraph.aws4.cloudfront", PURPLE), azure: AzureImg("img/lib/azure2/networking/Front_Doors.svg") },
    Service { key: "dns", aliases: &["route53", "dns-zone"], nf: true, aws: AwsRes("mxgraph.aws4.route_53", PURPLE), azure: AzureImg("img/lib/azure2/networking/DNS_Zones.svg") },
    Service { key: "object-storage", aliases: &["s3", "blob", "blob-storage", "storage-account"], nf: false, aws: AwsRes("mxgraph.aws4.s3", GREEN), azure: AzureImg("img/lib/azure2/storage/Storage_Accounts.svg") },
    Service { key: "vm", aliases: &["ec2", "virtual-machine"], nf: false, aws: AwsRes("mxgraph.aws4.ec2", ORANGE), azure: AzureImg("img/lib/azure2/compute/Virtual_Machine.svg") },
    Service { key: "function", aliases: &["lambda", "azure-function", "functions"], nf: false, aws: AwsRes("mxgraph.aws4.lambda", ORANGE), azure: AzureImg("img/lib/azure2/compute/Function_Apps.svg") },
    Service { key: "container-service", aliases: &["lightsail", "ecs", "container-apps", "container-app"], nf: false, aws: AwsRes("mxgraph.aws4.lightsail", ORANGE), azure: AzureImg("img/lib/azure2/other/Container_App_Environments.svg") },
    Service { key: "container-registry", aliases: &["ecr", "acr"], nf: true, aws: AwsRes("mxgraph.aws4.ecr", ORANGE), azure: AzureImg("img/lib/azure2/containers/Container_Registries.svg") },
    Service { key: "api-gateway", aliases: &["apigw", "api-management", "apim"], nf: false, aws: AwsRes("mxgraph.aws4.api_gateway", PINK), azure: AzureImg("img/lib/azure2/app_services/API_Management_Services.svg") },
    Service { key: "role", aliases: &["iam", "iam-role", "managed-identity"], nf: true, aws: AwsShape("mxgraph.aws4.role", RED), azure: AzureImg("img/lib/azure2/identity/Managed_Identities.svg") },
    Service { key: "parameter-store", aliases: &["ssm-parameter-store", "key-vault", "secrets"], nf: true, aws: AwsRes("mxgraph.aws4.parameter_store", PINK), azure: AzureImg("img/lib/azure2/security/Key_Vaults.svg") },
    Service { key: "ops-console", aliases: &["ssm", "systems-manager", "bastion"], nf: true, aws: AwsRes("mxgraph.aws4.systems_manager", PINK), azure: AzureImg("img/lib/azure2/networking/Bastions.svg") },
    Service { key: "monitoring", aliases: &["cloudwatch", "monitor", "azure-monitor"], nf: true, aws: AwsRes("mxgraph.aws4.cloudwatch", PINK), azure: AzureImg("img/lib/azure2/management_governance/Monitor.svg") },
    Service { key: "topic", aliases: &["sns", "event-grid", "service-bus"], nf: true, aws: AwsRes("mxgraph.aws4.sns", PINK), azure: AzureImg("img/lib/azure2/integration/Event_Grid_Topics.svg") },
    Service { key: "email", aliases: &["ses", "communication-services"], nf: false, aws: AwsRes("mxgraph.aws4.simple_email_service", RED), azure: AzureImg("img/lib/azure2/other/Azure_Communication_Services.svg") },
    Service { key: "query", aliases: &["athena", "synapse"], nf: false, aws: AwsRes("mxgraph.aws4.athena", PURPLE), azure: AzureImg("img/lib/azure2/analytics/Azure_Synapse_Analytics.svg") },
    Service { key: "private-endpoint", aliases: &["vpc-endpoint", "vpc-origin", "private-link"], nf: false, aws: AwsShape("mxgraph.aws4.endpoints", PURPLE), azure: AzureImg("img/lib/azure2/networking/Private_Endpoint.svg") },
    Service { key: "static-ip", aliases: &["elastic-ip", "eip", "public-ip"], nf: false, aws: AwsRes("mxgraph.aws4.elastic_ip_address", ORANGE), azure: AzureImg("img/lib/azure2/networking/Public_IP_Addresses.svg") },
    Service { key: "note", aliases: &["text", "annotation"], nf: false, aws: Note, azure: Note },
];

/// how a group frame is drawn
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupLook {
    pub key: &'static str,
    pub aliases: &'static [&'static str],
    /// aws group icon (None: plain frame)
    pub aws_icon: Option<&'static str>,
    pub aws_color: &'static str,
    pub azure_color: &'static str,
}

pub const GROUPS: &[GroupLook] = &[
    GroupLook { key: "network", aliases: &["vpc", "vnet", "virtual-network"], aws_icon: Some("mxgraph.aws4.group_vpc2"), aws_color: PURPLE, azure_color: "#0078D4" },
    GroupLook { key: "region", aliases: &[], aws_icon: Some("mxgraph.aws4.group_region"), aws_color: "#147EBA", azure_color: "#0078D4" },
    GroupLook { key: "cloud", aliases: &["aws-cloud", "azure"], aws_icon: Some("mxgraph.aws4.group_aws_cloud"), aws_color: "#232F3E", azure_color: "#0078D4" },
    GroupLook { key: "group", aliases: &["generic"], aws_icon: None, aws_color: "#7D8998", azure_color: "#7D8998" },
];

fn find<'a, T>(items: &'a [T], name: &str, names: impl Fn(&T) -> Vec<&'static str>) -> Option<&'a T> {
    let n = name.to_lowercase();
    items.iter().find(|i| names(i).contains(&n.as_str()))
}

pub fn service(name: &str) -> Option<&'static Service> {
    find(SERVICES, name, |s| std::iter::once(s.key).chain(s.aliases.iter().copied()).collect())
}

pub fn group_look(name: &str) -> Option<&'static GroupLook> {
    find(GROUPS, name, |g| std::iter::once(g.key).chain(g.aliases.iter().copied()).collect())
}
