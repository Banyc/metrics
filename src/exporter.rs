use std::io::{self, Write};

use crate::{
    buf::{MetricBufReader, MetricBufReaders, BUF_SIZE},
    codec::{
        decode_key, decode_sample, decode_sample_count, encode_key, encode_sample,
        encode_sample_count,
    },
    consumer::MetricConsumer,
    MetricKey, SAMPLE_SIZE,
};

#[derive(Debug)]
pub struct HttpExporter {
    readers: MetricBufReaders,
    client: ureq::Agent,
    url: String,
    buf: Vec<u8>,
}
impl HttpExporter {
    pub fn new(readers: MetricBufReaders, url: String) -> Self {
        Self {
            readers,
            client: ureq::Agent::new(),
            url,
            buf: vec![],
        }
    }
    /// Blocking I/O
    pub fn export(&mut self) -> anyhow::Result<()> {
        for (key, reader) in self.readers.readers_mut() {
            self.buf.clear();
            let mut wtr = io::Cursor::new(&mut self.buf);
            if !encode_frame(key, reader, &mut wtr) {
                continue;
            }
            let _resp = self.client.post(&self.url).send_bytes(&self.buf)?;
        }
        Ok(())
    }
}

pub fn encode_frame(
    key: &MetricKey,
    metric_buf: &mut MetricBufReader,
    wtr: &mut io::Cursor<&mut Vec<u8>>,
) -> bool {
    encode_key(wtr, key);
    let sample_count_pos = wtr.position();
    let mut sample_count: u16 = 0;
    wtr.write_all(&encode_sample_count(sample_count)).unwrap();
    for _ in 0..BUF_SIZE {
        let Some(sample) = metric_buf.pop() else {
            break;
        };
        sample_count += 1;
        let sample = encode_sample(sample);
        wtr.write_all(&sample).unwrap();
    }
    if sample_count == 0 {
        return false;
    }
    let curr_pos = wtr.position();
    wtr.set_position(sample_count_pos);
    wtr.write_all(&encode_sample_count(sample_count)).unwrap();
    wtr.set_position(curr_pos);
    true
}
pub async fn decode_frame_copy<R>(
    rdr: &mut R,
    consumer: &mut MetricConsumer,
    key_buf: &mut String,
) -> io::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncReadExt;
    key_buf.clear();
    decode_key(rdr, key_buf).await?;
    let mut sample_count = [0; 2];
    rdr.read_exact(&mut sample_count).await?;
    let sample_count = decode_sample_count(sample_count);
    let mut queue = consumer.push(key_buf);
    for _ in 0..sample_count {
        let mut sample = [0; SAMPLE_SIZE];
        rdr.read_exact(&mut sample).await?;
        let sample = decode_sample(sample);
        queue(sample);
    }
    Ok(())
}

#[derive(Debug)]
pub struct InProcessExporter {
    readers: MetricBufReaders,
    consumer: MetricConsumer,
}
impl InProcessExporter {
    pub fn new(readers: MetricBufReaders, queue_size: usize) -> Self {
        let consumer = MetricConsumer::new(queue_size);
        Self { readers, consumer }
    }

    pub fn consumer(&self) -> &MetricConsumer {
        &self.consumer
    }
    pub async fn flush(&mut self) {
        for (key, reader) in self.readers.readers_mut() {
            let mut queue = self.consumer.push(key);
            for _ in 0..BUF_SIZE {
                let Some(sample) = reader.pop() else {
                    break;
                };
                queue(sample);
            }
            tokio::task::yield_now().await;
        }
    }
}
