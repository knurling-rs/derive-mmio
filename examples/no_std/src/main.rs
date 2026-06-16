#![no_std]
#![no_main]

mod inner {
    #[derive(derive_mmio::Mmio)]
    #[repr(C)]
    pub struct UartBank {
        // this is read-write by default
        data: u32,
        // This register is read-only
        #[mmio(Read)]
        status: u32,
    }
}

#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // This is explicitly read-write
    #[mmio(Read, Write)]
    control: u32,
    #[mmio(Inner)]
    bank_0: inner::UartBank,
    // Array of registers
    array: [u32; 4],
}

/// This is our UART driver - it is of zero size
pub struct Driver {
    uart: OwnedUart<{ Self::UART_BASE }>,
}

impl Driver {
    const UART_BASE: usize = 0xE000_0000;

    pub const fn new() -> Driver {
        Driver {
            uart: unsafe { OwnedUart::new() },
        }
    }

    pub fn read_control(&mut self) -> u32 {
        // safely create a handle by borrowing our Owned handle
        let mut mmio_uart = self.uart.borrow_mut();
        // used it to access the peripheral
        mmio_uart.read_control()
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
