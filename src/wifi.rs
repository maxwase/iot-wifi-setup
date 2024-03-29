use embedded_svc::wifi::Configuration;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    sys::EspError,
    wifi::{AccessPointConfiguration, AccessPointInfo, BlockingWifi, ClientConfiguration, EspWifi},
};
use parking_lot::Mutex;
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to set up ESP WiFi")]
    Setup(#[source] EspError),
    #[error("Failed to wrap ESP WiFi")]
    Wrap(#[source] EspError),
    #[error("Failed to scan WiFi")]
    Scan(#[source] EspError),
    #[error("Failed to start WiFi")]
    Start(#[source] EspError),
    #[error("Failed to connect to WiFi")]
    Connect(#[source] EspError),
    #[error("Failed to wait WiFi set up")]
    Wait(#[source] EspError),
    #[error("Failed to configure ESP WiFI")]
    Configuration(#[source] EspError),
}

type Ssid = heapless::String<32>;
type Password = heapless::String<64>;

#[derive(Deserialize, Clone)]
pub struct Credentials {
    ssid: Ssid,
    password: Password,
}

impl From<Credentials> for ClientConfiguration {
    fn from(Credentials { ssid, password }: Credentials) -> Self {
        ClientConfiguration {
            ssid,
            password,
            ..Default::default()
        }
    }
}

/// A wrapper over [BlockingWifi] to use in the setup-server handler.
pub struct WifiWrapper<'a>(Mutex<BlockingWifi<EspWifi<'a>>>);

impl<'a> WifiWrapper<'a> {
    pub fn new(
        modem: Modem,
        sys_loop: EspSystemEventLoop,
        nvs: EspDefaultNvsPartition,
    ) -> Result<Self, Error> {
        let wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs)).map_err(Error::Setup)?;
        let wifi = BlockingWifi::wrap(wifi, sys_loop.clone()).map_err(Error::Wrap)?;
        Ok(Self(parking_lot::Mutex::new(wifi)))
    }

    /// Configures the modem to host the server and scan for networks.
    pub fn use_setup_configuration(&self) -> Result<(), Error> {
        // Wi-Fi scan is not impossible to scan in the Access Point mode, that's why we need
        // to connect in a client mode as well, even though we are not planning to connect
        let config = Configuration::Mixed(
            Default::default(),
            AccessPointConfiguration {
                ssid: "UTC-Fetcher-Setup".try_into().expect("Short SSID name"),
                ..Default::default()
            },
        );
        let mut wifi = self.0.lock();
        wifi.set_configuration(&config)
            .map_err(Error::Configuration)?;

        wifi.start().map_err(Error::Start)
    }

    /// Connects the board to the SSID with the provided [Credentials].
    pub fn use_client_configuration(&self, credentials: Credentials) -> Result<(), Error> {
        let config = Configuration::Client(credentials.into());

        let mut wifi = self.0.lock();
        wifi.set_configuration(&config)
            .map_err(Error::Configuration)?;

        wifi.connect().map_err(Error::Connect)?;
        wifi.wait_netif_up().map_err(Error::Wait)
    }

    /// Scans the network
    pub fn scan(&self) -> Result<Vec<AccessPointInfo>, Error> {
        self.0.lock().scan().map_err(Error::Scan)
    }
}
