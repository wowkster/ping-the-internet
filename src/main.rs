use std::{
    net::{IpAddr, Ipv4Addr},
    sync::atomic::{AtomicU16, Ordering},
    time::Duration,
};

use futures::future::join_all;

#[tokio::main]
async fn main() {
    let addr = [8, 8, 8, 1];

    ping_block(addr.into(), IpBlock::D).await;
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum IpBlock {
    A,
    B,
    C,
    D,
}

async fn ping_block(base_address: Ipv4Addr, block: IpBlock) -> Vec<PingResult> {
    let mut address = base_address.octets();

    let mut handles = Vec::new();

    for i in 0..=255 {
        address[block as u8 as usize] = i;
        let address = address.into();

        handles.push(ping(address));
    }

    join_all(handles).await
}

enum PingResult {
    Success(Duration),
    Timeout,
    Error(tokio_icmp_echo::Error),
}

async fn ping(address: Ipv4Addr) -> PingResult {
    static IDENTIFIER: AtomicU16 = AtomicU16::new(0);
    static SEQUENCE: AtomicU16 = AtomicU16::new(0);

    let i = IDENTIFIER.fetch_add(1, Ordering::AcqRel);
    let s = SEQUENCE.fetch_add(1, Ordering::AcqRel);

    // POTENTIAL OPTIMIZATION: Create pool of pingers to avoid socket re-alloc
    let pinger = tokio_icmp_echo::Pinger::new().await.unwrap();
    let mb_time = pinger
        .ping(IpAddr::V4(address), i, s, Duration::from_secs(3))
        .await;

    match mb_time {
        Ok(Some(time)) => PingResult::Success(time),
        Ok(None) => PingResult::Timeout,
        Err(err) => PingResult::Error(err),
    }
}
