use std::ops::Range;

#[derive(Clone, Debug)]
pub struct CodeWindow {
    pub exec: Range<u32>,
    pub console_offset: u32,
}

impl Default for CodeWindow {
    fn default() -> Self {
        Self { exec: 0x0200_0000..0x0400_0000, console_offset: 0x0C00_0000 }
    }
}

impl CodeWindow {
    pub fn to_console(&self, addr: u32) -> u32 {
        if self.exec.contains(&addr) { addr.wrapping_add(self.console_offset) } else { addr }
    }

    pub fn to_exec(&self, addr: u32) -> u32 {
        if self.console_range().contains(&addr) { addr.wrapping_sub(self.console_offset) } else { addr }
    }

    pub fn console_range(&self) -> Range<u32> {
        self.exec.start.wrapping_add(self.console_offset)..self.exec.end.wrapping_add(self.console_offset)
    }

    pub fn is_code(&self, exec_addr: u32) -> bool {
        self.exec.contains(&exec_addr)
    }
}

#[derive(Clone, Debug)]
pub struct AddressMap {
    pub code: CodeWindow,
    pub data: CodeWindow,
}

impl Default for AddressMap {
    fn default() -> Self {
        Self {
            code: CodeWindow::default(),
            data: CodeWindow { exec: 0x1000_0000..0x2000_0000, console_offset: 0x0050_3000 },
        }
    }
}

impl AddressMap {
    pub fn to_console(&self, addr: u32) -> u32 {
        if self.data.exec.contains(&addr) { self.data.to_console(addr) } else { self.code.to_console(addr) }
    }

    pub fn to_exec(&self, addr: u32) -> u32 {
        if self.data.console_range().contains(&addr) { self.data.to_exec(addr) } else { self.code.to_exec(addr) }
    }

    pub fn is_code(&self, exec_addr: u32) -> bool {
        self.code.is_code(exec_addr)
    }

    pub fn is_mapped_exec(&self, addr: u32) -> bool {
        self.code.exec.contains(&addr) || self.data.exec.contains(&addr)
    }

    pub fn is_mapped_console(&self, addr: u32) -> bool {
        self.code.console_range().contains(&addr) || self.data.console_range().contains(&addr)
    }
}
