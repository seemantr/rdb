use std::{mem, ptr, slice};
use std::mem::transmute;

/// Get the raw byte representation of a struct
#[inline]
unsafe fn to_slice<'a, T>(p: *const u8) -> &'a [u8] {
    slice::from_raw_parts(p, mem::size_of::<T>())
}

/// Create an struct from the pointer
#[inline]
pub fn from_ptr<T>(p: *const u8) -> T {
    unsafe { ptr::read(p as *const T) }
}

/// Create an struct from the pointer
#[inline]
pub fn from_ptr_with_offset<T>(p: *const u8, offset: isize) -> T {
    unsafe { ptr::read(p.offset(offset) as *const T) }
}

#[inline]
pub fn encode<T>(p: *const u8, v: T) {
    encode_with_offset(p, 0, v)
}

#[inline]
pub fn encode_with_offset<T>(p: *const u8, offset: u64, v: T) {
    unsafe { ptr::write(p.offset(offset as isize) as *mut T, v) }
}

/// Based on: https://github.com/google/leveldb/blob/master/util/coding.cc
#[inline]
pub fn encode_leb_u64(p: *const u8, v: u64) -> u64 {
    let mut offset = 0;
    let mut value = v;
    while value > 127 {
        encode_with_offset(p, offset, (value as u8) | 128);
        offset += 1;
        value >>= 7;
    }
    encode_with_offset(p, offset, value as u8);
    offset
}

#[inline]
pub fn decode_leb_u64(p: *const u8) -> u64 {
    let mut b0 = from_ptr::<u8>(p);
    if b0 < 128 {
        return b0 as u64;
    }

    let mut val: u64 = (b0 & 0x7f) as u64;
    let mut offset = 1;
    let mut shift = 7;
    loop {
        b0 = from_ptr_with_offset::<u8>(p, offset);
        val |= ((b0 & 0x7f) as u64) << shift;
        shift += 7;
        offset += 1;
        if b0 < 128 {
            break;
        }
    }
    val
}

#[inline]
pub fn encode_leb_u32(p: *const u8, value: u32) -> u64 {
    if value < (1 << 7) {
        encode_with_offset(p, 0, value as u8);
        1
    } else if value < (1 << 14) {
        encode_with_offset(p, 0, value | 128);
        encode_with_offset(p, 1, value >> 7);
        2
    } else if value < (1 << 21) {
        encode_with_offset(p, 0, value | 128);
        encode_with_offset(p, 1, (value >> 7) | 128);
        encode_with_offset(p, 2, value >> 14);
        3
    } else if value < (1 << 28) {
        encode_with_offset(p, 0, value | 128);
        encode_with_offset(p, 1, (value >> 7) | 128);
        encode_with_offset(p, 2, (value >> 14) | 128);
        encode_with_offset(p, 3, value >> 21);
        4
    } else {
        encode_with_offset(p, 0, value | 128);
        encode_with_offset(p, 1, (value >> 7) | 128);
        encode_with_offset(p, 2, (value >> 14) | 128);
        encode_with_offset(p, 3, (value >> 21) | 128);
        encode_with_offset(p, 4, value >> 28);
        5
    }
}

#[inline]
pub fn decode_leb_u32(p: *const u8) -> u32 {
    decode_leb_u64(p) as u32
}

/// This encoding is inspired by SQLite var encoding with minor differences.
/// The maximum value represented by 2 bytes is higher than sqlite varint
/// as we want to represnt atleast a value of 4096 using 2 bytes. This is to
/// ensure that any place in a page (4096 bytes) can be addressed using 2 bytes
/// instead of 3 bytes.
///
/// The maximum number that can be necoded with this is 2^56-1 which is lower
/// than 64 bit but is alright for our purpose.
///
/// V <= 200                => b0 = V
/// V <= 12743              => b0 = (V - 200)/256 + 201; b1 = (V - 200) % 256
/// V <= 78278              => b0 = 250; [b1, b2] = V - 12743
/// V <= 16777215           => b0 = 251; [b1..b3] = 3 byte integer
/// V <= 4294967295         => b0 = 252; [b1..b4] = 4 byte integer
/// V <= 1099511627775      => b0 = 253; [b1..b5] = 5 byte integer
/// V <= 281474976710655    => b0 = 254; [b1..b6] = 6 byte integer
/// V <= 72057594037927935  => b0 = 255; [b1..b7] = 7 byte integer
#[inline]
pub fn encode_varint_u64(p: *const u8, v: u64) -> u64 {
    if v < 201 {
        encode_with_offset(p, 0, v as u8);
        1
    } else if v < 12744 {
        encode_with_offset(p, 0, ((v - 200) / 256 + 201) as u8);
        encode_with_offset(p, 1, ((v - 200) % 256) as u8);
        2
    } else if v < 78279 {
        encode_with_offset(p, 0, 250 as u8);
        let v = v - 12743;
        encode_with_offset(p, 1, v as u8);
        encode_with_offset(p, 2, (v >> 8) as u8);
        3
    } else if v < 16777216 {
        encode_with_offset(p, 0, 251 as u8);
        encode_with_offset(p, 1, v as u8);
        encode_with_offset(p, 2, (v >> 8) as u8);
        encode_with_offset(p, 3, (v >> 16) as u8);
        4
    } else if v < 4294967296 {
        encode_with_offset(p, 0, 252 as u8);
        encode_with_offset(p, 1, v as u8);
        encode_with_offset(p, 2, (v >> 8) as u8);
        encode_with_offset(p, 3, (v >> 16) as u8);
        encode_with_offset(p, 4, (v >> 24) as u8);
        5
    } else if v < 1099511627776 {
        encode_with_offset(p, 0, 253 as u8);
        encode_with_offset(p, 1, v as u8);
        encode_with_offset(p, 2, (v >> 8) as u8);
        encode_with_offset(p, 3, (v >> 16) as u8);
        encode_with_offset(p, 4, (v >> 24) as u8);
        encode_with_offset(p, 5, (v >> 32) as u8);
        6
    } else if v < 281474976710656 {
        encode_with_offset(p, 0, 254 as u8);
        encode_with_offset(p, 1, v as u8);
        encode_with_offset(p, 2, (v >> 8) as u8);
        encode_with_offset(p, 3, (v >> 16) as u8);
        encode_with_offset(p, 4, (v >> 24) as u8);
        encode_with_offset(p, 5, (v >> 32) as u8);
        encode_with_offset(p, 6, (v >> 40) as u8);
        7
    } else if v < 72057594037927936 {
        encode_with_offset(p, 0, 255 as u8);
        encode_with_offset(p, 1, v as u8);
        encode_with_offset(p, 2, (v >> 8) as u8);
        encode_with_offset(p, 3, (v >> 16) as u8);
        encode_with_offset(p, 4, (v >> 24) as u8);
        encode_with_offset(p, 5, (v >> 32) as u8);
        encode_with_offset(p, 6, (v >> 40) as u8);
        encode_with_offset(p, 7, (v >> 48) as u8);
        8
    } else {
        panic!("Out of range number");
    }
}

/// Decodes a varint
///
/// b0 >= 0 && b0 <= 200, b0
/// b0 >= 201 && b0 <= 249, 200 + 256 * (b0 - 201) + b1, Max value = 12743
/// b0 = 250, b1 & b2 are u16, Max value = 2^16 - 1 + 12743 = 78278
/// b0 = 251, b1..b3 are u24, Max value = 2^24 - 1
/// b0 = 252, b1..b4 are u32, Max value = 2^32 - 1
/// b0 = 253, b1..b5 are u40, Max value = 2^40 - 1
/// b0 = 254, b1..b6 are u48, Max value = 2^48 - 1
/// b0 = 255, b1..b7 are u56, Max value = 2^56 - 1
#[inline]
pub fn decode_varint_u64(p: *const u8) -> u64 {
    let b0 = from_ptr::<u8>(p);
    if b0 < 201 {
        b0 as u64
    } else if b0 < 250 {
        let b1 = from_ptr_with_offset::<u8>(p, 1);
        200 + 256 * (b0 - 201) as u64 + b1 as u64
    } else if b0 == 250 {
        let b1 = from_ptr_with_offset::<u8>(p, 1);
        let b2 = from_ptr_with_offset::<u8>(p, 2);
        (b1 as u64 | (b2 as u64) << 8) + 12743
    } else if b0 == 251 {
        let b1 = from_ptr_with_offset::<u8>(p, 1);
        let b2 = from_ptr_with_offset::<u8>(p, 2);
        let b3 = from_ptr_with_offset::<u8>(p, 3);
        b1 as u64 | (b2 as u64) << 8 | (b3 as u64) << 16
    } else if b0 == 252 {
        let b1 = from_ptr_with_offset::<u8>(p, 1);
        let b2 = from_ptr_with_offset::<u8>(p, 2);
        let b3 = from_ptr_with_offset::<u8>(p, 3);
        let b4 = from_ptr_with_offset::<u8>(p, 4);
        b1 as u64 | (b2 as u64) << 8 | (b3 as u64) << 16 | (b4 as u64) << 24
    } else if b0 == 253 {
        let b1 = from_ptr_with_offset::<u8>(p, 1);
        let b2 = from_ptr_with_offset::<u8>(p, 2);
        let b3 = from_ptr_with_offset::<u8>(p, 3);
        let b4 = from_ptr_with_offset::<u8>(p, 4);
        let b5 = from_ptr_with_offset::<u8>(p, 5);
        b1 as u64 | (b2 as u64) << 8 | (b3 as u64) << 16 | (b4 as u64) << 24 | (b5 as u64) << 32
    } else if b0 == 254 {
        let b1 = from_ptr_with_offset::<u8>(p, 1);
        let b2 = from_ptr_with_offset::<u8>(p, 2);
        let b3 = from_ptr_with_offset::<u8>(p, 3);
        let b4 = from_ptr_with_offset::<u8>(p, 4);
        let b5 = from_ptr_with_offset::<u8>(p, 5);
        let b6 = from_ptr_with_offset::<u8>(p, 6);
        b1 as u64 | (b2 as u64) << 8 | (b3 as u64) << 16 | (b4 as u64) << 24 | (b5 as u64) << 32 |
        (b6 as u64) << 40
    } else if b0 == 255 {
        let b1 = from_ptr_with_offset::<u8>(p, 1);
        let b2 = from_ptr_with_offset::<u8>(p, 2);
        let b3 = from_ptr_with_offset::<u8>(p, 3);
        let b4 = from_ptr_with_offset::<u8>(p, 4);
        let b5 = from_ptr_with_offset::<u8>(p, 5);
        let b6 = from_ptr_with_offset::<u8>(p, 6);
        let b7 = from_ptr_with_offset::<u8>(p, 7);
        b1 as u64 | (b2 as u64) << 8 | (b3 as u64) << 16 | (b4 as u64) << 24 |
        (b5 as u64) << 32 | (b6 as u64) << 40 | (b7 as u64) << 48
    } else {
        panic!("Out of range number");
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use self::test::Bencher;

    const TEST_NUMBERS : [u64; 30] = [0,
                                 1,
                                 254,
                                 255,
                                 256,
                                 1023,
                                 1024,
                                 1025,
                                 12742,
                                 12743,
                                 12744,
                                 65534,
                                 65535,
                                 65536,
                                 78277,
                                 78278,
                                 78279,
                                 16777214,
                                 16777215,
                                 16777217,
                                 4294967294,
                                 4294967295,
                                 4294967296,
                                 1099511627774,
                                 1099511627775,
                                 1099511627776,
                                 281474976710654,
                                 281474976710655,
                                 72057594037927934,
                                 72057594037927935];
    
    #[quickcheck]
    fn can_encode_and_decode_int32(sut: u32) {
        let target = [0 as u8; 4];
        encode(target.as_ptr(), sut);
        assert_eq!(from_ptr::<u32>(target.as_ptr()), sut);
    }

    #[quickcheck]
    fn can_encode_and_decode_leb32(sut: u32) {
        let target = [0 as u8; 4];
        encode_leb_u32(target.as_ptr(), sut);
        assert_eq!(decode_leb_u32(target.as_ptr()), sut);
    }

    #[inline]
    fn encode_decode_range(enc : fn(*const u8, u64) -> u64, dec : fn(*const u8) -> u64) {
        for sut in 0..1000000 {
            let target = [0 as u8; 8];
            enc(target.as_ptr(), sut);
            assert_eq!(dec(target.as_ptr()), sut);
        }

        for sut in TEST_NUMBERS.iter() {
            let target = [0 as u8; 8];
            enc(target.as_ptr(), *sut);
            assert_eq!(dec(target.as_ptr()), *sut);
        }
    }

    #[test]
    fn can_encode_and_decode_varint_64() {
        encode_decode_range(encode_varint_u64, decode_varint_u64);
    }

    #[test]
    fn can_encode_and_decode_leb_64() {
        encode_decode_range(encode_leb_u64, decode_leb_u64);
    }

    //#[bench]
    fn encode_speed_leb_u32(b: &mut Bencher) {
        let target = [0 as u8; 4];
        let mut value = 0;
        b.iter(|| {
                   encode_leb_u32(target.as_ptr(), value);
                   value += 100;
               })
    }

    #[bench]
    fn bench_encode_varint_u64(b: &mut Bencher) {
        let target = [0 as u8; 8];
        b.iter(|| {
            for sut in 0..100000 {
                let a = encode_varint_u64(target.as_ptr(), sut);
            }  
        })
    }

    #[bench]
    fn bench_encode_leb_u64(b: &mut Bencher) {
        let target = [0 as u8; 8];
        b.iter(|| {
                for sut in 0..100000 {
                   let a = encode_leb_u64(target.as_ptr(), sut);
                }
            })
    }

    #[bench]
    fn bench_encode_decode_varint_u64(b: &mut Bencher) {
        b.iter(|| { can_encode_and_decode_varint_64() })
    } 

    #[bench]
    fn bench_encode_decode_leb_u64(b: &mut Bencher) {
        let mut value = 0;
        let mut res = 0;
        b.iter(|| { can_encode_and_decode_leb_64() })
    }   
}
