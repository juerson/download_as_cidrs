use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub status: String,
    pub data: Data,
}

#[derive(Debug, Deserialize)]
pub struct Data {
    pub ipv4_prefixes: Vec<Prefix>,
    pub ipv6_prefixes: Vec<Prefix>,
}

#[derive(Debug, Deserialize)]
pub struct Prefix {
    pub prefix: String,
    pub name: Option<String>,
    pub country_code: Option<String>,
    pub description: Option<String>,
    pub parent: Parent,
}

#[derive(Debug, Deserialize)]
pub struct Parent {
    pub rir_name: Option<String>,
}
