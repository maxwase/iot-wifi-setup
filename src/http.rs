use embedded_svc::io::Read;
use esp_idf_svc::{http::client, sys::EspError};
use log::trace;

/// Sets up ESP HTTPS client.
pub fn setup_https_client() -> Result<client::EspHttpConnection, EspError> {
    client::EspHttpConnection::new(&client::Configuration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    })
}

/// Reads all bytes from the response and returns them without trailing bytes, returns [None] if the response is empty.
pub fn read_response<R: Read>(response: &mut R, buf: &mut Vec<u8>) -> Option<usize> {
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
        return None;
    }

    trace!("Truncate {} bytes", buf.len() - total_bytes_read);
    buf.truncate(total_bytes_read);

    Some(total_bytes_read)
}
