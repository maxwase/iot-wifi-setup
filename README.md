# UTC IoT fetcher

This repo is an example of how to configure a Wi-Fi client on an ESP32-based IoT device using an external device connection to the internal ESP32 HTTP server using the `esp-idf-svc` crate.
As a connection test, the app fetches UTC time once in 5 secs.

## Build & Flash
```sh
cargo build --release
espflash /dev/ttyUSB0 target/riscv32imc-esp-espidf/release/esp-wifi-web-setup --monitor
```