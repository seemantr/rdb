use std::{mem, ptr, slice};

/// Get the raw byte representation of a struct
#[inline(always)]
unsafe fn to_slice<'a, T>(p: *const u8) -> &'a [u8] {
    slice::from_raw_parts(p, mem::size_of::<T>())
}

/// Create an struct from the pointer
#[inline(always)]
pub fn from_ptr<T>(p: *const u8) -> T {
    unsafe { ptr::read(p as *const T) }
}

/// Create an struct from the pointer
#[inline(always)]
pub fn from_ptr_with_offset<T>(p: *const u8, offset: isize) -> T {
    unsafe { ptr::read(p.offset(offset) as *const T) }
}

#[inline(always)]
fn u8(p: *const u8, offset: isize) -> u8 {
    from_ptr_with_offset::<u8>(p, offset)
}

#[inline(always)]
fn b0(p: *const u8) -> u8 {
    from_ptr_with_offset::<u8>(p, 0)
}

#[inline(always)]
fn b1(p: *const u8) -> u8 {
    from_ptr_with_offset::<u8>(p, 1)
}

#[inline(always)]
fn b2(p: *const u8) -> u8 {
    from_ptr_with_offset::<u8>(p, 2)
}

#[inline(always)]
fn b3(p: *const u8) -> u8 {
    from_ptr_with_offset::<u8>(p, 3)
}

#[inline(always)]
fn b4(p: *const u8) -> u8 {
    from_ptr_with_offset::<u8>(p, 4)
}

#[inline(always)]
fn b5(p: *const u8) -> u8 {
    from_ptr_with_offset::<u8>(p, 5)
}

#[inline(always)]
fn b6(p: *const u8) -> u8 {
    from_ptr_with_offset::<u8>(p, 6)
}

#[inline(always)]
fn b7(p: *const u8) -> u8 {
    from_ptr_with_offset::<u8>(p, 7)
}

#[inline(always)]
pub fn encode<T>(p: *const u8, v: T) {
    encode_with_offset(p, 0, v)
}

#[inline(always)]
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

const VARINT_CUT1: u64 = 201;
const VARINT_CUT2: u64 = 249;

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
/// V <= 12487              => b0 = (V - 200)/256 + 201; b1 = (V - 200) % 256
/// V <= 65535              => b0 = 249; [b1, b2] = 2 byte integer
/// V <= 16777215           => b0 = 250; [b1..b3] = 3 byte integer
/// V <= 4294967295         => b0 = 251; [b1..b4] = 4 byte integer
/// V <= 1099511627775      => b0 = 252; [b1..b5] = 5 byte integer
/// V <= 281474976710655    => b0 = 253; [b1..b6] = 6 byte integer
/// V <= 72057594037927935  => b0 = 254; [b1..b7] = 7 byte integer
/// V                       => b0 = 255; [b1..b8] = 8 byte integer
#[inline]
pub fn encode_varint_u64(p: *const u8, v: u64) -> u64 {
    let mut v = v;
    if v < VARINT_CUT1 {
        encode_with_offset(p, 0, v as u8);
        1
    } else if v < 12488 {
        v -= 200;
        encode_with_offset(p, 0, ((v >> 8) + VARINT_CUT1) as u8);
        encode_with_offset(p, 1, (v & 255) as u8);
        2
    } else {
        // 3-9 bytes
        let bits = 64 - v.leading_zeros();
        let bytes = (bits + 7) / 8;
        let b0 = VARINT_CUT2 + (bytes as u64 - 2);
        //trace!("Encoder: input:{}, bits:{}, bytes:{}, b0:{}", v, bits, bytes, b0);
        encode(p, b0 as u8);
        for i in 1..bytes + 1 {
            encode_with_offset(p, i as u64, v as u8);
            //trace!("Encoder: b{}:{}", i, v as u8);
            v >>= 8;
        }
        bytes as u64
    }
}

/// Decodes a varint
///
/// b0 >= 0 && b0 <= 200, b0
/// b0 >= 201 && b0 <= 248, 200 + 256 * (b0 - 201) + b1, Max value = 12487
/// b0 = 249, b1..b2 are u16, Max value = 2^16 - 1
/// b0 = 250, b1..b3 are u24, Max value = 2^24 - 1
/// b0 = 251, b1..b4 are u32, Max value = 2^32 - 1
/// b0 = 252, b1..b5 are u40, Max value = 2^40 - 1
/// b0 = 253, b1..b6 are u48, Max value = 2^48 - 1
/// b0 = 254, b1..b7 are u56, Max value = 2^56 - 1
/// b0 = 255, b1..b7 are u56, Max value = 2^64 - 1
#[inline]
pub fn decode_varint_u64(p: *const u8) -> u64 {
    let mut b0 = b0(p) as u64;
    if b0 < VARINT_CUT1 {
        b0 as u64
    } else if b0 < VARINT_CUT2 {
        200 + ((b0 - 201) << 8) as u64 + b1(p) as u64
    } else {
        let bytes = b0 - VARINT_CUT2 + 2;
        let mut v: u64 = 0;
        //trace!("Decoder: bytes:{}, b0:{}", bytes, b0);
        for i in 1..bytes + 1 {
            b0 = from_ptr_with_offset::<u8>(p, i as isize) as u64;
            v |= b0 << (8 * (i - 1));
            //trace!("Decoder: b{}:{}", i, v);
        }
        //trace!("Decoder: output:{}, bytes:{}", v, bytes);
        v
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    extern crate env_logger;
    use super::*;
    use self::test::Bencher;

    const TEST_NUMBERS: [u64; 30] = [0,
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

    //#[bench]
    fn bench_encode_leb_u32(b: &mut Bencher) {
        let target = [0 as u8; 4];
        let mut value = 0;
        b.iter(|| {
                   encode_leb_u32(target.as_ptr(), value);
                   value += 100;
               })
    }

    #[derive(Copy, Clone)]
    struct EncoderDecoder(fn(*const u8, u64) -> u64, fn(*const u8) -> u64);
    const ENC_LEB128: EncoderDecoder = EncoderDecoder(encode_leb_u64, decode_leb_u64);
    const ENC_VARINT: EncoderDecoder = EncoderDecoder(encode_varint_u64, decode_varint_u64);

    /// Helper setup method to run a range of benchmarks and tests.
    /// Having a common method will ensure that all the encoding types are
    /// tested using the same style.
    fn encode_decode_range(enc: EncoderDecoder,
                           fullRange: bool,
                           testSubset: bool,
                           smallNumbers: bool,
                           largeNumber: bool,
                           decode: bool,
                           testVector: bool)
                           -> Vec<[u8; 8]> {
        let EncoderDecoder(enc, dec) = enc;
        let mut encoded = vec![];
        if fullRange {
            for sut in 0..1000000 {
                let target = [0 as u8; 8];
                let res = enc(target.as_ptr(), sut);
                if decode {
                    assert_eq!(dec(target.as_ptr()), sut);
                }
            }
        }

        if testSubset {
            for sut in TEST_NUMBERS.iter() {
                let target = [0 as u8; 8];
                let res = enc(target.as_ptr(), *sut);
                if decode {
                    assert_eq!(dec(target.as_ptr()), *sut);
                }
            }
        }

        if smallNumbers {
            for sut in TEST_NUMBERS.iter().take(15) {
                let target = [0 as u8; 8];
                let res = enc(target.as_ptr(), *sut);
                if testVector {
                    encoded.push(target);
                }
                if decode {
                    assert_eq!(dec(target.as_ptr()), *sut);
                }
            }
        }

        if largeNumber {
            for sut in TEST_NUMBERS.iter().skip(15) {
                let target = [0 as u8; 8];
                let res = enc(target.as_ptr(), *sut);
                if testVector {
                    encoded.push(target);
                }
                if decode {
                    assert_eq!(dec(target.as_ptr()), *sut);
                }
            }
        }
        encoded
    }

    fn full_test_suite(enc: EncoderDecoder) {
        encode_decode_range(enc, true, true, false, false, true, false);
    }

    #[test]
    fn can_encode_and_decode_varint_64() {
        let _ = env_logger::init();
        full_test_suite(ENC_VARINT);
    }

    #[test]
    fn can_encode_and_decode_leb_64() {
        full_test_suite(ENC_LEB128);
    }

    fn bench_encode(enc: EncoderDecoder, smallNumbers: bool, largeNumbers: bool, b: &mut Bencher) {  
        b.iter(|| encode_decode_range(enc, false, false, smallNumbers, largeNumbers, false, false))
    }

    /*
    Benchmarks for encoding and decoding
    */
    #[bench]
    fn bench_encode_leb_u64_small(b: &mut Bencher) {
        bench_encode(ENC_LEB128, true, false, b);
    }

    #[bench]
    fn bench_encode_varint_u64_small(b: &mut Bencher) {
        bench_encode(ENC_VARINT, true, false, b);
    }

    #[bench]
    fn bench_encode_leb_u64_large(b: &mut Bencher) {
        bench_encode(ENC_LEB128, false, true, b);
    }

    #[bench]
    fn bench_encode_varint_u64_large(b: &mut Bencher) {
        bench_encode(ENC_VARINT, false, true, b);
    }

    fn bench_decode(enc: EncoderDecoder, smallNumbers: bool, largeNumbers: bool, b: &mut Bencher) {
        let encoded =
            encode_decode_range(enc, false, false, smallNumbers, largeNumbers, false, true);
        let mut res = 0;
        let EncoderDecoder(enc, dec) = enc;
        b.iter(|| for sut in encoded.iter() {
                   res = dec(sut.as_ptr());
               })
    }

    #[bench]
    fn bench_decode_leb_u64_small(b: &mut Bencher) {
        bench_decode(ENC_LEB128, true, false, b);
    }

    #[bench]
    fn bench_decode_varint_u64_small(b: &mut Bencher) {
        bench_decode(ENC_VARINT, true, false, b);
    }

    #[bench]
    fn bench_decode_leb_u64_large(b: &mut Bencher) {
        bench_decode(ENC_LEB128, false, true, b);
    }

    #[bench]
    fn bench_decode_varint_u64_large(b: &mut Bencher) {
        bench_decode(ENC_VARINT, false, true, b);
    }
}
