use std::time::Duration;

use crate::time::utc_time;

use embedded_svc::wifi::{AccessPointConfiguration, ClientConfiguration, Configuration};
use esp_idf_hal::modem::WifiModem;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::EspWifi};
use http::setup_https_client;
use log::{error, info};
use server::setup_wifi_setup_server;
use thiserror::Error;
use wifi::set_wifi_configuration;

mod http;
mod server;
mod time;
mod wifi;

#[derive(Error, Debug)]
enum Error {
    #[error("{0}")]
    Http(#[from] http::Error),
    #[error("{0}")]
    Server(#[from] server::Error),
    #[error("{0}")]
    WiFi(#[from] wifi::Error),
    #[error("{0}")]
    Time(#[from] crate::time::Error),
}

fn main() -> Result<(), Error> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Safety: This is the only place of the modem initialization
    let wifi_modem = unsafe { WifiModem::new() };
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut wifi =
        EspWifi::new(wifi_modem, sys_loop.clone(), Some(nvs)).map_err(wifi::Error::Setup)?;

    loop {
        // scan all AP before setting up server as scanning operation requires wifi disconnection
        let ap_info = wifi.scan().map_err(wifi::Error::Scan)?;

        set_wifi_configuration(
            &mut wifi,
            &sys_loop,
            Configuration::AccessPoint(AccessPointConfiguration {
                ssid: "UTC-Fetcher-Setup".into(),
                ..Default::default()
            }),
        )?;

        let credentials = setup_wifi_setup_server(ap_info)?;

        let connection_result = set_wifi_configuration(
            &mut wifi,
            &sys_loop,
            Configuration::Client(ClientConfiguration {
                ssid: credentials.ssid(),
                password: credentials.password(),
                ..Default::default()
            }),
        );

        if let Err(e) = connection_result {
            error!("{e}");
            continue;
        }

        break;
    }

    let mut client = setup_https_client()?;

    loop {
        let date_time = utc_time(&mut client)?;
        info!("UTC date time: {date_time}");

        std::thread::sleep(Duration::from_secs(5));
    }
}
