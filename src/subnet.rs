use std::{
    fmt::Display,
    net::Ipv4Addr,
    ops::{Deref, RangeInclusive},
    str::FromStr,
    sync::Arc,
};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    sequence::tuple,
    IResult,
};

use crate::ping::PingResult;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SubnetClass {
    All,
    A,
    B,
    C,
    D,
}

/// A non standard representation of a sub-network. Has a base address and
/// a "class" which is the number of 1 byte blocks the subnet is authoritative
/// over and nothing else.
///
/// For example the subnet `1.2.x.x` would be class B with a base address of
/// `1.2.0.0`, and the subnet `12.x.x.x` would be class A with a base address
/// of `12.0.0.0`.
///
/// The `All` class represents the entire internet since it has no bytes
/// reserved for a subnet id. It always has a base address of `0.0.0.0`.
/// It is the so called "universal set". On the contrary, class D subnets only
/// contain a single IP address. They are so called empty sets because they contain
/// no further elements themselves.
#[derive(Debug, Clone, Copy)]
pub struct Subnet {
    base_address: Ipv4Addr,
    class: SubnetClass,
}

impl Subnet {
    pub fn new(base_address: Ipv4Addr, class: SubnetClass) -> Self {
        let octets = base_address.octets();

        assert!(match class {
            SubnetClass::All => matches!(octets, [0, 0, 0, 0]),
            SubnetClass::A => matches!(octets, [_, 0, 0, 0]),
            SubnetClass::B => matches!(octets, [_, _, 0, 0]),
            SubnetClass::C => matches!(octets, [_, _, _, 0]),
            SubnetClass::D => matches!(octets, [_, _, _, _]),
        });

        Self {
            base_address,
            class,
        }
    }

    pub fn base_address(&self) -> Ipv4Addr {
        self.base_address
    }

    pub fn class(&self) -> SubnetClass {
        self.class
    }

    /// Iterates through all the addresses in the given subnet
    pub fn iter_addresses(&self) -> impl Iterator<Item = Ipv4Addr> {
        let start = u32::from_be_bytes(self.base_address.octets());

        let power = match self.class {
            SubnetClass::All => 32,
            SubnetClass::A => 24,
            SubnetClass::B => 16,
            SubnetClass::C => 8,
            SubnetClass::D => 0,
        };
        let length = 2u32.pow(power);

        (start..=start + length - 1).map(|a| a.to_be_bytes().into())
    }

    /// Iterates through all the subnets one class lower than this subnet
    pub fn iter_subnets(&self) -> impl Iterator<Item = Subnet> {
        SubnetIterator::new(*self)
    }

    /// Iterates through all the class b subnets in the given subnet
    pub fn iter_class_b_subnets(base_address: Ipv4Addr) -> impl Iterator<Item = Subnet> {
        let octets = base_address.octets();

        assert!(matches!(octets, [_, _, 0, 0]));

        let start = u32::from_be_bytes(octets);
        let start = start >> 16;

        (start..=u16::MAX as u32)
            .map(|a| Subnet::new((a << 16).to_be_bytes().into(), SubnetClass::B))
    }
}

impl Default for Subnet {
    fn default() -> Self {
        Self {
            base_address: [0, 0, 0, 0].into(),
            class: SubnetClass::All,
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

        match self.class {
            SubnetClass::All => write!(f, "x.x.x.x"),
            SubnetClass::A => write!(f, "{}.x.x.x", octets[0]),
            SubnetClass::B => write!(f, "{}.{}.x.x", octets[0], octets[1]),
            SubnetClass::C => write!(f, "{}.{}.{}.x", octets[0], octets[1], octets[2]),
            SubnetClass::D => write!(f, "{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3]),
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
                Subnet::new([0, 0, 0, 0].into(), SubnetClass::All)
            }
            [ByteBlock::Int(a), ByteBlock::WildCard, ByteBlock::WildCard, ByteBlock::WildCard] => {
                Subnet::new([a, 0, 0, 0].into(), SubnetClass::A)
            }
            [ByteBlock::Int(a), ByteBlock::Int(b), ByteBlock::WildCard, ByteBlock::WildCard] => {
                Subnet::new([a, b, 0, 0].into(), SubnetClass::B)
            }
            [ByteBlock::Int(a), ByteBlock::Int(b), ByteBlock::Int(c), ByteBlock::WildCard] => {
                Subnet::new([a, b, c, 0].into(), SubnetClass::C)
            }
            [ByteBlock::Int(a), ByteBlock::Int(b), ByteBlock::Int(c), ByteBlock::Int(d)] => {
                Subnet::new([a, b, c, d].into(), SubnetClass::D)
            }
            _ => return Err(SubnetParseError),
        })
    }
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

enum ByteBlock {
    Int(u8),
    WildCard,
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
        let new_class = match self.base_subnet.class {
            SubnetClass::All => SubnetClass::A,
            SubnetClass::A => SubnetClass::B,
            SubnetClass::B => SubnetClass::C,
            SubnetClass::C => SubnetClass::D,
            SubnetClass::D => return None,
        };

        let idx = match new_class {
            SubnetClass::All => unreachable!(),
            SubnetClass::A => 0,
            SubnetClass::B => 1,
            SubnetClass::C => 2,
            SubnetClass::D => 3,
        };

        let mut octets = self.base_subnet.base_address.octets();

        octets[idx] = self.range.next()?;

        Some(Subnet::new(octets.into(), new_class))
    }
}

pub type ClassBResult = Arc<[Option<ClassCResult>; 256]>;
pub type ClassAResult = Arc<[Option<ClassBResult>; 256]>;
pub type ClassCResult = Arc<[ClassDResult; 256]>;
pub type ClassDResult = PingResult;

#[derive(Debug, Clone)]
pub enum SubnetClassResults {
    ClassA(ClassAResult),
    ClassB(ClassBResult),
    ClassC(ClassCResult),
    ClassD(ClassDResult),
}
