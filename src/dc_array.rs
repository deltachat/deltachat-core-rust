use crate::dc_location::dc_location;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

const DC_ARRAY_MAGIC: uint32_t = 0x000a11aa;

/* * the structure behind dc_array_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_array_t {
    pub magic: uint32_t,
    pub allocated: size_t,
    pub count: size_t,
    pub type_0: libc::c_int,
    pub array: *mut uintptr_t,
}

/**
 * @class dc_array_t
 *
 * An object containing a simple array.
 * This object is used in several places where functions need to return an array.
 * The items of the array are typically IDs.
 * To free an array object, use dc_array_unref().
 */
pub unsafe fn dc_array_unref(mut array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC {
        return;
    }
    if (*array).type_0 == 1i32 {
        dc_array_free_ptr(array);
    }
    free((*array).array as *mut libc::c_void);
    (*array).magic = 0i32 as uint32_t;
    free(array as *mut libc::c_void);
}

pub unsafe fn dc_array_free_ptr(array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC {
        return;
    }
    let mut i: size_t = 0i32 as size_t;
    while i < (*array).count {
        Box::from_raw(*(*array).array.offset(i as isize) as *mut dc_location);
        *(*array).array.offset(i as isize) = 0i32 as uintptr_t;
        i = i.wrapping_add(1)
    }
}

pub unsafe fn dc_array_add_uint(mut array: *mut dc_array_t, item: uintptr_t) {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC {
        return;
    }
    if (*array).count == (*array).allocated {
        let newsize = (*array).allocated.wrapping_mul(2).wrapping_add(10);
        (*array).array = realloc(
            (*array).array as *mut libc::c_void,
            (newsize).wrapping_mul(::std::mem::size_of::<uintptr_t>()),
        ) as *mut uintptr_t;
        assert!(!(*array).array.is_null());
        (*array).allocated = newsize as size_t
    }
    *(*array).array.offset((*array).count as isize) = item;
    (*array).count = (*array).count.wrapping_add(1);
}

pub unsafe fn dc_array_add_id(array: *mut dc_array_t, item: uint32_t) {
    dc_array_add_uint(array, item as uintptr_t);
}

pub unsafe fn dc_array_add_ptr(array: *mut dc_array_t, item: *mut libc::c_void) {
    dc_array_add_uint(array, item as uintptr_t);
}

pub unsafe fn dc_array_get_cnt(array: *const dc_array_t) -> size_t {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC {
        return 0i32 as size_t;
    }
    (*array).count
}

pub unsafe fn dc_array_get_uint(array: *const dc_array_t, index: size_t) -> uintptr_t {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC || index >= (*array).count {
        return 0i32 as uintptr_t;
    }
    *(*array).array.offset(index as isize)
}

pub unsafe fn dc_array_get_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC || index >= (*array).count {
        return 0i32 as uint32_t;
    }
    if (*array).type_0 == 1i32 {
        return (*(*(*array).array.offset(index as isize) as *mut dc_location)).location_id;
    }
    *(*array).array.offset(index as isize) as uint32_t
}

pub unsafe fn dc_array_get_ptr(array: *const dc_array_t, index: size_t) -> *mut libc::c_void {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC || index >= (*array).count {
        return 0 as *mut libc::c_void;
    }
    *(*array).array.offset(index as isize) as *mut libc::c_void
}

pub unsafe fn dc_array_get_latitude(array: *const dc_array_t, index: size_t) -> libc::c_double {
    if array.is_null()
        || (*array).magic != DC_ARRAY_MAGIC
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as libc::c_double;
    }
    (*(*(*array).array.offset(index as isize) as *mut dc_location)).latitude
}

pub unsafe fn dc_array_get_longitude(array: *const dc_array_t, index: size_t) -> libc::c_double {
    if array.is_null()
        || (*array).magic != DC_ARRAY_MAGIC
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as libc::c_double;
    }
    (*(*(*array).array.offset(index as isize) as *mut dc_location)).longitude
}

pub unsafe fn dc_array_get_accuracy(array: *const dc_array_t, index: size_t) -> libc::c_double {
    if array.is_null()
        || (*array).magic != DC_ARRAY_MAGIC
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as libc::c_double;
    }
    (*(*(*array).array.offset(index as isize) as *mut dc_location)).accuracy
}

pub unsafe fn dc_array_get_timestamp(array: *const dc_array_t, index: size_t) -> i64 {
    if array.is_null()
        || (*array).magic != DC_ARRAY_MAGIC
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0;
    }
    (*(*(*array).array.offset(index as isize) as *mut dc_location)).timestamp
}

pub unsafe fn dc_array_get_chat_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null()
        || (*array).magic != DC_ARRAY_MAGIC
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as uint32_t;
    }
    (*(*(*array).array.offset(index as isize) as *mut dc_location)).chat_id
}

pub unsafe fn dc_array_get_contact_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null()
        || (*array).magic != DC_ARRAY_MAGIC
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as uint32_t;
    }
    (*(*(*array).array.offset(index as isize) as *mut dc_location)).contact_id
}

pub unsafe fn dc_array_get_msg_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null()
        || (*array).magic != DC_ARRAY_MAGIC
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as uint32_t;
    }
    (*(*(*array).array.offset(index as isize) as *mut dc_location)).msg_id
}

pub unsafe fn dc_array_get_marker(array: *const dc_array_t, index: size_t) -> *mut libc::c_char {
    if array.is_null()
        || (*array).magic != DC_ARRAY_MAGIC
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0 as *mut libc::c_char;
    }
    if let Some(s) = &(*(*(*array).array.offset(index as isize) as *mut dc_location)).marker {
        to_cstring(s)
    } else {
        std::ptr::null_mut()
    }
}

/**
 * Return the independent-state of the location at the given index.
 * Independent locations do not belong to the track of the user.
 *
 * @memberof dc_array_t
 * @param array The array object.
 * @param index Index of the item. Must be between 0 and dc_array_get_cnt()-1.
 * @return 0=Location belongs to the track of the user,
 *     1=Location was reported independently.
 */
pub unsafe fn dc_array_is_independent(array: *const dc_array_t, index: size_t) -> libc::c_int {
    if array.is_null()
        || (*array).magic != DC_ARRAY_MAGIC
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0;
    }

    (*(*(*array).array.offset(index as isize) as *mut dc_location)).independent as libc::c_int
}

pub unsafe fn dc_array_search_id(
    array: *const dc_array_t,
    needle: uint32_t,
    ret_index: *mut size_t,
) -> bool {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC {
        return false;
    }
    let data: *mut uintptr_t = (*array).array;
    let mut i: size_t = 0;
    let cnt: size_t = (*array).count;
    while i < cnt {
        if *data.offset(i as isize) == needle as size_t {
            if !ret_index.is_null() {
                *ret_index = i
            }
            return true;
        }
        i = i.wrapping_add(1)
    }
    false
}

pub unsafe fn dc_array_get_raw(array: *const dc_array_t) -> *const uintptr_t {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC {
        return 0 as *const uintptr_t;
    }
    (*array).array
}

pub unsafe fn dc_array_new(initsize: size_t) -> *mut dc_array_t {
    dc_array_new_typed(0, initsize)
}

pub unsafe fn dc_array_new_typed(type_0: libc::c_int, initsize: size_t) -> *mut dc_array_t {
    let mut array: *mut dc_array_t;
    array = calloc(1, ::std::mem::size_of::<dc_array_t>()) as *mut dc_array_t;
    assert!(!array.is_null());

    (*array).magic = DC_ARRAY_MAGIC;
    (*array).count = 0i32 as size_t;
    (*array).allocated = if initsize < 1 { 1 } else { initsize };
    (*array).type_0 = type_0;
    (*array).array = malloc(
        (*array)
            .allocated
            .wrapping_mul(::std::mem::size_of::<uintptr_t>()),
    ) as *mut uintptr_t;
    if (*array).array.is_null() {
        exit(48i32);
    }
    array
}

pub unsafe fn dc_array_empty(mut array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC {
        return;
    }
    (*array).count = 0i32 as size_t;
}

pub unsafe fn dc_array_duplicate(array: *const dc_array_t) -> *mut dc_array_t {
    let mut ret: *mut dc_array_t;
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC {
        return 0 as *mut dc_array_t;
    }
    ret = dc_array_new((*array).allocated);
    (*ret).count = (*array).count;
    memcpy(
        (*ret).array as *mut libc::c_void,
        (*array).array as *const libc::c_void,
        (*array)
            .count
            .wrapping_mul(::std::mem::size_of::<uintptr_t>()),
    );
    ret
}

pub unsafe fn dc_array_sort_ids(array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC || (*array).count <= 1 {
        return;
    }
    qsort(
        (*array).array as *mut libc::c_void,
        (*array).count,
        ::std::mem::size_of::<uintptr_t>(),
        Some(cmp_intptr_t),
    );
}

unsafe extern "C" fn cmp_intptr_t(p1: *const libc::c_void, p2: *const libc::c_void) -> libc::c_int {
    let v1: uintptr_t = *(p1 as *mut uintptr_t);
    let v2: uintptr_t = *(p2 as *mut uintptr_t);
    return if v1 < v2 {
        -1i32
    } else if v1 > v2 {
        1i32
    } else {
        0i32
    };
}

pub unsafe fn dc_array_sort_strings(array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC || (*array).count <= 1 {
        return;
    }
    qsort(
        (*array).array as *mut libc::c_void,
        (*array).count,
        ::std::mem::size_of::<*mut libc::c_char>(),
        Some(cmp_strings_t),
    );
}

unsafe extern "C" fn cmp_strings_t(
    p1: *const libc::c_void,
    p2: *const libc::c_void,
) -> libc::c_int {
    let v1: *const libc::c_char = *(p1 as *mut *const libc::c_char);
    let v2: *const libc::c_char = *(p2 as *mut *const libc::c_char);

    strcmp(v1, v2)
}

pub unsafe fn dc_array_get_string(
    array: *const dc_array_t,
    sep: *const libc::c_char,
) -> *mut libc::c_char {
    if array.is_null() || (*array).magic != DC_ARRAY_MAGIC || sep.is_null() {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    let cnt = (*array).count as usize;
    let slice = std::slice::from_raw_parts((*array).array, cnt);
    let sep = as_str(sep);

    let res = slice
        .iter()
        .enumerate()
        .fold(String::with_capacity(2 * cnt), |mut res, (i, n)| {
            if i == 0 {
                res += &n.to_string();
            } else {
                res += sep;
                res += &n.to_string();
            }
            res
        });
    to_cstring(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_dc_array() {
        unsafe {
            let arr = dc_array_new(7 as size_t);
            assert_eq!(dc_array_get_cnt(arr), 0);

            for i in 0..1000 {
                dc_array_add_id(arr, (i + 2) as uint32_t);
            }

            assert_eq!(dc_array_get_cnt(arr), 1000);

            for i in 0..1000 {
                assert_eq!(
                    dc_array_get_id(arr, i as size_t),
                    (i + 1i32 * 2i32) as libc::c_uint
                );
            }

            assert_eq!(dc_array_get_id(arr, -1i32 as size_t), 0);
            assert_eq!(dc_array_get_id(arr, 1000 as size_t), 0);
            assert_eq!(dc_array_get_id(arr, 1001 as size_t), 0);

            dc_array_empty(arr);

            assert_eq!(dc_array_get_cnt(arr), 0);

            dc_array_add_id(arr, 13 as uint32_t);
            dc_array_add_id(arr, 7 as uint32_t);
            dc_array_add_id(arr, 666 as uint32_t);
            dc_array_add_id(arr, 0 as uint32_t);
            dc_array_add_id(arr, 5000 as uint32_t);

            dc_array_sort_ids(arr);

            assert_eq!(dc_array_get_id(arr, 0 as size_t), 0);
            assert_eq!(dc_array_get_id(arr, 1 as size_t), 7);
            assert_eq!(dc_array_get_id(arr, 2 as size_t), 13);
            assert_eq!(dc_array_get_id(arr, 3 as size_t), 666);

            let str = dc_array_get_string(arr, b"-\x00" as *const u8 as *const libc::c_char);
            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                "0-7-13-666-5000"
            );
            free(str as *mut libc::c_void);

            dc_array_empty(arr);

            dc_array_add_ptr(
                arr,
                b"XX\x00" as *const u8 as *const libc::c_char as *mut libc::c_void,
            );
            dc_array_add_ptr(
                arr,
                b"item1\x00" as *const u8 as *const libc::c_char as *mut libc::c_void,
            );
            dc_array_add_ptr(
                arr,
                b"bbb\x00" as *const u8 as *const libc::c_char as *mut libc::c_void,
            );
            dc_array_add_ptr(
                arr,
                b"aaa\x00" as *const u8 as *const libc::c_char as *mut libc::c_void,
            );
            dc_array_sort_strings(arr);

            let str = dc_array_get_ptr(arr, 0 as size_t) as *mut libc::c_char;
            assert_eq!(CStr::from_ptr(str).to_str().unwrap(), "XX");

            let str = dc_array_get_ptr(arr, 1 as size_t) as *mut libc::c_char;
            assert_eq!(CStr::from_ptr(str).to_str().unwrap(), "aaa");

            let str = dc_array_get_ptr(arr, 2 as size_t) as *mut libc::c_char;
            assert_eq!(CStr::from_ptr(str).to_str().unwrap(), "bbb");

            let str = dc_array_get_ptr(arr, 3 as size_t) as *mut libc::c_char;
            assert_eq!(CStr::from_ptr(str).to_str().unwrap(), "item1");

            dc_array_unref(arr);
        }
    }

}
