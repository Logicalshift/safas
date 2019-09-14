///
/// How SAFAS represents a number
///
#[derive(Copy, Clone, Debug)]
pub enum SafasNumber {
    /// A number that was specified with no precision
    Plain(u128),

    /// A number that should occupy the specified number of bits
    BitNumber(u8, u128)
}
