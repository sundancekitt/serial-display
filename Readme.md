# Display sensor data from serial port in the web browser.

## Description

This small program displays live data received on a serial port in the web browser. I developed this
in the context of an Arduino project to display temperature and humidity data of a 
_Adafruit Si7021 sensor_ connected to an Arduino UNO. The Arduino itself was connected to a 
_Raspberry Pi_ via USB serial connection where it periodically wrote the newest sensor data to. 
This program reads out the serial port and displays the data on a local web server. 

It uses [Actix](https://actix.rs/) built-in Websockets handler to push new data to the frontend.
Currently, the serial port is hardcoded to `/dev/ttyUSB0` in `src/serial.rs` and the serial data is
assumed to be in the format 'temperature, humidity'. See `static/index.html` for processing of
the data.
