use embedded_svc::{
    http::{
        server::{HandlerError, Request},
        Method,
    },
    io::Write,
    wifi::AccessPointInfo,
};
use esp_idf_svc::{
    errors::EspIOError,
    http::server::{EspHttpConnection, EspHttpServer},
};
use esp_idf_sys::EspError;
use log::info;
use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use thiserror::Error;

use crate::{http::read_response, wifi::Credentials};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to setup HTTP client: {0}")]
    HttpHandle(#[source] EspError),

    #[error("Failed to setup ESP server: {0}")]
    SetupEspServer(#[source] EspIOError),

    #[error("Failed to get credentials")]
    NoCredentials,
}

/// Setups HTTP server on ESP.
pub fn setup_wifi_setup_server(ap_infos: Vec<AccessPointInfo>) -> Result<Credentials, Error> {
    let mut server = EspHttpServer::new(&Default::default()).map_err(Error::SetupEspServer)?;

    let selector = generate_wifi_selector(&ap_infos);

    let pair = Arc::new((Mutex::new(None), Condvar::new()));
    let pair_handler = pair.clone();

    server
        .fn_handler("/", Method::Get, move |resp| {
            let content = format!(include_str!("index.html"), selector);
            resp.into_ok_response()?.write_all(content.as_bytes())?;

            Ok(())
        })
        .map_err(Error::HttpHandle)?
        .fn_handler("/wifi_select", Method::Post, move |req| {
            wifi_selected(&pair_handler, req)
        })
        .map_err(Error::HttpHandle)?;

    info!("Server is running on http://192.168.71.1/");

    let (credentials, ready) = &*pair;
    let mut credentials_guard = credentials.lock();
    ready.wait(&mut credentials_guard);

    credentials_guard
        .as_ref()
        .cloned()
        .ok_or(Error::NoCredentials)
}

/// Handler for wifi selection.
fn wifi_selected(
    pair: &(Mutex<Option<Credentials>>, Condvar),
    mut req: Request<&mut EspHttpConnection>,
) -> Result<(), HandlerError> {
    let (credentials_mutex, ready) = &pair;

    let mut body = vec![0; 40];
    read_response(&mut req, &mut body)?;

    let credentials = serde_urlencoded::from_bytes(&body)?;
    let content = r#"
            <!DOCTYPE html>
            <html>
                <body>
                    Submitted!
                </body>
            </html>
            "#;
    req.into_ok_response()?.write_all(content.as_bytes())?;

    *credentials_mutex.lock() = Some(credentials);
    ready.notify_all();
    Ok(())
}

fn generate_wifi_selector(wifi: &[AccessPointInfo]) -> String {
    wifi.iter()
        .map(|access_point| format!("<option>{}</option>", access_point.ssid))
        .collect()
}
