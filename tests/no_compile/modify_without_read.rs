
#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // Modify without read access specifier.
    #[mmio(Modify, Write)]
    fifo: u32,
}

fn main() {
    let mut uart = Uart { fifo: 0x0 };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };
    mmio_uart.modify_uart();
}
