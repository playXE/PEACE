pub fn is_power_of_two(x: usize) -> Option<u8> {
    use std::u8;

    let mut power_of_two = 1;
    let mut i: u8 = 0;
    while power_of_two < x && i < u8::MAX {
        power_of_two *= 2;
        i += 1;
    }

    if power_of_two == x {
        Some(i)
    } else {
        None
    }
}

/// aligns up a number
/// (returns the nearest multiply of the align value that is larger than the
/// given value)
#[inline(always)]
pub fn align_up(x: usize, align: usize) -> usize {
    //use ((x + align - 1)/align)*align if align is not a power of two
    debug_assert!(align.is_power_of_two());
    (x + align - 1) & !(align - 1)
}
