use std::{
    path::Path,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use futures::future::join_all;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, runtime::Runtime};
use tokio_utils::RateLimiter;

use chrono::prelude::*;

use ping_the_internet::{
    file::{read_class_b, save_class_b, ClassBResults, ClassCResults},
    ping::ping,
    stats::{print_stats_table_header, print_stats_table_row, Analysis},
    subnet::{Subnet, SubnetClass},
};

fn main() {
    let mut rt = Runtime::new().unwrap();

    let addr = if let Some(addr) = std::env::args().nth(1) {
        addr.parse().unwrap()
    } else {
        [1, 0, 0, 0].into()
    };

    print_stats_table_header();

    let global_start_time = Instant::now();

    for b in Subnet::iter_class_b_subnets(addr) {
        let start_time = Instant::now();

        if let Some(results) = ping_class_b(b, &mut rt).unwrap() {
            let anal = Analysis::of_class_b(results);
            print_stats_table_row(b, Some(anal), false);

            println!(
                " in {:.2?} ({:.2?} total)",
                start_time.elapsed(),
                global_start_time.elapsed()
            );
        } else {
            println!("| {:>11} | {:^57} |", format!("{b}"), "Skipped");
        }
    }
}

fn ping_class_b(
    subnet: Subnet,
    rt: &mut Runtime,
) -> Result<Option<Box<ClassBResults>>, std::io::Error> {
    assert_eq!(subnet.class(), SubnetClass::B);

    rt.block_on(async {
        if read_class_b(subnet).await.unwrap().is_some() {
            return Ok(None);
        }

        let rate_limiter = RateLimiter::new(Duration::from_millis(30));

        let class_cs = subnet
            .iter_subnets()
            .map(ping_class_c)
            .map(|c| rate_limiter.throttle(|| c));

        let results: Arc<Box<ClassBResults>> = Arc::new(Box::new(
            join_all(class_cs)
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .try_into()
                .unwrap(),
        ));

        save_class_b(subnet, results.clone()).await?;

        Ok(Some(Arc::try_unwrap(results).unwrap()))
    })
}

async fn ping_class_c(subnet: Subnet) -> Result<Option<ClassCResults>, std::io::Error> {
    let results = join_all(subnet.iter_addresses().map(ping)).await;

    let results = results.try_into().unwrap();

    let anal = Analysis::of_class_c(&results);
    print_stats_table_row(subnet, Some(anal.clone()), true);

    static FAILURES: AtomicU32 = AtomicU32::new(0);

    if anal.errored == 256 {
        let failures = FAILURES.fetch_add(1, Ordering::AcqRel);

        if failures > 2 * 1024 {
            eprintln!(
                "Over 2048 failures! Something is up. Check `./data/failures.log` for more info."
            );
            std::process::exit(1);
        }

        eprintln!(
            "{} => All attempts to ping subnet failed. {} failures so far.",
            subnet,
            failures + 1
        );

        let file_path = Path::new(".").join("data").join("failures.log");
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_path)
            .await?;

        file.write_all(format!("[{}] {}\n", Local::now(), subnet).as_bytes())
            .await?;

        return Ok(None);
    }

    Ok(Some(results))
}
