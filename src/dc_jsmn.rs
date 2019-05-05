use libc;

use crate::types::*;

/*
Copyright (c) 2010 Serge A. Zaitsev

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
*/
/* *
 * JSON type identifier. Basic types are:
 * 	o Object
 * 	o Array
 * 	o String
 * 	o Other primitive: number, boolean (true/false) or null
 */
pub type jsmntype_t = libc::c_uint;
pub const JSMN_PRIMITIVE: jsmntype_t = 4;
pub const JSMN_STRING: jsmntype_t = 3;
pub const JSMN_ARRAY: jsmntype_t = 2;
pub const JSMN_OBJECT: jsmntype_t = 1;
pub const JSMN_UNDEFINED: jsmntype_t = 0;
pub type jsmnerr = libc::c_int;
/* The string is not a full JSON packet, more bytes expected */
pub const JSMN_ERROR_PART: jsmnerr = -3;
/* Invalid character inside JSON string */
pub const JSMN_ERROR_INVAL: jsmnerr = -2;
/* Not enough tokens were provided */
pub const JSMN_ERROR_NOMEM: jsmnerr = -1;
/* *
 * JSON token description.
 * type		type (object, array, string etc.)
 * start	start position in JSON data string
 * end		end position in JSON data string
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct jsmntok_t {
    pub type_0: jsmntype_t,
    pub start: libc::c_int,
    pub end: libc::c_int,
    pub size: libc::c_int,
}
/* *
 * JSON parser. Contains an array of token blocks available. Also stores
 * the string being parsed now and current position in that string
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct jsmn_parser {
    pub pos: libc::c_uint,
    pub toknext: libc::c_uint,
    pub toksuper: libc::c_int,
}

/* *
 * Create JSON parser over an array of tokens
 */
pub unsafe fn jsmn_init(mut parser: *mut jsmn_parser) {
    (*parser).pos = 0i32 as libc::c_uint;
    (*parser).toknext = 0i32 as libc::c_uint;
    (*parser).toksuper = -1i32;
}

/* *
 * Run JSON parser. It parses a JSON data string into and array of tokens, each describing
 * a single JSON object.
 */
pub unsafe fn jsmn_parse(
    mut parser: *mut jsmn_parser,
    mut js: *const libc::c_char,
    mut len: size_t,
    mut tokens: *mut jsmntok_t,
    mut num_tokens: libc::c_uint,
) -> libc::c_int {
    let mut r: libc::c_int;
    let mut i: libc::c_int;
    let mut token: *mut jsmntok_t;
    let mut count: libc::c_int = (*parser).toknext as libc::c_int;
    while (*parser).pos < len as libc::c_uint
        && *js.offset((*parser).pos as isize) as libc::c_int != '\u{0}' as i32
    {
        let mut c: libc::c_char;
        let mut type_0: jsmntype_t;
        c = *js.offset((*parser).pos as isize);
        match c as libc::c_int {
            123 | 91 => {
                count += 1;
                if !tokens.is_null() {
                    token = jsmn_alloc_token(parser, tokens, num_tokens as size_t);
                    if token.is_null() {
                        return JSMN_ERROR_NOMEM as libc::c_int;
                    }
                    if (*parser).toksuper != -1i32 {
                        let ref mut fresh0 = (*tokens.offset((*parser).toksuper as isize)).size;
                        *fresh0 += 1
                    }
                    (*token).type_0 = (if c as libc::c_int == '{' as i32 {
                        JSMN_OBJECT as libc::c_int
                    } else {
                        JSMN_ARRAY as libc::c_int
                    }) as jsmntype_t;
                    (*token).start = (*parser).pos as libc::c_int;
                    (*parser).toksuper =
                        (*parser).toknext.wrapping_sub(1i32 as libc::c_uint) as libc::c_int
                }
            }
            125 | 93 => {
                if !tokens.is_null() {
                    type_0 = (if c as libc::c_int == '}' as i32 {
                        JSMN_OBJECT as libc::c_int
                    } else {
                        JSMN_ARRAY as libc::c_int
                    }) as jsmntype_t;
                    i = (*parser).toknext.wrapping_sub(1i32 as libc::c_uint) as libc::c_int;
                    while i >= 0i32 {
                        token = &mut *tokens.offset(i as isize) as *mut jsmntok_t;
                        if (*token).start != -1i32 && (*token).end == -1i32 {
                            if (*token).type_0 as libc::c_uint != type_0 as libc::c_uint {
                                return JSMN_ERROR_INVAL as libc::c_int;
                            }
                            (*parser).toksuper = -1i32;
                            (*token).end =
                                (*parser).pos.wrapping_add(1i32 as libc::c_uint) as libc::c_int;
                            break;
                        } else {
                            i -= 1
                        }
                    }
                    if i == -1i32 {
                        return JSMN_ERROR_INVAL as libc::c_int;
                    }
                    while i >= 0i32 {
                        token = &mut *tokens.offset(i as isize) as *mut jsmntok_t;
                        if (*token).start != -1i32 && (*token).end == -1i32 {
                            (*parser).toksuper = i;
                            break;
                        } else {
                            i -= 1
                        }
                    }
                }
            }
            34 => {
                r = jsmn_parse_string(parser, js, len, tokens, num_tokens as size_t);
                if r < 0i32 {
                    return r;
                }
                count += 1;
                if (*parser).toksuper != -1i32 && !tokens.is_null() {
                    let ref mut fresh1 = (*tokens.offset((*parser).toksuper as isize)).size;
                    *fresh1 += 1
                }
            }
            9 | 13 | 10 | 32 => {}
            58 => {
                (*parser).toksuper =
                    (*parser).toknext.wrapping_sub(1i32 as libc::c_uint) as libc::c_int
            }
            44 => {
                if !tokens.is_null()
                    && (*parser).toksuper != -1i32
                    && (*tokens.offset((*parser).toksuper as isize)).type_0 as libc::c_uint
                        != JSMN_ARRAY as libc::c_int as libc::c_uint
                    && (*tokens.offset((*parser).toksuper as isize)).type_0 as libc::c_uint
                        != JSMN_OBJECT as libc::c_int as libc::c_uint
                {
                    i = (*parser).toknext.wrapping_sub(1i32 as libc::c_uint) as libc::c_int;
                    while i >= 0i32 {
                        if (*tokens.offset(i as isize)).type_0 as libc::c_uint
                            == JSMN_ARRAY as libc::c_int as libc::c_uint
                            || (*tokens.offset(i as isize)).type_0 as libc::c_uint
                                == JSMN_OBJECT as libc::c_int as libc::c_uint
                        {
                            if (*tokens.offset(i as isize)).start != -1i32
                                && (*tokens.offset(i as isize)).end == -1i32
                            {
                                (*parser).toksuper = i;
                                break;
                            }
                        }
                        i -= 1
                    }
                }
            }
            _ => {
                r = jsmn_parse_primitive(parser, js, len, tokens, num_tokens as size_t);
                if r < 0i32 {
                    return r;
                }
                count += 1;
                if (*parser).toksuper != -1i32 && !tokens.is_null() {
                    let ref mut fresh2 = (*tokens.offset((*parser).toksuper as isize)).size;
                    *fresh2 += 1
                }
            }
        }
        (*parser).pos = (*parser).pos.wrapping_add(1)
    }
    if !tokens.is_null() {
        i = (*parser).toknext.wrapping_sub(1i32 as libc::c_uint) as libc::c_int;
        while i >= 0i32 {
            if (*tokens.offset(i as isize)).start != -1i32
                && (*tokens.offset(i as isize)).end == -1i32
            {
                return JSMN_ERROR_PART as libc::c_int;
            }
            i -= 1
        }
    }

    count
}

/* *
 * Fills next available token with JSON primitive.
 */
unsafe fn jsmn_parse_primitive(
    mut parser: *mut jsmn_parser,
    mut js: *const libc::c_char,
    mut len: size_t,
    mut tokens: *mut jsmntok_t,
    mut num_tokens: size_t,
) -> libc::c_int {
    let mut token: *mut jsmntok_t;
    let mut start: libc::c_int;
    start = (*parser).pos as libc::c_int;
    while (*parser).pos < len as libc::c_uint
        && *js.offset((*parser).pos as isize) as libc::c_int != '\u{0}' as i32
    {
        match *js.offset((*parser).pos as isize) as libc::c_int {
            58 | 9 | 13 | 10 | 32 | 44 | 93 | 125 => {
                break;
            }
            _ => {}
        }
        if (*js.offset((*parser).pos as isize) as libc::c_int) < 32i32
            || *js.offset((*parser).pos as isize) as libc::c_int >= 127i32
        {
            (*parser).pos = start as libc::c_uint;
            return JSMN_ERROR_INVAL as libc::c_int;
        }
        (*parser).pos = (*parser).pos.wrapping_add(1)
    }
    if tokens.is_null() {
        (*parser).pos = (*parser).pos.wrapping_sub(1);
        return 0i32;
    }
    token = jsmn_alloc_token(parser, tokens, num_tokens);
    if token.is_null() {
        (*parser).pos = start as libc::c_uint;
        return JSMN_ERROR_NOMEM as libc::c_int;
    }
    jsmn_fill_token(token, JSMN_PRIMITIVE, start, (*parser).pos as libc::c_int);
    (*parser).pos = (*parser).pos.wrapping_sub(1);

    0
}

/* *
 * Fills token type and boundaries.
 */
unsafe fn jsmn_fill_token(
    mut token: *mut jsmntok_t,
    mut type_0: jsmntype_t,
    mut start: libc::c_int,
    mut end: libc::c_int,
) {
    (*token).type_0 = type_0;
    (*token).start = start;
    (*token).end = end;
    (*token).size = 0i32;
}

/*
Copyright (c) 2010 Serge A. Zaitsev

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
*/

/* *
 * Allocates a fresh unused token from the token pool.
 */
unsafe fn jsmn_alloc_token(
    mut parser: *mut jsmn_parser,
    mut tokens: *mut jsmntok_t,
    mut num_tokens: size_t,
) -> *mut jsmntok_t {
    let mut tok: *mut jsmntok_t;
    if (*parser).toknext as size_t >= num_tokens {
        return 0 as *mut jsmntok_t;
    }
    let fresh3 = (*parser).toknext;
    (*parser).toknext = (*parser).toknext.wrapping_add(1);
    tok = &mut *tokens.offset(fresh3 as isize) as *mut jsmntok_t;
    (*tok).end = -1i32;
    (*tok).start = (*tok).end;
    (*tok).size = 0i32;

    tok
}

/* *
 * Fills next token with JSON string.
 */
unsafe fn jsmn_parse_string(
    mut parser: *mut jsmn_parser,
    mut js: *const libc::c_char,
    mut len: size_t,
    mut tokens: *mut jsmntok_t,
    mut num_tokens: size_t,
) -> libc::c_int {
    let mut token: *mut jsmntok_t;
    let mut start: libc::c_int = (*parser).pos as libc::c_int;
    (*parser).pos = (*parser).pos.wrapping_add(1);
    while ((*parser).pos as size_t) < len
        && *js.offset((*parser).pos as isize) as libc::c_int != '\u{0}' as i32
    {
        let mut c: libc::c_char = *js.offset((*parser).pos as isize);
        if c as libc::c_int == '\"' as i32 {
            if tokens.is_null() {
                return 0i32;
            }
            token = jsmn_alloc_token(parser, tokens, num_tokens);
            if token.is_null() {
                (*parser).pos = start as libc::c_uint;
                return JSMN_ERROR_NOMEM as libc::c_int;
            }
            jsmn_fill_token(
                token,
                JSMN_STRING,
                start + 1i32,
                (*parser).pos as libc::c_int,
            );
            return 0i32;
        }
        if c as libc::c_int == '\\' as i32 && ((*parser).pos.wrapping_add(1) as size_t) < len {
            let mut i: libc::c_int;
            (*parser).pos = (*parser).pos.wrapping_add(1);
            match *js.offset((*parser).pos as isize) as libc::c_int {
                34 | 47 | 92 | 98 | 102 | 114 | 110 | 116 => {}
                117 => {
                    (*parser).pos = (*parser).pos.wrapping_add(1);
                    i = 0i32;
                    while i < 4i32
                        && ((*parser).pos as size_t) < len
                        && *js.offset((*parser).pos as isize) as libc::c_int != '\u{0}' as i32
                    {
                        if !(*js.offset((*parser).pos as isize) as libc::c_int >= 48i32
                            && *js.offset((*parser).pos as isize) as libc::c_int <= 57i32
                            || *js.offset((*parser).pos as isize) as libc::c_int >= 65i32
                                && *js.offset((*parser).pos as isize) as libc::c_int <= 70i32
                            || *js.offset((*parser).pos as isize) as libc::c_int >= 97i32
                                && *js.offset((*parser).pos as isize) as libc::c_int <= 102i32)
                        {
                            (*parser).pos = start as libc::c_uint;
                            return JSMN_ERROR_INVAL as libc::c_int;
                        }
                        (*parser).pos = (*parser).pos.wrapping_add(1);
                        i += 1
                    }
                    (*parser).pos = (*parser).pos.wrapping_sub(1)
                }
                _ => {
                    (*parser).pos = start as libc::c_uint;
                    return JSMN_ERROR_INVAL as libc::c_int;
                }
            }
        }
        (*parser).pos = (*parser).pos.wrapping_add(1)
    }
    (*parser).pos = start as libc::c_uint;

    JSMN_ERROR_PART as libc::c_int
}
