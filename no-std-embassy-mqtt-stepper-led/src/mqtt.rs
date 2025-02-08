use embassy_net::tcp::TcpSocket;
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;
use rust_mqtt::utils::rng_generator::CountingRng;

pub struct MQTT<'a> {
    mqtt: &'a mut MqttClient<'a, TcpSocket<'a>, 5, CountingRng>,
}

impl MQTT<'static> {
    pub fn new(mqtt: &'static mut MqttClient<'static, TcpSocket<'static>, 5, CountingRng>) -> Self {
        MQTT { mqtt }
    }

    pub async fn msg(&mut self) -> Result<(&str, &[u8]), ReasonCode> {
        match self.mqtt.receive_message().await {
            Ok(msg) => Ok(msg),
            Err(reason) => Err(reason),
        }
    }

    pub async fn ping(&mut self) -> Result<&'static str, ReasonCode> {
        match self.mqtt.send_ping().await {
            Ok(_) => Ok("pingpong"),
            Err(reason) => Err(reason),
        }
    }
}
