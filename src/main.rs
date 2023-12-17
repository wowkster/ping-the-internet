use std::{
    error::Error,
    path::Path,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Instant,
};

use futures::future::join_all;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use chrono::prelude::*;

use ping_the_internet::{
    file::{read_class_b, save_class_b},
    ping::{init_pinger_pool, ping},
    stats::{print_stats_table_header, print_stats_table_row, Analysis},
    subnet::{ClassBResult, ClassCResult, Subnet, SubnetClass, SubnetClassResults},
};

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr =
        if let Some(addr) = std::env::args().nth(1) {
            addr.parse().unwrap()
        } else {
            [1, 0, 0, 0].into()
        };

    init_pinger_pool().await;

    print_stats_table_header();

    let global_start_time = Instant::now();

    for b in Subnet::iter_class_b_subnets(addr) {
        let start_time = Instant::now();

        if let Some(results) = ping_class_b(b).await? {
            let anal = Analysis::of_subnet(SubnetClassResults::ClassB(results));
            print_stats_table_row(b, Some(anal), false);

            println!(
                " in {:.2?} ({:.2?} total)",
                start_time.elapsed(),
                global_start_time.elapsed()
            );
        } else {
            println!("| {:>13} | {:^57} |", format!("{b}"), "Skipped");
        }
    }

    Ok(())
}

async fn ping_class_b(subnet: Subnet) -> Result<Option<ClassBResult>, std::io::Error> {
    assert_eq!(subnet.class(), SubnetClass::B);

    if read_class_b(subnet).await.unwrap().is_some() {
        return Ok(None);
    }

    let class_cs = subnet.iter_subnets().map(ping_class_c);

    let results: ClassBResult = Arc::new(
        join_all(class_cs)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .unwrap(),
    );

    // start of test

    // let mut results = Vec::with_capacity(256);

    // for class_c in class_cs {
    //     results.push(class_c.await);
    // }

    // let results: ClassBResult = Arc::new(
    //     results
    //         .into_iter()
    //         .collect::<Result<Vec<_>, _>>()?
    //         .try_into()
    //         .unwrap(),
    // );

    // end of test

    save_class_b(subnet, results.clone()).await?;

    Ok(Some(results))
}

async fn ping_class_c(subnet: Subnet) -> Result<Option<ClassCResult>, std::io::Error> {
    // println!("pinging subnet {}", subnet);
    let results = join_all(subnet.iter_addresses().map(ping)).await;

    let results: ClassCResult = Arc::new(results.try_into().unwrap());

    let anal = Analysis::of_subnet(SubnetClassResults::ClassC(results.clone()));
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
        let mut file =
            OpenOptions::new()
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
