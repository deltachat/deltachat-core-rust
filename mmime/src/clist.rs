use libc;

use crate::other::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct clistcell {
    pub data: *mut libc::c_void,
    pub previous: *mut clistcell,
    pub next: *mut clistcell,
}

#[derive(Clone)]
#[repr(C)]
pub struct clist {
    pub first: *mut clistcell,
    pub last: *mut clistcell,
    pub count: libc::c_int,
}

impl Default for clist {
    fn default() -> Self {
        Self {
            first: std::ptr::null_mut(),
            last: std::ptr::null_mut(),
            count: 0,
        }
    }
}

impl Drop for clist {
    fn drop(&mut self) {
        unsafe {
            let mut l1 = self.first;
            while !l1.is_null() {
                let l2 = (*l1).next;
                free(l1 as *mut libc::c_void);
                l1 = l2
            }
        }
    }
}

pub type clistiter = clistcell;
pub struct CListIterator {
    cur: *mut clistiter,
}
impl Iterator for CListIterator {
    type Item = *mut libc::c_void;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.cur.is_null() {
                None
            } else {
                let data = (*self.cur).data;
                self.cur = (*self.cur).next;
                Some(data)
            }
        }
    }
}

impl IntoIterator for &clist {
    type Item = *mut libc::c_void;
    type IntoIter = CListIterator;
    fn into_iter(self) -> Self::IntoIter {
        return CListIterator { cur: self.first };
    }
}

pub type clist_func =
    Option<unsafe extern "C" fn(_: *mut libc::c_void, _: *mut libc::c_void) -> ()>;

/* Allocate a new pointer list */
pub fn clist_new() -> *mut clist {
    Box::into_raw(Box::new(Default::default()))
}
/* Destroys a list. Data pointed by data pointers is NOT freed. */
pub unsafe fn clist_free(mut lst: *mut clist) {
    Box::from_raw(lst);
}
/* Inserts this data pointer after the element pointed by the iterator */
pub unsafe fn clist_insert_after(
    mut lst: *mut clist,
    mut iter: *mut clistiter,
    mut data: *mut libc::c_void,
) -> libc::c_int {
    let mut c: *mut clistcell = 0 as *mut clistcell;
    c = malloc(::std::mem::size_of::<clistcell>() as libc::size_t) as *mut clistcell;
    if c.is_null() {
        return -1i32;
    }
    (*c).data = data;
    (*lst).count += 1;
    if (*lst).first == (*lst).last && (*lst).last.is_null() {
        (*c).next = 0 as *mut clistcell;
        (*c).previous = (*c).next;
        (*lst).last = c;
        (*lst).first = (*lst).last;
        return 0i32;
    }
    if iter.is_null() {
        (*c).previous = (*lst).last;
        (*(*c).previous).next = c;
        (*c).next = 0 as *mut clistcell;
        (*lst).last = c;
        return 0i32;
    }
    (*c).previous = iter;
    (*c).next = (*iter).next;
    if !(*c).next.is_null() {
        (*(*c).next).previous = c
    } else {
        (*lst).last = c
    }
    (*(*c).previous).next = c;
    return 0i32;
}
/* Deletes the element pointed by the iterator.
Returns an iterator to the next element. */
pub unsafe fn clist_delete(mut lst: *mut clist, mut iter: *mut clistiter) -> *mut clistiter {
    let mut ret: *mut clistiter = 0 as *mut clistiter;
    if iter.is_null() {
        return 0 as *mut clistiter;
    }
    if !(*iter).previous.is_null() {
        (*(*iter).previous).next = (*iter).next
    } else {
        (*lst).first = (*iter).next
    }
    if !(*iter).next.is_null() {
        (*(*iter).next).previous = (*iter).previous;
        ret = (*iter).next
    } else {
        (*lst).last = (*iter).previous;
        ret = 0 as *mut clistiter
    }
    free(iter as *mut libc::c_void);
    (*lst).count -= 1;
    return ret;
}
pub unsafe fn clist_foreach(
    mut lst: *mut clist,
    mut func: clist_func,
    mut data: *mut libc::c_void,
) {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    cur = (*lst).first;
    while !cur.is_null() {
        func.expect("non-null function pointer")((*cur).data, data);
        cur = (*cur).next
    }
}

pub unsafe fn clist_nth_data(mut lst: *mut clist, mut indx: libc::c_int) -> *mut libc::c_void {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    cur = internal_clist_nth(lst, indx);
    if cur.is_null() {
        return 0 as *mut libc::c_void;
    }
    return (*cur).data;
}
#[inline]
unsafe fn internal_clist_nth(mut lst: *mut clist, mut indx: libc::c_int) -> *mut clistiter {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    cur = (*lst).first;
    while indx > 0i32 && !cur.is_null() {
        cur = (*cur).next;
        indx -= 1
    }
    if cur.is_null() {
        return 0 as *mut clistiter;
    }
    return cur;
}

pub unsafe fn clist_nth(mut lst: *mut clist, mut indx: libc::c_int) -> *mut clistiter {
    return internal_clist_nth(lst, indx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;
    #[test]
    fn test_clist_iterator() {
        unsafe {
            let mut c = clist_new();
            assert!(!c.is_null());
            clist_insert_after(c, ptr::null_mut(), clist_nth as _);
            assert_eq!((*c).count, 1);

            /* Only one iteration */
            for data in &*c {
                assert_eq!(data, clist_nth as _);
            }
            assert_eq!((*c).count, 1);

            clist_free(c);
        }
    }
}
