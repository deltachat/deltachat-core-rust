use libc;

use crate::types::*;
use crate::x::*;

static mut s_init_lock: pthread_mutex_t = _opaque_pthread_mutex_t {
    __sig: 0x32aaaba7i32 as libc::c_long,
    __opaque: [
        0i32 as libc::c_char,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ],
};
static mut s_init_not_required: libc::c_int = 0i32;
static mut s_init_counter: libc::c_int = 0i32;
static mut s_mutex_buf: *mut pthread_mutex_t = 0 as *const pthread_mutex_t as *mut pthread_mutex_t;
unsafe extern "C" fn id_function() -> libc::c_ulong {
    return pthread_self() as libc::c_ulong;
}
unsafe extern "C" fn locking_function(
    mut mode: libc::c_int,
    mut n: libc::c_int,
    mut file: *const libc::c_char,
    mut line: libc::c_int,
) {
    if 0 != mode & 1i32 {
        pthread_mutex_lock(&mut *s_mutex_buf.offset(n as isize));
    } else {
        pthread_mutex_unlock(&mut *s_mutex_buf.offset(n as isize));
    };
}
unsafe extern "C" fn dyn_create_function(
    mut file: *const libc::c_char,
    mut line: libc::c_int,
) -> *mut CRYPTO_dynlock_value {
    let mut value: *mut CRYPTO_dynlock_value =
        malloc(::std::mem::size_of::<CRYPTO_dynlock_value>() as libc::c_ulong)
            as *mut CRYPTO_dynlock_value;
    if value.is_null() {
        return 0 as *mut CRYPTO_dynlock_value;
    }
    pthread_mutex_init(&mut (*value).mutex, 0 as *const pthread_mutexattr_t);
    return value;
}
unsafe extern "C" fn dyn_lock_function(
    mut mode: libc::c_int,
    mut l: *mut CRYPTO_dynlock_value,
    mut file: *const libc::c_char,
    mut line: libc::c_int,
) {
    if 0 != mode & 1i32 {
        pthread_mutex_lock(&mut (*l).mutex);
    } else {
        pthread_mutex_unlock(&mut (*l).mutex);
    };
}
unsafe extern "C" fn dyn_destroy_function(
    mut l: *mut CRYPTO_dynlock_value,
    mut file: *const libc::c_char,
    mut line: libc::c_int,
) {
    pthread_mutex_destroy(&mut (*l).mutex);
    free(l as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_openssl_init() {
    pthread_mutex_lock(&mut s_init_lock);
    s_init_counter += 1;
    if s_init_counter == 1i32 {
        if 0 == s_init_not_required {
            s_mutex_buf = malloc(
                (CRYPTO_num_locks() as libc::c_ulong)
                    .wrapping_mul(::std::mem::size_of::<pthread_mutex_t>() as libc::c_ulong),
            ) as *mut pthread_mutex_t;
            if s_mutex_buf.is_null() {
                exit(53i32);
            }
            let mut i: libc::c_int = 0i32;
            while i < CRYPTO_num_locks() {
                pthread_mutex_init(
                    &mut *s_mutex_buf.offset(i as isize),
                    0 as *const pthread_mutexattr_t,
                );
                i += 1
            }
            CRYPTO_set_id_callback(Some(id_function));
            CRYPTO_set_locking_callback(Some(locking_function));
            CRYPTO_set_dynlock_create_callback(Some(dyn_create_function));
            CRYPTO_set_dynlock_lock_callback(Some(dyn_lock_function));
            CRYPTO_set_dynlock_destroy_callback(Some(dyn_destroy_function));
            OPENSSL_init();
            OPENSSL_add_all_algorithms_noconf();
        }
        mailstream_openssl_init_not_required();
    }
    pthread_mutex_unlock(&mut s_init_lock);
}
#[no_mangle]
pub unsafe extern "C" fn dc_openssl_exit() {
    pthread_mutex_lock(&mut s_init_lock);
    if s_init_counter > 0i32 {
        s_init_counter -= 1;
        if s_init_counter == 0i32 && 0 == s_init_not_required {
            CRYPTO_set_id_callback(None);
            CRYPTO_set_locking_callback(None);
            CRYPTO_set_dynlock_create_callback(None);
            CRYPTO_set_dynlock_lock_callback(None);
            CRYPTO_set_dynlock_destroy_callback(None);
            let mut i: libc::c_int = 0i32;
            while i < CRYPTO_num_locks() {
                pthread_mutex_destroy(&mut *s_mutex_buf.offset(i as isize));
                i += 1
            }
            free(s_mutex_buf as *mut libc::c_void);
            s_mutex_buf = 0 as *mut pthread_mutex_t
        }
    }
    pthread_mutex_unlock(&mut s_init_lock);
}
