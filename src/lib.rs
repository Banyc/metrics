pub mod dump;
pub mod exporter;

type MetricKey = String;

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub time: u64,
    pub value: f64,
}
