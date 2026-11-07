mod address;
mod assembler;
mod cemu_to_console;
mod console_to_cemu;
mod data;
mod dis_float;
mod dis_integer;
mod disassembler;
mod encode;
mod expr;
mod gecko;
mod isa;
mod lexer;
mod operands;
mod options;
mod pairs;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use address::AddressMap;
use assembler::assemble_at;
use disassembler::disassemble_with;

pub use cemu_to_console::cemu_to_console;
pub use console_to_cemu::console_to_cemu;
pub use options::PortOptions;

pub fn disassemble_instruction(word: u32, current_addr: u32) -> String {
    disassemble_with(word, current_addr, &HashMap::new())
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn assemble_instruction(line: &str) -> Result<u32, String> {
    assemble_at(line, 0, &HashMap::new())
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn gecko_to_asm(gecko_text: &str) -> Result<String, String> {
    console_to_cemu(gecko_text, &PortOptions::default()).map(|ported| ported.text)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn cemu_asm_to_gecko(asm_text: &str, fallback_address: u32) -> Result<String, String> {
    let opts = PortOptions::default();
    let cave_base = opts.window.to_exec(fallback_address);
    cemu_to_console(asm_text, cave_base, &opts).map(|ported| ported.text)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn console_addr_to_cemu(addr: u32) -> u32 {
    AddressMap::default().to_exec(addr)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn cemu_addr_to_console(addr: u32) -> u32 {
    AddressMap::default().to_console(addr)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn gecko_convert_addresses(gecko_text: &str, to_cemu: bool) -> String {
    gecko::convert_addresses(gecko_text, to_cemu)
}
