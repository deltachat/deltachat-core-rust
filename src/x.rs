use crate::types::*;

pub use libc::{
    calloc, exit, free, malloc, memcmp, memcpy, memmove, memset, realloc, strcat, strchr, strcmp,
    strcpy, strcspn, strlen, strncmp, strncpy, strrchr, strspn, strstr, strtol, system,
};

pub unsafe fn strdup(s: *const libc::c_char) -> *mut libc::c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }

    let slen = strlen(s);
    let result = malloc(slen + 1);
    if result.is_null() {
        return std::ptr::null_mut();
    }

    memcpy(result, s as *const _, slen + 1);
    result as *mut _
}

pub fn strndup(s: *const libc::c_char, n: libc::c_ulong) -> *mut libc::c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }

    let end = std::cmp::min(n as usize, unsafe { strlen(s) });
    unsafe {
        let result = malloc(end + 1);
        memcpy(result, s as *const _, end);
        std::ptr::write_bytes(result.offset(end as isize), b'\x00', 1);

        result as *mut _
    }
}

extern "C" {
    pub fn clock() -> libc::clock_t;
    pub fn qsort(
        __base: *mut libc::c_void,
        __nel: size_t,
        __width: size_t,
        __compar: Option<
            unsafe extern "C" fn(_: *const libc::c_void, _: *const libc::c_void) -> libc::c_int,
        >,
    );
    pub fn vsnprintf(
        _: *mut libc::c_char,
        _: libc::c_ulong,
        _: *const libc::c_char,
        _: ::std::ffi::VaList,
    ) -> libc::c_int;

    // -- DC Methods
    pub fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
}

pub(crate) unsafe fn strcasecmp(s1: *const libc::c_char, s2: *const libc::c_char) -> libc::c_int {
    let s1 = std::ffi::CStr::from_ptr(s1)
        .to_string_lossy()
        .to_lowercase();
    let s2 = std::ffi::CStr::from_ptr(s2)
        .to_string_lossy()
        .to_lowercase();
    if s1 == s2 {
        0
    } else {
        1
    }
}

pub(crate) unsafe fn strncasecmp(
    s1: *const libc::c_char,
    s2: *const libc::c_char,
    n: libc::size_t,
) -> libc::c_int {
    let s1 = std::ffi::CStr::from_ptr(s1)
        .to_string_lossy()
        .to_lowercase();
    let s2 = std::ffi::CStr::from_ptr(s2)
        .to_string_lossy()
        .to_lowercase();
    let m1 = std::cmp::min(n, s1.len());
    let m2 = std::cmp::min(n, s2.len());

    if s1[..m1] == s2[..m2] {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dc_tools::to_string;

    #[test]
    fn test_strndup() {
        unsafe {
            let res = strndup(b"helloworld\x00" as *const u8 as *const libc::c_char, 4);
            assert_eq!(
                to_string(res),
                to_string(b"hell\x00" as *const u8 as *const libc::c_char)
            );
            assert_eq!(strlen(res), 4);
            free(res as *mut _);
        }
    }
}
