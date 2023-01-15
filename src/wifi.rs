use std::time::Duration;

use embedded_svc::wifi::{Configuration, Wifi};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    netif::{EspNetif, EspNetifWait},
    wifi::{EspWifi, WifiWait},
};
use esp_idf_sys::EspError;
use log::{error, trace};
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to set up ESP WiFi {0}")]
    Setup(#[source] EspError),
    #[error("Failed to scan WiFi {0}")]
    Scan(#[source] EspError),
    #[error("Failed to start WiFi {0}")]
    Start(#[source] EspError),
    #[error("Failed to connect to WiFi {0}")]
    Connect(#[source] EspError),
    #[error("Failed to wait WiFi set up {0}")]
    Wait(#[source] EspError),
    #[error("WiFi did not start")]
    WaitStart,
    #[error("WiFi did not connect")]
    WaitConnect,
    #[error("Failed to configure ESP WiFI {0}")]
    Configuration(#[source] EspError),
}

type Ssid = heapless::String<32>;
type Password = heapless::String<64>;

#[derive(Deserialize, Clone)]
pub struct Credentials {
    ssid: String,
    password: String,
}

impl Credentials {
    pub fn ssid(&self) -> Ssid {
        self.ssid.as_str().into()
    }

    pub fn password(&self) -> Password {
        self.password.as_str().into()
    }
}

/// Setup wifi configuration.
pub fn set_wifi_configuration(
    wifi: &mut EspWifi,
    sys_loop: &EspSystemEventLoop,
    configuration: Configuration,
) -> Result<(), Error> {
    wifi.set_configuration(&configuration)
        .map_err(Error::Configuration)?;
    wifi.start().map_err(Error::Start)?;

    if !WifiWait::new(sys_loop)
        .map_err(Error::Wait)?
        .wait_with_timeout(Duration::from_secs(20), || {
            wifi.is_started().unwrap_or_default()
        })
    {
        return Err(Error::WaitStart);
    }

    trace!("Wifi started");
    // nowhere to connect
    if let Configuration::AccessPoint(_) = configuration {
        return Ok(());
    }

    wifi.connect().map_err(Error::Connect)?;

    if !EspNetifWait::new::<EspNetif>(wifi.sta_netif(), sys_loop)
        .map_err(Error::Wait)?
        .wait_with_timeout(Duration::from_secs(20), || {
            wifi.is_connected().unwrap_or_default()
                && wifi
                    .sta_netif()
                    .get_ip_info()
                    .map(|info| !info.ip.is_unspecified())
                    .unwrap_or_default()
        })
    {
        return Err(Error::WaitConnect);
    }

    trace!("Wifi connected");
    Ok(())
}
