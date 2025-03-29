#[derive(derive_mmio::Mmio)]
#[repr(C)]
pub struct Uart {
    // No access modifiers: PureRead / Write / Modify by default
    control: u32,
    array_0: [u32; 4],
    array_1: [u32; 2],
    #[mmio(PureRead)]
    array_read_only: [u32; 4],
    // Write-only e.g. 1WC regs.
    #[mmio(Write)]
    array_write_only: [u32; 2],
}

fn main() {
    let mut uart = Uart {
        control: 0xC,
        array_0: [0x1, 0x2, 0x3, 0x4],
        array_1: [0x44, 0x22],
        array_read_only: [0x4, 0x3, 0x2, 0x1],
        array_write_only: [0, 0],
    };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };
    let val = mmio_uart.read_array_0(0).unwrap();
    assert_eq!(val, 0x1);
    println!("MMIO array 0 [0]: 0x{:X}", val);

    mmio_uart.write_array_0(0, 0x4).unwrap();
    let val = mmio_uart.read_array_0(0).unwrap();
    assert_eq!(val, 0x4);
    println!("MMIO array 0 [0]: 0x{:X}", val);

    mmio_uart.write_array_write_only(0, 0xFF).unwrap();
    mmio_uart.write_array_write_only(1, 0xFF).unwrap();

    let mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };

    let ro_array_0 = mmio_uart.read_array_read_only(0).unwrap();
    let ro_array_1 = mmio_uart.read_array_read_only(1).unwrap();
    // We can only use this to read the pure read-only array.
    assert_eq!(ro_array_0, 0x4);
    assert_eq!(ro_array_1, 0x3);
    println!("MMIO read-only array[0]: 0x{:X}", ro_array_0);
    println!("MMIO read-only array[1]: 0x{:X}", ro_array_1);
}
