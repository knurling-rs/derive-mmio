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
    #[mmio(inner)]
    bank_0: inner::UartBank,
    // Array of registers
    array: [u32; 4],
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
