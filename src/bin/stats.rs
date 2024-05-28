#![forbid(unsafe_code)]

use ping_the_internet::{
    file::read_slash_16, stats::{print_stats_table_header, print_stats_table_row, Analysis, SubnetResults}, subnet::Subnet
};

#[tokio::main]
async fn main() {
    print_stats_table_header();

    let mut total_pinged: u32 = 0;
    let mut total_alive: u32 = 0;

    // a.0.0.0
    for a in Subnet::default().iter_subnets() {
        for b in a.iter_subnets() {
            let anal = analyze_slash_16(b).await.unwrap();

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

pub async fn analyze_slash_16(subnet: Subnet) -> Result<Option<Analysis>, std::io::Error> {
    let Some(results) = read_slash_16(subnet).await? else {
        return Ok(None);
    };

    let anal = Analysis::of_subnet(SubnetResults::Slash16(results));

    Ok(Some(anal))
}
