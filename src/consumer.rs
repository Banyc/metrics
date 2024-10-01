use std::collections::{HashMap, VecDeque};

use plotly::{layout::Axis, Layout, Plot, Scatter};
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

    pub async fn scatter_chart_html(
        &self,
        keys: impl Iterator<Item = impl AsRef<str>>,
        range: impl core::ops::RangeBounds<Time> + Clone,
        div_id: Option<&str>,
    ) -> String {
        let mut traces = vec![];
        for key in keys {
            let key: &str = key.as_ref();
            let Some(queue) = self.metrics.get(key) else {
                continue;
            };
            let (a, b) = queue.span(range.clone());
            let x = a.iter().chain(b).map(|x| x.time);
            let y = a.iter().chain(b).map(|x| x.value);
            let trace = Scatter::new(x.collect(), y.collect()).name(key);
            traces.push(trace);
            tokio::task::yield_now().await;
        }
        let mut plot = Plot::new();
        for trace in traces {
            plot.add_trace(trace);
        }
        let layout = Layout::default()
            .x_axis(Axis::default().title("time"))
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
                let pos = slice.iter().position(|sample| match end {
                    std::ops::Bound::Included(&end) => sample.time <= end,
                    std::ops::Bound::Excluded(&end) => sample.time < end,
                    std::ops::Bound::Unbounded => todo!(),
                });
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hyped::*;
    use poem::{
        get, handler,
        middleware::AddData,
        web::{Data, Html, Query},
        EndpointExt, Route, Server,
    };
    use serde::Deserialize;

    use super::*;

    #[tokio::test]
    #[ignore]
    /// <http://127.0.0.1:3000/?keys=a>
    async fn test_web() {
        let mut consumer = MetricConsumer::new(1024);
        let key = String::from("a");
        {
            let mut queue = consumer.push(&key);
            for i in 0..16 {
                queue(Sample {
                    time: i,
                    value: i as f64,
                });
            }
        }

        enum ConsumerMessage {
            Chart(ChartQuery, tokio::sync::oneshot::Sender<String>),
        }
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    ConsumerMessage::Chart(query, resp) => {
                        let keys = query.keys.split(',');
                        let html = consumer.scatter_chart_html(keys, .., None).await;
                        resp.send(html).unwrap();
                    }
                }
            }
        });

        struct AppState {
            pub consumer: tokio::sync::mpsc::Sender<ConsumerMessage>,
        }
        let state = Arc::new(AppState { consumer: tx });

        #[derive(Deserialize)]
        struct ChartQuery {
            pub keys: String,
        }

        #[handler]
        async fn chart(query: Query<ChartQuery>, state: Data<&Arc<AppState>>) -> Html<String> {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let msg = ConsumerMessage::Chart(query.0, tx);
            state.consumer.send(msg).await.unwrap();
            let chart = rx.await.unwrap();
            let chart = danger(chart);
            let math_jax = "https://cdn.jsdelivr.net/npm/mathjax@3.2.2/es5/tex-svg.js";
            let math_jax = script(()).attr("src", math_jax);
            let plotly = "https://cdn.plot.ly/plotly-2.12.1.min.js";
            let plotly = script(()).attr("src", plotly);
            let scripts = (math_jax, plotly);
            let root_div = div((scripts, chart));
            let body = hyped::body(root_div);
            let root = (doctype(), html(body));
            Html(render(root))
        }

        let app = Route::new().at("/", get(chart)).with(AddData::new(state));

        let listener = poem::listener::TcpListener::bind("0.0.0.0:3000");
        Server::new(listener).run(app).await.unwrap();
    }
}
