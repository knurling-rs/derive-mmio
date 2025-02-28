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
    let _ = mmio_uart.bank_1().read_data();
    let _ = bank0.read_data();
}
