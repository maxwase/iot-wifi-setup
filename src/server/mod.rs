use std::sync::Arc;

use embedded_svc::{
    http::{server::Request, Method},
    io::Write,
    wifi::AccessPointInfo,
};
use esp_idf_hal::io::EspIOError;
use esp_idf_svc::{
    http::server::{EspHttpConnection, EspHttpServer},
    sys::EspError,
};
use log::info;
use parking_lot::{Condvar, Mutex};
use thiserror::Error;

use crate::{
    http::read_response,
    wifi::{self, Credentials, WifiWrapper},
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to setup HTTP client")]
    HttpHandle(#[source] EspError),
    #[error("Failed to setup ESP server")]
    SetupEspServer(#[source] EspIOError),
    #[error("Failed to get credentials")]
    NoCredentials,
    #[error("Failed to write a response content")]
    Write(#[source] EspIOError),
    #[error("Failed to initiate a response")]
    OkResp(#[source] EspIOError),
    #[error("Failed to serialize credentials")]
    Serialize(#[source] serde::de::value::Error),
    #[error(transparent)]
    Scan(#[from] wifi::Error),
}

/// Sets up HTTP server on ESP, waits for credentials from a user.
/// The IP can be found from logs, in my case it was: `I (708) esp_netif_lwip: DHCP server started on interface WIFI_AP_DEF with IP: 192.168.71.1`
pub fn setup_wifi_setup_server(wifi: &WifiWrapper) -> Result<Credentials, Error> {
    let mut server = EspHttpServer::new(&Default::default()).map_err(Error::SetupEspServer)?;

    let pair = Arc::new((Mutex::new(None), Condvar::new()));
    let pair_handler = pair.clone();

    server
        .fn_handler("/", Method::Get, move |resp| select_wifi(wifi, resp))
        .map_err(Error::HttpHandle)?
        .fn_handler("/wifi_select", Method::Post, move |req| {
            wifi_selected(&pair_handler, req)
        })
        .map_err(Error::HttpHandle)?;

    info!("Server is running");

    {
        let (credentials, ready) = &*pair;
        let mut credentials_guard = credentials.lock();
        ready.wait(&mut credentials_guard);
    }
    // Got the credentials, we don't need the server anymore, drop it to [Drop] the [Arc] reference.
    drop(server);

    let (credentials, _) = Arc::into_inner(pair).expect("Dropped server");
    credentials.into_inner().ok_or(Error::NoCredentials)
}

/// Scans and generates the selection page. Due to the fact that the `scan` method
/// takes a couple of seconds, the page loads a bit long.
/// Wi-Fi networks are updated on the page reload.
fn select_wifi(
    wifi: &WifiWrapper<'_>,
    resp: Request<&mut EspHttpConnection<'_>>,
) -> Result<(), Error> {
    let ap_infos = wifi.scan()?;
    let selector = generate_wifi_selector(&ap_infos);

    let content = format!(include_str!("index.html"), selector);
    resp.into_ok_response()
        .map_err(Error::OkResp)?
        .write_all(content.as_bytes())
        .map_err(Error::Write)
}

/// A submission page.
fn wifi_selected(
    pair: &(Mutex<Option<Credentials>>, Condvar),
    mut req: Request<&mut EspHttpConnection>,
) -> Result<(), Error> {
    let (credentials_mutex, ready) = &pair;

    let mut body = vec![0; 40];
    read_response(&mut req, &mut body).unwrap();

    let credentials = serde_urlencoded::from_bytes(&body).map_err(Error::Serialize)?;
    let content = "
            <!DOCTYPE html>\n\
            <html>\n\
                <body>\n\
                    Submitted!
                </body>\n\
            </html>";
    req.into_ok_response()
        .map_err(Error::OkResp)?
        .write_all(content.as_bytes())
        .map_err(Error::Write)?;

    *credentials_mutex.lock() = Some(credentials);
    ready.notify_all();
    Ok(())
}

/// Generates HTML `select` variants.
fn generate_wifi_selector(wifi: &[AccessPointInfo]) -> String {
    wifi.iter()
        .map(|access_point| format!("<option>{}</option>", access_point.ssid))
        .collect()
}
