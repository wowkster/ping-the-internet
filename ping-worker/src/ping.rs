use std::{
    net::{IpAddr, Ipv4Addr},
    sync::atomic::{AtomicU16, Ordering},
    time::Duration,
};

use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    IResult,
};

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Semaphore,
};

use crate::gui::{Slash32State, SLASH_32_STATES};

pub static PING_PERMITS: Semaphore = Semaphore::const_new(1024);

#[derive(Debug, Clone, PartialEq)]
pub enum PingResult {
    Success(Duration),
    Timeout,
    Error,
}

impl PingResult {
    pub async fn serialize_into<W: AsyncWrite + Unpin>(
        &self,
        mut w: W,
    ) -> Result<(), std::io::Error> {
        match self {
            PingResult::Success(time) => {
                w.write_all(&[0]).await?;

                let time = time.as_millis() as u16;
                w.write_all(&time.to_le_bytes()).await?;
            }
            PingResult::Timeout => w.write_all(&[1]).await?,
            PingResult::Error => w.write_all(&[2]).await?,
        }

        Ok(())
    }

    pub fn parse_from_bytes(input: &[u8]) -> IResult<&[u8], Self> {
        let success_parser = tag(&[0x00]);
        let timeout_parser = tag(&[0x01]);
        let error_parser = tag(&[0x02]);

        let (input, tag) = alt((success_parser, timeout_parser, error_parser))(input)?;

        match tag {
            [0x00] => {
                let (input, time) = take(2usize)(input)?;

                let time = u16::from_le_bytes([time[0], time[1]]);

                let res = Self::Success(Duration::from_millis(time as u64));

                Ok((input, res))
            }
            [0x01] => Ok((input, Self::Timeout)),
            [0x02] => Ok((input, Self::Error)),
            _ => unreachable!(),
        }
    }
}

pub async fn ping(address: Ipv4Addr) -> PingResult {
    static IDENTIFIER: AtomicU16 = AtomicU16::new(0);
    static SEQUENCE: AtomicU16 = AtomicU16::new(0);

    let i = IDENTIFIER.fetch_add(1, Ordering::AcqRel);
    let s = SEQUENCE.fetch_add(1, Ordering::AcqRel);

    let permit = PING_PERMITS.acquire().await.unwrap();

    let pinger = tokio_icmp_echo::Pinger::new()
        .await
        .expect("Failed to create tokio_icmp_echo::Pinger ({} open files)");

    const RETRY_LIMIT: u16 = 2;

    let state_i = address.octets()[2] as usize;
    let state_j = address.octets()[3] as usize;

    tokio::time::sleep(Duration::from_millis(
        address.octets()[2] as u64 * 4 + rand::random::<u8>() as u64,
    ))
    .await;

    {
        let mut states = SLASH_32_STATES.lock().unwrap();
        states[state_i][state_j] = Slash32State::Pending;
    }

    for retry_counter in 1..=RETRY_LIMIT {
        let mb_time = pinger
            .ping(IpAddr::V4(address), i, s, Duration::from_millis(3500))
            .await;

        let result = match mb_time {
            Ok(Some(time)) => PingResult::Success(time),
            Ok(None) => PingResult::Timeout,
            Err(_) => {
                if retry_counter < RETRY_LIMIT {
                    tokio::time::sleep(Duration::from_millis(rand::random::<u8>() as u64)).await;
                    continue;
                }

                PingResult::Error
            }
        };

        drop(permit);

        let state = match result {
            PingResult::Success(_) => Slash32State::Success,
            PingResult::Timeout => Slash32State::Timeout,
            PingResult::Error => Slash32State::Error,
        };

        {
            let mut states = SLASH_32_STATES.lock().unwrap();
            states[state_i][state_j] = state;
        }

        return result;
    }

    unreachable!();
}
