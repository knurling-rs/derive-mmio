#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct UartBank {
    // this is read-write by default
    data: u32,
    status: u32,
}

#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // you can be explicit if you like
    control: u32,
    #[mmio(inner)]
    bank_0: UartBank,
    #[mmio(inner)]
    bank_1: UartBank,
}

fn main() {
    let mut uart = Uart {
        control: 0xC,
        bank_0: UartBank {
            data: 0x1,
            status: 0x2,
        },
        bank_1: UartBank {
            data: 0x3,
            status: 0x4,
        },
    };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };
    let mut bank0 = mmio_uart.bank_0();
    let bank0_data = bank0.read_data();
    assert_eq!(bank0_data, 0x1);
    let bank0_status = bank0.read_status();
    assert_eq!(bank0_status, 0x2);
    bank0.write_status(0x5);
    assert_eq!(bank0.read_status(), 0x5);
    let bank1_data = mmio_uart.bank_1().read_data();
    assert_eq!(bank1_data, 0x3);
    let bank1_data = mmio_uart.bank_1().read_status();
    assert_eq!(bank1_data, 0x4);

    unsafe {
        let inner_owned_for_p0 = mmio_uart.steal_bank_0();
        let inner_owned_for_p1 = mmio_uart.steal_bank_0();
        // Can be used independently now.
        assert_eq!(inner_owned_for_p0.read_data(), 0x1);
        assert_eq!(inner_owned_for_p1.read_data(), 0x1);
    }
}
