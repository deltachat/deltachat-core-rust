use libc;

use crate::other::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct chashdatum {
    pub data: *mut libc::c_void,
    pub len: libc::c_uint,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct chash {
    pub size: libc::c_uint,
    pub count: libc::c_uint,
    pub copyvalue: libc::c_int,
    pub copykey: libc::c_int,
    pub cells: *mut *mut chashcell,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct chashcell {
    pub func: libc::c_uint,
    pub key: chashdatum,
    pub value: chashdatum,
    pub next: *mut chashcell,
}

pub type chashiter = chashcell;
/* Allocates a new (empty) hash using this initial size and the given flags,
  specifying which data should be copied in the hash.
   CHASH_COPYNONE  : Keys/Values are not copied.
   CHASH_COPYKEY   : Keys are dupped and freed as needed in the hash.
   CHASH_COPYVALUE : Values are dupped and freed as needed in the hash.
   CHASH_COPYALL   : Both keys and values are dupped in the hash.
*/
pub unsafe fn chash_new(mut size: libc::c_uint, mut flags: libc::c_int) -> *mut chash {
    let mut h: *mut chash = 0 as *mut chash;
    h = malloc(::std::mem::size_of::<chash>() as libc::size_t) as *mut chash;
    if h.is_null() {
        return 0 as *mut chash;
    }
    if size < 13i32 as libc::c_uint {
        size = 13i32 as libc::c_uint
    }
    (*h).count = 0i32 as libc::c_uint;
    (*h).cells = calloc(
        size as libc::size_t,
        ::std::mem::size_of::<*mut chashcell>() as libc::size_t,
    ) as *mut *mut chashcell;
    if (*h).cells.is_null() {
        free(h as *mut libc::c_void);
        return 0 as *mut chash;
    }
    (*h).size = size;
    (*h).copykey = flags & 1i32;
    (*h).copyvalue = flags & 2i32;
    return h;
}

/* Frees a hash */
pub unsafe fn chash_free(mut hash: *mut chash) {
    let mut indx: libc::c_uint = 0;
    let mut iter: *mut chashiter = 0 as *mut chashiter;
    let mut next: *mut chashiter = 0 as *mut chashiter;
    indx = 0i32 as libc::c_uint;
    while indx < (*hash).size {
        iter = *(*hash).cells.offset(indx as isize);
        while !iter.is_null() {
            next = (*iter).next;
            if 0 != (*hash).copykey {
                free((*iter).key.data);
            }
            if 0 != (*hash).copyvalue {
                free((*iter).value.data);
            }
            free(iter as *mut libc::c_void);
            iter = next
        }
        indx = indx.wrapping_add(1)
    }
    free((*hash).cells as *mut libc::c_void);
    free(hash as *mut libc::c_void);
}

/* Removes all elements from a hash */
pub unsafe fn chash_clear(mut hash: *mut chash) {
    let mut indx: libc::c_uint = 0;
    let mut iter: *mut chashiter = 0 as *mut chashiter;
    let mut next: *mut chashiter = 0 as *mut chashiter;
    indx = 0i32 as libc::c_uint;
    while indx < (*hash).size {
        iter = *(*hash).cells.offset(indx as isize);
        while !iter.is_null() {
            next = (*iter).next;
            if 0 != (*hash).copykey {
                free((*iter).key.data);
            }
            if 0 != (*hash).copyvalue {
                free((*iter).value.data);
            }
            free(iter as *mut libc::c_void);
            iter = next
        }
        indx = indx.wrapping_add(1)
    }
    memset(
        (*hash).cells as *mut libc::c_void,
        0i32,
        ((*hash).size as libc::size_t)
            .wrapping_mul(::std::mem::size_of::<*mut chashcell>() as libc::size_t),
    );
    (*hash).count = 0i32 as libc::c_uint;
}
/* Adds an entry in the hash table.
Length can be 0 if key/value are strings.
If an entry already exists for this key, it is replaced, and its value
is returned. Otherwise, the data pointer will be NULL and the length
field be set to TRUE or FALSe to indicate success or failure. */
pub unsafe fn chash_set(
    mut hash: *mut chash,
    mut key: *mut chashdatum,
    mut value: *mut chashdatum,
    mut oldvalue: *mut chashdatum,
) -> libc::c_int {
    let mut current_block: u64;
    let mut func: libc::c_uint = 0;
    let mut indx: libc::c_uint = 0;
    let mut iter: *mut chashiter = 0 as *mut chashiter;
    let mut cell: *mut chashiter = 0 as *mut chashiter;
    let mut r: libc::c_int = 0;
    if (*hash).count > (*hash).size.wrapping_mul(3i32 as libc::c_uint) {
        r = chash_resize(
            hash,
            (*hash)
                .count
                .wrapping_div(3i32 as libc::c_uint)
                .wrapping_mul(2i32 as libc::c_uint)
                .wrapping_add(1i32 as libc::c_uint),
        );
        if r < 0i32 {
            current_block = 17701753836843438419;
        } else {
            current_block = 7095457783677275021;
        }
    } else {
        current_block = 7095457783677275021;
    }
    match current_block {
        7095457783677275021 => {
            func = chash_func((*key).data as *const libc::c_char, (*key).len);
            indx = func.wrapping_rem((*hash).size);
            iter = *(*hash).cells.offset(indx as isize);
            loop {
                if iter.is_null() {
                    current_block = 17788412896529399552;
                    break;
                }
                if (*iter).key.len == (*key).len
                    && (*iter).func == func
                    && 0 == memcmp((*iter).key.data, (*key).data, (*key).len as libc::size_t)
                {
                    /* found, replacing entry */
                    if 0 != (*hash).copyvalue {
                        let mut data: *mut libc::c_char = 0 as *mut libc::c_char;
                        data = chash_dup((*value).data, (*value).len);
                        if data.is_null() {
                            current_block = 17701753836843438419;
                            break;
                        }
                        free((*iter).value.data);
                        (*iter).value.data = data as *mut libc::c_void;
                        (*iter).value.len = (*value).len
                    } else {
                        if !oldvalue.is_null() {
                            (*oldvalue).data = (*iter).value.data;
                            (*oldvalue).len = (*iter).value.len
                        }
                        (*iter).value.data = (*value).data;
                        (*iter).value.len = (*value).len
                    }
                    if 0 == (*hash).copykey {
                        (*iter).key.data = (*key).data
                    }
                    if !oldvalue.is_null() {
                        (*oldvalue).data = (*value).data;
                        (*oldvalue).len = (*value).len
                    }
                    return 0i32;
                } else {
                    iter = (*iter).next
                }
            }
            match current_block {
                17701753836843438419 => {}
                _ => {
                    if !oldvalue.is_null() {
                        (*oldvalue).data = 0 as *mut libc::c_void;
                        (*oldvalue).len = 0i32 as libc::c_uint
                    }
                    cell = malloc(::std::mem::size_of::<chashcell>() as libc::size_t)
                        as *mut chashcell;
                    if !cell.is_null() {
                        if 0 != (*hash).copykey {
                            (*cell).key.data =
                                chash_dup((*key).data, (*key).len) as *mut libc::c_void;
                            if (*cell).key.data.is_null() {
                                current_block = 4267898785354516004;
                            } else {
                                current_block = 7226443171521532240;
                            }
                        } else {
                            (*cell).key.data = (*key).data;
                            current_block = 7226443171521532240;
                        }
                        match current_block {
                            7226443171521532240 => {
                                (*cell).key.len = (*key).len;
                                if 0 != (*hash).copyvalue {
                                    (*cell).value.data =
                                        chash_dup((*value).data, (*value).len) as *mut libc::c_void;
                                    if (*cell).value.data.is_null() {
                                        if 0 != (*hash).copykey {
                                            free((*cell).key.data);
                                        }
                                        current_block = 4267898785354516004;
                                    } else {
                                        current_block = 6717214610478484138;
                                    }
                                } else {
                                    (*cell).value.data = (*value).data;
                                    current_block = 6717214610478484138;
                                }
                                match current_block {
                                    4267898785354516004 => {}
                                    _ => {
                                        (*cell).value.len = (*value).len;
                                        (*cell).func = func;
                                        (*cell).next = *(*hash).cells.offset(indx as isize);
                                        let ref mut fresh0 = *(*hash).cells.offset(indx as isize);
                                        *fresh0 = cell;
                                        (*hash).count = (*hash).count.wrapping_add(1);
                                        return 0i32;
                                    }
                                }
                            }
                            _ => {}
                        }
                        free(cell as *mut libc::c_void);
                    }
                }
            }
        }
        _ => {}
    }
    return -1i32;
}
#[inline]
unsafe fn chash_dup(mut data: *const libc::c_void, mut len: libc::c_uint) -> *mut libc::c_char {
    let mut r: *mut libc::c_void = 0 as *mut libc::c_void;
    r = malloc(len as libc::size_t) as *mut libc::c_char as *mut libc::c_void;
    if r.is_null() {
        return 0 as *mut libc::c_char;
    }
    memcpy(r, data, len as libc::size_t);
    return r as *mut libc::c_char;
}

#[inline]
unsafe fn chash_func(mut key: *const libc::c_char, mut len: libc::c_uint) -> libc::c_uint {
    let mut c: libc::c_uint = 5381i32 as libc::c_uint;
    let mut k: *const libc::c_char = key;
    loop {
        let fresh1 = len;
        len = len.wrapping_sub(1);
        if !(0 != fresh1) {
            break;
        }
        let fresh2 = k;
        k = k.offset(1);
        c = (c << 5i32)
            .wrapping_add(c)
            .wrapping_add(*fresh2 as libc::c_uint)
    }
    return c;
}

/* Resizes the hash table to the passed size. */
pub unsafe fn chash_resize(mut hash: *mut chash, mut size: libc::c_uint) -> libc::c_int {
    let mut cells: *mut *mut chashcell = 0 as *mut *mut chashcell;
    let mut indx: libc::c_uint = 0;
    let mut nindx: libc::c_uint = 0;
    let mut iter: *mut chashiter = 0 as *mut chashiter;
    let mut next: *mut chashiter = 0 as *mut chashiter;
    if (*hash).size == size {
        return 0i32;
    }
    cells = calloc(
        size as libc::size_t,
        ::std::mem::size_of::<*mut chashcell>() as libc::size_t,
    ) as *mut *mut chashcell;
    if cells.is_null() {
        return -1i32;
    }
    indx = 0i32 as libc::c_uint;
    while indx < (*hash).size {
        iter = *(*hash).cells.offset(indx as isize);
        while !iter.is_null() {
            next = (*iter).next;
            nindx = (*iter).func.wrapping_rem(size);
            (*iter).next = *cells.offset(nindx as isize);
            let ref mut fresh3 = *cells.offset(nindx as isize);
            *fresh3 = iter;
            iter = next
        }
        indx = indx.wrapping_add(1)
    }
    free((*hash).cells as *mut libc::c_void);
    (*hash).size = size;
    (*hash).cells = cells;
    return 0i32;
}

/* Retrieves the data associated to the key if it is found in the hash table.
The data pointer and the length will be NULL if not found*/
pub unsafe fn chash_get(
    mut hash: *mut chash,
    mut key: *mut chashdatum,
    mut result: *mut chashdatum,
) -> libc::c_int {
    let mut func: libc::c_uint = 0;
    let mut iter: *mut chashiter = 0 as *mut chashiter;
    func = chash_func((*key).data as *const libc::c_char, (*key).len);
    iter = *(*hash)
        .cells
        .offset(func.wrapping_rem((*hash).size) as isize);
    while !iter.is_null() {
        if (*iter).key.len == (*key).len
            && (*iter).func == func
            && 0 == memcmp((*iter).key.data, (*key).data, (*key).len as libc::size_t)
        {
            *result = (*iter).value;
            return 0i32;
        }
        iter = (*iter).next
    }
    return -1i32;
}
/* Removes the entry associated to this key if it is found in the hash table,
and returns its contents if not dupped (otherwise, pointer will be NULL
and len TRUE). If entry is not found both pointer and len will be NULL. */
pub unsafe fn chash_delete(
    mut hash: *mut chash,
    mut key: *mut chashdatum,
    mut oldvalue: *mut chashdatum,
) -> libc::c_int {
    /*  chashdatum result = { NULL, TRUE }; */
    let mut func: libc::c_uint = 0;
    let mut indx: libc::c_uint = 0;
    let mut iter: *mut chashiter = 0 as *mut chashiter;
    let mut old: *mut chashiter = 0 as *mut chashiter;
    func = chash_func((*key).data as *const libc::c_char, (*key).len);
    indx = func.wrapping_rem((*hash).size);
    old = 0 as *mut chashiter;
    iter = *(*hash).cells.offset(indx as isize);
    while !iter.is_null() {
        if (*iter).key.len == (*key).len
            && (*iter).func == func
            && 0 == memcmp((*iter).key.data, (*key).data, (*key).len as libc::size_t)
        {
            if !old.is_null() {
                (*old).next = (*iter).next
            } else {
                let ref mut fresh4 = *(*hash).cells.offset(indx as isize);
                *fresh4 = (*iter).next
            }
            if 0 != (*hash).copykey {
                free((*iter).key.data);
            }
            if 0 != (*hash).copyvalue {
                free((*iter).value.data);
            } else if !oldvalue.is_null() {
                (*oldvalue).data = (*iter).value.data;
                (*oldvalue).len = (*iter).value.len
            }
            free(iter as *mut libc::c_void);
            (*hash).count = (*hash).count.wrapping_sub(1);
            return 0i32;
        }
        old = iter;
        iter = (*iter).next
    }
    return -1i32;
}
/* Returns an iterator to the first non-empty entry of the hash table */
pub unsafe fn chash_begin(mut hash: *mut chash) -> *mut chashiter {
    let mut iter: *mut chashiter = 0 as *mut chashiter;
    let mut indx: libc::c_uint = 0i32 as libc::c_uint;
    iter = *(*hash).cells.offset(0isize);
    while iter.is_null() {
        indx = indx.wrapping_add(1);
        if indx >= (*hash).size {
            return 0 as *mut chashiter;
        }
        iter = *(*hash).cells.offset(indx as isize)
    }
    return iter;
}
/* Returns the next non-empty entry of the hash table */
pub unsafe fn chash_next(mut hash: *mut chash, mut iter: *mut chashiter) -> *mut chashiter {
    let mut indx: libc::c_uint = 0;
    if iter.is_null() {
        return 0 as *mut chashiter;
    }
    indx = (*iter).func.wrapping_rem((*hash).size);
    iter = (*iter).next;
    while iter.is_null() {
        indx = indx.wrapping_add(1);
        if indx >= (*hash).size {
            return 0 as *mut chashiter;
        }
        iter = *(*hash).cells.offset(indx as isize)
    }
    return iter;
}
