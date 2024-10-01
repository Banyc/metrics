use std::collections::{HashMap, VecDeque};

use primitive::map::hash_map::HashMapExt;

use crate::{MetricKey, Sample, Time};

#[derive(Debug, Clone)]
pub struct MetricConsumer {
    metrics: HashMap<MetricKey, MetricQueue>,
    queue_size: usize,
}
impl MetricConsumer {
    pub fn new(queue_size: usize) -> Self {
        Self {
            metrics: HashMap::new(),
            queue_size,
        }
    }

    pub fn push(&mut self, key: &MetricKey) -> impl FnMut(Sample) + use<'_> {
        let queue = self.metrics.ensure(key, MetricQueue::new);
        |sample: Sample| {
            queue.push(sample, self.queue_size);
        }
    }
    pub fn metrics(&self) -> &HashMap<MetricKey, MetricQueue> {
        &self.metrics
    }
}

#[derive(Debug, Clone)]
pub struct MetricQueue {
    buf: VecDeque<Sample>,
}
impl MetricQueue {
    pub fn new() -> Self {
        let buf = VecDeque::new();
        Self { buf }
    }

    pub fn push(&mut self, sample: Sample, queue_size: usize) {
        let queue_size = self.buf.capacity().max(queue_size);
        if self.buf.len() == queue_size {
            self.buf.pop_front();
        }
        self.buf.push_back(sample);
    }

    pub fn span(&self, range: core::ops::Range<Option<Time>>) -> (&[Sample], &[Sample]) {
        let (a, b) = self.buf.as_slices();
        let mut slices = [a, b];
        for slice in &mut slices {
            if let Some(start) = range.start {
                let pos = slice.iter().position(|sample| start <= sample.time);
                if let Some(pos) = pos {
                    *slice = &slice[pos..];
                }
            }
            if let Some(end) = range.end {
                let pos = slice.iter().position(|sample| sample.time < end);
                if let Some(pos) = pos {
                    *slice = &slice[..pos];
                }
            }
        }
        (slices[0], slices[1])
    }
}
impl Default for MetricQueue {
    fn default() -> Self {
        Self::new()
    }
}
