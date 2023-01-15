use std::fmt::Display;

use embedded_svc::http::Method;
use esp_idf_svc::http::client;
use esp_idf_sys::EspError;
use serde::{Deserialize, Deserializer};
use thiserror::Error;
use time::{format_description::FormatItem, macros::format_description, PrimitiveDateTime};

use crate::http::{self, read_response};

const URL: &str = "https://www.timeapi.io/api/Time/current/zone?timeZone=UTC";
const FORMAT: &[FormatItem] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]");

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Http(#[from] http::Error),
    #[error("Failed to initiate response: {0}")]
    InitiateResponse(#[source] EspError),
    #[error("Failed to submit request: {0}")]
    SubmitRequest(#[source] EspError),
    #[error("Failed to parse response as `TimeResult`: {0}")]
    TimeResultDeserialize(#[source] serde_json::Error),
    #[error("Failed to parse date/time: {0}")]
    DateTimeParse(time::error::Parse),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DateTime {
    #[serde(deserialize_with = "deserialize_date_time")]
    date_time: PrimitiveDateTime,
}

impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.date_time.to_string())
    }
}

impl DateTime {
    const DEFAULT_RESPONSE_LEN: usize = 366;
}

fn deserialize_date_time<'de, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let date_time = String::deserialize(deserializer)?;

    PrimitiveDateTime::parse(&date_time, FORMAT)
        .map_err(Error::DateTimeParse)
        .map_err(serde::de::Error::custom)
}

pub fn utc_time(client: &mut client::EspHttpConnection) -> Result<DateTime, Error> {
    client
        .initiate_request(Method::Get, URL, &[])
        .map_err(Error::InitiateResponse)?;

    client.initiate_response().map_err(Error::SubmitRequest)?;

    // pre-allocate some memory for the response, `read_response` will allocate more if needed
    let mut body = vec![0; DateTime::DEFAULT_RESPONSE_LEN];

    read_response(client, &mut body)?;

    serde_json::from_slice(&body).map_err(Error::TimeResultDeserialize)
}
