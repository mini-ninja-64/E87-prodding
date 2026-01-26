use core::{fmt, num};
use std::ops::Add;

pub fn checksum(data: &[u8]) -> u8 {
    // return data.iter().cloned().map(Wrapping).sum::<Wrapping<u8>>().0;
    let mut value = 0u8;
    for n in data {
        value = value.wrapping_add(*n);
    }
    return value;
}

#[test]
fn checksum_test() {
    let result = checksum(&[0x62, 0xC6, 0x01, 0x00, 0x01]);
    assert_eq!(result, 0x2A)
}

pub struct Counter<T> {
    id: T,
    start: T,
    increment: T,
    max: T,
}

impl<T> Counter<T>
where
    T: num_traits::PrimInt,
    T: fmt::Display,
{
    pub fn new(start: T, increment: T, max: T) -> Counter<T> {
        return Counter {
            id: start,
            max: max,
            start,
            increment,
        };
    }
    pub fn next(&mut self) -> T {
        let value = self.id;

        self.id = self.id + self.increment;
        if self.id >= self.max {
            self.id = self.start;
        }
        println!("p: {}", value);

        return value;
    }
}
