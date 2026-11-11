use jiff::Timestamp;
use log::{error, info, warn};
use serde::Serialize;
use ureq::http::StatusCode;

use crate::{
    Values,
    config::{Sanity, Site},
};

#[derive(Serialize)]
struct ApiBody {
    observed_at: Timestamp,
    latitude: f64,
    longitude: f64,
    l1_v: f64,
    l2_v: f64,
    l3_v: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    grid_frequency_hz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plant_capacity_kwp: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pv_forecast_kwh: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    smartmeter_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    integration_version: Option<&'static str>,
}

impl ApiBody {
    pub fn new(values: &Values, site: &Site, sanity: &Sanity) -> Result<Self, Vec<String>> {
        const VOLTAGE_MIN: f64 = 0.0;
        const VOLTAGE_MAX: f64 = 500.0;
        const FREQ_MIN: f64 = 45.0;
        const FREQ_MAX: f64 = 50.0;
        const FORECAST_MIN: f64 = 0.0;
        const FORECAST_MAX: f64 = 100000.0;
        const FORECAST_PER_KWP_MAX: f64 = 10.0;

        fn san_min(sanity_value: Option<f64>, const_value: f64) -> f64 {
            sanity_value.unwrap_or(const_value).max(const_value)
        }

        fn san_max(sanity_value: Option<f64>, const_value: f64) -> f64 {
            sanity_value.unwrap_or(const_value).min(const_value)
        }

        const INTEGRATION_VERSION: Option<&str> =
            Some(concat!("mqtt-rs ", env!("CARGO_PKG_VERSION")));

        const {
            // statically check length of integration version
            if let Some(integration_version) = INTEGRATION_VERSION {
                assert!(
                    integration_version.len() <= 32,
                    "Integration version too long",
                );
            }
        }

        let mut errors = Vec::new();

        if ![values.l1, values.l2, values.l3]
            .iter()
            .any(Option::is_some)
        {
            errors.push("No voltages measured!".into());
        }
        for (v, n) in [(values.l1, "L1"), (values.l2, "L2"), (values.l3, "L3")] {
            if let Some(v) = v
                && (v < san_min(sanity.voltage_min, VOLTAGE_MIN)
                    || v > san_max(sanity.voltage_max, VOLTAGE_MAX))
            {
                errors.push(format!("{n}'s voltage is out of range ({v})"));
            }
        }

        if let Some(f) = values.frequency
            && (f < san_min(sanity.freq_min, FREQ_MIN) || f > san_max(sanity.freq_max, FREQ_MAX))
        {
            errors.push(format!("Frequency is out of range ({f})"));
        }

        let forecast_valid = if let Some(forecast) = values.forecast {
            if forecast < san_min(sanity.forecast_min, FORECAST_MIN)
                || forecast > san_max(sanity.forecast_max, FORECAST_MAX)
            {
                warn!("Forecast is out of range ({forecast}); ignoring...");
                false
            } else if let Some(kwp) = site.kwp
                && forecast / kwp > san_max(sanity.forecast_per_kwp_max, FORECAST_PER_KWP_MAX)
            {
                warn!(
                    "Forecast per kWp is out of range ({}); ignoring...",
                    forecast / kwp
                );
                false
            } else {
                true
            }
        } else {
            true
        };

        if let Some(meter_model) = site.meter_model.as_ref()
            && meter_model.len() > 120
        {
            errors.push(format!("Meter model is too long ({})", meter_model.len()));
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(Self {
            observed_at: values.time,
            latitude: site.latitude,
            longitude: site.longitude,
            l1_v: values.l1.unwrap_or(-1.),
            l2_v: values.l2.unwrap_or(-1.),
            l3_v: values.l3.unwrap_or(-1.),
            grid_frequency_hz: values.frequency,
            plant_capacity_kwp: site.kwp,
            pv_forecast_kwh: if forecast_valid {
                values.forecast
            } else {
                None
            },
            smartmeter_model: site.meter_model.clone(),
            integration_version: INTEGRATION_VERSION,
        })
    }
}

impl Values {
    pub fn send(&mut self, url: &str, site: &Site, sanity: &Sanity) {
        match ApiBody::new(self, site, sanity) {
            Ok(body) => {
                info!("Sending data from {}: {self}", self.time);
                match ureq::post(url)
                    .header("User-Agent", "mqtt-client")
                    .send_json(body)
                    .map(|r| r.status())
                {
                    Ok(StatusCode::ACCEPTED) => {}
                    Ok(s) => error!("Server refused data with code {s}"),
                    Err(e) => error!("Failed to send data to server: {e}"),
                }
            }
            Err(es) => {
                error!("Did not send data to API:\n{}", es.join("\n"));
            }
        };

        *self = Default::default();
    }
}
