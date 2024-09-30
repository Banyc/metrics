use std::io::{self, Write};

use crate::{
    dump::{MetricQueueReaders, QUEUE_SIZE},
    MetricKey, Sample,
};

#[derive(Debug)]
pub struct HttpExporter {
    readers: MetricQueueReaders,
    client: ureq::Agent,
    url: String,
    buf: Vec<u8>,
}
impl HttpExporter {
    pub fn new(readers: MetricQueueReaders, url: String) -> Self {
        let buf = vec![];
        let client = ureq::Agent::new();
        Self {
            readers,
            client,
            url,
            buf,
        }
    }
    /// Blocking I/O
    pub fn export(&mut self) -> anyhow::Result<()> {
        for (key, reader) in self.readers.readers_mut() {
            self.buf.clear();
            let mut wtr = io::Cursor::new(&mut self.buf);
            encode_key(&mut wtr, key);
            let sample_count_ptr = usize::try_from(wtr.position()).unwrap();
            let mut sample_count: u16 = 0;
            wtr.write_all(&sample_count.to_be_bytes()).unwrap();
            for _ in 0..QUEUE_SIZE {
                let Some(sample) = reader.pop() else {
                    break;
                };
                sample_count += 1;
                encode_sample(&mut wtr, sample);
            }
            if sample_count == 0 {
                continue;
            }
            self.buf[sample_count_ptr..sample_count_ptr + core::mem::size_of::<u16>()]
                .copy_from_slice(&sample_count.to_be_bytes());
            let _resp = self.client.post(&self.url).send_bytes(&self.buf)?;
        }
        Ok(())
    }
}

fn encode_key(wtr: &mut impl io::Write, key: &MetricKey) {
    let len = u16::try_from(key.len()).unwrap();
    wtr.write_all(&len.to_be_bytes()).unwrap();
    wtr.write_all(key.as_bytes()).unwrap();
}
fn encode_sample(wtr: &mut impl io::Write, sample: Sample) {
    wtr.write_all(&sample.time.to_be_bytes()).unwrap();
    wtr.write_all(&sample.value.to_be_bytes()).unwrap();
}
