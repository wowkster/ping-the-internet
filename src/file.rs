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

use crate::{
    ping::PingResult,
    subnet::{Subnet, SubnetClass},
};

pub type ClassCResults = [PingResult; 256];
pub type ClassBResults = [Option<ClassCResults>; 256];

/// Saves the results of an entire class b subnet to a file
///
/// Compresses using Zlib and saves to a file named `./data/a/b` which includes all of
/// the ping results for that full subnet. If a class C subnet is missing it is completely omitted
///
/// This allows for a very good compression ration
pub async fn save_class_b(
    subnet: Subnet,
    results: Arc<Box<ClassBResults>>,
) -> Result<(), std::io::Error> {
    assert_eq!(
        subnet.class(),
        SubnetClass::B,
        "save_class_b only takes class b subnets"
    );

    /* Serialize and compress data */

    let mut encoder = ZlibEncoder::with_quality(Vec::new(), Level::Best);

    for class_c in &**results {
        match class_c {
            None => {
                encoder.write_all(&[0x00]).await?;
            }
            Some(class_c) => {
                encoder.write_all(&[0x01]).await?;
                for ping_result in class_c {
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

/// Reads a class b subnet from a file or directory of class c subnet files.
///
/// Returns None if the class b subnet is not found on the disk at all. Otherwise,
/// returns an array of Options of the class c subnets
pub async fn read_class_b(subnet: Subnet) -> Result<Option<Box<ClassBResults>>, std::io::Error> {
    assert_eq!(
        subnet.class(),
        SubnetClass::B,
        "read_class_b only takes class b subnets"
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

    let Ok((input, class_b)) = parse_class_b(&data) else {
        return Ok(None);
    };
    assert_eq!(input.len(), 0);

    Ok(Some(class_b))
}

fn parse_class_b(input: &[u8]) -> IResult<&[u8], Box<ClassBResults>> {
    let (input, class_b) = count(parse_optional_class_c, 256)(input)?;

    Ok((input, class_b.try_into().unwrap()))
}

fn parse_optional_class_c(input: &[u8]) -> IResult<&[u8], Option<ClassCResults>> {
    let (input, enum_tag) = alt((tag(&[0x00]), tag(&[0x01])))(input)?;

    match enum_tag {
        [0x00] => Ok((input, None)),
        [0x01] => {
            let (input, class_c) = parse_class_c(input)?;

            Ok((input, Some(class_c)))
        }
        _ => unreachable!(),
    }
}

fn parse_class_c(input: &[u8]) -> IResult<&[u8], ClassCResults> {
    let (input, ping_results) = count(PingResult::parse_from_bytes, 256)(input)?;

    Ok((input, ping_results.try_into().unwrap()))
}

fn create_file_path(subnet: Subnet) -> PathBuf {
    let octets = subnet.octets();

    Path::new(".")
        .join("data")
        .join(octets[0].to_string())
        .join(octets[1].to_string())
}
