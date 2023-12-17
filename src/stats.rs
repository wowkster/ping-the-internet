use crate::{
    ping::PingResult,
    subnet::{
        ClassAResult, ClassBResult, ClassCResult, ClassDResult, Subnet, SubnetClass,
        SubnetClassResults,
    },
};

#[derive(Debug, Clone)]
pub struct Analysis {
    pub class: SubnetClass,
    pub alive: u32,
    pub timed_out: u32,
    pub errored: u32,
}

impl Analysis {
    fn new(class: SubnetClass) -> Self {
        Self {
            class,
            alive: 0,
            timed_out: 0,
            errored: 0,
        }
    }

    fn get_max(&self) -> u32 {
        let power = match self.class {
            SubnetClass::B => 16,
            SubnetClass::C => 8,
            SubnetClass::D => 0,
            _ => unreachable!(),
        };

        2u32.pow(power)
    }

    fn compute_percent(&self, value: u32) -> f32 {
        (value as f32 / (self.get_max()) as f32) * 100.0
    }

    pub fn alive_percent(&self) -> f32 {
        self.compute_percent(self.alive)
    }

    pub fn timed_out_percent(&self) -> f32 {
        self.compute_percent(self.timed_out)
    }

    pub fn errored_percent(&self) -> f32 {
        self.compute_percent(self.errored)
    }

    pub fn of_subnet(results: SubnetClassResults) -> Self {
        match results {
            SubnetClassResults::ClassA(results) => Self::of_class_a(results),
            SubnetClassResults::ClassB(results) => Self::of_class_b(results),
            SubnetClassResults::ClassC(results) => Self::of_class_c(results),
            SubnetClassResults::ClassD(results) => Self::of_class_d(results),
        }
    }

    fn of_class_a(results: ClassAResult) -> Self {
        let mut anal = Analysis::new(SubnetClass::A);

        for class_b in &*results {
            let Some(class_b) = class_b else {
                anal.errored += 65536;
                continue;
            };

            for class_c in &**class_b {
                let Some(class_c) = class_c else {
                    anal.errored += 256;
                    continue;
                };

                for ping_result in &**class_c {
                    match ping_result {
                        PingResult::Success(_) => anal.alive += 1,
                        PingResult::Timeout => anal.timed_out += 1,
                        PingResult::Error => anal.errored += 1,
                    }
                }
            }
        }

        anal
    }

    fn of_class_b(results: ClassBResult) -> Self {
        let mut anal = Analysis::new(SubnetClass::B);

        for class_c in &*results {
            let Some(class_c) = class_c else {
                anal.errored += 256;
                continue;
            };

            for ping_result in &**class_c {
                match ping_result {
                    PingResult::Success(_) => anal.alive += 1,
                    PingResult::Timeout => anal.timed_out += 1,
                    PingResult::Error => anal.errored += 1,
                }
            }
        }

        anal
    }

    fn of_class_c(results: ClassCResult) -> Self {
        let mut anal = Analysis::new(SubnetClass::C);

        for ping_result in &*results {
            match ping_result {
                PingResult::Success(_) => anal.alive += 1,
                PingResult::Timeout => anal.timed_out += 1,
                PingResult::Error => anal.errored += 1,
            }
        }

        anal
    }

    fn of_class_d(ping_result: ClassDResult) -> Self {
        let mut anal = Analysis::new(SubnetClass::D);

        match ping_result {
            PingResult::Success(_) => anal.alive += 1,
            PingResult::Timeout => anal.timed_out += 1,
            PingResult::Error => anal.errored += 1,
        }

        anal
    }
}

pub fn print_stats_table_header() {
    println!(
        "| {:^13} | {:^17} | {:^17} | {:^17} |",
        "IP ADDRESS", "SUCCEEDED", "TIMED OUT", "ERRORED",
    );
    println!("|{:->15}|{:->19}|{:->19}|{:->19}|", "", "", "", "");
}

pub fn print_stats_table_row(subnet: Subnet, anal: Option<Analysis>, new_line: bool) {
    if let Some(anal) = anal {
        print!(
            "| {:>13} | {:>5} | {:>9} | {:>5} | {:>9} | {:>5} | {:>9} |",
            format!("{subnet}"),
            anal.alive,
            format!("({:.2}%)", anal.alive_percent()),
            anal.timed_out,
            format!("({:.2}%)", anal.timed_out_percent()),
            anal.errored,
            format!("({:.2}%)", anal.errored_percent()),
        );
    } else {
        print!("| {:>13} | {:^57} |", format!("{subnet}"), "NOT FOUND");
    }

    if new_line {
        println!();
    }
}
