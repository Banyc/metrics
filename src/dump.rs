use std::sync::Arc;

use primitive::{
    dyn_ref::DynRef,
    sync::spmc::{MpmcQueue, MpmcQueueReader},
};

use crate::{MetricKey, Sample};

pub const QUEUE_SIZE: usize = 1024;
pub type MetricQueue = MpmcQueue<Sample, QUEUE_SIZE>;
pub type MetricQueueReader = MpmcQueueReader<Sample, QUEUE_SIZE, Arc<MetricQueue>>;

#[derive(Debug, Clone)]
pub struct MetricQueueReaders {
    readers: Vec<(MetricKey, MetricQueueReader)>,
}
impl MetricQueueReaders {
    pub fn new() -> Self {
        Self { readers: vec![] }
    }
    pub fn new_metrics(&mut self, key: MetricKey) -> Arc<MetricQueue> {
        let queue = MetricQueue::new();
        let queue = Arc::new(queue);
        let reader = MpmcQueueReader::new(DynRef::new(queue.clone(), |q| q));
        self.readers.push((key, reader));
        queue
    }
    pub fn readers_mut(&mut self) -> &mut [(MetricKey, MetricQueueReader)] {
        &mut self.readers
    }
}
impl Default for MetricQueueReaders {
    fn default() -> Self {
        Self::new()
    }
}
