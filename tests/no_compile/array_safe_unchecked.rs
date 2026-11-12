#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    array: [u32; 2],
}

fn main() {
    let mut uart = Uart { array: [0; 2] };

    // Safety: We're pointing at a real object
    let mmio_uart = unsafe { MmioUart::new(core::ptr::addr_of_mut!(uart)) };

    // we cannot safely access the array field without bounds checks

    let _inner_bank = mmio_uart.read_array_unchecked(5);
}
