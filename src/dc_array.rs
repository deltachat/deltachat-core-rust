use crate::dc_location::dc_location;
use crate::dc_tools::*;
use crate::types::*;

const DC_ARRAY_LOCATIONS: libc::c_int = 1;

/* * the structure behind dc_array_t */
#[derive(Clone)]
pub struct dc_array_t {
    pub type_0: libc::c_int,
    pub array: Vec<uintptr_t>,
}

impl dc_array_t {
    pub fn new(capacity: usize) -> Self {
        dc_array_t {
            type_0: 0,
            array: Vec::with_capacity(capacity),
        }
    }

    pub fn as_ptr(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    pub fn add_uint(&mut self, item: uintptr_t) {
        self.array.push(item);
    }

    pub fn add_id(&mut self, item: uint32_t) {
        self.add_uint(item as uintptr_t);
    }
}

/**
 * @class dc_array_t
 *
 * An object containing a simple array.
 * This object is used in several places where functions need to return an array.
 * The items of the array are typically IDs.
 * To free an array object, use dc_array_unref().
 */
pub unsafe fn dc_array_unref(array: *mut dc_array_t) {
    if array.is_null() {
        return;
    }
    if (*array).type_0 == DC_ARRAY_LOCATIONS {
        dc_array_free_ptr(array);
    }
    Box::from_raw(array);
}

pub unsafe fn dc_array_free_ptr(array: *mut dc_array_t) {
    if array.is_null() {
        return;
    }
    for i in 0..(*array).array.len() {
        if (*array).type_0 == DC_ARRAY_LOCATIONS {
            Box::from_raw((*array).array[i] as *mut dc_location);
        } else {
            free((*array).array[i] as *mut libc::c_void);
        }
        (*array).array[i] = 0i32 as uintptr_t;
    }
}

pub unsafe fn dc_array_add_uint(array: *mut dc_array_t, item: uintptr_t) {
    if !array.is_null() {
        (*array).add_uint(item);
    }
}

pub unsafe fn dc_array_add_id(array: *mut dc_array_t, item: uint32_t) {
    if !array.is_null() {
        (*array).add_id(item);
    }
}

pub unsafe fn dc_array_add_ptr(array: *mut dc_array_t, item: *mut libc::c_void) {
    dc_array_add_uint(array, item as uintptr_t);
}

pub unsafe fn dc_array_get_cnt(array: *const dc_array_t) -> size_t {
    if array.is_null() {
        return 0i32 as size_t;
    }
    (*array).array.len()
}

pub unsafe fn dc_array_get_uint(array: *const dc_array_t, index: size_t) -> uintptr_t {
    if array.is_null() || index >= (*array).array.len() {
        return 0i32 as uintptr_t;
    }
    (*array).array[index]
}

pub unsafe fn dc_array_get_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null() || index >= (*array).array.len() {
        return 0i32 as uint32_t;
    }
    if (*array).type_0 == DC_ARRAY_LOCATIONS {
        return (*((*array).array[index] as *mut dc_location)).location_id;
    }
    (*array).array[index] as uint32_t
}

pub unsafe fn dc_array_get_ptr(array: *const dc_array_t, index: size_t) -> *mut libc::c_void {
    if array.is_null() || index >= (*array).array.len() {
        return 0 as *mut libc::c_void;
    }
    (*array).array[index] as *mut libc::c_void
}

pub unsafe fn dc_array_get_latitude(array: *const dc_array_t, index: size_t) -> libc::c_double {
    if array.is_null()
        || index >= (*array).array.len()
        || (*array).type_0 != DC_ARRAY_LOCATIONS
        || (*array).array[index] == 0
    {
        return 0i32 as libc::c_double;
    }
    (*((*array).array[index] as *mut dc_location)).latitude
}

pub unsafe fn dc_array_get_longitude(array: *const dc_array_t, index: size_t) -> libc::c_double {
    if array.is_null()
        || index >= (*array).array.len()
        || (*array).type_0 != DC_ARRAY_LOCATIONS
        || (*array).array[index] == 0
    {
        return 0i32 as libc::c_double;
    }
    (*((*array).array[index] as *mut dc_location)).longitude
}

pub unsafe fn dc_array_get_accuracy(array: *const dc_array_t, index: size_t) -> libc::c_double {
    if array.is_null()
        || index >= (*array).array.len()
        || (*array).type_0 != DC_ARRAY_LOCATIONS
        || (*array).array[index] == 0
    {
        return 0i32 as libc::c_double;
    }
    (*((*array).array[index] as *mut dc_location)).accuracy
}

pub unsafe fn dc_array_get_timestamp(array: *const dc_array_t, index: size_t) -> i64 {
    if array.is_null()
        || index >= (*array).array.len()
        || (*array).type_0 != DC_ARRAY_LOCATIONS
        || (*array).array[index] == 0
    {
        return 0;
    }
    (*((*array).array[index] as *mut dc_location)).timestamp
}

pub unsafe fn dc_array_get_chat_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null()
        || index >= (*array).array.len()
        || (*array).type_0 != DC_ARRAY_LOCATIONS
        || (*array).array[index] == 0
    {
        return 0i32 as uint32_t;
    }
    (*((*array).array[index] as *mut dc_location)).chat_id
}

pub unsafe fn dc_array_get_contact_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null()
        || index >= (*array).array.len()
        || (*array).type_0 != DC_ARRAY_LOCATIONS
        || (*array).array[index] == 0
    {
        return 0i32 as uint32_t;
    }
    (*((*array).array[index] as *mut dc_location)).contact_id
}

pub unsafe fn dc_array_get_msg_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null()
        || index >= (*array).array.len()
        || (*array).type_0 != DC_ARRAY_LOCATIONS
        || (*array).array[index] == 0
    {
        return 0i32 as uint32_t;
    }
    (*((*array).array[index] as *mut dc_location)).msg_id
}

pub unsafe fn dc_array_get_marker(array: *const dc_array_t, index: size_t) -> *mut libc::c_char {
    if array.is_null()
        || index >= (*array).array.len()
        || (*array).type_0 != DC_ARRAY_LOCATIONS
        || (*array).array[index] == 0
    {
        return 0 as *mut libc::c_char;
    }
    if let Some(s) = &(*((*array).array[index] as *mut dc_location)).marker {
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
        || index >= (*array).array.len()
        || (*array).type_0 != DC_ARRAY_LOCATIONS
        || (*array).array[index] == 0
    {
        return 0;
    }

    (*((*array).array[index] as *mut dc_location)).independent as libc::c_int
}

pub unsafe fn dc_array_search_id(
    array: *const dc_array_t,
    needle: uint32_t,
    ret_index: *mut size_t,
) -> bool {
    if array.is_null() {
        return false;
    }
    for (i, &u) in (*array).array.iter().enumerate() {
        if u == needle as size_t {
            if !ret_index.is_null() {
                *ret_index = i
            }
            return true;
        }
    }
    false
}

pub unsafe fn dc_array_get_raw(array: *const dc_array_t) -> *const uintptr_t {
    if array.is_null() {
        return 0 as *const uintptr_t;
    }
    (*array).array.as_ptr()
}

pub fn dc_array_new(initsize: size_t) -> *mut dc_array_t {
    dc_array_new_typed(0, initsize)
}

pub fn dc_array_new_typed(type_0: libc::c_int, initsize: size_t) -> *mut dc_array_t {
    let capacity = if initsize < 1 { 1 } else { initsize as usize };
    let mut array = dc_array_t::new(capacity);
    array.type_0 = type_0;
    array.as_ptr()
}

pub unsafe fn dc_array_empty(array: *mut dc_array_t) {
    if array.is_null() {
        return;
    }
    (*array).array.clear();
}

pub unsafe fn dc_array_duplicate(array: *const dc_array_t) -> *mut dc_array_t {
    if array.is_null() {
        std::ptr::null_mut()
    } else {
        (*array).clone().as_ptr()
    }
}

pub unsafe fn dc_array_sort_ids(array: *mut dc_array_t) {
    if array.is_null() || (*array).array.len() <= 1 {
        return;
    }
    (*array).array.sort();
}

pub unsafe fn dc_array_get_string(
    array: *const dc_array_t,
    sep: *const libc::c_char,
) -> *mut libc::c_char {
    if array.is_null() || sep.is_null() {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    let cnt = (*array).array.len();
    let sep = as_str(sep);

    let res =
        (*array)
            .array
            .iter()
            .enumerate()
            .fold(String::with_capacity(2 * cnt), |res, (i, n)| {
                if i == 0 {
                    res + &n.to_string()
                } else {
                    res + sep + &n.to_string()
                }
            });
    to_cstring(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x::*;
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

            dc_array_unref(arr);
        }
    }

}
