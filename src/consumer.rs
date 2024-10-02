use std::{
    collections::{HashMap, VecDeque},
    mem::MaybeUninit,
};

use plotly::{
    layout::{Axis, AxisType},
    Layout, Plot, Scatter,
};
use primitive::{iter::Chunks, map::hash_map::HashMapExt};

use crate::{MetricKey, Sample, Time};

const MAX_DISPLAY_DATA_POINTS: usize = 1024;

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

    pub async fn scatter_chart_html(
        &self,
        keys: impl Iterator<Item = impl AsRef<str>>,
        range: impl core::ops::RangeBounds<Time> + Clone,
        div_id: Option<&str>,
    ) -> String {
        let mut data_point_count = 0;
        let mut data_sets = vec![];
        for key in keys {
            let Some(queue) = self.metrics.get(key.as_ref()) else {
                continue;
            };
            let (a, b) = queue.span(range.clone());
            let len = a.len() + b.len();
            if len == 0 {
                continue;
            }
            data_point_count += len;
            let x = a.iter().chain(b).map(|x| x.time);
            let y = a.iter().chain(b).map(|x| x.value);
            data_sets.push((key, x, y));
            tokio::task::yield_now().await;
        }
        let mut traces = vec![];
        let chunk_size = data_point_count.div_ceil(MAX_DISPLAY_DATA_POINTS);
        let mut tmp_x_tray = vec![MaybeUninit::uninit(); chunk_size];
        let mut tmp_y_tray = vec![MaybeUninit::uninit(); chunk_size];
        for (key, x, y) in data_sets {
            let mut reduced_x = vec![];
            x.chunks(&mut tmp_x_tray, |tray| {
                reduced_x.push(tray.last().copied().unwrap())
            });
            let mut reduced_y = vec![];
            y.chunks(&mut tmp_y_tray, |tray| {
                reduced_y.push(tray.last().copied().unwrap())
            });
            let trace = Scatter::new(reduced_x, reduced_y).name(key);
            traces.push(trace);
            tokio::task::yield_now().await;
        }
        let mut plot = Plot::new();
        for trace in traces {
            plot.add_trace(trace);
        }
        let layout = Layout::default()
            .x_axis(Axis::default().title("time").type_(AxisType::Date))
            .y_axis(Axis::default().title("value"));
        plot.set_layout(layout);
        plot.to_inline_html(div_id)
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
                if let Some(pos) = pos {
                    *slice = &slice[pos..];
                }
            }
            let end = range.end_bound();
            if end != core::ops::Bound::Unbounded {
                let pos = slice.iter().rev().position(|sample| match end {
                    std::ops::Bound::Included(&end) => sample.time <= end,
                    std::ops::Bound::Excluded(&end) => sample.time < end,
                    std::ops::Bound::Unbounded => unreachable!(),
                });
                if let Some(pos) = pos {
                    *slice = &slice[..slice.len() - pos];
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
