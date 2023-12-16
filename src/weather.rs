use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Timelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Deserializer};
use zellij_tile::prelude::*;

pub struct WeatherState {
    fetch_state: FetchState,
    forecast: Option<WeatherForecastResp>,
    last_updated: Option<DateTime<Utc>>,
    last_requested: Option<DateTime<Utc>>,
    last_rendered: Option<DateTime<Utc>>,
    rendered: String,
}

impl WeatherState {
    pub fn new() -> Self {
        Self {
            fetch_state: FetchState::Idle,
            forecast: None,
            last_updated: None,
            last_requested: None,
            last_rendered: None,
            rendered: String::from(""),
        }
    }

    pub fn run(&mut self) {
        self.fetch_state = FetchState::FetchingLocation;
        web_request(
            "http://ip-api.com/json/",
            HttpVerb::Get,
            BTreeMap::new(),
            Vec::new(),
            BTreeMap::new(),
        );
    }

    pub fn on_web_request(&mut self, e: Event) {
        if let Event::WebRequestResult(status, _headers, body, _context) = e {
            match self.fetch_state {
                FetchState::Idle => {}
                FetchState::FetchingLocation => {}
                FetchState::FetchingForecast => {}
            }
        }
    }

    pub fn on_timer(&mut self) {
        let now = Utc::now();
        match self.fetch_state {
            FetchState::Idle if self.last_updated.map_or(true, |v| now.hour() != v.hour()) => {
                self.fetch_location();
            }
            _ => {}
        }
    }

    fn fetch_location(&mut self) {
        self.fetch_state = FetchState::FetchingLocation;
        let mut context = BTreeMap::new();
        context.insert(String::from("api"), String::from("location"));
        web_request(
            "http://ip-api.com/json/",
            HttpVerb::Get,
            BTreeMap::new(),
            Vec::new(),
            context,
        );
        self.last_requested = Some(Utc::now());
    }

    fn fetch_forecast(&mut self, location: &IpGeolocationResp) {
        self.fetch_state = FetchState::FetchingForecast;
        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m,apparent_temperature,weather_code,wind_speed_10m&daily=weather_code,temperature_2m_max,temperature_2m_min,apparent_temperature_max,apparent_temperature_min&timezone={}&forecast_days=2",
            location.latitude,
            location.longitude,
            location.timezone.to_string().replace('/', "%2F"),
        );
        let mut context = BTreeMap::new();
        context.insert(String::from("api"), String::from("forecast"));
        web_request(url, HttpVerb::Get, BTreeMap::new(), Vec::new(), context);
        self.last_requested = Some(Utc::now());
    }
}

enum FetchState {
    Idle,
    FetchingLocation,
    FetchingForecast,
}

pub enum WeatherCode {
    ClearSky,
    MainlyClear,
    PartlyCloudy,
    Overcast,
    Fog,
    RimeFog,
    DrizzleLight,
    DrizzleModerate,
    DrizzleDense,
    FreezingDrizzleLight,
    FreezingDrizzleHeavy,
    RainSlight,
    RainModerate,
    RainHeavy,
    FreezingRainLight,
    FreezingRainHeavy,
    SnowFallSlight,
    SnowFallModerate,
    SnowFallHeavy,
    SnowGrains,
    RainShowersSlight,
    RainShowersModerate,
    RainShowersViolent,
    SnowShowersSlight,
    SnowShowersHeavy,
    Thunderstorm,
    ThunderstormSlightHail,
    ThunderstormHeavyHail,
    Unknown(u32),
}

pub enum NeedsUmbrella {
    No,
    Maybe,
    Sure,
}

impl WeatherCode {
    pub fn need_umbrella(&self) -> NeedsUmbrella {
        match self {
            Self::DrizzleLight
            | Self::DrizzleModerate
            | Self::SnowFallSlight
            | Self::SnowGrains
            | Self::SnowShowersSlight => NeedsUmbrella::Maybe,

            Self::DrizzleDense
            | Self::FreezingDrizzleLight
            | Self::FreezingDrizzleHeavy
            | Self::RainSlight
            | Self::RainModerate
            | Self::RainHeavy
            | Self::SnowFallModerate
            | Self::SnowFallHeavy
            | Self::RainShowersSlight
            | Self::RainShowersModerate
            | Self::RainShowersViolent
            | Self::SnowShowersHeavy
            | Self::Thunderstorm
            | Self::ThunderstormSlightHail
            | Self::ThunderstormHeavyHail => NeedsUmbrella::Sure,

            _ => NeedsUmbrella::No,
        }
    }
}

impl From<u32> for WeatherCode {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::ClearSky,
            1 => Self::MainlyClear,
            2 => Self::PartlyCloudy,
            3 => Self::Overcast,
            45 => Self::Fog,
            48 => Self::RimeFog,
            51 => Self::DrizzleLight,
            53 => Self::DrizzleModerate,
            55 => Self::DrizzleDense,
            56 => Self::FreezingDrizzleLight,
            57 => Self::FreezingDrizzleHeavy,
            61 => Self::RainSlight,
            63 => Self::RainModerate,
            65 => Self::RainHeavy,
            66 => Self::FreezingRainLight,
            67 => Self::FreezingRainHeavy,
            71 => Self::SnowFallSlight,
            73 => Self::SnowFallModerate,
            75 => Self::SnowFallHeavy,
            77 => Self::SnowGrains,
            80 => Self::RainShowersSlight,
            81 => Self::RainShowersModerate,
            82 => Self::RainShowersViolent,
            85 => Self::SnowShowersSlight,
            86 => Self::SnowShowersHeavy,
            95 => Self::Thunderstorm,
            96 => Self::ThunderstormSlightHail,
            99 => Self::ThunderstormHeavyHail,
            x => Self::Unknown(x),
        }
    }
}

#[derive(Deserialize)]
struct IpGeolocationResp {
    #[serde(rename = "lat")]
    latitude: f32,
    #[serde(rename = "lon")]
    longitude: f32,
    #[serde(deserialize_with = "deserialize_tz")]
    timezone: Tz,
}

fn deserialize_tz<'de, D>(deserializer: D) -> Result<Tz, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

#[derive(Deserialize)]
struct WeatherForecastResp {
    #[serde(deserialize_with = "deserialize_tz")]
    timezone: Tz,
    hourly: HourlyWeatherData,
    daily: DailyWeatherData,
}

#[derive(Deserialize)]
struct HourlyWeatherData {
    #[serde(deserialize_with = "deserialize_date_vec")]
    time: Vec<NaiveDateTime>,
    #[serde(deserialize_with = "deserialize_weather_code_vec")]
    weather_code: Vec<WeatherCode>,
    temperature_2m: Vec<f32>,
    apparent_temperature: Vec<f32>,
    wind_speed_10m: Vec<f32>,
}

#[derive(Deserialize)]
struct DailyWeatherData {
    time: Vec<NaiveDate>,
    #[serde(deserialize_with = "deserialize_weather_code_vec")]
    weather_code: Vec<WeatherCode>,
    temperature_2m_min: Vec<f32>,
    temperature_2m_max: Vec<f32>,
    apparent_temperature_min: Vec<f32>,
    apparent_temperature_max: Vec<f32>,
}

fn deserialize_date_vec<'de, D>(deserializer: D) -> Result<Vec<NaiveDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Vec::<String>::deserialize(deserializer)?;
    v.into_iter()
        .map(|s| {
            NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M").map_err(serde::de::Error::custom)
        })
        .collect()
}

fn deserialize_weather_code_vec<'de, D>(deserializer: D) -> Result<Vec<WeatherCode>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Vec::<u32>::deserialize(deserializer)?;
    v.into_iter().map(|n| Ok(n.into())).collect()
}

#[cfg(test)]
mod tests {
    use super::{IpGeolocationResp, WeatherForecastResp};

    #[test]
    fn parse_ip_geolocation_response() {
        let src = "{\"status\":\"success\",\"country\":\"Japan\",\"countryCode\":\"JP\",\"region\":\"13\",\"regionName\":\"Tokyo\",\"city\":\"Chiyoda\",\"zip\":\"100-0001\",\"lat\":35.694,\"lon\":139.754,\"timezone\":\"Asia/Tokyo\",\"isp\":\"Google LLC\",\"org\":\"Google LLC\",\"as\":\"AS15169 Google LLC\",\"query\":\"142.250.196.110\"}";
        let parsed: IpGeolocationResp = serde_json::from_str(src).unwrap();
        assert!(parsed.latitude - 35.694 < 0.0001);
        assert!(parsed.longitude - 139.754 < 0.0001);
        assert_eq!(parsed.timezone, chrono_tz::Asia::Tokyo);
    }

    #[test]
    fn parse_weather_forecast_response() {
        let src = r#"
{
    "latitude": 40.710335,
    "longitude": -73.99307,
    "generationtime_ms": 0.09298324584960938,
    "utc_offset_seconds": 32400,
    "timezone": "Asia/Seoul",
    "timezone_abbreviation": "KST",
    "elevation": 32.0,
    "hourly_units": {
        "time": "iso8601",
        "temperature_2m": "°C",
        "apparent_temperature": "°C",
        "weather_code": "wmo code",
        "wind_speed_10m": "km/h"
    },
    "hourly": {
        "time": [
            "2023-12-15T00:00",
            "2023-12-15T01:00",
            "2023-12-15T02:00",
            "2023-12-15T03:00",
            "2023-12-15T04:00",
            "2023-12-15T05:00",
            "2023-12-15T06:00",
            "2023-12-15T07:00",
            "2023-12-15T08:00",
            "2023-12-15T09:00",
            "2023-12-15T10:00",
            "2023-12-15T11:00",
            "2023-12-15T12:00",
            "2023-12-15T13:00",
            "2023-12-15T14:00",
            "2023-12-15T15:00",
            "2023-12-15T16:00",
            "2023-12-15T17:00",
            "2023-12-15T18:00",
            "2023-12-15T19:00",
            "2023-12-15T20:00",
            "2023-12-15T21:00",
            "2023-12-15T22:00",
            "2023-12-15T23:00",
            "2023-12-16T00:00",
            "2023-12-16T01:00",
            "2023-12-16T02:00",
            "2023-12-16T03:00",
            "2023-12-16T04:00",
            "2023-12-16T05:00",
            "2023-12-16T06:00",
            "2023-12-16T07:00",
            "2023-12-16T08:00",
            "2023-12-16T09:00",
            "2023-12-16T10:00",
            "2023-12-16T11:00",
            "2023-12-16T12:00",
            "2023-12-16T13:00",
            "2023-12-16T14:00",
            "2023-12-16T15:00",
            "2023-12-16T16:00",
            "2023-12-16T17:00",
            "2023-12-16T18:00",
            "2023-12-16T19:00",
            "2023-12-16T20:00",
            "2023-12-16T21:00",
            "2023-12-16T22:00",
            "2023-12-16T23:00"
        ],
        "temperature_2m": [
            2.3,
            3.2,
            3.8,
            4.4,
            4.8,
            5.0,
            4.4,
            2.9,
            2.1,
            2.2,
            1.7,
            1.5,
            1.4,
            1.1,
            1.2,
            0.9,
            0.5,
            0.5,
            0.2,
            0.1,
            0.3,
            0.4,
            1.0,
            2.9,
            4.7,
            7.1,
            10.4,
            11.8,
            12.8,
            12.6,
            10.5,
            8.1,
            6.8,
            5.9,
            5.3,
            5.0,
            4.9,
            4.7,
            4.5,
            4.2,
            3.6,
            3.1,
            2.7,
            2.5,
            2.3,
            2.0,
            2.4,
            4.8
        ],
        "apparent_temperature": [
            -2.2,
            -1.6,
            -1.0,
            -0.5,
            -0.7,
            -0.0,
            0.0,
            -1.8,
            -2.0,
            -1.9,
            -2.9,
            -2.6,
            -3.2,
            -3.6,
            -4.0,
            -4.3,
            -4.3,
            -4.2,
            -4.8,
            -4.7,
            -4.2,
            -4.4,
            -3.7,
            -1.7,
            0.6,
            3.5,
            6.9,
            8.4,
            9.5,
            9.5,
            7.6,
            5.1,
            3.9,
            3.1,
            2.4,
            1.8,
            1.6,
            1.3,
            1.0,
            0.6,
            0.1,
            -0.4,
            -0.8,
            -1.1,
            -1.3,
            -1.4,
            -0.6,
            2.9
        ],
        "weather_code": [
            3,
            3,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            1,
            0,
            3,
            3,
            3,
            3
        ],
        "wind_speed_10m": [
            10.3,
            12.1,
            12.9,
            12.8,
            15.9,
            13.0,
            7.8,
            9.9,
            7.7,
            7.7,
            11.6,
            8.3,
            11.5,
            12.7,
            15.8,
            15.1,
            12.5,
            12.1,
            14.6,
            14.0,
            13.1,
            14.6,
            15.0,
            15.8,
            14.3,
            12.9,
            13.8,
            14.1,
            13.3,
            10.8,
            9.7,
            10.4,
            10.2,
            8.9,
            9.2,
            10.5,
            11.6,
            11.3,
            11.3,
            11.4,
            10.4,
            9.9,
            9.4,
            9.7,
            9.4,
            8.7,
            6.0,
            1.8
        ]
    },
    "daily_units": {
        "time": "iso8601",
        "temperature_2m_min": "°C",
        "temperature_2m_max": "°C",
        "apparent_temperature_min": "°C",
        "apparent_temperature_max": "°C",
        "weather_code": "wmo code"
    },
    "daily": {
        "time": [
            "2023-12-15",
            "2023-12-16"
        ],
        "temperature_2m_min": [
            0.1,
            2.0
        ],
        "temperature_2m_max": [
            5.0,
            12.8
        ],
        "apparent_temperature_min": [
            -4.8,
            -1.4
        ],
        "apparent_temperature_max": [
            0.0,
            9.5
        ],
        "weather_code": [
            3,
            3
        ]
    }
}        "#;
        let _parsed: WeatherForecastResp = serde_json::from_str(src).unwrap();
    }
}
