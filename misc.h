#define DC_MAX(X, Y) (((X) > (Y))? (X) : (Y))

typedef struct _dc_strbuilder dc_strbuilder_t;


char*   dc_mprintf                 (const char* format, ...); /* The result must be free()'d. */
char* dc_strdup(const char* s);



struct _dc_strbuilder
{
	char* buf;
	int   allocated;
	int   free;
	char* eos;
};


//void  dc_strbuilder_init    (dc_strbuilder_t*, int init_bytes);
char* dc_strbuilder_cat     (dc_strbuilder_t*, const char* text);
void  dc_strbuilder_catf    (dc_strbuilder_t*, const char* format, ...);
//void  dc_strbuilder_empty   (dc_strbuilder_t*);
