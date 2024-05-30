use std::{collections::BTreeMap, net::Ipv4Addr};

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slash32State {
    Reserved,
    Scheduled,
    Pending,
    Succeeded,
    TimedOut,
    Errored,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum PingResult {
    Reserved,
    Succeeded,
    TimedOut,
    Errored,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Slash24Result(#[serde(with = "serde_big_array::BigArray")] [PingResult; 256]);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum M2WMessage {
    Shutdown,
    PingSlash16(Ipv4Addr),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum W2MMessage {
    Stats {
        reserved: u16,
        scheduled: u16,
        pending: u16,
        succeeded: u16,
        timed_out: u16,
        errored: u16,
        elapsed_ms: u64,
        estimated_remaining_ms: u64,
        estimated_total_ms: u64,
    },
    StateChanged {
        addr: Ipv4Addr,
        state: Slash32State,
    },
    /// Doesn't store any /24 subnets that all timed out
    Results(BTreeMap<u8, Slash24Result>),
}
