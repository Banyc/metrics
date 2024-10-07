use std::{collections::HashMap, mem::MaybeUninit};

use plotly::{
    layout::{Axis, AxisType},
    Layout, Plot, Scatter,
};
use primitive::{iter::Chunks, ops::range::RangeAny};

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

pub async fn scatter_chart_html(
    metrics: &MetricQueues,
    syntheses: &HashMap<MetricKey, Box<dyn MetricSynthesis>>,
    keys: impl Iterator<Item = impl AsRef<str>>,
    time_range: impl core::ops::RangeBounds<Time> + Clone,
    value_range: Option<(f64, f64)>,
    div_id: Option<&str>,
) -> String {
    let mut data_point_count = 0;
    let mut data_sets = vec![];
    for key in keys {
        let queue = metrics.get(key.as_ref());
        let synthesis = syntheses.get(key.as_ref());
        let span = match (queue, synthesis) {
            (Some(queue), _) => TimeSeries::span(queue, time_range.clone()),
            (None, Some(synthesis)) => {
                synthesis.span(metrics, RangeAny::from_range(time_range.clone()))
            }
            (None, None) => continue,
        };
        let Some(span) = span else {
            continue;
        };
        if span.n == 0 {
            continue;
        }
        data_point_count += span.n;
        data_sets.push((key, span.x, span.y));
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
