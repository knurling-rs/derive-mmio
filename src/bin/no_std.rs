#![no_std]
#![cfg_attr(not(feature = "std"), no_main)]

mod inner {
    #[derive(derive_mmio::Mmio)]
    #[repr(C)]
    pub struct UartBank {
        // this is read-write by default
        data: u32,
        status: u32,
    }
}

#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // you can be explicit if you like
    control: u32,
    #[mmio(inner)]
    bank_0: inner::UartBank,
    // Array of registers
    array: [u32; 4],
}

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(feature = "std")]
fn main() {}
