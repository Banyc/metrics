use std::{collections::HashMap, mem::MaybeUninit};

use plotly::{
    layout::{Axis, AxisType},
    Layout, Plot, Scatter,
};
use primitive::{iter::chunk::Chunks, ops::range::RangeAny};

use crate::{
    consumer::{MetricQueues, TimeSeries, TimeSeriesSpan},
    MetricKey, Time,
};

const MAX_DISPLAY_DATA_POINTS: usize = 1024;

pub trait MetricSynthesis: core::fmt::Debug + Sync + Send {
    fn span(
        &self,
        metrics: &MetricQueues,
        time_range: RangeAny<Time>,
    ) -> Option<TimeSeriesSpan<'_>>;
}
pub type MetricSyntheses = HashMap<MetricKey, Box<dyn MetricSynthesis>>;

pub fn metric_span<'a>(
    metrics: &'a MetricQueues,
    syntheses: &'a MetricSyntheses,
    key: &str,
    time_range: impl core::ops::RangeBounds<Time> + Clone,
) -> Option<TimeSeriesSpan<'a>> {
    let queue = metrics.get(key);
    let synthesis = syntheses.get(key);
    match (queue, synthesis) {
        (Some(queue), _) => TimeSeries::span(queue, time_range.clone()),
        (None, Some(synthesis)) => {
            synthesis.span(metrics, RangeAny::from_range(time_range.clone()))
        }
        (None, None) => None,
    }
}

pub async fn scatter_chart_html(
    metrics: &MetricQueues,
    syntheses: &MetricSyntheses,
    keys: impl Iterator<Item = impl AsRef<str>>,
    time_range: impl core::ops::RangeBounds<Time> + Clone,
    value_range: Option<(f64, f64)>,
    div_id: Option<&str>,
) -> String {
    let mut data_point_count = 0;
    let mut data_sets = vec![];
    for key in keys {
        let span = metric_span(metrics, syntheses, key.as_ref(), time_range.clone());
        let Some(span) = span else {
            continue;
        };
        if span.count == 0 {
            continue;
        }
        data_point_count += span.count;
        data_sets.push((key, span));
        tokio::task::yield_now().await;
    }
    let mut traces = vec![];
    let chunk_size = data_point_count.div_ceil(MAX_DISPLAY_DATA_POINTS);
    let mut tmp_tray = vec![MaybeUninit::uninit(); chunk_size];
    for (key, span) in data_sets {
        let mut reduced_x = vec![];
        let mut reduced_y = vec![];
        span.samples.chunks(&mut tmp_tray, |tray| {
            reduced_x.push(tray.last().unwrap().time);
            reduced_y.push(tray.last().unwrap().value);
        });
        let trace = Scatter::new(reduced_x, reduced_y).name(key);
        traces.push(trace);
        tokio::task::yield_now().await;
    }
    let mut plot = Plot::new();
    for trace in traces {
        plot.add_trace(trace);
    }
    let y = Axis::default().title("value");
    let y = match value_range {
        Some(range) => y.range(vec![range.0, range.1]),
        None => y,
    };
    let layout = Layout::default()
        .x_axis(Axis::default().title("time").type_(AxisType::Date))
        .y_axis(y);
    plot.set_layout(layout);
    plot.to_inline_html(div_id)
}
