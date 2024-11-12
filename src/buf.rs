use std::sync::Arc;

use primitive::{
    ops::dyn_ref::DynRef,
    sync::mcast::{MpMcast, MpMcastReader},
};

use crate::{MetricKey, Sample};

pub const BUF_SIZE: usize = 1024;
pub type MetricBuf = MpMcast<Sample, BUF_SIZE>;
pub type MetricBufReader = MpMcastReader<Sample, BUF_SIZE, Arc<MetricBuf>>;

#[derive(Debug, Clone)]
pub struct MetricBufReaders {
    readers: Vec<(MetricKey, MetricBufReader)>,
}
impl MetricBufReaders {
    pub fn new() -> Self {
        Self { readers: vec![] }
    }
    pub fn new_metrics(&mut self, key: MetricKey) -> Arc<MetricBuf> {
        let queue = MetricBuf::new();
        let queue = Arc::new(queue);
        let reader = MpMcastReader::new(DynRef::new(queue.clone(), |q| q));
        self.readers.push((key, reader));
        queue
    }
    pub fn readers_mut(&mut self) -> &mut [(MetricKey, MetricBufReader)] {
        &mut self.readers
    }
}
impl Default for MetricBufReaders {
    fn default() -> Self {
        Self::new()
    }
}
