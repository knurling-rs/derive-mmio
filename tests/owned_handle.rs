#[derive(derive_mmio::Mmio)]
#[mmio(no_ctors)]
#[repr(C)]
struct Uart {
    data: u32,
}

struct Driver {
    uart: OwnedUart<0x1234_5678>,
}

impl Driver {
    #[allow(dead_code)]
    pub fn write_data(&mut self, data: u32) {
        let mut mmio_uart = self.uart.borrow_mut();
        mmio_uart.write_data(data);
    }

    #[allow(dead_code)]
    pub fn read_data(&self) -> u32 {
        let mmio_uart = self.uart.borrow();
        mmio_uart.read_data()
    }
}

fn main() {
    // Can not really test this on a host environment. Simply verify that it builds.
}
