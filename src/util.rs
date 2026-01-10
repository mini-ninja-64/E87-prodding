pub fn checksum(data: &[u8]) -> u8 {
    // return data.iter().cloned().map(Wrapping).sum::<Wrapping<u8>>().0;
    let mut value = 0u8;
    for n in data {
        value = value.wrapping_add(*n);
    }
    return value;
}
