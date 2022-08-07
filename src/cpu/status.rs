#![allow(non_snake_case)]

bitflags! {
    pub struct Status: u8 {
        const NEGATIVE    = 0x80;
        const OVERFLOW    = 0x40;
        const PUSH_IRQ    = 0x20;
        const BRK  = 0x10;
        const DECIMAL     = 0x08;
        const INT_DISABLE = 0x04;
        const ZERO        = 0x02;
        const CARRY       = 0x01;
    }
}

impl Default for Status {
    fn default() -> Self {
        Status::PUSH_IRQ | Status::INT_DISABLE
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty() {
        assert_eq!(Status::empty().bits(), 0);
    }
}
