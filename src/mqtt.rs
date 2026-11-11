use bytes::Bytes;
use log::{debug, error, trace};
use rumqttc::{Client, Connection, Event, MqttOptions, Packet, QoS, SubscribeFilter};

use crate::config::{Credentials, MqttConfig, Sources};

pub struct Message {
    pub topic: String,
    pub value: Bytes,
}

impl Message {
    pub fn new(topic: String, value: Bytes) -> Self {
        Self { topic, value }
    }
}

impl Sources {
    pub fn topics(&self) -> impl Iterator<Item = String> {
        [
            &self.l1,
            &self.l2,
            &self.l3,
            &self.frequency,
            &self.forecast,
        ]
        .into_iter()
        .filter_map(Option::as_ref)
        .map(|s| s.topic.clone())
    }
}

pub struct MqttClient(Connection);

impl MqttClient {
    pub fn connect(
        connection_config: MqttConfig,
        sources: &Sources,
    ) -> Result<Self, rumqttc::ClientError> {
        let mut mqtt_options = MqttOptions::new(
            connection_config.id,
            connection_config.host,
            connection_config.port,
        );
        if let Some(Credentials { username, password }) = connection_config.credentials {
            mqtt_options.set_credentials(username, password);
        }

        let (client, connection) = Client::new(mqtt_options, 16);
        client.subscribe_many(
            sources
                .topics()
                .map(|p| SubscribeFilter::new(p, QoS::AtMostOnce)),
        )?;

        Ok(Self(connection))
    }

    pub fn messages(&mut self) -> impl Iterator<Item = Message> {
        self.0.iter().filter_map(|n| {
            if let Ok(Event::Incoming(Packet::Publish(publish))) = n {
                debug!("New message on {}", publish.topic);
                Some(Message::new(publish.topic, publish.payload))
            } else if let Err(e) = n {
                error!("Error from MQTT connection: {e}");
                panic!("MQTT connection error: {e}");
            } else {
                trace!("Ignoring MQTT message {n:?}");
                None
            }
        })
    }
}
