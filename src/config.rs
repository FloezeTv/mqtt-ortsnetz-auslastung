use jiff::{Span, ToSpan};
use serde::{Deserialize, Deserializer};
use serde_default::DefaultFromSerde;

fn deserialize_filter<'de, D>(deserializer: D) -> Result<jaq_all::data::Filter, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    jaq_all::data::compile(&s)
        .map_err(|v| format!("Failed to parse jq filter: {v:?}"))
        .map_err(serde::de::Error::custom)
}

fn default_filter() -> jaq_all::data::Filter {
    jaq_all::data::compile(".").unwrap()
}

fn default_upload_url() -> String {
    "https://www.ortsnetz-auslastung.de/v1/measurements".into()
}

fn default_interval() -> Span {
    5.minutes()
}

fn default_max_age() -> Span {
    1.minute()
}

fn default_port() -> u16 {
    1883
}

fn default_id() -> String {
    "ortsnetz-auslastung".into()
}

#[derive(Deserialize)]
pub struct SourceSpec {
    pub topic: String,
    #[serde(deserialize_with = "deserialize_filter", default = "default_filter")]
    pub filter: jaq_all::data::Filter,
}

#[derive(Deserialize)]
pub struct Sources {
    #[serde(default)]
    pub l1: Option<SourceSpec>,
    #[serde(default)]
    pub l2: Option<SourceSpec>,
    #[serde(default)]
    pub l3: Option<SourceSpec>,
    #[serde(default)]
    pub frequency: Option<SourceSpec>,
    #[serde(default)]
    pub forecast: Option<SourceSpec>,
}

#[derive(Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct MqttConfig {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_id")]
    pub id: String,
    pub credentials: Option<Credentials>,
}

#[derive(Deserialize)]
pub struct Site {
    pub kwp: Option<f64>,
    pub meter_model: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Deserialize)]
pub struct Sanity {
    pub voltage_min: Option<f64>,
    pub voltage_max: Option<f64>,
    pub freq_min: Option<f64>,
    pub freq_max: Option<f64>,
    pub forecast_min: Option<f64>,
    pub forecast_max: Option<f64>,
    pub forecast_per_kwp_max: Option<f64>,
}

#[derive(Deserialize, DefaultFromSerde)]
pub struct Upload {
    #[serde(default = "default_upload_url")]
    pub url: String,
    #[serde(default = "default_interval")]
    pub interval: Span,
    #[serde(default = "default_max_age")]
    pub max_age: Span,
}

#[derive(Deserialize)]
pub struct Config {
    pub site: Site,
    pub sanity: Sanity,
    pub sources: Sources,
    #[serde(default)]
    pub upload: Upload,
    pub mqtt: MqttConfig,
}
