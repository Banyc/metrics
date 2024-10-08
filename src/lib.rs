pub mod alert;
pub mod buf;
pub mod codec;
pub mod consumer;
pub mod exporter;
pub mod view;

pub type MetricKey = String;
pub type Time = u64;

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub time: Time,
    pub value: f64,
}
const SAMPLE_SIZE: usize = core::mem::size_of::<Sample>();
