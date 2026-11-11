/*
 * driver.c - `rcc ROOT.rs -o OUT [options]`: expand -> resolve -> typeck ->
 * mono/MIR -> C -> $RCC_CC.
 *
 *   --extern NAME=LIB.rs   compile a library crate from source alongside
 *   --cfg SPEC, --env K=V  passed to the expander
 *   --std DIR              directory containing src/lib.rs of std
 *                          (default: $RCC_HOME/lib/std, else relative to
 *                          the rcc executable: ../lib/std)
 *   --no-std               do not load std (core tests only)
 *   --emit-c FILE          write the generated C to FILE (and keep it)
 *   --crate-name NAME      name of the root crate (default: file stem)
 *   --dump-resolve         print the module tree with resolved imports
 *   --dump-types           print the THIR of every fn reachable from main
 *                          (with --check-all: of every fn in the root crate)
 *   --dump-mir             print the MIR of every instance
 *   --check-all            type-check every body of the root crate (and
 *                          --extern crates), not only reachable ones
 *   --check-std            also type-check every non-generic-agnostic body
 *                          of std (for std authors)
 *   --no-cc                stop after writing C
 * Environment: RCC_CC (C compiler, default `cc`), RCC_CFLAGS (default
 * `-O1`), RCC_HOME, RCC_TIME (print CPU time per phase and peak memory),
 * RCC_UNITS (number of C translation units; default 1, or one per ~1500
 * instances for programs with >= 3000 instances: FILE.h + FILE.N.c),
 * RCC_JOBS (parallel C compiler processes, default: number of CPUs).
 */
#include "driver.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <sys/resource.h>
#include <time.h>
#include <unistd.h>

#include "cgen.h"
#include "expand.h"
#include "lex.h"
#include "mir.h"
#include "resolve.h"
#include "typeck.h"

int rcc_is_compile_command(int argc, char **argv) {
    static const char *flags[] = {"-o", "--emit-c", "--dump-resolve", "--dump-types", "--dump-mir",
                                  "--check-all", "--check-std", "--no-cc", NULL};
    for (int i = 1; i < argc; i++)
        for (int k = 0; flags[k]; k++)
            if (strcmp(argv[i], flags[k]) == 0)
                return 1;
    return 0;
}

static const char *dir_of(const char *path) {
    const char *slash = strrchr(path, '/');
    if (!slash)
        return ".";
    return arena_strndup(path, (size_t)(slash - path));
}

static const char *stem_of(const char *path) {
    const char *b = strrchr(path, '/');
    b = b ? b + 1 : path;
    size_t n = strlen(b);
    if (n > 3 && strcmp(b + n - 3, ".rs") == 0)
        n -= 3;
    char *s = arena_strndup(b, n);
    for (char *c = s; *c; c++)
        if (*c == '-')
            *c = '_';
    return s;
}

static bool file_exists(const char *p) { return access(p, R_OK) == 0; }

static const char *find_home(const char *argv0) {
    const char *env = getenv("RCC_HOME");
    if (env && *env)
        return env;
    /* build/rcc -> repository root */
    const char *d = dir_of(argv0);
    const char *cand = arena_printf("%s/..", d);
    if (file_exists(arena_printf("%s/rt/rt.c", cand)))
        return cand;
    if (file_exists(arena_printf("%s/rt/rt.c", d)))
        return d;
    return ".";
}

/* Type-check every body of a crate. All errors are fatal, so the bodies are
   checked in a forked child that reports its progress through a pipe; after
   a failure a new child continues behind the failed body. */
static void check_bodies(uint32_t crate_idx) {
    const DefIdList *all = resolve_all_bodies();
    uint32_t n = 0, failed = 0, start = 0;
    for (uint32_t i = 0; i < all->len; i++)
        if (def_get(all->data[i])->crate_ == crate_idx)
            n++;
    while (start < all->len) {
        int fds[2];
        fflush(stdout);
        fflush(stderr);
        if (pipe(fds) != 0) {
            fprintf(stderr, "rcc: pipe failed\n");
            exit(1);
        }
        pid_t pid = fork();
        if (pid == 0) {
            close(fds[0]);
            for (uint32_t i = start; i < all->len; i++) {
                if (def_get(all->data[i])->crate_ != crate_idx)
                    continue;
                /* announce the body about to be checked */
                if (write(fds[1], &i, sizeof i) != (ssize_t)sizeof i)
                    _exit(2);
                (void)typeck_body(all->data[i]);
            }
            _exit(0);
        }
        close(fds[1]);
        uint32_t last = all->len, v;
        while (read(fds[0], &v, sizeof v) == (ssize_t)sizeof v)
            last = v;
        close(fds[0]);
        int status = 0;
        if (pid < 0 || waitpid(pid, &status, 0) < 0) {
            fprintf(stderr, "rcc: fork failed\n");
            exit(1);
        }
        if (status == 0)
            break;
        failed++;
        if (last == all->len)
            break; /* died before checking anything */
        start = last + 1;
    }
    fprintf(stderr, "rcc: checked %u bodies of crate `%s`: %u failed\n", n,
            sym_str(crate_get(crate_idx)->name), failed);
}

/* Phase timing (RCC_TIME=1): CPU seconds per phase and peak memory. */
static bool time_phases;
static clock_t phase_start;

static void phase_done(const char *name) {
    if (!time_phases)
        return;
    clock_t now = clock();
    struct rusage ru;
    long rss = 0;
    if (getrusage(RUSAGE_SELF, &ru) == 0)
        rss = (long)ru.ru_maxrss;
#ifdef __APPLE__
    rss /= 1024; /* bytes -> KiB */
#endif
    fprintf(stderr, "rcc: %-10s %7.2fs  (peak rss %ld MiB)\n", name,
            (double)(now - phase_start) / CLOCKS_PER_SEC, rss / 1024);
    phase_start = now;
}

int rcc_compile_main(int argc, char **argv) {
    const char *root = NULL, *output = NULL, *emit_c = NULL, *std_dir = NULL, *crate_name = NULL;
    const char *cfgs[256], *envs[256], *externs[64];
    uint32_t ncfgs = 0, nenvs = 0, nexterns = 0;
    bool no_std = false, dump_resolve = false, dump_types = false, dump_mir = false;
    bool check_all = false, check_std = false, no_cc = false;
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        bool has_next = i + 1 < argc;
        if (strcmp(a, "-o") == 0 && has_next) output = argv[++i];
        else if (strcmp(a, "--emit-c") == 0 && has_next) emit_c = argv[++i];
        else if (strcmp(a, "--std") == 0 && has_next) std_dir = argv[++i];
        else if (strcmp(a, "--crate-name") == 0 && has_next) crate_name = argv[++i];
        else if (strcmp(a, "--cfg") == 0 && has_next && ncfgs < 256) cfgs[ncfgs++] = argv[++i];
        else if (strcmp(a, "--env") == 0 && has_next && nenvs < 256) envs[nenvs++] = argv[++i];
        else if (strcmp(a, "--extern") == 0 && has_next && nexterns < 64) externs[nexterns++] = argv[++i];
        else if (strcmp(a, "--no-std") == 0) no_std = true;
        else if (strcmp(a, "--dump-resolve") == 0) dump_resolve = true;
        else if (strcmp(a, "--dump-types") == 0) dump_types = true;
        else if (strcmp(a, "--dump-mir") == 0) dump_mir = true;
        else if (strcmp(a, "--check-all") == 0) check_all = true;
        else if (strcmp(a, "--check-std") == 0) check_std = true;
        else if (strcmp(a, "--no-cc") == 0) no_cc = true;
        else if (a[0] == '-') {
            fprintf(stderr, "rcc: unknown option `%s`\n", a);
            return 2;
        } else if (!root) root = a;
        else {
            fprintf(stderr, "rcc: more than one input file\n");
            return 2;
        }
    }
    if (!root) {
        fprintf(stderr, "rcc: no input file\n");
        return 2;
    }
    time_phases = getenv("RCC_TIME") != NULL;
    phase_start = clock();
    lex_init();
    types_init();
    const char *home = find_home(argv[0]);
    ExpandOptions eo;
    memset(&eo, 0, sizeof eo);
    eo.cfgs = cfgs;
    eo.ncfgs = ncfgs;
    eo.envs = envs;
    eo.nenvs = nenvs;

    uint32_t std_idx = UINT32_MAX;
    if (!no_std) {
        const char *sd = std_dir ? std_dir : arena_printf("%s/lib/std", home);
        const char *lib = arena_printf("%s/src/lib.rs", sd);
        if (!file_exists(lib)) {
            fprintf(stderr, "rcc: cannot find std at %s (use --std DIR or RCC_HOME)\n", lib);
            return 1;
        }
        std_idx = resolve_add_crate(sym_intern_cstr("std"), expand_crate(lib, "std", &eo));
    }
    for (uint32_t i = 0; i < nexterns; i++) {
        const char *eq = strchr(externs[i], '=');
        if (!eq) {
            fprintf(stderr, "rcc: --extern expects NAME=PATH\n");
            return 2;
        }
        const char *name = arena_strndup(externs[i], (size_t)(eq - externs[i]));
        resolve_add_crate(sym_intern_cstr(name), expand_crate(eq + 1, name, &eo));
    }
    if (!crate_name)
        crate_name = stem_of(root);
    uint32_t root_idx = resolve_add_crate(sym_intern_cstr(crate_name), expand_crate(root, crate_name, &eo));
    resolve_finish();
    phase_done("expand");
    if (dump_resolve)
        resolve_dump(stdout);
    typeck_init();
    phase_done("collect");
    if (check_std && std_idx != UINT32_MAX)
        check_bodies(std_idx);
    if (check_all)
        for (uint32_t c = 0; c < crate_count(); c++)
            if (c != std_idx)
                check_bodies(c);

    Res m = module_lookup(crate_get(root_idx)->root, sym_intern_cstr("main"), NS_VALUE);
    if (m.kind != RES_DEF || def_get(m.def)->kind != DEF_FN) {
        if (check_all || check_std || dump_resolve)
            return 0;
        fprintf(stderr, "rcc: `main` function not found in crate `%s`\n", crate_name);
        return 1;
    }
    Program *p = mono_program(m.def);
    phase_done("typeck+mir");
    if (time_phases)
        fprintf(stderr, "rcc: %u instances, %u globals, %u vtables, %u drop glues\n", p->instances.len,
                p->globals.len, p->vtables.len, p->drop_glue.len);
    if (dump_types) {
        for (uint32_t i = 0; i < p->instances.len; i++) {
            Instance *inst = p->instances.data[i];
            if (inst->kind == INST_FN && def_get(inst->def)->body &&
                (dump_types && def_get(inst->def)->crate_ == root_idx))
                thir_dump_body(stdout, def_get(inst->def)->body);
        }
    }
    if (dump_mir) {
        for (uint32_t i = 0; i < p->instances.len; i++)
            if (p->instances.data[i]->mir)
                mir_dump_body(stdout, p->instances.data[i]->mir);
    }
    if (!output && !emit_c)
        return 0;
    const char *c_path = emit_c ? emit_c : arena_printf("%s.rcc.c", output);
    const char *cc = getenv("RCC_CC");
    if (!cc || !*cc)
        cc = "cc";
    /* ccc needs workarounds in the generated C (see rt/rcc_rt.h) */
    const char *cc_base = strrchr(cc, '/');
    cc_base = cc_base ? cc_base + 1 : cc;
    bool cc_is_ccc = strstr(cc_base, "ccc") != NULL;
    if (cc_is_ccc) {
        /* ccc's arm64 peephole pass `drop_mask_of_narrow_value` miscompiles
           code that tests a comparison result (docs/STATUS.md, "ccc issues
           met"); ccc lets passes be skipped by name. */
        const char *skip = getenv("CCC_PEEPHOLE_SKIP");
        if (!skip || !*skip)
            setenv("CCC_PEEPHOLE_SKIP", "drop_mask_of_narrow_value", 1);
        else if (!strstr(skip, "drop_mask_of_narrow_value"))
            setenv("CCC_PEEPHOLE_SKIP", arena_printf("%s,drop_mask_of_narrow_value", skip), 1);
    }
    /* Large programs are split into several translation units that are
       compiled in parallel (RCC_UNITS forces a count, RCC_JOBS the number
       of concurrent compiler processes). */
    uint32_t jobs = 0, nunits = 0;
    const char *ev = getenv("RCC_JOBS");
    if (ev && *ev)
        jobs = (uint32_t)atoi(ev);
    if (!jobs) {
        long n = sysconf(_SC_NPROCESSORS_ONLN);
        jobs = n > 0 ? (uint32_t)n : 1;
    }
    ev = getenv("RCC_UNITS");
    if (ev && *ev)
        nunits = (uint32_t)atoi(ev);
    if (!nunits) {
        nunits = 1;
        if (p->instances.len >= 3000) {
            nunits = p->instances.len / 1500;
            if (nunits < jobs)
                nunits = jobs;
            if (nunits > 64)
                nunits = 64;
        }
    }
    const char **unit_paths = NEW_ARRAY(const char *, nunits);
    if (nunits == 1) {
        unit_paths[0] = c_path;
        FILE *f = fopen(c_path, "w");
        if (!f) {
            fprintf(stderr, "rcc: cannot write %s\n", c_path);
            return 1;
        }
        cgen_program(p, f);
        if (fclose(f) != 0) {
            fprintf(stderr, "rcc: error writing %s\n", c_path);
            return 1;
        }
    } else {
        const char *h_path = arena_printf("%s.h", c_path);
        const char *h_name = strrchr(h_path, '/');
        h_name = h_name ? h_name + 1 : h_path;
        FILE *h = fopen(h_path, "w");
        FILE **uf = NEW_ARRAY(FILE *, nunits);
        bool ok = h != NULL;
        for (uint32_t u = 0; u < nunits; u++) {
            unit_paths[u] = arena_printf("%s.%u.c", c_path, u);
            uf[u] = fopen(unit_paths[u], "w");
            ok = ok && uf[u];
        }
        if (!ok) {
            fprintf(stderr, "rcc: cannot write %s and its units\n", h_path);
            return 1;
        }
        cgen_program_split(p, h, h_name, uf, nunits);
        ok = fclose(h) == 0;
        for (uint32_t u = 0; u < nunits; u++)
            ok = (fclose(uf[u]) == 0) && ok;
        if (!ok) {
            fprintf(stderr, "rcc: error writing %s\n", h_path);
            return 1;
        }
    }
    phase_done("cgen");
    if (!output || no_cc)
        return 0;
    const char *cflags = getenv("RCC_CFLAGS");
    if (!cflags)
        cflags = "-O1";
    const char *defs = cc_is_ccc ? " -DRCC_CC_IS_CCC" : "";
    if (nunits == 1) {
        const char *cmd = arena_printf("%s %s%s -w -I '%s/rt' '%s' '%s/rt/rt.c' -o '%s' -lm", cc, cflags,
                                       defs, home, c_path, home, output);
        int st = system(cmd);
        phase_done("cc (wall time not included)");
        if (st != 0) {
            fprintf(stderr, "rcc: C compiler failed: %s\n", cmd);
            return 1;
        }
        if (!emit_c)
            remove(c_path);
        return 0;
    }
    /* compile the units, at most `jobs` at a time */
    uint32_t next = 0, running = 0, failed = 0;
    fflush(stdout);
    fflush(stderr);
    while (next < nunits || running) {
        if (next < nunits && running < jobs && !failed) {
            const char *cmd = arena_printf("%s %s%s -w -I '%s/rt' -c '%s' -o '%s.o'", cc, cflags, defs, home,
                                           unit_paths[next], unit_paths[next]);
            pid_t pid = fork();
            if (pid == 0) {
                execl("/bin/sh", "sh", "-c", cmd, (char *)NULL);
                _exit(127);
            }
            if (pid < 0) {
                fprintf(stderr, "rcc: cannot start the C compiler\n");
                return 1;
            }
            next++;
            running++;
            continue;
        }
        if (!running)
            break;
        int status = 0;
        if (wait(&status) < 0)
            break;
        running--;
        if (status != 0)
            failed++;
    }
    if (failed) {
        fprintf(stderr, "rcc: C compiler failed on %u of %u units (%s.N.c)\n", failed, nunits, c_path);
        return 1;
    }
    StrBuf link = {0};
    sb_printf(&link, "%s %s%s -w -I '%s/rt'", cc, cflags, defs, home);
    for (uint32_t u = 0; u < nunits; u++)
        sb_printf(&link, " '%s.o'", unit_paths[u]);
    sb_printf(&link, " '%s/rt/rt.c' -o '%s' -lm", home, output);
    int st = system(sb_str(&link));
    phase_done("cc (wall time not included)");
    if (st != 0) {
        fprintf(stderr, "rcc: linking failed: %s\n", sb_str(&link));
        return 1;
    }
    for (uint32_t u = 0; u < nunits; u++) {
        remove(arena_printf("%s.o", unit_paths[u]));
        if (!emit_c)
            remove(unit_paths[u]);
    }
    if (!emit_c)
        remove(arena_printf("%s.h", c_path));
    return 0;
}
