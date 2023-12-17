#![forbid(unsafe_code)]

use ping_the_internet::{
    file::read_class_b,
    stats::{print_stats_table_header, print_stats_table_row, Analysis},
    subnet::{Subnet, SubnetClassResults},
};

#[tokio::main]
async fn main() {
    print_stats_table_header();

    let mut total_pinged: u32 = 0;
    let mut total_alive: u32 = 0;

    // a.0.0.0
    for a in Subnet::default().iter_subnets() {
        for b in a.iter_subnets() {
            let anal = analyze_class_b(b).await.unwrap();

            if let Some(ref anal) = anal {
                total_pinged += 65536;
                total_alive += anal.alive;
            }

            print_stats_table_row(b, anal, true);
        }
    }

    println!(
        "Total Pinged: {} ({:.2}%)",
        total_pinged,
        total_pinged as f32 / u32::MAX as f32 * 100.0
    );
    println!(
        "Total Alive: {} ({:.2}%)",
        total_alive,
        total_alive as f32 / total_pinged as f32 * 100.0
    );
}

pub async fn analyze_class_b(subnet: Subnet) -> Result<Option<Analysis>, std::io::Error> {
    let Some(results) = read_class_b(subnet).await? else {
        return Ok(None);
    };

    let anal = Analysis::of_subnet(SubnetClassResults::ClassB(results));

    Ok(Some(anal))
}
