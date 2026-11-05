//! The WebAssembly interface used by the npm package and Go module.
//!
//! To validate a value, a host copies its UTF-8 bytes into the input buffer and calls
//! `appsync_validate`. The module has no allocator and no imports, so any WebAssembly runtime
//! can load it. An instance handles one call at a time.

#![cfg_attr(target_arch = "wasm32", no_std)]

use core::cell::UnsafeCell;

use appsync_scalars::{Reason, Scalar, validate};

/// Far longer than any valid value, so anything that does not fit is invalid.
const INPUT_CAPACITY: usize = 1024;

struct InputBuffer(UnsafeCell<[u8; INPUT_CAPACITY]>);

// SAFETY: hosts only touch the buffer between calls, and an instance handles one call at a time.
unsafe impl Sync for InputBuffer {}

static INPUT: InputBuffer = InputBuffer(UnsafeCell::new([0; INPUT_CAPACITY]));

#[unsafe(no_mangle)]
pub extern "C" fn appsync_input_ptr() -> *mut u8 {
    INPUT.0.get().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn appsync_input_capacity() -> usize {
    INPUT_CAPACITY
}

/// Checks the first `len` bytes of the input buffer, returning 0 if valid or a reason code.
/// `scalar` is 0 for AWSDate, 1 for AWSTime, 2 for AWSDateTime and 3 for AWSTimestamp.
/// A host holding more than `INPUT_CAPACITY` bytes passes a larger `len` and copies nothing.
#[unsafe(no_mangle)]
pub extern "C" fn appsync_validate(scalar: u32, len: usize) -> u32 {
    let scalar = match scalar {
        0 => Scalar::Date,
        1 => Scalar::Time,
        2 => Scalar::DateTime,
        3 => Scalar::Timestamp,
        _ => return reason_code(Reason::Format),
    };
    if len > INPUT_CAPACITY {
        return reason_code(Reason::Format);
    }
    // SAFETY: `len` is within the buffer, and the host is not writing during this call.
    let bytes = unsafe { core::slice::from_raw_parts(appsync_input_ptr().cast_const(), len) };
    let Ok(text) = core::str::from_utf8(bytes) else {
        return reason_code(Reason::Format);
    };
    match validate(scalar, text) {
        Ok(()) => 0,
        Err(error) => reason_code(error.reason()),
    }
}

/// The address of a reason code's message, or null for an unknown code.
#[unsafe(no_mangle)]
pub extern "C" fn appsync_reason_ptr(code: u32) -> *const u8 {
    reason_message(code).map_or(core::ptr::null(), str::as_ptr)
}

/// The byte length of a reason code's message, or 0 for an unknown code.
#[unsafe(no_mangle)]
pub extern "C" fn appsync_reason_len(code: u32) -> usize {
    reason_message(code).map_or(0, str::len)
}

fn reason_code(reason: Reason) -> u32 {
    let position = Reason::ALL.iter().position(|&candidate| candidate == reason);
    position.map_or(1, |index| index as u32 + 1)
}

fn reason_message(code: u32) -> Option<&'static str> {
    let index = code.checked_sub(1)? as usize;
    Reason::ALL.get(index).map(|reason| reason.message())
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(scalar: u32, value: &[u8]) -> u32 {
        unsafe {
            core::ptr::copy_nonoverlapping(value.as_ptr(), appsync_input_ptr(), value.len());
        }
        appsync_validate(scalar, value.len())
    }

    // The buffer is shared, so these checks run in one test rather than in parallel.
    #[test]
    fn validates_through_the_buffer() {
        assert_eq!(run(0, b"2021-01-11"), 0);
        assert_eq!(run(1, b"12:30"), 0);
        assert_eq!(run(2, b"2021-01-11T12:30Z"), 0);
        assert_eq!(run(3, b"1610368224"), 0);

        let code = run(2, b"2021-01-11T12:30");
        assert_eq!(code, reason_code(Reason::OffsetRequired));
        assert_eq!(reason_message(code), Some("time zone offset is required"));

        assert_eq!(run(0, &[0xff, 0xfe]), reason_code(Reason::Format));
        assert_eq!(run(99, b"2021-01-11"), reason_code(Reason::Format));
        assert_eq!(appsync_validate(0, INPUT_CAPACITY + 1), reason_code(Reason::Format));

        // Hosts can send any bytes, so every result has to be valid or a known reason.
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..50_000 {
            let bytes: Vec<u8> = (0..next() % 48).map(|_| next() as u8).collect();
            let code = run((next() % 5) as u32, &bytes);
            assert!(code == 0 || reason_message(code).is_some(), "{bytes:?} gave {code}");
        }
    }

    #[test]
    fn reason_lookup_handles_unknown_codes() {
        assert!(appsync_reason_ptr(0).is_null());
        assert_eq!(appsync_reason_len(0), 0);
        assert!(appsync_reason_ptr(Reason::ALL.len() as u32 + 1).is_null());
        for (index, reason) in Reason::ALL.iter().enumerate() {
            assert_eq!(appsync_reason_len(index as u32 + 1), reason.message().len());
        }
    }
}
