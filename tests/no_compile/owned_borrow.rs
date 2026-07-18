#[derive(derive_mmio::Mmio)]
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
        let mmio_uart = self.uart.borrow();
        // we did the wrong kind of borrow
        mmio_uart.write_data(data);
    }
}

fn main() {
    // Can not really test this on a host environment. Simply verify that it fails to build.
}
