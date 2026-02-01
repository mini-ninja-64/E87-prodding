use core::{fmt, num};
use std::{num::Wrapping, ops::Add};

pub fn checksum(data: &[u8]) -> u8 {
    // return data.iter().cloned().map(Wrapping).sum::<Wrapping<u8>>().0;
    let mut value = 0i8;
    for n in data {
        value = value.wrapping_add(*n as i8);
    }
    return value as u8;
}

#[test]
fn checksum_test() {
    let result = checksum(&[0x62, 0xC6, 0x01, 0x00, 0x01]);
    assert_eq!(result, 0x2A);
}

pub trait JdkHash {
    fn jdk_hash_code(&self) -> i32;
}

impl JdkHash for str {
    fn jdk_hash_code(&self) -> i32 {
        let encoded = self.encode_utf16();
        let len = encoded.clone().count();
        encoded
            .enumerate()
            .map(|(i, char)| {
                let char_value = char as i32;
                let multiplicand = 31i32.wrapping_pow((len - (i + 1)) as u32);
                let result = char_value.wrapping_mul(multiplicand);
                return Wrapping(result as i32);
            })
            .sum::<Wrapping<i32>>()
            .0
    }
}

#[test]
fn jdk_hash_test_str() {
    let hash_code = "🥺".jdk_hash_code();
    assert_eq!(hash_code, 1772796i32);

    let hash_code = "aasd🥺fghj🥺kl;;'asd's".jdk_hash_code();
    assert_eq!(hash_code, -1411885244i32);
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
