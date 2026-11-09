use std::net::SocketAddr;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const MAX_STATUS_LEN: i32 = 1 << 20;

pub struct Pong {
    pub json: String,
    pub latency: Duration,
}

pub fn write_varint(buf: &mut Vec<u8>, value: i32) {
    let mut value = value as u32;
    loop {
        if value & !0x7f == 0 {
            buf.push(value as u8);
            return;
        }
        buf.push((value & 0x7f | 0x80) as u8);
        value >>= 7;
    }
}

pub async fn read_varint(reader: &mut (impl AsyncRead + Unpin)) -> Result<i32> {
    let mut value = 0u32;
    for i in 0..5 {
        let byte = reader.read_u8().await?;
        value |= u32::from(byte & 0x7f) << (7 * i);
        if byte & 0x80 == 0 {
            return Ok(value as i32);
        }
    }
    bail!("varint is too long")
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    write_varint(buf, s.len() as i32);
    buf.extend_from_slice(s.as_bytes());
}

fn packet(id: i32, data: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    write_varint(&mut body, id);
    body.extend_from_slice(data);

    let mut out = Vec::new();
    write_varint(&mut out, body.len() as i32);
    out.extend(body);
    out
}

fn handshake(host: &str, port: u16) -> Vec<u8> {
    let mut data = Vec::new();
    write_varint(&mut data, -1);
    write_string(&mut data, host);
    data.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut data, 1);
    packet(0x00, &data)
}

async fn expect_packet(stream: &mut TcpStream, id: i32) -> Result<()> {
    read_varint(stream).await?;
    let got = read_varint(stream).await?;
    if got != id {
        bail!("expected packet {id:#04x}, got {got:#04x}");
    }
    Ok(())
}

pub async fn ping(
    addrs: &[SocketAddr],
    host: &str,
    port: u16,
    log: &mut Vec<String>,
) -> Result<Pong> {
    let list: Vec<String> = addrs.iter().map(SocketAddr::to_string).collect();
    log.push(format!("connecting to {}", list.join(", ")));
    let started = Instant::now();
    let mut stream = TcpStream::connect(addrs).await?;
    stream.set_nodelay(true)?;
    log.push(format!(
        "connected to {} ({} ms)",
        stream.peer_addr()?,
        started.elapsed().as_millis()
    ));

    let started = Instant::now();
    let mut out = handshake(host, port);
    out.extend(packet(0x00, &[]));
    stream.write_all(&out).await?;
    log.push(format!(
        "sent handshake and status request ({} bytes)",
        out.len()
    ));

    expect_packet(&mut stream, 0x00).await?;
    let len = read_varint(&mut stream).await?;
    if !(0..=MAX_STATUS_LEN).contains(&len) {
        bail!("status response is {len} bytes");
    }
    let mut json = vec![0; len as usize];
    stream.read_exact(&mut json).await?;
    let status_rtt = started.elapsed();
    log.push(format!(
        "got status response ({len} bytes, {} ms)",
        status_rtt.as_millis()
    ));

    let payload = started.elapsed().as_nanos() as i64;
    let ping = packet(0x01, &payload.to_be_bytes());
    let sent = Instant::now();
    stream.write_all(&ping).await?;
    log.push(format!("sent ping ({} bytes)", ping.len()));
    let latency = match expect_packet(&mut stream, 0x01).await {
        Ok(()) if stream.read_i64().await.ok() == Some(payload) => {
            log.push(format!("got pong ({} ms)", sent.elapsed().as_millis()));
            sent.elapsed()
        }
        _ => {
            log.push("no pong, using the status response time".to_string());
            status_rtt
        }
    };

    Ok(Pong {
        json: String::from_utf8(json)?,
        latency,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(value: i32) -> Vec<u8> {
        let mut buf = Vec::new();
        write_varint(&mut buf, value);
        buf
    }

    #[test]
    fn varint_matches_known_bytes() {
        assert_eq!(encode(0), [0x00]);
        assert_eq!(encode(127), [0x7f]);
        assert_eq!(encode(128), [0x80, 0x01]);
        assert_eq!(encode(25565), [0xdd, 0xc7, 0x01]);
        assert_eq!(encode(2147483647), [0xff, 0xff, 0xff, 0xff, 0x07]);
        assert_eq!(encode(-1), [0xff, 0xff, 0xff, 0xff, 0x0f]);
    }

    #[tokio::test]
    async fn varint_round_trips() {
        for value in [0, 1, 300, 25565, i32::MAX, i32::MIN, -1] {
            let bytes = encode(value);
            assert_eq!(read_varint(&mut bytes.as_slice()).await.unwrap(), value);
        }
    }

    #[tokio::test]
    async fn varint_rejects_six_bytes() {
        let bytes = [0x80, 0x80, 0x80, 0x80, 0x80, 0x01];
        assert!(read_varint(&mut bytes.as_slice()).await.is_err());
    }
}
