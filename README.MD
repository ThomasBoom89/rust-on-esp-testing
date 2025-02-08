# Blazingly fast rust on esp testing

## no-std-embassy-mqtt-stepper-led

This directory contains a test setup with common smarthome tasks.

- connection to wifi and mqtt
- controlling a stepper motor
- controlling a rgb led
- docker setup for a mqtt broker

Powered by the amazing embassy async framework.

THIS IS NOT PRODUCTION READY!!!
The stepper task is blocking for several seconds and while its running mqtt/wifi connection can disconnect.
This is only for testing purpose!