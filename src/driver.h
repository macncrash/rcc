/* driver.h - the compile pipeline behind `rcc ROOT.rs -o OUT`. */
#ifndef RCC_DRIVER_H
#define RCC_DRIVER_H

/* True if the command line asks for compilation (or a core dump mode)
   rather than one of the front-end test modes. */
int rcc_is_compile_command(int argc, char **argv);
int rcc_compile_main(int argc, char **argv);

#endif
