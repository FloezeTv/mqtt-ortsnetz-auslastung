use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use jiff::Timestamp;
use log::info;

mod config;
use crate::config::*;

mod mqtt;
use crate::mqtt::*;

mod api;

mod values;
use crate::values::*;

fn main() {
    env_logger::init();

    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::prefixed("OA_"))
        .extract()
        .expect("Failed to load config");

    if config.sources.topics().count() == 0 {
        panic!("No sources defined!");
    }

    info!("Starting mqtt 2 ortsnetz-auslastung");

    let mut values = Values::default();

    let mut last_message_time = Timestamp::MIN;

    let mut client = MqttClient::connect(config.mqtt, &config.sources)
        .expect("Failed to connect to MQTT server");
    info!("Successfully connected to MQTT server");
    for message in client.messages() {
        values.message(&message, &config.sources);

        if Timestamp::now() >= last_message_time + config.upload.interval
            && Timestamp::now() <= values.time + config.upload.max_age
        {
            values.send(&config.upload.url, &config.site, &config.sanity);
            last_message_time = Timestamp::now();
        }
    }
}
