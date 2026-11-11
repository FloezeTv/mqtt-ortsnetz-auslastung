use std::fmt::Display;

use jaq_all::jaq_std::ValT;
use jiff::Timestamp;
use log::{debug, error};

use crate::{
    config::{SourceSpec, Sources},
    mqtt::Message,
};

#[derive(Default)]
pub struct Values {
    pub l1: Option<f64>,
    pub l2: Option<f64>,
    pub l3: Option<f64>,
    pub frequency: Option<f64>,
    pub forecast: Option<f64>,
    pub time: Timestamp,
}

impl Display for Values {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut write_value = |name, v, comma| {
            let comma = if comma { ", " } else { "" };
            if let Some(v) = v {
                write!(f, "{name} = {v}{comma}")
            } else {
                write!(f, "{name} = ?{comma}")
            }
        };
        write_value("l1", self.l1, true)?;
        write_value("l2", self.l2, true)?;
        write_value("l3", self.l3, true)?;
        write_value("frequency", self.frequency, true)?;
        write_value("forecast", self.forecast, false)?;

        Ok(())
    }
}

impl Values {
    fn try_message(
        name: &'static str,
        field: &mut Option<f64>,
        message: &Message,
        source: &Option<SourceSpec>,
    ) -> Option<Timestamp> {
        if let Some(source) = source
            && source.topic == message.topic
        {
            match apply_jq(&source.filter, &message.value, &message.topic) {
                Ok(v) => {
                    if let Some(v) = v.as_f64() {
                        *field = Some(v);
                        debug!("Updated {name} to {v}");
                        return Some(Timestamp::now());
                    } else {
                        error!(
                            "Filter {} from {} did not return a number: {}",
                            source.topic, message.topic, v
                        );
                    }
                }
                Err(e) => error!("{e}"),
            }
        }
        None
    }

    pub fn message(&mut self, message: &Message, sources: &Sources) {
        if let Some(ts) = [
            Self::try_message("l1", &mut self.l1, message, &sources.l1),
            Self::try_message("l2", &mut self.l2, message, &sources.l2),
            Self::try_message("l3", &mut self.l3, message, &sources.l3),
            Self::try_message(
                "frequency",
                &mut self.frequency,
                message,
                &sources.frequency,
            ),
        ]
        .into_iter()
        .flatten()
        .max()
        {
            // Update time if one of the net parameters has update
            self.time = ts;
        }

        // Don't update time with forecast
        Self::try_message("forecast", &mut self.forecast, message, &sources.forecast);
    }
}

fn apply_jq(
    filter: &jaq_all::data::Filter,
    json_payload: &[u8],
    topic_name: &str,
) -> Result<jaq_all::json::Val, String> {
    let input = jaq_all::fmts::read::json::parse_many(json_payload);
    let input = (&topic_name.to_string().into(), input);

    let mut result: Result<jaq_all::json::Val, String> = Err("No Result".into());

    jaq_all::data::run(
        &Default::default(),
        filter,
        Default::default(),
        input,
        |e| error!("Error parsing json: {e}"),
        |v| {
            result = v.map_err(|e| format!("{e:?}"));
            Ok(())
        },
    )
    .expect("Error");

    result
}
