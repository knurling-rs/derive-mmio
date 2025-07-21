#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct UartBank {
    // this is read-write by default
    data: u32,
    status: u32,
}

#[derive(derive_mmio::Mmio)]
#[mmio(no_ctors, const_ptr, const_inner)]
#[repr(C)]
struct Uart {
    // you can be explicit if you like
    control: u32,
    #[mmio(Inner)]
    bank_0: UartBank,
    #[mmio(Inner)]
    multi_banks: [UartBank; 2],
}

pub struct UartDriver {
    regs: MmioUart<'static>,
}

impl UartDriver {
    pub const fn const_ptr_to_control(&self) -> *mut u32 {
        self.regs.pointer_to_control()
    }

    pub const fn const_steal_bank_0(&mut self) -> MmioUartBank<'static> {
        unsafe { self.regs.steal_bank_0() }
    }

    pub const fn const_bank_0(&mut self) -> MmioUartBank<'_> {
        self.regs.bank_0()
    }

    pub const fn const_bank_0_shared(&mut self) -> derive_mmio::SharedInner<MmioUartBank<'_>> {
        self.regs.bank_0_shared()
    }

    pub const fn const_bank_1_shared(&mut self) -> MmioUartBank<'_> {
        unsafe { self.regs.multi_banks_unchecked(0) }
    }

    pub const fn const_steal_bank_2_shared(&mut self) -> MmioUartBank<'static> {
        unsafe { self.regs.steal_multi_banks_unchecked(1) }
    }

    pub const fn const_steal_bank_0_shared(
        &mut self,
    ) -> derive_mmio::SharedInner<MmioUartBank<'static>> {
        unsafe { self.regs.steal_bank_0_shared() }
    }
}

fn main() {}
