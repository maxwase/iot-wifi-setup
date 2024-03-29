use std::time::Duration;

use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, sys::EspError};
use log::{error, info};
use thiserror::Error;

use http::setup_https_client;
use server::setup_wifi_setup_server;
use time::utc_time;
use wifi::WifiWrapper;

mod error;
mod http;
mod server;
mod time;
mod wifi;

#[derive(Error, Debug)]
enum Error {
    #[error("Server error")]
    Server(#[from] server::Error),
    #[error("Wi-Fi error")]
    WiFi(#[from] wifi::Error),
    #[error("Time error")]
    Time(#[from] time::Error),
    #[error("Failed to initialize peripherals")]
    InitPeripherals(#[source] EspError),
    #[error("Failed to obtain EspSystemEventLoop")]
    InitEventLoop(#[source] EspError),
    #[error("Failed to init NVS")]
    InitNvs(#[source] EspError),
    #[error("Failed to setup HTTP client")]
    HttpClientSetup(#[source] EspError),
}

fn run() -> Result<(), Error> {
    let peripherals = Peripherals::take().map_err(Error::InitPeripherals)?;
    let sys_loop = EspSystemEventLoop::take().map_err(Error::InitEventLoop)?;
    let nvs = EspDefaultNvsPartition::take().map_err(Error::InitNvs)?;

    let wifi = WifiWrapper::new(peripherals.modem, sys_loop.clone(), nvs)?;

    loop {
        wifi.use_setup_configuration()?;
        info!("Wi-Fi started");

        let credentials = setup_wifi_setup_server(&wifi)?;

        let connection_result = wifi.use_client_configuration(credentials);

        if let Err(e) = connection_result {
            error!("{}", error::OneLineFormatter::new(e));
            continue;
        }

        break;
    }

    let mut client = setup_https_client().map_err(Error::HttpClientSetup)?;

    loop {
        match utc_time(&mut client) {
            Ok(date_time) => info!("UTC date time: {date_time}"),
            Err(e) => error!("{}", error::OneLineFormatter::new(e)),
        };

        std::thread::sleep(Duration::from_secs(5));
    }
}

fn main() -> Result<(), Error> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    if let Err(e) = run() {
        error!("{}", error::OneLineFormatter::new(&e));
        // also print in [Debug]
        return Err(e);
    }
    Ok(())
}
