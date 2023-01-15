use embedded_svc::io::Read;
use esp_idf_svc::http::client;
use esp_idf_sys::EspError;
use log::trace;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to setup HTTP client: {0}")]
    HttpClientSetup(#[source] EspError),

    #[error("Failed to read response: nothing to read")]
    EmptyResponse,
}

/// Sets up ESP HTTPS client.
pub fn setup_https_client() -> Result<client::EspHttpConnection, Error> {
    client::EspHttpConnection::new(&client::Configuration {
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })
    .map_err(Error::HttpClientSetup)
}

/// Reads all bytes from the response and returns them without trailing bytes.
/// This function does not handle interrupted error due to the fact that [embedded_io::ErrorKind] contains only [embedded_io::ErrorKind::Other] kind.
pub fn read_response<R: Read>(response: &mut R, buf: &mut Vec<u8>) -> Result<usize, Error> {
    let mut total_bytes_read = 0;

    while let Ok(bytes_read) = response.read(&mut buf[total_bytes_read..]) {
        trace!("Read {} bytes", bytes_read);
        if bytes_read == 0 {
            break;
        } else {
            total_bytes_read += bytes_read;
            buf.resize(buf.len() * 2, 0);
        }
    }

    if total_bytes_read == 0 {
        return Err(Error::EmptyResponse);
    }

    trace!("Truncate {} bytes", buf.len() - total_bytes_read);
    buf.truncate(total_bytes_read);

    Ok(total_bytes_read)
}
