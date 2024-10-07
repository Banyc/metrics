use std::collections::{HashMap, VecDeque};

use primitive::map::hash_map::HashMapExt;

use crate::{MetricKey, Sample, Time};

pub type MetricQueues = HashMap<MetricKey, MetricQueue>;

#[derive(Debug, Clone)]
pub struct MetricConsumer {
    metrics: MetricQueues,
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
    pub fn metrics(&self) -> &MetricQueues {
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

    pub fn span(&self, range: impl core::ops::RangeBounds<Time>) -> (&[Sample], &[Sample]) {
        let (a, b) = self.buf.as_slices();
        let mut slices = [a, b];
        for slice in &mut slices {
            let start = range.start_bound();
            if start != core::ops::Bound::Unbounded {
                let pos = slice.iter().position(|sample| match start {
                    std::ops::Bound::Included(&start) => start <= sample.time,
                    std::ops::Bound::Excluded(&start) => start < sample.time,
                    std::ops::Bound::Unbounded => unreachable!(),
                });
                *slice = match pos {
                    Some(pos) => &slice[pos..],
                    None => &[],
                };
            }
            let end = range.end_bound();
            if end != core::ops::Bound::Unbounded {
                let pos = slice.iter().rev().position(|sample| match end {
                    std::ops::Bound::Included(&end) => sample.time <= end,
                    std::ops::Bound::Excluded(&end) => sample.time < end,
                    std::ops::Bound::Unbounded => unreachable!(),
                });
                *slice = match pos {
                    Some(pos) => &slice[..slice.len() - pos],
                    None => &[],
                };
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

pub trait TimeSeries {
    fn span(&self, time_range: impl core::ops::RangeBounds<Time>) -> Option<TimeSeriesSpan<'_>>;
}
pub struct TimeSeriesSpan<'a> {
    pub samples: Box<dyn Iterator<Item = Sample> + Send + 'a>,
    pub count: usize,
}
impl TimeSeries for MetricQueue {
    fn span(&self, time_range: impl core::ops::RangeBounds<Time>) -> Option<TimeSeriesSpan<'_>> {
        let (a, b) = self.span(time_range);
        let n = a.len() + b.len();
        let samples = a.iter().chain(b).copied();
        Some(TimeSeriesSpan {
            samples: Box::new(samples),
            count: n,
        })
    }
}
