/* cgen.h - C code generation from MIR. See cgen.c. */
#ifndef RCC_CGEN_H
#define RCC_CGEN_H

#include <stdio.h>

#include "mir.h"

/* Write the whole program as one C translation unit. */
void cgen_program(Program *p, FILE *out);

/* Write the program as `nunits` translation units that all include a
   common header (written to `header`, included as `header_name`). The
   function bodies are distributed evenly; unit 0 holds the globals, the
   vtables and `main`. */
void cgen_program_split(Program *p, FILE *header, const char *header_name, FILE **units, uint32_t nunits);

#endif
