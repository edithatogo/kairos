#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fmi3Status {
    Ok = 0,
    Warning = 1,
    Discard = 2,
    Error = 3,
    Fatal = 4,
}

pub struct Fmi3Support;

impl Fmi3Support {
    pub const STATUS: &'static str =
        "FMI 3.0 co-simulation ABI is reserved; Track 38 ships FMI 2.0 first.";
}
