use crate::types::*;
use crate::x::*;

/* A complete hash table is an instance of the following structure.
 * The internals of this structure are intended to be opaque -- client
 * code should not attempt to access or modify the fields of this structure
 * directly.  Change this structure only by using the routines below.
 * However, many of the "procedures" and "functions" for modifying and
 * accessing this structure are really macros, so we can't really make
 * this structure opaque.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_hash_t {
    pub keyClass: libc::c_char,
    pub copyKey: libc::c_char,
    pub count: libc::c_int,
    pub first: *mut dc_hashelem_t,
    pub htsize: libc::c_int,
    pub ht: *mut _ht,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct _ht {
    pub count: libc::c_int,
    pub chain: *mut dc_hashelem_t,
}

pub type dc_hashelem_t = _dc_hashelem;

/* Each element in the hash table is an instance of the following
 * structure.  All elements are stored on a single doubly-linked list.
 *
 * Again, this structure is intended to be opaque, but it can't really
 * be opaque because it is used by macros.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_hashelem {
    pub next: *mut dc_hashelem_t,
    pub prev: *mut dc_hashelem_t,
    pub data: *mut libc::c_void,
    pub pKey: *mut libc::c_void,
    pub nKey: libc::c_int,
}

/*
 * There are 4 different modes of operation for a hash table:
 *
 *   DC_HASH_INT         nKey is used as the key and pKey is ignored.
 *
 *   DC_HASH_POINTER     pKey is used as the key and nKey is ignored.
 *
 *   DC_HASH_STRING      pKey points to a string that is nKey bytes long
 *                      (including the null-terminator, if any).  Case
 *                      is ignored in comparisons.
 *
 *   DC_HASH_BINARY      pKey points to binary data nKey bytes long.
 *                      memcmp() is used to compare keys.
 *
 * A copy of the key is made for DC_HASH_STRING and DC_HASH_BINARY
 * if the copyKey parameter to dc_hash_init() is 1.
 */
/*
 * Just to make the last parameter of dc_hash_init() more readable.
 */
/*
 * Access routines.  To delete an element, insert a NULL pointer.
 */
pub unsafe fn dc_hash_init(
    mut pNew: *mut dc_hash_t,
    mut keyClass: libc::c_int,
    mut copyKey: libc::c_int,
) {
    if 0 != pNew.is_null() as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 13], &[libc::c_char; 13]>(b"dc_hash_init\x00"))
                .as_ptr(),
            b"../src/dc_hash.c\x00" as *const u8 as *const libc::c_char,
            93i32,
            b"pNew!=0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !(keyClass >= 1i32 && keyClass <= 4i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 13], &[libc::c_char; 13]>(b"dc_hash_init\x00"))
                .as_ptr(),
            b"../src/dc_hash.c\x00" as *const u8 as *const libc::c_char,
            94i32,
            b"keyClass>=DC_HASH_INT && keyClass<=DC_HASH_BINARY\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
    };
    (*pNew).keyClass = keyClass as libc::c_char;
    if keyClass == 2i32 || keyClass == 1i32 {
        copyKey = 0i32
    }
    (*pNew).copyKey = copyKey as libc::c_char;
    (*pNew).first = 0 as *mut dc_hashelem_t;
    (*pNew).count = 0i32;
    (*pNew).htsize = 0i32;
    (*pNew).ht = 0 as *mut _ht;
}

pub unsafe fn dc_hash_insert(
    mut pH: *mut dc_hash_t,
    mut pKey: *const libc::c_void,
    mut nKey: libc::c_int,
    mut data: *mut libc::c_void,
) -> *mut libc::c_void {
    /* Raw hash value of the key */
    let mut hraw: libc::c_int;
    /* the hash of the key modulo hash table size */
    let mut h: libc::c_int;
    /* Used to loop thru the element list */
    let mut elem: *mut dc_hashelem_t;
    /* New element added to the pH */
    let mut new_elem: *mut dc_hashelem_t;
    /* The hash function */
    let mut xHash: Option<unsafe fn(_: *const libc::c_void, _: libc::c_int) -> libc::c_int>;
    if 0 != pH.is_null() as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 15], &[libc::c_char; 15]>(b"dc_hash_insert\x00"))
                .as_ptr(),
            b"../src/dc_hash.c\x00" as *const u8 as *const libc::c_char,
            429i32,
            b"pH!=0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    xHash = hashFunction((*pH).keyClass as libc::c_int);
    if 0 != xHash.is_none() as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 15], &[libc::c_char; 15]>(b"dc_hash_insert\x00"))
                .as_ptr(),
            b"../src/dc_hash.c\x00" as *const u8 as *const libc::c_char,
            431i32,
            b"xHash!=0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    hraw = xHash.expect("non-null function pointer")(pKey, nKey);
    if 0 != !((*pH).htsize & (*pH).htsize - 1i32 == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 15], &[libc::c_char; 15]>(b"dc_hash_insert\x00"))
                .as_ptr(),
            b"../src/dc_hash.c\x00" as *const u8 as *const libc::c_char,
            433i32,
            b"(pH->htsize & (pH->htsize-1))==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    h = hraw & (*pH).htsize - 1i32;
    elem = findElementGivenHash(pH, pKey, nKey, h);
    if !elem.is_null() {
        let mut old_data: *mut libc::c_void = (*elem).data;
        if data.is_null() {
            removeElementGivenHash(pH, elem, h);
        } else {
            (*elem).data = data
        }
        return old_data;
    }
    if data.is_null() {
        return 0 as *mut libc::c_void;
    }
    new_elem =
        sjhashMalloc(::std::mem::size_of::<dc_hashelem_t>() as libc::c_int) as *mut dc_hashelem_t;
    if new_elem.is_null() {
        return data;
    }
    if 0 != (*pH).copyKey as libc::c_int && !pKey.is_null() {
        (*new_elem).pKey = malloc(nKey as usize);
        if (*new_elem).pKey.is_null() {
            free(new_elem as *mut libc::c_void);
            return data;
        }
        memcpy((*new_elem).pKey as *mut libc::c_void, pKey, nKey as usize);
    } else {
        (*new_elem).pKey = pKey as *mut libc::c_void
    }
    (*new_elem).nKey = nKey;
    (*pH).count += 1;
    if (*pH).htsize == 0i32 {
        rehash(pH, 8);
        if (*pH).htsize == 0i32 {
            (*pH).count = 0i32;
            free(new_elem as *mut libc::c_void);
            return data;
        }
    }
    if (*pH).count > (*pH).htsize {
        rehash(pH, (*pH).htsize * 2);
    }
    if 0 != !((*pH).htsize > 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 15], &[libc::c_char; 15]>(b"dc_hash_insert\x00"))
                .as_ptr(),
            b"../src/dc_hash.c\x00" as *const u8 as *const libc::c_char,
            491i32,
            b"pH->htsize>0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    if 0 != !((*pH).htsize & (*pH).htsize - 1i32 == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 15], &[libc::c_char; 15]>(b"dc_hash_insert\x00"))
                .as_ptr(),
            b"../src/dc_hash.c\x00" as *const u8 as *const libc::c_char,
            492i32,
            b"(pH->htsize & (pH->htsize-1))==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    h = hraw & (*pH).htsize - 1i32;
    insertElement(pH, &mut *(*pH).ht.offset(h as isize), new_elem);
    (*new_elem).data = data;

    0 as *mut libc::c_void
}

/* Link an element into the hash table
 */
unsafe extern "C" fn insertElement(
    mut pH: *mut dc_hash_t,
    mut pEntry: *mut _ht,
    mut pNew: *mut dc_hashelem_t,
) {
    /* First element already in pEntry */
    let mut pHead: *mut dc_hashelem_t;
    pHead = (*pEntry).chain;
    if !pHead.is_null() {
        (*pNew).next = pHead;
        (*pNew).prev = (*pHead).prev;
        if !(*pHead).prev.is_null() {
            (*(*pHead).prev).next = pNew
        } else {
            (*pH).first = pNew
        }
        (*pHead).prev = pNew
    } else {
        (*pNew).next = (*pH).first;
        if !(*pH).first.is_null() {
            (*(*pH).first).prev = pNew
        }
        (*pNew).prev = 0 as *mut dc_hashelem_t;
        (*pH).first = pNew
    }
    (*pEntry).count += 1;
    (*pEntry).chain = pNew;
}

/* Resize the hash table so that it cantains "new_size" buckets.
 * "new_size" must be a power of 2.  The hash table might fail
 * to resize if sjhashMalloc() fails.
 */
unsafe fn rehash(mut pH: *mut dc_hash_t, mut new_size: libc::c_int) {
    /* The new hash table */
    let mut new_ht: *mut _ht;
    /* For looping over existing elements */
    let mut elem: *mut dc_hashelem_t;
    let mut next_elem: *mut dc_hashelem_t;
    /* The hash function */
    let mut xHash: Option<unsafe fn(_: *const libc::c_void, _: libc::c_int) -> libc::c_int>;
    if 0 != !(new_size & new_size - 1i32 == 0i32) as libc::c_int as libc::c_long {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 7], &[libc::c_char; 7]>(b"rehash\x00")).as_ptr(),
            b"../src/dc_hash.c\x00" as *const u8 as *const libc::c_char,
            287i32,
            b"(new_size & (new_size-1))==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    new_ht = sjhashMalloc(
        new_size.wrapping_mul(::std::mem::size_of::<_ht>() as libc::c_int) as libc::c_int,
    ) as *mut _ht;
    if new_ht.is_null() {
        return;
    }
    if !(*pH).ht.is_null() {
        free((*pH).ht as *mut libc::c_void);
    }
    (*pH).ht = new_ht;
    (*pH).htsize = new_size;
    xHash = hashFunction((*pH).keyClass as libc::c_int);
    elem = (*pH).first;
    (*pH).first = 0 as *mut dc_hashelem_t;
    while !elem.is_null() {
        let mut h: libc::c_int =
            xHash.expect("non-null function pointer")((*elem).pKey, (*elem).nKey) & new_size - 1i32;
        next_elem = (*elem).next;
        insertElement(pH, &mut *new_ht.offset(h as isize), elem);
        elem = next_elem
    }
}

/* Return a pointer to the appropriate hash function given the key class.
 *
 * About the syntax:
 * The name of the function is "hashFunction".  The function takes a
 * single parameter "keyClass".  The return value of hashFunction()
 * is a pointer to another function.  Specifically, the return value
 * of hashFunction() is a pointer to a function that takes two parameters
 * with types "const void*" and "int" and returns an "int".
 */
unsafe fn hashFunction(
    mut keyClass: libc::c_int,
) -> Option<unsafe fn(_: *const libc::c_void, _: libc::c_int) -> libc::c_int> {
    match keyClass {
        1 => return Some(intHash),
        2 => return Some(ptrHash),
        3 => return Some(strHash),
        4 => return Some(binHash),
        _ => {}
    }

    None
}

/* Hash and comparison functions when the mode is SJHASH_BINARY
 */
unsafe fn binHash(mut pKey: *const libc::c_void, mut nKey: libc::c_int) -> libc::c_int {
    let mut h: libc::c_int = 0i32;
    let mut z: *const libc::c_char = pKey as *const libc::c_char;
    loop {
        let fresh0 = nKey;
        nKey = nKey - 1;
        if !(fresh0 > 0i32) {
            break;
        }
        let fresh1 = z;
        z = z.offset(1);
        h = h << 3i32 ^ h ^ *fresh1 as libc::c_int
    }

    h & 0x7fffffffi32
}

/* Hash and comparison functions when the mode is SJHASH_STRING
 */
unsafe fn strHash(mut pKey: *const libc::c_void, mut nKey: libc::c_int) -> libc::c_int {
    sjhashNoCase(pKey as *const libc::c_char, nKey)
}

/* This function computes a hash on the name of a keyword.
 * Case is not significant.
 */
unsafe fn sjhashNoCase(mut z: *const libc::c_char, mut n: libc::c_int) -> libc::c_int {
    let mut h: libc::c_int = 0i32;
    if n <= 0i32 {
        n = strlen(z) as libc::c_int
    }
    while n > 0i32 {
        let fresh2 = z;
        z = z.offset(1);
        h = h << 3i32 ^ h ^ sjhashUpperToLower[*fresh2 as libc::c_uchar as usize] as libc::c_int;
        n -= 1
    }

    h & 0x7fffffffi32
}

/* An array to map all upper-case characters into their corresponding
 * lower-case character.
 */
static mut sjhashUpperToLower: [libc::c_uchar; 256] = [
    0i32 as libc::c_uchar,
    1i32 as libc::c_uchar,
    2i32 as libc::c_uchar,
    3i32 as libc::c_uchar,
    4i32 as libc::c_uchar,
    5i32 as libc::c_uchar,
    6i32 as libc::c_uchar,
    7i32 as libc::c_uchar,
    8i32 as libc::c_uchar,
    9i32 as libc::c_uchar,
    10i32 as libc::c_uchar,
    11i32 as libc::c_uchar,
    12i32 as libc::c_uchar,
    13i32 as libc::c_uchar,
    14i32 as libc::c_uchar,
    15i32 as libc::c_uchar,
    16i32 as libc::c_uchar,
    17i32 as libc::c_uchar,
    18i32 as libc::c_uchar,
    19i32 as libc::c_uchar,
    20i32 as libc::c_uchar,
    21i32 as libc::c_uchar,
    22i32 as libc::c_uchar,
    23i32 as libc::c_uchar,
    24i32 as libc::c_uchar,
    25i32 as libc::c_uchar,
    26i32 as libc::c_uchar,
    27i32 as libc::c_uchar,
    28i32 as libc::c_uchar,
    29i32 as libc::c_uchar,
    30i32 as libc::c_uchar,
    31i32 as libc::c_uchar,
    32i32 as libc::c_uchar,
    33i32 as libc::c_uchar,
    34i32 as libc::c_uchar,
    35i32 as libc::c_uchar,
    36i32 as libc::c_uchar,
    37i32 as libc::c_uchar,
    38i32 as libc::c_uchar,
    39i32 as libc::c_uchar,
    40i32 as libc::c_uchar,
    41i32 as libc::c_uchar,
    42i32 as libc::c_uchar,
    43i32 as libc::c_uchar,
    44i32 as libc::c_uchar,
    45i32 as libc::c_uchar,
    46i32 as libc::c_uchar,
    47i32 as libc::c_uchar,
    48i32 as libc::c_uchar,
    49i32 as libc::c_uchar,
    50i32 as libc::c_uchar,
    51i32 as libc::c_uchar,
    52i32 as libc::c_uchar,
    53i32 as libc::c_uchar,
    54i32 as libc::c_uchar,
    55i32 as libc::c_uchar,
    56i32 as libc::c_uchar,
    57i32 as libc::c_uchar,
    58i32 as libc::c_uchar,
    59i32 as libc::c_uchar,
    60i32 as libc::c_uchar,
    61i32 as libc::c_uchar,
    62i32 as libc::c_uchar,
    63i32 as libc::c_uchar,
    64i32 as libc::c_uchar,
    97i32 as libc::c_uchar,
    98i32 as libc::c_uchar,
    99i32 as libc::c_uchar,
    100i32 as libc::c_uchar,
    101i32 as libc::c_uchar,
    102i32 as libc::c_uchar,
    103i32 as libc::c_uchar,
    104i32 as libc::c_uchar,
    105i32 as libc::c_uchar,
    106i32 as libc::c_uchar,
    107i32 as libc::c_uchar,
    108i32 as libc::c_uchar,
    109i32 as libc::c_uchar,
    110i32 as libc::c_uchar,
    111i32 as libc::c_uchar,
    112i32 as libc::c_uchar,
    113i32 as libc::c_uchar,
    114i32 as libc::c_uchar,
    115i32 as libc::c_uchar,
    116i32 as libc::c_uchar,
    117i32 as libc::c_uchar,
    118i32 as libc::c_uchar,
    119i32 as libc::c_uchar,
    120i32 as libc::c_uchar,
    121i32 as libc::c_uchar,
    122i32 as libc::c_uchar,
    91i32 as libc::c_uchar,
    92i32 as libc::c_uchar,
    93i32 as libc::c_uchar,
    94i32 as libc::c_uchar,
    95i32 as libc::c_uchar,
    96i32 as libc::c_uchar,
    97i32 as libc::c_uchar,
    98i32 as libc::c_uchar,
    99i32 as libc::c_uchar,
    100i32 as libc::c_uchar,
    101i32 as libc::c_uchar,
    102i32 as libc::c_uchar,
    103i32 as libc::c_uchar,
    104i32 as libc::c_uchar,
    105i32 as libc::c_uchar,
    106i32 as libc::c_uchar,
    107i32 as libc::c_uchar,
    108i32 as libc::c_uchar,
    109i32 as libc::c_uchar,
    110i32 as libc::c_uchar,
    111i32 as libc::c_uchar,
    112i32 as libc::c_uchar,
    113i32 as libc::c_uchar,
    114i32 as libc::c_uchar,
    115i32 as libc::c_uchar,
    116i32 as libc::c_uchar,
    117i32 as libc::c_uchar,
    118i32 as libc::c_uchar,
    119i32 as libc::c_uchar,
    120i32 as libc::c_uchar,
    121i32 as libc::c_uchar,
    122i32 as libc::c_uchar,
    123i32 as libc::c_uchar,
    124i32 as libc::c_uchar,
    125i32 as libc::c_uchar,
    126i32 as libc::c_uchar,
    127i32 as libc::c_uchar,
    128i32 as libc::c_uchar,
    129i32 as libc::c_uchar,
    130i32 as libc::c_uchar,
    131i32 as libc::c_uchar,
    132i32 as libc::c_uchar,
    133i32 as libc::c_uchar,
    134i32 as libc::c_uchar,
    135i32 as libc::c_uchar,
    136i32 as libc::c_uchar,
    137i32 as libc::c_uchar,
    138i32 as libc::c_uchar,
    139i32 as libc::c_uchar,
    140i32 as libc::c_uchar,
    141i32 as libc::c_uchar,
    142i32 as libc::c_uchar,
    143i32 as libc::c_uchar,
    144i32 as libc::c_uchar,
    145i32 as libc::c_uchar,
    146i32 as libc::c_uchar,
    147i32 as libc::c_uchar,
    148i32 as libc::c_uchar,
    149i32 as libc::c_uchar,
    150i32 as libc::c_uchar,
    151i32 as libc::c_uchar,
    152i32 as libc::c_uchar,
    153i32 as libc::c_uchar,
    154i32 as libc::c_uchar,
    155i32 as libc::c_uchar,
    156i32 as libc::c_uchar,
    157i32 as libc::c_uchar,
    158i32 as libc::c_uchar,
    159i32 as libc::c_uchar,
    160i32 as libc::c_uchar,
    161i32 as libc::c_uchar,
    162i32 as libc::c_uchar,
    163i32 as libc::c_uchar,
    164i32 as libc::c_uchar,
    165i32 as libc::c_uchar,
    166i32 as libc::c_uchar,
    167i32 as libc::c_uchar,
    168i32 as libc::c_uchar,
    169i32 as libc::c_uchar,
    170i32 as libc::c_uchar,
    171i32 as libc::c_uchar,
    172i32 as libc::c_uchar,
    173i32 as libc::c_uchar,
    174i32 as libc::c_uchar,
    175i32 as libc::c_uchar,
    176i32 as libc::c_uchar,
    177i32 as libc::c_uchar,
    178i32 as libc::c_uchar,
    179i32 as libc::c_uchar,
    180i32 as libc::c_uchar,
    181i32 as libc::c_uchar,
    182i32 as libc::c_uchar,
    183i32 as libc::c_uchar,
    184i32 as libc::c_uchar,
    185i32 as libc::c_uchar,
    186i32 as libc::c_uchar,
    187i32 as libc::c_uchar,
    188i32 as libc::c_uchar,
    189i32 as libc::c_uchar,
    190i32 as libc::c_uchar,
    191i32 as libc::c_uchar,
    192i32 as libc::c_uchar,
    193i32 as libc::c_uchar,
    194i32 as libc::c_uchar,
    195i32 as libc::c_uchar,
    196i32 as libc::c_uchar,
    197i32 as libc::c_uchar,
    198i32 as libc::c_uchar,
    199i32 as libc::c_uchar,
    200i32 as libc::c_uchar,
    201i32 as libc::c_uchar,
    202i32 as libc::c_uchar,
    203i32 as libc::c_uchar,
    204i32 as libc::c_uchar,
    205i32 as libc::c_uchar,
    206i32 as libc::c_uchar,
    207i32 as libc::c_uchar,
    208i32 as libc::c_uchar,
    209i32 as libc::c_uchar,
    210i32 as libc::c_uchar,
    211i32 as libc::c_uchar,
    212i32 as libc::c_uchar,
    213i32 as libc::c_uchar,
    214i32 as libc::c_uchar,
    215i32 as libc::c_uchar,
    216i32 as libc::c_uchar,
    217i32 as libc::c_uchar,
    218i32 as libc::c_uchar,
    219i32 as libc::c_uchar,
    220i32 as libc::c_uchar,
    221i32 as libc::c_uchar,
    222i32 as libc::c_uchar,
    223i32 as libc::c_uchar,
    224i32 as libc::c_uchar,
    225i32 as libc::c_uchar,
    226i32 as libc::c_uchar,
    227i32 as libc::c_uchar,
    228i32 as libc::c_uchar,
    229i32 as libc::c_uchar,
    230i32 as libc::c_uchar,
    231i32 as libc::c_uchar,
    232i32 as libc::c_uchar,
    233i32 as libc::c_uchar,
    234i32 as libc::c_uchar,
    235i32 as libc::c_uchar,
    236i32 as libc::c_uchar,
    237i32 as libc::c_uchar,
    238i32 as libc::c_uchar,
    239i32 as libc::c_uchar,
    240i32 as libc::c_uchar,
    241i32 as libc::c_uchar,
    242i32 as libc::c_uchar,
    243i32 as libc::c_uchar,
    244i32 as libc::c_uchar,
    245i32 as libc::c_uchar,
    246i32 as libc::c_uchar,
    247i32 as libc::c_uchar,
    248i32 as libc::c_uchar,
    249i32 as libc::c_uchar,
    250i32 as libc::c_uchar,
    251i32 as libc::c_uchar,
    252i32 as libc::c_uchar,
    253i32 as libc::c_uchar,
    254i32 as libc::c_uchar,
    255i32 as libc::c_uchar,
];

/* Hash and comparison functions when the mode is SJHASH_POINTER
 */
unsafe fn ptrHash(pKey: *const libc::c_void, _nKey: libc::c_int) -> libc::c_int {
    let mut x: uintptr_t = pKey as uintptr_t;
    (x ^ x << 8i32 ^ x >> 8i32) as libc::c_int
}

/* Hash and comparison functions when the mode is SJHASH_INT
 */
unsafe fn intHash(_pKey: *const libc::c_void, mut nKey: libc::c_int) -> libc::c_int {
    nKey ^ nKey << 8i32 ^ nKey >> 8i32
}

/*
** Based upon hash.c from sqlite which author disclaims copyright to this source code. In place of
** a legal notice, here is a blessing:
**
** May you do good and not evil.
** May you find forgiveness for yourself and forgive others.
** May you share freely, never taking more than you give.
*/
unsafe fn sjhashMalloc(mut bytes: libc::c_int) -> *mut libc::c_void {
    let mut p: *mut libc::c_void = malloc(bytes as size_t);
    if !p.is_null() {
        memset(p, 0i32, bytes as size_t);
    }
    p
}

/* Remove a single entry from the hash table given a pointer to that
 * element and a hash on the element's key.
 */
unsafe fn removeElementGivenHash(
    mut pH: *mut dc_hash_t,
    mut elem: *mut dc_hashelem_t,
    mut h: libc::c_int,
) {
    let mut pEntry: *mut _ht;
    if !(*elem).prev.is_null() {
        (*(*elem).prev).next = (*elem).next
    } else {
        (*pH).first = (*elem).next
    }
    if !(*elem).next.is_null() {
        (*(*elem).next).prev = (*elem).prev
    }
    pEntry = &mut *(*pH).ht.offset(h as isize) as *mut _ht;
    if (*pEntry).chain == elem {
        (*pEntry).chain = (*elem).next
    }
    (*pEntry).count -= 1;
    if (*pEntry).count <= 0i32 {
        (*pEntry).chain = 0 as *mut dc_hashelem_t
    }
    if 0 != (*pH).copyKey as libc::c_int && !(*elem).pKey.is_null() {
        free((*elem).pKey);
    }
    free(elem as *mut libc::c_void);
    (*pH).count -= 1;
}

/* This function (for internal use only) locates an element in an
 * hash table that matches the given key.  The hash for this key has
 * already been computed and is passed as the 4th parameter.
 */
unsafe fn findElementGivenHash(
    mut pH: *const dc_hash_t,
    mut pKey: *const libc::c_void,
    mut nKey: libc::c_int,
    mut h: libc::c_int,
) -> *mut dc_hashelem_t {
    /* Used to loop thru the element list */
    let mut elem: *mut dc_hashelem_t;
    /* Number of elements left to test */
    let mut count: libc::c_int;
    /* comparison function */
    let mut xCompare: Option<
        unsafe fn(
            _: *const libc::c_void,
            _: libc::c_int,
            _: *const libc::c_void,
            _: libc::c_int,
        ) -> libc::c_int,
    >;
    if !(*pH).ht.is_null() {
        let mut pEntry: *mut _ht = &mut *(*pH).ht.offset(h as isize) as *mut _ht;
        elem = (*pEntry).chain;
        count = (*pEntry).count;
        xCompare = compareFunction((*pH).keyClass as libc::c_int);
        loop {
            let fresh3 = count;
            count = count - 1;
            if !(0 != fresh3 && !elem.is_null()) {
                break;
            }
            if xCompare.expect("non-null function pointer")((*elem).pKey, (*elem).nKey, pKey, nKey)
                == 0i32
            {
                return elem;
            }
            elem = (*elem).next
        }
    }

    0 as *mut dc_hashelem_t
}

/* Return a pointer to the appropriate hash function given the key class.
 */
unsafe fn compareFunction(
    mut keyClass: libc::c_int,
) -> Option<
    unsafe fn(
        _: *const libc::c_void,
        _: libc::c_int,
        _: *const libc::c_void,
        _: libc::c_int,
    ) -> libc::c_int,
> {
    match keyClass {
        1 => return Some(intCompare),
        2 => return Some(ptrCompare),
        3 => return Some(strCompare),
        4 => return Some(binCompare),
        _ => {}
    }
    None
}

unsafe fn binCompare(
    mut pKey1: *const libc::c_void,
    mut n1: libc::c_int,
    mut pKey2: *const libc::c_void,
    mut n2: libc::c_int,
) -> libc::c_int {
    if n1 != n2 {
        return 1i32;
    }
    memcmp(pKey1, pKey2, n1 as libc::size_t)
}

unsafe fn strCompare(
    mut pKey1: *const libc::c_void,
    mut n1: libc::c_int,
    mut pKey2: *const libc::c_void,
    mut n2: libc::c_int,
) -> libc::c_int {
    if n1 != n2 {
        return 1i32;
    }
    sjhashStrNICmp(
        pKey1 as *const libc::c_char,
        pKey2 as *const libc::c_char,
        n1,
    )
}

/* Some systems have stricmp().  Others have strcasecmp().  Because
 * there is no consistency, we will define our own.
 */
unsafe fn sjhashStrNICmp(
    mut zLeft: *const libc::c_char,
    mut zRight: *const libc::c_char,
    mut N: libc::c_int,
) -> libc::c_int {
    let mut a: *mut libc::c_uchar;
    let mut b: *mut libc::c_uchar;
    a = zLeft as *mut libc::c_uchar;
    b = zRight as *mut libc::c_uchar;
    loop {
        let fresh4 = N;
        N = N - 1;
        if !(fresh4 > 0i32
            && *a as libc::c_int != 0i32
            && sjhashUpperToLower[*a as usize] as libc::c_int
                == sjhashUpperToLower[*b as usize] as libc::c_int)
        {
            break;
        }
        a = a.offset(1isize);
        b = b.offset(1isize)
    }
    return if N < 0i32 {
        0i32
    } else {
        sjhashUpperToLower[*a as usize] as libc::c_int
            - sjhashUpperToLower[*b as usize] as libc::c_int
    };
}

unsafe fn ptrCompare(
    pKey1: *const libc::c_void,
    _n1: libc::c_int,
    pKey2: *const libc::c_void,
    _n2: libc::c_int,
) -> libc::c_int {
    if pKey1 == pKey2 {
        return 0i32;
    }
    if pKey1 < pKey2 {
        return -1i32;
    }
    return 1i32;
}

unsafe fn intCompare(
    _pKey1: *const libc::c_void,
    n1: libc::c_int,
    _pKey2: *const libc::c_void,
    n2: libc::c_int,
) -> libc::c_int {
    return n2 - n1;
}

pub unsafe fn dc_hash_find(
    mut pH: *const dc_hash_t,
    mut pKey: *const libc::c_void,
    mut nKey: libc::c_int,
) -> *mut libc::c_void {
    /* A hash on key */
    let mut h: libc::c_int;
    /* The element that matches key */
    let mut elem: *mut dc_hashelem_t;
    /* The hash function */
    let mut xHash: Option<unsafe fn(_: *const libc::c_void, _: libc::c_int) -> libc::c_int>;
    if pH.is_null() || (*pH).ht.is_null() {
        return 0 as *mut libc::c_void;
    }
    xHash = hashFunction((*pH).keyClass as libc::c_int);
    if 0 != xHash.is_none() as libc::c_int {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 13], &[libc::c_char; 13]>(b"dc_hash_find\x00"))
                .as_ptr(),
            b"../src/dc_hash.c\x00" as *const u8 as *const libc::c_char,
            397i32,
            b"xHash!=0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    h = xHash.expect("non-null function pointer")(pKey, nKey);
    if 0 != !((*pH).htsize & (*pH).htsize - 1i32 == 0i32) as libc::c_int {
        __assert_rtn(
            (*::std::mem::transmute::<&[u8; 13], &[libc::c_char; 13]>(b"dc_hash_find\x00"))
                .as_ptr(),
            b"../src/dc_hash.c\x00" as *const u8 as *const libc::c_char,
            399i32,
            b"(pH->htsize & (pH->htsize-1))==0\x00" as *const u8 as *const libc::c_char,
        );
    } else {
    };
    elem = findElementGivenHash(pH, pKey, nKey, h & (*pH).htsize - 1i32);
    return if !elem.is_null() {
        (*elem).data
    } else {
        0 as *mut libc::c_void
    };
}

pub unsafe fn dc_hash_clear(mut pH: *mut dc_hash_t) {
    /* For looping over all elements of the table */
    let mut elem: *mut dc_hashelem_t;
    if pH.is_null() {
        return;
    }
    elem = (*pH).first;
    (*pH).first = 0 as *mut dc_hashelem_t;
    if !(*pH).ht.is_null() {
        free((*pH).ht as *mut libc::c_void);
    }
    (*pH).ht = 0 as *mut _ht;
    (*pH).htsize = 0i32;
    while !elem.is_null() {
        let mut next_elem: *mut dc_hashelem_t = (*elem).next;
        if 0 != (*pH).copyKey as libc::c_int && !(*elem).pKey.is_null() {
            free((*elem).pKey);
        }
        free(elem as *mut libc::c_void);
        elem = next_elem
    }
    (*pH).count = 0i32;
}
