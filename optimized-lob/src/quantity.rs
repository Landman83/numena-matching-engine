//quantity.rs

use std::ops::{AddAssign, SubAssign, Sub};

#[derive(Debug, Default, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct Qty(pub u32);

impl AddAssign for Qty {
    fn add_assign(&mut self, other: Qty) {
        self.0 += other.0;
    }
}

impl SubAssign for Qty {
    fn sub_assign(&mut self, other: Qty) {
        self.0 -= other.0;
    }
}

impl Sub for Qty {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl Qty {
    #[inline]
    pub fn value(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}
