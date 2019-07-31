use crate::dc_location::dc_location;
use crate::dc_tools::*;
use crate::types::*;

/* * the structure behind dc_array_t */
#[derive(Clone)]
#[allow(non_camel_case_types)]
pub enum dc_array_t {
    Locations(Vec<dc_location>),
    Uint(Vec<uintptr_t>),
}

impl dc_array_t {
    pub fn new(capacity: usize) -> Self {
        dc_array_t::Uint(Vec::with_capacity(capacity))
    }

    /// Constructs a new, empty `dc_array_t` holding locations with specified `capacity`.
    pub fn new_locations(capacity: usize) -> Self {
        dc_array_t::Locations(Vec::with_capacity(capacity))
    }

    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    pub fn add_uint(&mut self, item: uintptr_t) {
        if let Self::Uint(array) = self {
            array.push(item);
        } else {
            panic!("Attempt to add uint to array of other type");
        }
    }

    pub fn add_id(&mut self, item: uint32_t) {
        self.add_uint(item as uintptr_t);
    }

    pub fn add_location(&mut self, location: dc_location) {
        if let Self::Locations(array) = self {
            array.push(location)
        } else {
            panic!("Attempt to add a location to array of other type");
        }
    }

    pub fn get_uint(&self, index: usize) -> uintptr_t {
        if let Self::Uint(array) = self {
            array[index]
        } else {
            panic!("Attempt to get uint from array of other type");
        }
    }

    pub fn get_id(&self, index: usize) -> uint32_t {
        match self {
            Self::Locations(array) => array[index].location_id,
            Self::Uint(array) => array[index] as uint32_t,
        }
    }

    pub fn get_ptr(&self, index: size_t) -> *mut libc::c_void {
        if let Self::Uint(array) = self {
            array[index] as *mut libc::c_void
        } else {
            panic!("Not an array of pointers");
        }
    }

    pub fn get_location(&self, index: usize) -> &dc_location {
        if let Self::Locations(array) = self {
            &array[index]
        } else {
            panic!("Not an array of locations")
        }
    }

    pub fn get_latitude(&self, index: usize) -> libc::c_double {
        self.get_location(index).latitude
    }

    pub fn get_longitude(&self, index: size_t) -> libc::c_double {
        self.get_location(index).longitude
    }

    pub fn get_accuracy(&self, index: size_t) -> libc::c_double {
        self.get_location(index).accuracy
    }

    pub fn get_timestamp(&self, index: size_t) -> i64 {
        self.get_location(index).timestamp
    }

    pub fn get_chat_id(&self, index: size_t) -> uint32_t {
        self.get_location(index).chat_id
    }

    pub fn get_contact_id(&self, index: size_t) -> uint32_t {
        self.get_location(index).contact_id
    }

    pub fn get_msg_id(&self, index: size_t) -> uint32_t {
        self.get_location(index).msg_id
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Locations(array) => array.is_empty(),
            Self::Uint(array) => array.is_empty(),
        }
    }

    /// Returns the number of elements in the array.
    pub fn len(&self) -> usize {
        match self {
            Self::Locations(array) => array.len(),
            Self::Uint(array) => array.len(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            Self::Locations(array) => array.clear(),
            Self::Uint(array) => array.clear(),
        }
    }

    pub fn search_id(&self, needle: uintptr_t) -> Option<usize> {
        if let Self::Uint(array) = self {
            for (i, &u) in array.iter().enumerate() {
                if u == needle {
                    return Some(i);
                }
            }
            None
        } else {
            panic!("Attempt to search for id in array of other type");
        }
    }

    pub fn sort_ids(&mut self) {
        if let dc_array_t::Uint(v) = self {
            v.sort();
        } else {
            panic!("Attempt to sort array of something other than uints");
        }
    }
}

impl From<Vec<dc_location>> for dc_array_t {
    fn from(array: Vec<dc_location>) -> Self {
        dc_array_t::Locations(array)
    }
}

pub unsafe fn dc_array_unref(array: *mut dc_array_t) {
    if array.is_null() {
        return;
    }
    Box::from_raw(array);
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
        0
    } else {
        (*array).len()
    }
}

pub unsafe fn dc_array_get_uint(array: *const dc_array_t, index: size_t) -> uintptr_t {
    if array.is_null() || index >= (*array).len() {
        0
    } else {
        (*array).get_uint(index)
    }
}

pub unsafe fn dc_array_get_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null() || index >= (*array).len() {
        0
    } else {
        (*array).get_id(index)
    }
}

pub unsafe fn dc_array_get_ptr(array: *const dc_array_t, index: size_t) -> *mut libc::c_void {
    if array.is_null() || index >= (*array).len() {
        std::ptr::null_mut()
    } else {
        (*array).get_ptr(index)
    }
}

pub unsafe fn dc_array_get_latitude(array: *const dc_array_t, index: size_t) -> libc::c_double {
    if array.is_null() || index >= (*array).len() {
        0.0
    } else {
        (*array).get_latitude(index)
    }
}

pub unsafe fn dc_array_get_longitude(array: *const dc_array_t, index: size_t) -> libc::c_double {
    if array.is_null() || index >= (*array).len() {
        0.0
    } else {
        (*array).get_longitude(index)
    }
}

pub unsafe fn dc_array_get_accuracy(array: *const dc_array_t, index: size_t) -> libc::c_double {
    if array.is_null() || index >= (*array).len() {
        0.0
    } else {
        (*array).get_accuracy(index)
    }
}

pub unsafe fn dc_array_get_timestamp(array: *const dc_array_t, index: size_t) -> i64 {
    if array.is_null() || index >= (*array).len() {
        0
    } else {
        (*array).get_timestamp(index)
    }
}

pub unsafe fn dc_array_get_chat_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null() || index >= (*array).len() {
        0
    } else {
        (*array).get_chat_id(index)
    }
}

pub unsafe fn dc_array_get_contact_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null() || index >= (*array).len() {
        0
    } else {
        (*array).get_contact_id(index)
    }
}

pub unsafe fn dc_array_get_msg_id(array: *const dc_array_t, index: size_t) -> uint32_t {
    if array.is_null() || index >= (*array).len() {
        0
    } else {
        (*array).get_msg_id(index)
    }
}

pub unsafe fn dc_array_get_marker(array: *const dc_array_t, index: size_t) -> *mut libc::c_char {
    if array.is_null() || index >= (*array).len() {
        return std::ptr::null_mut();
    }

    if let dc_array_t::Locations(v) = &*array {
        if let Some(s) = &v[index].marker {
            to_cstring(s)
        } else {
            std::ptr::null_mut()
        }
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
    if array.is_null() || index >= (*array).len() {
        return 0;
    }

    if let dc_array_t::Locations(v) = &*array {
        v[index].independent as libc::c_int
    } else {
        panic!("Attempt to get location independent field from array of something other than locations");
    }
}

pub unsafe fn dc_array_search_id(
    array: *const dc_array_t,
    needle: uint32_t,
    ret_index: *mut size_t,
) -> bool {
    if array.is_null() {
        return false;
    }
    if let Some(i) = (*array).search_id(needle as uintptr_t) {
        if !ret_index.is_null() {
            *ret_index = i
        }
        true
    } else {
        false
    }
}

pub unsafe fn dc_array_get_raw(array: *const dc_array_t) -> *const uintptr_t {
    if array.is_null() {
        return 0 as *const uintptr_t;
    }
    if let dc_array_t::Uint(v) = &*array {
        v.as_ptr()
    } else {
        panic!("Attempt to convert array of something other than uints to raw");
    }
}

pub fn dc_array_new(initsize: size_t) -> *mut dc_array_t {
    dc_array_t::new(initsize).into_raw()
}

pub fn dc_array_new_locations(initsize: size_t) -> *mut dc_array_t {
    dc_array_t::new_locations(initsize).into_raw()
}

pub unsafe fn dc_array_empty(array: *mut dc_array_t) {
    if array.is_null() {
        return;
    }
    (*array).clear()
}

pub unsafe fn dc_array_duplicate(array: *const dc_array_t) -> *mut dc_array_t {
    if array.is_null() {
        std::ptr::null_mut()
    } else {
        (*array).clone().into_raw()
    }
}

pub unsafe fn dc_array_get_string(
    array: *const dc_array_t,
    sep: *const libc::c_char,
) -> *mut libc::c_char {
    if array.is_null() || sep.is_null() {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    if let dc_array_t::Uint(v) = &*array {
        let cnt = v.len();
        let sep = as_str(sep);

        let res = v
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
    } else {
        panic!("Attempt to get string from array of other type");
    }
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

            (*arr).sort_ids();

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
