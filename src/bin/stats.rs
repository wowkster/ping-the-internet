#![forbid(unsafe_code)]

use ping_the_internet::{
    file::read_class_b,
    stats::{print_stats_table_header, print_stats_table_row, Analysis},
    subnet::Subnet,
};

#[tokio::main]
async fn main() {
    print_stats_table_header();

    // a.0.0.0
    for a in Subnet::default().iter_subnets() {
        for b in a.iter_subnets() {
            let anal = analyze_class_b(b).await.unwrap();

            print_stats_table_row(b, anal, true);
        }
    }
}

pub async fn analyze_class_b(subnet: Subnet) -> Result<Option<Analysis>, std::io::Error> {
    let Some(results) = read_class_b(subnet).await? else {
        return Ok(None);
    };

    let anal = Analysis::of_class_b(results);

    Ok(Some(anal))
}
