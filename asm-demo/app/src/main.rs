//! Uses the register blocks from another crate, which is where missing `#[inline]` attributes
//! become visible.
#![no_std]
#![no_main]

const UART_ADDR: usize = 0x4000_0000;

// `no_mangle` gives the functions a readable symbol name.
#[no_mangle]
pub fn write_control(value: u32) {
    let mut uart = unsafe { regs::Uart::new_mmio_at(UART_ADDR) };
    uart.write_control(value);
}

#[no_mangle]
pub fn write_bank_data(value: u32) {
    let mut uart = unsafe { regs::Uart::new_mmio_at(UART_ADDR) };
    uart.bank().write_data(value);
}

// Entry point for the linker. Calling through `black_box` keeps both functions in the binary.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let functions: [fn(u32); 2] = [write_control, write_bank_data];
    for f in core::hint::black_box(functions) {
        f(0);
    }
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
