#![no_main]
#![no_std]

extern crate alloc;

mod led;
mod modor;
mod mqtt;

use crate::led::Color;
use alloc::string::String;
#[allow(unused_imports)]
use esp_backtrace as _;

use crate::led::SuperEzLed;
use crate::modor::SuperSimpleModor;
use crate::mqtt::MQTT;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_net::{tcp::TcpSocket, Runner, StackResources};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};

use esp_hal::rng::Rng;
use esp_hal::{clock::CpuClock, timer::timg::TimerGroup};
use esp_println::println;
use esp_wifi::wifi::{
    ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
    WifiState,
};
use esp_wifi::{init, EspWifiController};
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;
use rust_mqtt::utils::rng_generator::CountingRng;
use smoltcp::wire::{IpAddress, IpEndpoint};

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());

    #[allow(unused)]
    let peripherals = esp_hal::init(config);
    esp_println::logger::init_logger_from_env();
    esp_alloc::heap_allocator!(72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);

    let init = &*mk_static!(
        EspWifiController<'static>,
        init(timg0.timer0, rng.clone(), peripherals.RADIO_CLK).unwrap()
    );

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timg1.timer0);

    let config = embassy_net::Config::dhcpv4(Default::default());

    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<5>, StackResources::<5>::new()),
        seed,
    );

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    let rx_buffer = mk_static!([u8; 128], [0; 128]);
    let tx_buffer = mk_static!([u8; 128], [0; 128]);
    let addr = IpEndpoint::new(IpAddress::v4(10, 0, 10, 125), 1883);

    let mut socket = TcpSocket::new(stack, rx_buffer, tx_buffer);
    // socket.set_timeout(Some(embassy_time::Duration::from_secs(20)));
    // socket.set_keep_alive(Some(embassy_time::Duration::from_secs(5))
    // socket.set_timeout(None);

    let connection = socket.connect(addr).await;
    if let Err(e) = connection {
        println!("connect error: {:?}", e);
    }
    println!("socket connected!");

    let mut config = ClientConfig::new(
        rust_mqtt::client::client_config::MqttVersion::MQTTv5,
        CountingRng(20000),
    );
    config.add_max_subscribe_qos(rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1);
    config.add_client_id("clientId-8rhWgBODClsdfsdfsdfsdfsdfsdfsdfsdf");
    config.max_packet_size = 10000;
    // config.keep_alive = 5;

    let recv_buffer = mk_static!([u8; 128], [0; 128]);
    let write_buffer = mk_static!([u8; 128], [0; 128]);

    let client = mk_static!(
        MqttClient<'static, TcpSocket<'static>, 5, CountingRng>,
        MqttClient::<'static, _, 5, _>::new(socket, write_buffer, 128, recv_buffer, 128, config)
    );
    match client.connect_to_broker().await {
        Ok(()) => {
            println!("mqtt connected!")
        }
        Err(mqtt_error) => match mqtt_error {
            ReasonCode::NetworkError => {
                println!("MQTT Network Error");
            }
            _ => {
                println!("Other MQTT Error: {:?}", mqtt_error);
            }
        },
    }

    let topics = mk_static!(
        heapless::Vec<&str,2>,
        heapless::Vec::from_slice(&["led-color-esp32","modor-esp32"]).unwrap()
    );

    client.subscribe_to_topics(topics).await.unwrap();

    let super_ez_led = SuperEzLed::new(peripherals.GPIO8.into(), peripherals.RMT);
    let modor = SuperSimpleModor::new(
        peripherals.GPIO20.into(),
        peripherals.GPIO21.into(),
        peripherals.GPIO22.into(),
        peripherals.GPIO23.into(),
    );

    let modor_channel =
        mk_static!(Channel<NoopRawMutex, bool, 20>, Channel::<NoopRawMutex, bool, 20>::new());
    let led_channel =
        mk_static!(Channel<NoopRawMutex, String, 20>, Channel::<NoopRawMutex,String, 20>::new());

    let mqtt: MQTT<'static> = MQTT::new(client);
    spawner.spawn(modor_loop(modor, modor_channel)).ok();
    spawner.spawn(led_loop(super_ez_led, led_channel)).ok();

    spawner
        .spawn(mqtt_keep_alive(mqtt, led_channel, modor_channel))
        .ok();
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());

    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: "".parse().unwrap(),
                password: "".parse().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn modor_loop(
    mut modor: SuperSimpleModor<'static>,
    modor_channel: &'static Channel<NoopRawMutex, bool, 20>,
) {
    loop {
        if modor_channel.receive().await {
            modor.steps(512)
        }
    }
}

#[embassy_executor::task]
async fn led_loop(mut led: SuperEzLed, led_channel: &'static Channel<NoopRawMutex, String, 20>) {
    loop {
        match led_channel.receive().await.as_str() {
            "red" => led.set_color(Color::Red),
            "green" => led.set_color(Color::Green),
            "blue" => led.set_color(Color::Blue),
            "yellow" => led.set_color(Color::Yellow),
            "orange" => led.set_color(Color::Orange),
            "purple" => led.set_color(Color::Purple),
            "strongorange" => led.set_color(Color::StrongOrange),
            _ => {}
        };
    }
}

#[embassy_executor::task]
async fn mqtt_keep_alive(
    mut mqtt: MQTT<'static>,
    led_channel: &'static Channel<NoopRawMutex, String, 20>,
    modor_channel: &'static Channel<NoopRawMutex, bool, 20>,
) {
    loop {
        match select(mqtt.msg(), Timer::after(Duration::from_secs(45))).await {
            Either::First(msg) => {
                // Received message!
                match msg {
                    Ok(msg) => {
                        let topic = msg.0;
                        let message = core::str::from_utf8(msg.1).unwrap();
                        match topic {
                            "led-color-esp32" => led_channel.send(String::from(message)).await,
                            "modor-esp32" => {
                                if message == "start" {
                                    modor_channel.send(true).await;
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(reason) => {
                        println!("mqtt msg_when_ready failed: {:?}", reason);
                    }
                }
            }
            Either::Second(_timeout) => {
                // Send ping
                match mqtt.ping().await {
                    Ok(_) => {
                        println!("MQTT ping ok");
                    }
                    Err(reason) => {
                        println!("mqtt.ping() failed: {:?}", reason);
                    }
                }
            }
        }
    }
}
