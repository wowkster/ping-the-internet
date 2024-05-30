use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_compression::{
    tokio::write::{ZlibDecoder, ZlibEncoder},
    Level,
};
use nom::{branch::alt, bytes::complete::tag, multi::count, IResult};
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{ping::PingResult, stats::{Slash16Result, Slash24Result}, subnet::{Subnet, SubnetMask}};

/// Saves the results of an entire /16 subnet to a file
///
/// Compresses using Zlib and saves to a file named `./data/8/16` which includes all of
/// the ping results for that full subnet. If a /24 subnet is missing it is completely omitted
///
/// This allows for a very good compression ration
pub async fn save_slash_16(subnet: Subnet, results: Slash16Result) -> Result<(), std::io::Error> {
    assert_eq!(
        subnet.mask(),
        SubnetMask::Slash16,
        "save_slash_16 only takes /16 subnets"
    );

    /* Serialize and compress data */

    let mut encoder = ZlibEncoder::with_quality(Vec::new(), Level::Best);

    for slash_24 in &*results {
        match slash_24 {
            None => {
                encoder.write_all(&[0x00]).await?;
            }
            Some(slash_24) => {
                encoder.write_all(&[0x01]).await?;
                for ping_result in &**slash_24 {
                    ping_result.serialize_into(&mut encoder).await?;
                }
            }
        }
    }

    encoder.shutdown().await?;

    /* Ensure parent directory exists */

    let file_path = create_file_path(subnet);

    tokio::fs::create_dir_all(file_path.parent().unwrap()).await?;

    /* Write to file */

    let mut file = File::create(file_path).await?;
    file.write_all(&encoder.into_inner()).await?;

    Ok(())
}

/// Reads a /16 subnet from a file or directory of /24 subnet files.
///
/// Returns None if the /16 subnet is not found on the disk at all. Otherwise,
/// returns an array of Options of the /24 subnets
pub async fn read_slash_16(subnet: Subnet) -> Result<Option<Slash16Result>, std::io::Error> {
    assert_eq!(
        subnet.mask(),
        SubnetMask::Slash16,
        "read_slash_16 only takes /16 subnets"
    );

    /* Check directory exists */

    let file_path = create_file_path(subnet);

    if !file_path.exists() {
        return Ok(None);
    }

    /* Read and decompress data from file */

    let data = tokio::fs::read(&file_path).await?;

    let mut decoder = ZlibDecoder::new(Vec::new());
    decoder.write_all(&data).await?;
    decoder.shutdown().await?;

    let data = decoder.into_inner();

    let Ok((input, slash_16)) = parse_slash_16(&data) else {
        return Ok(None);
    };
    assert_eq!(input.len(), 0);

    Ok(Some(slash_16))
}

fn parse_slash_16(input: &[u8]) -> IResult<&[u8], Slash16Result> {
    let (input, slash_16) = count(parse_optional_slash_24, 256)(input)?;

    Ok((input, Arc::new(slash_16.try_into().unwrap())))
}

fn parse_optional_slash_24(input: &[u8]) -> IResult<&[u8], Option<Slash24Result>> {
    let (input, enum_tag) = alt((tag(&[0x00]), tag(&[0x01])))(input)?;

    match enum_tag {
        [0x00] => Ok((input, None)),
        [0x01] => {
            let (input, slash_24) = parse_slash_24(input)?;

            Ok((input, Some(slash_24)))
        }
        _ => unreachable!(),
    }
}

fn parse_slash_24(input: &[u8]) -> IResult<&[u8], Slash24Result> {
    let (input, ping_results) = count(PingResult::parse_from_bytes, 256)(input)?;

    Ok((input, Arc::new(ping_results.try_into().unwrap())))
}

fn create_file_path(subnet: Subnet) -> PathBuf {
    let octets = subnet.octets();

    Path::new(".")
        .join("data")
        .join(octets[0].to_string())
        .join(octets[1].to_string())
}
