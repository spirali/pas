struct BitIterator {
    value: u64,
    count: u32,
}

impl Iterator for BitIterator {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        self.count -= 1;
        let result = (self.value & 1) == 1;
        self.value >>= 1;
        Some(result)
    }
}

pub fn iterate_bits_no_lz(value: u64) -> impl Iterator<Item=bool> {
    let lz = value.leading_zeros();
    BitIterator {
        value: if value > 0 { value.reverse_bits() >> lz } else { 0 },
        count: std::mem::size_of::<u64>() as u32 * 8 - lz,
    }
}

#[test]
fn test_iterate_bits() {
    let v: Vec<bool> = iterate_bits_no_lz(0b111).collect();
    assert_eq!(v, vec![true, true, true]);

    let v: Vec<bool> = iterate_bits_no_lz(0b11001).collect();
    assert_eq!(v, vec![true, true, false, false, true]);

    let v: Vec<bool> = iterate_bits_no_lz(0b11000).collect();
    assert_eq!(v, vec![true, true, false, false, false]);

    let v: Vec<bool> = iterate_bits_no_lz(0b0).collect();
    assert_eq!(v, vec![]);

    let v: Vec<bool> = iterate_bits_no_lz(0b1).collect();
    assert_eq!(v, vec![true]);
}