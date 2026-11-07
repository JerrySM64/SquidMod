use std::ops::Range;

use super::address::AddressMap;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub struct CemuHeader {
    pub name: String,
    pub module_matches: u32,
}

#[derive(Clone, Debug)]
pub struct PortOptions {
    pub window: AddressMap,
    pub cave: Range<u32>,
    pub max_gap: u32,
    pub header: Option<CemuHeader>,
}

impl Default for PortOptions {
    fn default() -> Self {
        Self {
            window: AddressMap::default(),
            cave: 0x02DF_0000..0x02E0_0000,
            max_gap: 0x100,
            header: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Ported {
    pub text: String,
    pub warnings: Vec<String>,
}
