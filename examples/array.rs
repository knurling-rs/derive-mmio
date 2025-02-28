#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // you can be explicit if you like
    control: u32,
    some_array_0: [u32; 4],
    some_array_1: [u32; 2],
}

fn main() {
    let mut uart = Uart {
        control: 0xC,
        some_array_0: [0x1, 0x2, 0x3, 0x4],
        some_array_1: [0x44, 0x22],
    };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };
    let val = mmio_uart.read_some_array_0(0).unwrap();
    assert_eq!(val, 0x1);

    mmio_uart.write_some_array_0(0, 0x4).unwrap();
    let val = mmio_uart.read_some_array_0(0).unwrap();
    assert_eq!(val, 0x4);
}
