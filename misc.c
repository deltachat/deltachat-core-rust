#include <stdlib.h>
#include <stdarg.h>
#include <ctype.h>
#include <string.h>
#include <stdio.h>
#include "misc.h"


char* dc_strdup(const char* s) /* strdup(NULL) is undefined, save_strdup(NULL) returns an empty string in this case */
{
	char* ret = NULL;
	if (s) {
		if ((ret=strdup(s))==NULL) {
			exit(16); /* cannot allocate (little) memory, unrecoverable error */
		}
	}
	else {
		if ((ret=(char*)calloc(1, 1))==NULL) {
			exit(17); /* cannot allocate little memory, unrecoverable error */
		}
	}
	return ret;
}

char* dc_mprintf(const char* format, ...)
{
	char  testbuf[1];
	char* buf = NULL;
	int   char_cnt_without_zero = 0;

	va_list argp;
	va_list argp_copy;
	va_start(argp, format);
	va_copy(argp_copy, argp);

	char_cnt_without_zero = vsnprintf(testbuf, 0, format, argp);
	va_end(argp);
	if (char_cnt_without_zero < 0) {
		va_end(argp_copy);
		return dc_strdup("ErrFmt");
	}

	buf = malloc(char_cnt_without_zero+2 /* +1 would be enough, however, protect against off-by-one-errors */);
	if (buf==NULL) {
		va_end(argp_copy);
		return dc_strdup("ErrMem");
	}

	vsnprintf(buf, char_cnt_without_zero+1, format, argp_copy);
	va_end(argp_copy);
	return buf;
}


/**
 * Add a string to the end of the current string in a string-builder-object.
 * The internal buffer is reallocated as needed.
 * If reallocation fails, the program halts.
 *
 * @param strbuilder The object to initialze. Must be initialized with
 *      dc_strbuilder_init().
 * @param text Null-terminated string to add to the end of the string-builder-string.
 * @return Returns a pointer to the copy of the given text.
 *     The returned pointer is a pointer inside dc_strbuilder_t::buf and MUST NOT
 *     be freed.  If the string-builder was empty before, the returned
 *     pointer is equal to dc_strbuilder_t::buf.
 *     If the given text is NULL, NULL is returned and the string-builder-object is not modified.
 */
char* dc_strbuilder_cat(dc_strbuilder_t* strbuilder, const char* text)
{
	// this function MUST NOT call logging functions as it is used to output the log
	if (strbuilder==NULL || text==NULL) {
		return NULL;
	}

	int len = strlen(text);

	if (len > strbuilder->free) {
		int add_bytes  = DC_MAX(len, strbuilder->allocated);
		int old_offset = (int)(strbuilder->eos - strbuilder->buf);

		strbuilder->allocated = strbuilder->allocated + add_bytes;
		strbuilder->buf       = realloc(strbuilder->buf, strbuilder->allocated+add_bytes);

        if (strbuilder->buf==NULL) {
			exit(39);
		}

		strbuilder->free      = strbuilder->free + add_bytes;
		strbuilder->eos       = strbuilder->buf + old_offset;
	}

	char* ret = strbuilder->eos;

	strcpy(strbuilder->eos, text);
	strbuilder->eos += len;
	strbuilder->free -= len;

	return ret;
}

/**
 * Add a formatted string to a string-builder-object.
 * This function is similar to dc_strbuilder_cat() but allows the same
 * formatting options as eg. printf()
 *
 * @param strbuilder The object to initialze. Must be initialized with
 *      dc_strbuilder_init().
 * @param format The formatting string to add to the string-builder-object.
 *      This parameter may be followed by data to be inserted into the
 *      formatting string, see eg. printf()
 * @return None.
 */
void dc_strbuilder_catf(dc_strbuilder_t* strbuilder, const char* format, ...)
{
	char  testbuf[1];
	char* buf = NULL;
	int   char_cnt_without_zero = 0;

	va_list argp;
	va_list argp_copy;
	va_start(argp, format);
	va_copy(argp_copy, argp);

	char_cnt_without_zero = vsnprintf(testbuf, 0, format, argp);
	va_end(argp);
	if (char_cnt_without_zero < 0) {
		va_end(argp_copy);
		dc_strbuilder_cat(strbuilder, "ErrFmt");
		return;
	}

	buf = malloc(char_cnt_without_zero+2 /* +1 would be enough, however, protect against off-by-one-errors */);
	if (buf==NULL) {
		va_end(argp_copy);
		dc_strbuilder_cat(strbuilder, "ErrMem");
		return;
	}

	vsnprintf(buf, char_cnt_without_zero+1, format, argp_copy);
	va_end(argp_copy);

	dc_strbuilder_cat(strbuilder, buf);
	free(buf);
}
