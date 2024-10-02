use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hyped::*;
use metrics::{
    buf::MetricBufReaders, consumer::MetricConsumer, exporter::InProcessExporter, Sample, Time,
};
use poem::{
    get, handler,
    middleware::AddData,
    web::{Data, Html, Query},
    EndpointExt, Route, Server,
};
use serde::Deserialize;

#[tokio::main]
async fn main() {
    let mut metric_buf_readers = MetricBufReaders::new();
    let cpu_metrics = metric_buf_readers.new_metrics("cpu".into());
    let mem_metrics = metric_buf_readers.new_metrics("mem".into());
    let swap_metrics = metric_buf_readers.new_metrics("swap".into());
    let mem_used_metrics = metric_buf_readers.new_metrics("mem.used".into());
    let mem_total_metrics = metric_buf_readers.new_metrics("mem.total".into());
    let mem_free_metrics = metric_buf_readers.new_metrics("mem.free".into());
    let mem_available_metrics = metric_buf_readers.new_metrics("mem.available".into());
    let swap_used_metrics = metric_buf_readers.new_metrics("swap.used".into());
    let swap_total_metrics = metric_buf_readers.new_metrics("swap.total".into());
    let swap_free_metrics = metric_buf_readers.new_metrics("swap.free".into());
    std::thread::spawn(move || {
        let now = || {
            u64::try_from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
            )
            .unwrap()
        };
        let mut sys = sysinfo::System::new_all();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            sys.refresh_all();
            let now = now();
            cpu_metrics.try_push(Sample {
                time: now,
                value: sys.global_cpu_usage() as f64 / 100.,
            });
            mem_metrics.try_push(Sample {
                time: now,
                value: sys.used_memory() as f64 / sys.total_memory() as f64,
            });
            swap_metrics.try_push(Sample {
                time: now,
                value: sys.used_swap() as f64 / sys.total_swap() as f64,
            });
            mem_free_metrics.try_push(Sample {
                time: now,
                value: sys.free_memory() as f64,
            });
            mem_available_metrics.try_push(Sample {
                time: now,
                value: sys.available_memory() as f64,
            });
            mem_total_metrics.try_push(Sample {
                time: now,
                value: sys.total_memory() as f64,
            });
            mem_used_metrics.try_push(Sample {
                time: now,
                value: sys.used_memory() as f64,
            });
            swap_used_metrics.try_push(Sample {
                time: now,
                value: sys.used_swap() as f64,
            });
            swap_total_metrics.try_push(Sample {
                time: now,
                value: sys.total_swap() as f64,
            });
            swap_free_metrics.try_push(Sample {
                time: now,
                value: sys.free_swap() as f64,
            });
        }
    });
    let mut exporter = InProcessExporter::new(metric_buf_readers);

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
        let flush_interval = Duration::from_secs(1);
        loop {
            tokio::select! {
                () = tokio::time::sleep(flush_interval) => exporter.flush(&mut consumer).await,
                Some(msg) = rx.recv() => handle_msg(msg, &consumer).await,
            }
        }
        async fn handle_msg(msg: ConsumerMessage, consumer: &MetricConsumer) {
            match msg {
                ConsumerMessage::Chart(query, resp) => {
                    let keys = query.keys.split(',');
                    let html = match (query.start, query.end) {
                        (None, None) => consumer.scatter_chart_html(keys, .., None).await,
                        (Some(start), None) => {
                            consumer.scatter_chart_html(keys, start.., None).await
                        }
                        (Some(start), Some(end)) => {
                            consumer.scatter_chart_html(keys, start..=end, None).await
                        }
                        (None, Some(end)) => consumer.scatter_chart_html(keys, ..=end, None).await,
                    };
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
        pub start: Option<Time>,
        pub end: Option<Time>,
    }

    #[handler]
    async fn chart(query: Query<ChartQuery>, state: Data<&Arc<AppState>>) -> Html<String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = ConsumerMessage::Chart(query.0, tx);
        state.consumer.send(msg).await.unwrap();
        let chart = rx.await.unwrap();
        let chart = danger(chart);
        let math_jax = "https://cdn.jsdelivr.net/npm/mathjax@3.2.2/es5/tex-svg.js";
        let math_jax = script(()).src(math_jax);
        let plotly = "https://cdn.plot.ly/plotly-2.12.1.min.js";
        let plotly = script(()).src(plotly);
        let scripts = (math_jax, plotly);
        let root_div = div((scripts, chart));
        let body = hyped::body(root_div);
        let root = (doctype(), html(body));
        Html(render(root))
    }

    let app = Route::new().at("/", get(chart)).with(AddData::new(state));

    let listener = poem::listener::TcpListener::bind("0.0.0.0:3000");
    println!("- a: <http://127.0.0.1:3000/?keys=a&start=0&end=15>");
    println!("- usage: <http://127.0.0.1:3000/?keys=cpu,mem,swap>");
    println!("- mem: <http://127.0.0.1:3000/?keys=mem.free,mem.available,mem.used,mem.total>");
    println!("- swap: <http://127.0.0.1:3000/?keys=swap.free,swap.used,swap.total>");
    Server::new(listener).run(app).await.unwrap();
}
