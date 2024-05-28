use std::sync::Arc;

use crate::{
    ping::PingResult,
    subnet::{Subnet, SubnetMask},
};

pub type Slash8Result = Arc<[Option<Slash16Result>; 256]>;
pub type Slash16Result = Arc<[Option<Slash24Result>; 256]>;
pub type Slash24Result = Arc<[Slash32Result; 256]>;
pub type Slash32Result = PingResult;

#[derive(Debug, Clone)]
pub enum SubnetResults {
    Slash8(Slash8Result),
    Slash16(Slash16Result),
    Slash24(Slash24Result),
    Slash32(Slash32Result),
}

#[derive(Debug, Clone)]
pub struct Analysis {
    pub mask: SubnetMask,
    pub alive: u32,
    pub timed_out: u32,
    pub errored: u32,
}

impl Analysis {
    fn new(mask: SubnetMask) -> Self {
        Self {
            mask,
            alive: 0,
            timed_out: 0,
            errored: 0,
        }
    }

    fn get_max(&self) -> u32 {
        let power = match self.mask {
            SubnetMask::Slash16 => 16,
            SubnetMask::Slash24 => 8,
            SubnetMask::Slash32 => 0,
            _ => unreachable!(),
        };

        2u32.pow(power)
    }

    fn compute_percent(&self, value: u32) -> f32 {
        (value as f32 / (self.get_max()) as f32) * 100.0
    }

    pub fn alive_percent(&self) -> f32 {
        self.compute_percent(self.alive)
    }

    pub fn timed_out_percent(&self) -> f32 {
        self.compute_percent(self.timed_out)
    }

    pub fn errored_percent(&self) -> f32 {
        self.compute_percent(self.errored)
    }

    pub fn of_subnet(results: SubnetResults) -> Self {
        match results {
            SubnetResults::Slash8(results) => Self::of_slash_8(results),
            SubnetResults::Slash16(results) => Self::of_slash_16(results),
            SubnetResults::Slash24(results) => Self::of_slash_24(results),
            SubnetResults::Slash32(results) => Self::of_slash_32(results),
        }
    }

    fn of_slash_8(results: Slash8Result) -> Self {
        let mut anal = Analysis::new(SubnetMask::Slash8);

        for slash_16 in &*results {
            let Some(slash_16) = slash_16 else {
                anal.errored += 65536;
                continue;
            };

            for slash_24 in &**slash_16 {
                let Some(slash_24) = slash_24 else {
                    anal.errored += 256;
                    continue;
                };

                for ping_result in &**slash_24 {
                    match ping_result {
                        PingResult::Success(_) => anal.alive += 1,
                        PingResult::Timeout => anal.timed_out += 1,
                        PingResult::Error => anal.errored += 1,
                    }
                }
            }
        }

        anal
    }

    fn of_slash_16(results: Slash16Result) -> Self {
        let mut anal = Analysis::new(SubnetMask::Slash16);

        for slash_24 in &*results {
            let Some(slash_24) = slash_24 else {
                anal.errored += 256;
                continue;
            };

            for ping_result in &**slash_24 {
                match ping_result {
                    PingResult::Success(_) => anal.alive += 1,
                    PingResult::Timeout => anal.timed_out += 1,
                    PingResult::Error => anal.errored += 1,
                }
            }
        }

        anal
    }

    fn of_slash_24(results: Slash24Result) -> Self {
        let mut anal = Analysis::new(SubnetMask::Slash24);

        for ping_result in &*results {
            match ping_result {
                PingResult::Success(_) => anal.alive += 1,
                PingResult::Timeout => anal.timed_out += 1,
                PingResult::Error => anal.errored += 1,
            }
        }

        anal
    }

    fn of_slash_32(ping_result: Slash32Result) -> Self {
        let mut anal = Analysis::new(SubnetMask::Slash32);

        match ping_result {
            PingResult::Success(_) => anal.alive += 1,
            PingResult::Timeout => anal.timed_out += 1,
            PingResult::Error => anal.errored += 1,
        }

        anal
    }
}

pub fn print_stats_table_header() {
    println!(
        "| {:^13} | {:^17} | {:^17} | {:^17} |",
        "IP ADDRESS", "SUCCEEDED", "TIMED OUT", "ERRORED",
    );
    println!("|{:->15}|{:->19}|{:->19}|{:->19}|", "", "", "", "");
}

pub fn print_stats_table_row(subnet: Subnet, anal: Option<Analysis>, new_line: bool) {
    if let Some(anal) = anal {
        print!(
            "| {:>13} | {:>5} | {:>9} | {:>5} | {:>9} | {:>5} | {:>9} |",
            format!("{subnet}"),
            anal.alive,
            format!("({:.2}%)", anal.alive_percent()),
            anal.timed_out,
            format!("({:.2}%)", anal.timed_out_percent()),
            anal.errored,
            format!("({:.2}%)", anal.errored_percent()),
        );
    } else {
        print!("| {:>13} | {:^57} |", format!("{subnet}"), "NOT FOUND");
    }

    if new_line {
        println!();
    }
}
