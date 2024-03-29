use core::fmt::Display;

use embedded_svc::http::Method;
use esp_idf_svc::{http::client, sys::EspError};
use serde::{Deserialize, Deserializer};
use thiserror::Error;
use time::{format_description::FormatItem, macros::format_description, PrimitiveDateTime};

use crate::http::read_response;

const URL: &str = "https://www.timeapi.io/api/Time/current/zone?timeZone=UTC";
const FORMAT: &[FormatItem] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]");

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to initiate response")]
    InitiateResponse(#[source] EspError),
    #[error("Failed to submit request")]
    SubmitRequest(#[source] EspError),
    #[error("Failed to parse response as `TimeResult`")]
    TimeResultDeserialize(#[source] serde_json::Error),
    #[error("Failed to parse date/time")]
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
        self.date_time.fmt(f)
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

/// Requests the time from [URL] and parses the response.
pub fn utc_time(client: &mut client::EspHttpConnection) -> Result<DateTime, Error> {
    client
        .initiate_request(Method::Get, URL, &[])
        .map_err(Error::InitiateResponse)?;

    client.initiate_response().map_err(Error::SubmitRequest)?;

    // pre-allocate some memory for the response, `read_response` will allocate more if needed
    let mut body = vec![0; DateTime::DEFAULT_RESPONSE_LEN];

    read_response(client, &mut body).unwrap();

    serde_json::from_slice(&body).map_err(Error::TimeResultDeserialize)
}
