use libc;

use crate::dc_context::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

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
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return;
    }
    if (*array).type_0 == 1i32 {
        dc_array_free_ptr(array);
    }
    free((*array).array as *mut libc::c_void);
    (*array).magic = 0i32 as uint32_t;
    free(array as *mut libc::c_void);
}

pub unsafe fn dc_array_free_ptr(mut array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return;
    }
    let mut i: size_t = 0i32 as size_t;
    while i < (*array).count {
        if (*array).type_0 == 1i32 {
            free(
                (*(*(*array).array.offset(i as isize) as *mut _dc_location)).marker
                    as *mut libc::c_void,
            );
        }
        free(*(*array).array.offset(i as isize) as *mut libc::c_void);
        *(*array).array.offset(i as isize) = 0i32 as uintptr_t;
        i = i.wrapping_add(1)
    }
}

pub unsafe fn dc_array_add_uint(mut array: *mut dc_array_t, mut item: uintptr_t) {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return;
    }
    if (*array).count == (*array).allocated {
        let mut newsize = (*array).allocated.wrapping_mul(2).wrapping_add(10);
        (*array).array = realloc(
            (*array).array as *mut libc::c_void,
            (newsize).wrapping_mul(::std::mem::size_of::<uintptr_t>()),
        ) as *mut uintptr_t;
        if (*array).array.is_null() {
            exit(49i32);
        }
        (*array).allocated = newsize as size_t
    }
    *(*array).array.offset((*array).count as isize) = item;
    (*array).count = (*array).count.wrapping_add(1);
}

pub unsafe fn dc_array_add_id(mut array: *mut dc_array_t, mut item: uint32_t) {
    dc_array_add_uint(array, item as uintptr_t);
}

pub unsafe fn dc_array_add_ptr(mut array: *mut dc_array_t, mut item: *mut libc::c_void) {
    dc_array_add_uint(array, item as uintptr_t);
}

pub unsafe fn dc_array_get_cnt(mut array: *const dc_array_t) -> size_t {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return 0i32 as size_t;
    }
    (*array).count
}

pub unsafe fn dc_array_get_uint(mut array: *const dc_array_t, mut index: size_t) -> uintptr_t {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint || index >= (*array).count {
        return 0i32 as uintptr_t;
    }
    *(*array).array.offset(index as isize)
}

pub unsafe fn dc_array_get_id(mut array: *const dc_array_t, mut index: size_t) -> uint32_t {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint || index >= (*array).count {
        return 0i32 as uint32_t;
    }
    if (*array).type_0 == 1i32 {
        return (*(*(*array).array.offset(index as isize) as *mut _dc_location)).location_id;
    }
    *(*array).array.offset(index as isize) as uint32_t
}

pub unsafe fn dc_array_get_ptr(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> *mut libc::c_void {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint || index >= (*array).count {
        return 0 as *mut libc::c_void;
    }
    *(*array).array.offset(index as isize) as *mut libc::c_void
}

pub unsafe fn dc_array_get_latitude(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> libc::c_double {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as libc::c_double;
    }
    (*(*(*array).array.offset(index as isize) as *mut _dc_location)).latitude
}

pub unsafe fn dc_array_get_longitude(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> libc::c_double {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as libc::c_double;
    }
    (*(*(*array).array.offset(index as isize) as *mut _dc_location)).longitude
}

pub unsafe fn dc_array_get_accuracy(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> libc::c_double {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as libc::c_double;
    }
    (*(*(*array).array.offset(index as isize) as *mut _dc_location)).accuracy
}

pub unsafe fn dc_array_get_timestamp(mut array: *const dc_array_t, mut index: size_t) -> time_t {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as time_t;
    }
    (*(*(*array).array.offset(index as isize) as *mut _dc_location)).timestamp
}

pub unsafe fn dc_array_get_chat_id(mut array: *const dc_array_t, mut index: size_t) -> uint32_t {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as uint32_t;
    }
    (*(*(*array).array.offset(index as isize) as *mut _dc_location)).chat_id
}

pub unsafe fn dc_array_get_contact_id(mut array: *const dc_array_t, mut index: size_t) -> uint32_t {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as uint32_t;
    }
    (*(*(*array).array.offset(index as isize) as *mut _dc_location)).contact_id
}

pub unsafe fn dc_array_get_msg_id(mut array: *const dc_array_t, mut index: size_t) -> uint32_t {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0i32 as uint32_t;
    }
    (*(*(*array).array.offset(index as isize) as *mut _dc_location)).msg_id
}

pub unsafe fn dc_array_get_marker(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> *mut libc::c_char {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0 as *mut libc::c_char;
    }
    dc_strdup_keep_null((*(*(*array).array.offset(index as isize) as *mut _dc_location)).marker)
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
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0
    {
        return 0;
    }

    (*(*(*array).array.offset(index as isize) as *mut _dc_location)).independent as libc::c_int
}

// TODO should return bool /rtn
pub unsafe fn dc_array_search_id(
    mut array: *const dc_array_t,
    mut needle: uint32_t,
    mut ret_index: *mut size_t,
) -> libc::c_int {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return 0i32;
    }
    let mut data: *mut uintptr_t = (*array).array;
    let mut i: size_t = 0;
    let mut cnt: size_t = (*array).count;
    while i < cnt {
        if *data.offset(i as isize) == needle as size_t {
            if !ret_index.is_null() {
                *ret_index = i
            }
            return 1i32;
        }
        i = i.wrapping_add(1)
    }
    0
}

pub unsafe fn dc_array_get_raw(mut array: *const dc_array_t) -> *const uintptr_t {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return 0 as *const uintptr_t;
    }
    (*array).array
}

pub unsafe fn dc_array_new(initsize: size_t) -> *mut dc_array_t {
    dc_array_new_typed(0, initsize)
}

pub unsafe extern "C" fn dc_array_new_typed(
    mut type_0: libc::c_int,
    mut initsize: size_t,
) -> *mut dc_array_t {
    let mut array: *mut dc_array_t;
    array = calloc(1, ::std::mem::size_of::<dc_array_t>()) as *mut dc_array_t;
    if array.is_null() {
        exit(47i32);
    }
    (*array).magic = 0xa11aai32 as uint32_t;
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
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return;
    }
    (*array).count = 0i32 as size_t;
}

pub unsafe fn dc_array_duplicate(mut array: *const dc_array_t) -> *mut dc_array_t {
    let mut ret: *mut dc_array_t;
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
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

pub unsafe fn dc_array_sort_ids(mut array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint || (*array).count <= 1 {
        return;
    }
    qsort(
        (*array).array as *mut libc::c_void,
        (*array).count,
        ::std::mem::size_of::<uintptr_t>(),
        Some(cmp_intptr_t),
    );
}

unsafe extern "C" fn cmp_intptr_t(
    mut p1: *const libc::c_void,
    mut p2: *const libc::c_void,
) -> libc::c_int {
    let mut v1: uintptr_t = *(p1 as *mut uintptr_t);
    let mut v2: uintptr_t = *(p2 as *mut uintptr_t);
    return if v1 < v2 {
        -1i32
    } else if v1 > v2 {
        1i32
    } else {
        0i32
    };
}

pub unsafe fn dc_array_sort_strings(mut array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint || (*array).count <= 1 {
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
    mut p1: *const libc::c_void,
    mut p2: *const libc::c_void,
) -> libc::c_int {
    let mut v1: *const libc::c_char = *(p1 as *mut *const libc::c_char);
    let mut v2: *const libc::c_char = *(p2 as *mut *const libc::c_char);

    strcmp(v1, v2)
}

pub unsafe fn dc_array_get_string(
    mut array: *const dc_array_t,
    mut sep: *const libc::c_char,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char;
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint || sep.is_null() {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    let mut i = 0;
    ret = malloc(
        (*array)
            .count
            .wrapping_mul((11usize).wrapping_add(strlen(sep)))
            .wrapping_add(1),
    ) as *mut libc::c_char;
    if ret.is_null() {
        exit(35i32);
    }
    *ret.offset(0isize) = 0i32 as libc::c_char;
    while i < (*array).count {
        if 0 != i {
            strcat(ret, sep);
        }
        sprintf(
            &mut *ret.offset(strlen(ret) as isize) as *mut libc::c_char,
            b"%lu\x00" as *const u8 as *const libc::c_char,
            *(*array).array.offset(i as isize) as libc::c_ulong,
        );
        i += 1
    }

    ret
}

pub unsafe fn dc_arr_to_string(
    mut arr: *const uint32_t,
    mut cnt: libc::c_int,
) -> *mut libc::c_char {
    /* return comma-separated value-string from integer array */
    let mut ret: *mut libc::c_char;
    let mut sep: *const libc::c_char = b",\x00" as *const u8 as *const libc::c_char;
    if arr.is_null() || cnt <= 0i32 {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    let mut i: libc::c_int;
    ret = malloc(
        (cnt as usize)
            .wrapping_mul((11usize).wrapping_add(strlen(sep)))
            .wrapping_add(0usize),
    ) as *mut libc::c_char;
    if ret.is_null() {
        exit(35i32);
    }
    *ret.offset(0isize) = 0i32 as libc::c_char;
    i = 0i32;
    while i < cnt {
        if 0 != i {
            strcat(ret, sep);
        }
        sprintf(
            &mut *ret.offset(strlen(ret) as isize) as *mut libc::c_char,
            b"%lu\x00" as *const u8 as *const libc::c_char,
            *arr.offset(i as isize) as libc::c_ulong,
        );
        i += 1
    }

    ret
}
