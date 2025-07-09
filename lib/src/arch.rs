use std::fmt;

#[derive(Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Architecture {
    x86_64,
    aarch64,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Architecture::x86_64 => "x86_64",
            Architecture::aarch64 => "aarch64",
        };
        write!(f, "{s}")
    }
}
