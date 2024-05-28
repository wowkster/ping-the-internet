use std::{
    error::Error,
    net::Ipv4Addr,
    sync::{atomic::Ordering, Arc},
    time::Instant,
};

use futures::future::join_all;

use ping_the_internet::{
    file::{read_slash_16, save_slash_16},
    gui::{self, Slash16State, Slash32State, PENDING_SLASH_16, SLASH_16_STATES, SLASH_32_STATES},
    ping::{init_pinger_pool, ping, PingResult},
    stats::{
        print_stats_table_header, print_stats_table_row, Analysis, Slash16Result, SubnetResults,
    },
    subnet::{Subnet, SubnetMask},
};

fn main() {
    std::thread::spawn(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(pinger_main())
            .ok()
    });

    gui::gui_main();
}

async fn pinger_main() -> Result<(), Box<dyn Error>> {
    let base_address: Ipv4Addr = std::env::args()
        .nth(1)
        .map(|addr| addr.parse().unwrap())
        .unwrap_or([1, 0, 0, 0].into());

    init_pinger_pool().await;

    print_stats_table_header();

    let global_start_time = Instant::now();

    for slash_8 in Subnet::new([0, 0, 0, 0].into(), SubnetMask::Slash0).iter_subnets() {
        if slash_8.base_address().octets()[0] < base_address.octets()[0] {
            continue;
        }

        for slash_16 in slash_8.iter_subnets() {
            if slash_8.base_address().octets()[0] == base_address.octets()[0]
                && slash_16.base_address().octets()[1] < base_address.octets()[1]
            {
                continue;
            }

            let state_i = slash_16.octets()[0] as usize;
            let state_j = slash_16.octets()[1] as usize;

            PENDING_SLASH_16.store(
                u16::from_be_bytes([state_i as u8, state_j as u8]),
                Ordering::Release,
            );

            {
                let mut states = SLASH_16_STATES.lock().unwrap();
                states[state_i][state_j] = Slash16State::Pending;
            }

            let start_time = Instant::now();

            if let Some(results) = ping_slash_16(slash_16).await? {
                let anal = Analysis::of_subnet(SubnetResults::Slash16(results));

                print_stats_table_row(slash_16, Some(anal), false);

                println!(
                    " in {:.2?} ({:.2?} total)",
                    start_time.elapsed(),
                    global_start_time.elapsed()
                );

                {
                    let mut states = SLASH_16_STATES.lock().unwrap();
                    states[state_i][state_j] = Slash16State::Completed;
                }
            } else {
                println!("| {:>13} | {:^57} |", format!("{slash_16}"), "Skipped");

                {
                    let mut states = SLASH_16_STATES.lock().unwrap();
                    states[state_i][state_j] = Slash16State::Skipped;
                }
            }
        }
    }

    Ok(())
}

async fn ping_slash_16(slash_16: Subnet) -> Result<Option<Slash16Result>, std::io::Error> {
    assert_eq!(slash_16.mask(), SubnetMask::Slash16);

    if read_slash_16(slash_16).await.unwrap().is_some() {
        return Ok(None);
    }

    {
        let mut states = SLASH_32_STATES.lock().unwrap();
        *states = [[Slash32State::Scheduled; 256]; 256];
    }

    /* Iterate subnets in an order that distributes load more evenly across networks */

    let mut slash_24_iterators = Vec::with_capacity(256);

    for slash_24 in slash_16.iter_subnets() {
        slash_24_iterators.push(slash_24.iter_subnets());
    }

    let mut slash_32s = Vec::with_capacity(65536);

    for _ in 0..256 {
        for iter in &mut slash_24_iterators {
            slash_32s.push(ping(iter.next().unwrap().base_address()));
        }
    }

    let ping_results = join_all(slash_32s).await;

    let mut slash_16_result = Vec::with_capacity(256);

    for slash_24 in 0..256 {
        let mut slash_24_result = Vec::with_capacity(256);

        for slash_32 in 0..256 {
            slash_24_result.push(ping_results[slash_32 * 256 + slash_24].clone())
        }

        if slash_24_result.iter().any(|r| *r != PingResult::Timeout) {
            slash_16_result.push(Some(Arc::new(slash_24_result.try_into().unwrap())))
        } else {
            slash_16_result.push(None)
        }
    }

    let results: Slash16Result = Arc::new(slash_16_result.try_into().unwrap());

    save_slash_16(slash_16, results.clone()).await?;

    Ok(Some(results))
}
