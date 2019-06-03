#include <stdlib.h>
#include <stdarg.h>
#include <ctype.h>
#include <string.h>
#include <stdio.h>
#include "misc.h"


static char* internal_dc_strdup(const char* s) /* strdup(NULL) is undefined, save_strdup(NULL) returns an empty string in this case */
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
		return internal_dc_strdup("ErrFmt");
	}

	buf = malloc(char_cnt_without_zero+2 /* +1 would be enough, however, protect against off-by-one-errors */);
	if (buf==NULL) {
		va_end(argp_copy);
		return internal_dc_strdup("ErrMem");
	}

	vsnprintf(buf, char_cnt_without_zero+1, format, argp_copy);
	va_end(argp_copy);
	return buf;
}
