#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // this field is read-only (no write_x or modify_x method)
    #[mmio(PureRead)]
    status: u32,
}

fn main() {
    let mut uart = Uart { status: 0xF };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };

    let ptr_raw = core::ptr::addr_of_mut!(uart);
    mmio_uart.write_status();
}
