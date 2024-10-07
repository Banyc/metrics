use crate::{
    consumer::MetricQueues,
    view::{metric_span, MetricSyntheses},
    MetricKey, Sample, Time,
};

pub struct MetricAlerter {
    from: Time,
    key: MetricKey,
    trip: Box<dyn Fn(Sample) -> bool>,
}
impl MetricAlerter {
    pub fn new(from: Time, key: MetricKey, trip: Box<dyn Fn(Sample) -> bool>) -> Self {
        Self { from, key, trip }
    }

    pub fn alert(&mut self, metrics: &MetricQueues, syntheses: &MetricSyntheses) -> Option<bool> {
        let time_range = self.from..;
        let span = metric_span(metrics, syntheses, &self.key, time_range)?;
        let mut last_time = None;
        let mut tripped = false;
        for sample in span.samples {
            let update_time = match last_time {
                Some(time) => time < sample.time,
                None => true,
            };
            if update_time {
                last_time = Some(sample.time);
            }
            if (self.trip)(sample) {
                tripped = true;
            }
        }
        if let Some(last_time) = last_time {
            self.from = last_time + 1;
        }
        Some(tripped)
    }
}
impl core::fmt::Debug for MetricAlerter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MetricAlerter")
            .field("from", &self.from)
            .field("key", &self.key)
            .finish()
    }
}
