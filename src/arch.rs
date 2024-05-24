use strum::{AsRefStr, Display, EnumString, VariantNames};

#[derive(Debug, EnumString, Display, VariantNames, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum Arch {
    Arm64,
    Amd64,
    Arm,
}

impl Arch {
    pub fn from_str(s: &str) -> Option<Self> {
        let lower = s.to_lowercase();
        if lower.contains(Arch::Arm64.as_ref()) {
            Some(Arch::Arm64)
        } else if lower.contains(Arch::Amd64.as_ref()) {
            Some(Arch::Amd64)
        } else if lower.contains(Arch::Arm.as_ref()) {
            Some(Arch::Arm)
        } else {
            None
        }
    }
}