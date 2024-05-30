use std::{
    fmt::Display,
    net::Ipv4Addr,
    ops::{Deref, RangeInclusive},
    str::FromStr,
};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    sequence::tuple,
    IResult,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SubnetMask {
    Slash0,
    Slash8,
    Slash16,
    Slash24,
    Slash32,
}

#[derive(Debug, Clone, Copy)]
pub struct Subnet {
    base_address: Ipv4Addr,
    mask: SubnetMask,
}

impl Subnet {
    pub fn new(base_address: Ipv4Addr, mask: SubnetMask) -> Self {
        let octets = base_address.octets();

        assert!(match mask {
            SubnetMask::Slash0 => matches!(octets, [0, 0, 0, 0]),
            SubnetMask::Slash8 => matches!(octets, [_, 0, 0, 0]),
            SubnetMask::Slash16 => matches!(octets, [_, _, 0, 0]),
            SubnetMask::Slash24 => matches!(octets, [_, _, _, 0]),
            SubnetMask::Slash32 => matches!(octets, [_, _, _, _]),
        });

        Self { base_address, mask }
    }

    pub fn base_address(&self) -> Ipv4Addr {
        self.base_address
    }

    pub fn mask(&self) -> SubnetMask {
        self.mask
    }

    /// Iterates through all the subnets one class lower than this subnet
    pub fn iter_subnets(&self) -> impl Iterator<Item = Subnet> {
        SubnetIterator::new(*self)
    }
}

impl Default for Subnet {
    fn default() -> Self {
        Self {
            base_address: [0, 0, 0, 0].into(),
            mask: SubnetMask::Slash0,
        }
    }
}

impl Deref for Subnet {
    type Target = Ipv4Addr;

    fn deref(&self) -> &Self::Target {
        &self.base_address
    }
}

impl Display for Subnet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let octets = self.base_address.octets();

        match self.mask {
            SubnetMask::Slash0 => write!(f, "x.x.x.x"),
            SubnetMask::Slash8 => write!(f, "{}.x.x.x", octets[0]),
            SubnetMask::Slash16 => write!(f, "{}.{}.x.x", octets[0], octets[1]),
            SubnetMask::Slash24 => write!(f, "{}.{}.{}.x", octets[0], octets[1], octets[2]),
            SubnetMask::Slash32 => {
                write!(f, "{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3])
            }
        }
    }
}

pub struct SubnetParseError;

impl FromStr for Subnet {
    type Err = SubnetParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (input, blocks) = parse_byte_blocks(input).map_err(|_| SubnetParseError)?;

        assert_eq!(input, "");

        Ok(match blocks {
            [ByteBlock::WildCard, ByteBlock::WildCard, ByteBlock::WildCard, ByteBlock::WildCard] => {
                Subnet::new([0, 0, 0, 0].into(), SubnetMask::Slash0)
            }
            [ByteBlock::Int(a), ByteBlock::WildCard, ByteBlock::WildCard, ByteBlock::WildCard] => {
                Subnet::new([a, 0, 0, 0].into(), SubnetMask::Slash8)
            }
            [ByteBlock::Int(a), ByteBlock::Int(b), ByteBlock::WildCard, ByteBlock::WildCard] => {
                Subnet::new([a, b, 0, 0].into(), SubnetMask::Slash16)
            }
            [ByteBlock::Int(a), ByteBlock::Int(b), ByteBlock::Int(c), ByteBlock::WildCard] => {
                Subnet::new([a, b, c, 0].into(), SubnetMask::Slash24)
            }
            [ByteBlock::Int(a), ByteBlock::Int(b), ByteBlock::Int(c), ByteBlock::Int(d)] => {
                Subnet::new([a, b, c, d].into(), SubnetMask::Slash32)
            }
            _ => return Err(SubnetParseError),
        })
    }
}

enum ByteBlock {
    Int(u8),
    WildCard,
}

fn parse_byte_blocks(input: &str) -> IResult<&str, [ByteBlock; 4]> {
    let (input, (a, _, b, _, c, _, d)) = tuple((
        parse_byte_block,
        tag("."),
        parse_byte_block,
        tag("."),
        parse_byte_block,
        tag("."),
        parse_byte_block,
    ))(input)?;

    Ok((input, [a, b, c, d]))
}

fn parse_byte_block(input: &str) -> IResult<&str, ByteBlock> {
    let (input, res) = alt((tag("x"), take_while(|c: char| c.is_ascii_digit())))(input)?;

    match res {
        "x" => Ok((input, ByteBlock::WildCard)),
        int => Ok((input, ByteBlock::Int(int.parse().unwrap()))),
    }
}

pub struct SubnetIterator {
    base_subnet: Subnet,
    range: RangeInclusive<u8>,
}

impl SubnetIterator {
    fn new(base_subnet: Subnet) -> Self {
        Self {
            base_subnet,
            range: 0..=255,
        }
    }
}

impl Iterator for SubnetIterator {
    type Item = Subnet;

    fn next(&mut self) -> Option<Self::Item> {
        let new_class = match self.base_subnet.mask {
            SubnetMask::Slash0 => SubnetMask::Slash8,
            SubnetMask::Slash8 => SubnetMask::Slash16,
            SubnetMask::Slash16 => SubnetMask::Slash24,
            SubnetMask::Slash24 => SubnetMask::Slash32,
            SubnetMask::Slash32 => return None,
        };

        let idx = match new_class {
            SubnetMask::Slash0 => unreachable!(),
            SubnetMask::Slash8 => 0,
            SubnetMask::Slash16 => 1,
            SubnetMask::Slash24 => 2,
            SubnetMask::Slash32 => 3,
        };

        let mut octets = self.base_subnet.base_address.octets();

        octets[idx] = self.range.next()?;

        Some(Subnet::new(octets.into(), new_class))
    }
}
