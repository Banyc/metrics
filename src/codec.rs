use std::io::{self, Read, Write};

use crate::{MetricKey, Sample, SAMPLE_SIZE};

pub fn encode_key(wtr: &mut impl Write, key: &MetricKey) {
    let len = u16::try_from(key.len()).unwrap();
    wtr.write_all(&len.to_be_bytes()).unwrap();
    wtr.write_all(key.as_bytes()).unwrap();
}
pub async fn decode_key<R>(rdr: &mut R, key: &mut MetricKey) -> io::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncReadExt;
    let len = rdr.read_u16().await?;
    let buf = std::mem::take(key);
    let mut buf = buf.into_bytes();
    buf.clear();
    buf.extend(core::iter::repeat(0).take(usize::from(len)));
    rdr.read_exact(&mut buf).await?;
    let buf = String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    *key = buf;
    Ok(())
}

pub fn encode_sample_count(count: u16) -> [u8; 2] {
    count.to_be_bytes()
}
pub fn decode_sample_count(buf: [u8; 2]) -> u16 {
    u16::from_be_bytes(buf)
}

pub fn encode_sample(sample: Sample) -> [u8; SAMPLE_SIZE] {
    let mut buf = [0; SAMPLE_SIZE];
    let mut wtr = io::Cursor::new(&mut buf[..]);
    wtr.write_all(&sample.time.to_be_bytes()).unwrap();
    wtr.write_all(&sample.value.to_be_bytes()).unwrap();
    buf
}
pub fn decode_sample(buf: [u8; SAMPLE_SIZE]) -> Sample {
    let mut rdr = io::Cursor::new(&buf[..]);
    let mut time = [0; 8];
    rdr.read_exact(&mut time).unwrap();
    let time = u64::from_be_bytes(time);
    let mut value = [0; 8];
    rdr.read_exact(&mut value).unwrap();
    let value = f64::from_be_bytes(value);
    Sample { time, value }
}
