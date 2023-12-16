use std::{
    fmt::Display,
    net::Ipv4Addr,
    ops::{Deref, RangeInclusive},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SubnetClass {
    All,
    A,
    B,
    C,
    D,
}

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

        (start..=u16::MAX as u32).map(|a| Subnet::new((a << 16).to_be_bytes().into(), SubnetClass::B))
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
