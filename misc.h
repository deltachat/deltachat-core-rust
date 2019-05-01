#define DC_MAX(X, Y) (((X) > (Y))? (X) : (Y))

typedef struct _dc_strbuilder dc_strbuilder_t;

struct _dc_strbuilder
{
	char* buf;
	int   allocated;
	int   free;
	char* eos;
};


char* dc_mprintf                 (const char* format, ...); /* The result must be free()'d. */
void  dc_strbuilder_catf    (dc_strbuilder_t*, const char* format, ...);
