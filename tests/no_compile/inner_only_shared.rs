#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct UartBank {
    // this is read-write by default
    data: u32,
}

#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    #[mmio(inner)]
    bank_0: UartBank,
}

fn main() {
    let mut uart = Uart {
        bank_0: UartBank {
            data: 0x1,
        },
    };
    // Safety: We're pointing at a real object
    let mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };

    // Shared inner block, can only use shared API.
    let bank0_shared = mmio_uart.bank_0_shared();
    bank0_shared.write_data(0x2);
}
