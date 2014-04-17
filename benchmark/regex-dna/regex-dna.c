/* The Computer Language Benchmarks Game
 * http://benchmarksgame.alioth.debian.org/
   contributed by Paul Serice
*/

#include <sys/types.h>
#include <pthread.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <glib.h>
#include <tcl.h>

/*************************************************************************
 * Data Structures and Typedefs
 *************************************************************************/

/* Mapping of a nucleic acid code to its meaning.  This is used with
 * regsub() to substitute each occurrence of "code" in the main input
 * string with its "meaning." */
static struct nucleic_acid_code {
    char* code;
    char* meaning;
} nacodes[] = {{"B", "(c|g|t)"},
               {"D", "(a|g|t)"},
               {"H", "(a|c|t)"},
               {"K", "(g|t)"},
               {"M", "(a|c)"},
               {"N", "(a|c|g|t)"},
               {"R", "(a|g)"},
               {"S", "(c|g)"},
               {"V", "(a|c|g)"},
               {"W", "(a|t)"},
               {"Y", "(c|t)"},
               {NULL, NULL}
};

/* The variants are used with regcount() to count the number of times
 * each variant appears in the main input string. */
static const char* variants[] = {
  "agggtaaa|tttaccct",
  "[cgt]gggtaaa|tttaccc[acg]",
  "a[act]ggtaaa|tttacc[agt]t",
  "ag[act]gtaaa|tttac[agt]ct",
  "agg[act]taaa|ttta[agt]cct",
  "aggg[acg]aaa|ttt[cgt]ccct",
  "agggt[cgt]aa|tt[acg]accct",
  "agggta[cgt]a|t[acg]taccct",
  "agggtaa[cgt]|[acg]ttaccct",
  NULL
};


/* To process the variants, a small thread pool is created.  Each
 * thread is passed an array of these tasks.  The threads combine to
 * perform the tasks.  When there are no more tasks, the threads exit
 * and the parent joins with them before continuing. */
typedef struct variant_worker_task {

    /* input: which variant to process */
    const char* variant;

    /* input: string against which "variant" will be matched */
    Tcl_Obj* s;

    /* output: number of times "variant" matched against "s" */
    unsigned long int count;

} *variant_worker_task_t;


/* Data passed into each thread that process the variants.  All the
 * threads in the pool share one copy of this data structure and must
 * use "lock" to synchronize access to it. */
typedef struct variant_worker_data {

    /* shared: lock that protects this structure */
    pthread_mutex_t lock;

    /* shared: array of tasks that the threads are trying to complete */
    variant_worker_task_t tasks;

    /* shared: pointer to shared index into "tasks" */
    volatile int next_task;

    /* shared: total number of tasks in the "tasks" array */
    int total_tasks;

} *variant_worker_data_t;


/* Data passed into each thread that substitutes nucleic acid codes. */
typedef struct nacodes_worker_data {

    /* input/output: String object that is input to the thread as a
     * copy of the range of characters from the main input string over
     * which the thread should work.  The thread should call
     * Tcl_SetStringObj() to set "range" to hold the result of the
     * substitutions. */
    Tcl_Obj* range;

} *nacodes_worker_data_t;


/* Create an explicit typedef for the pthread start functions. */
typedef void* (*thread_start_t)(void*);

/*************************************************************************
 * regcount()
 *************************************************************************/

/* Return the number of times the regular expression "regexp_cstr"
 * uniquely matches against the input string "s". */
static unsigned long
regcount(const char* regexp_cstr,
         Tcl_Obj* s)
{
    int regexec_rv = 0;
    int index = 0;
    int index_max = 0;
    unsigned long rv = 0;
    Tcl_Obj* regexp_cstr_obj = NULL;
    Tcl_RegExp regexp = NULL;
    struct Tcl_RegExpInfo info = {0};

    /* Get "regexp_cstr" as a Tcl string object. */
    regexp_cstr_obj = Tcl_NewStringObj(regexp_cstr, strlen(regexp_cstr));
    Tcl_IncrRefCount(regexp_cstr_obj);

    /* Compile the regular expression. */
    regexp = Tcl_GetRegExpFromObj(NULL, regexp_cstr_obj,
                 TCL_REG_ADVANCED | TCL_REG_NOCASE | TCL_REG_NEWLINE);
    if (!regexp) {
        fprintf(stderr, "*** Error: Tcl_GetRegExpFromObj: failed");
        exit(1);
    }

    /* Iterate over each match. */
    index = 0;
    index_max = Tcl_GetCharLength(s);
    while (index < index_max) {

        /* Test for a match. */
        regexec_rv = Tcl_RegExpExecObj(NULL, regexp, s, index, 1, 0);
        if (regexec_rv == -1) {
            fprintf(stderr, "*** Error: Tcl_RegExpExecObj: failed");
            exit(1);
        }
        if (regexec_rv == 0) {
            /* No matches. */
            break;
        }

        /* Get the match information. */
        Tcl_RegExpGetInfo(regexp, &info);

        /* Advance curr. */
        index += info.matches[0].end;

        /* Increment the match count. */
        ++rv;
    }

    /* Clean up.  Note that "regexp" is owned by "regexp_cstr_obj" so
     * it does not need explicit clean up. */
    Tcl_DecrRefCount(regexp_cstr_obj);

    return rv;
}

/*************************************************************************
 * regsub()
 *************************************************************************/

/* Substitute each occurrence of the regular expression "regex" in "s"
 * with "subst".  The result is returned in a newly allocate string
 * that must be freed with g_free(). */
static char*
regsub(const char* regex,
       const char* s,
       const char* subst,
       GError** err)
{
    char* rv = NULL;
    GRegex* prog = NULL;

    /* How glib propagates exceptions. */
    if (err && *err) {
        goto out;
    }

    /* Compile regex. */
    prog = g_regex_new(regex,
                       G_REGEX_CASELESS |
                       G_REGEX_RAW |
                       G_REGEX_NO_AUTO_CAPTURE |
                       G_REGEX_OPTIMIZE,
                       0,
                       err);
    if (err && *err) {
        goto out;
    }

    /* Substitute. */
    rv = g_regex_replace_literal(prog, s, -1, 0, subst, 0, err);
    if (err && *err) {
        goto out;
    }

 out:

    /* Clean up. */
    if (prog) {
        g_regex_unref(prog);
    }

    return rv;
}

/*************************************************************************
 * load_file()
 *************************************************************************/

/* Load the file f into the string s. */
static void
load_file(FILE* f,
          Tcl_Obj* s)
{
    char* block = NULL;
    size_t block_size = 16384;
    size_t rcount = 0;

    /* Allocate a block for I/O. */
    block = malloc(block_size);
    if (!block) {
        perror("malloc");
        exit(1);
    }

    /* Iterate over each block of input. */
    for (;;) {

        /* Read a block. */
        rcount = fread(block, 1, block_size, f);
        if (rcount == 0) {
            /* Check for errors. */
            if (ferror(f)) {
                perror("fread");
                exit(1);
            }
            /* EOF */
            break;
        }

        /* Append a block. */
        Tcl_AppendToObj(s, block, rcount);
    }

    /* Free block. */
    free(block);
}

/*************************************************************************
 * process_variant_worker() and process_variants()
 *************************************************************************/

/* This is a helper function for process_variant_worker() which is the
 * start routine for the threads that count how many times a variant
 * matches the main input string.  This routing locks "data" and
 * attempts to get the index of the next task.  If successful, it
 * takes ownership of that index by incrementing "data->next_task" so
 * that the next thread that comes along will get the next task.
 * Before returning, this routine releases the lock.  This routine
 * returns true if successful and false otherwise. */
static int
get_variant_index(variant_worker_data_t data,
                  int* index)
{
    int rv = 0;

    /* Lock "data". */
    pthread_mutex_lock(&data->lock);

    /* Get the index for the next task if any remain. */
    if (data->next_task < data->total_tasks) {
        *index = data->next_task++;
        rv = 1;
    }

    /* Unlock "data". */
    pthread_mutex_unlock(&data->lock);

    return rv;
}

/* This is the worker routine for the thread pool that processes the
 * variants.  This routine atomically gets the next task which holds
 * all the information needed to count the number of times the task's
 * "variant" value matches the main input string and stores the result
 * in the task's "count" value.  The main input string is passed in as
 * the task's read-only "s" value. */
static void*
process_variant_worker(variant_worker_data_t data)
{
    int index = 0;

    /* Carefully get the index for the next task. */
    while (get_variant_index(data, &index)) {
        /* Perform the task of counting regex matches. */
        data->tasks[index].count
            = regcount(data->tasks[index].variant,
                       data->tasks[index].s);
    }

    return NULL;
}

/* Process the list of variants by counting the frequency of each
 * regexp in the main input string "s" and printing the results. */
static void
process_variants(int cpu_count,
                 Tcl_Obj* s)
{
    int i = 0;
    int s_length = 0;
    int thread_rv = 0;
    int thread_count = 0;
    int task_count = 0;
    pthread_t* threads = NULL;
    variant_worker_task_t tasks = NULL;
    struct variant_worker_data data = {PTHREAD_MUTEX_INITIALIZER,};

    /* WARNING: Tcl_RegExpExecObj() always does an internal conversion
     * of "s" to a UCS-2 Unicode string if "s" is in UTF-8 format.
     * Normally, this is a nice feature, but as of tcl-8.5, it doesn't
     * appear to be thread-safe.  As a work-around, force the
     * conversion now before starting the threads. */
    Tcl_GetUnicodeFromObj(s, &s_length);

    /* Determine the total number of variants (minus the NULL sentinel). */
    task_count = (int)(sizeof(variants) / sizeof(variants[0]) - 1);

    /* Determine the number of threads to start. */
    thread_count = cpu_count * 2;
    if (thread_count > task_count) {
        thread_count = task_count;
    }

    /* Allocate the "threads" array which holds the thread IDs. */
    threads = calloc(thread_count, sizeof(*threads));
    if (!threads) {
        perror("calloc");
        exit(1);
    }

    /* Allocate the "tasks" array which holds one unit of work per
     * element in the array. */
    tasks = calloc(task_count, sizeof(*tasks));
    if (!tasks) {
        perror("calloc");
        exit(1);
    }

    /* Initialize the task array. */
    for (i = 0 ; i < task_count ; ++i) {
        tasks[i].variant = variants[i];
        tasks[i].s = s;
        tasks[i].count = 0;
    }

    /* Initialize the data shared by the threads. */
    data.tasks = tasks;
    data.next_task = 0;
    data.total_tasks = task_count;

    /* Start the threads. */
    for (i = 0 ; i < thread_count ; ++i) {
        thread_rv = pthread_create(&threads[i],
                                   NULL,
                                   (thread_start_t)process_variant_worker,
                                   &data);
        if (thread_rv) {
            fprintf(stderr, "*** Error: pthread_create: failed");
            exit(1);
        }
    }

    /* Wait for each thread to finish. */
    for (i = 0 ; i < thread_count ; ++i) {
        thread_rv = pthread_join(threads[i], NULL);
        if (thread_rv) {
            fprintf(stderr, "*** Error: pthread_join: failed");
            exit(1);
        }
    }

    /* Print results. */
    for (i = 0 ; i < task_count ; ++i) {
        printf("%s %lu\n", variants[i], tasks[i].count);
    }

    /* Clean up. */
    free(tasks);
    free(threads);
}

/*************************************************************************
 * process_nacodes_worker() and process_nacodes()
 *************************************************************************/

/* This is the worker routing for the threads that process the
 * substitution of the nucleic acid codes with their meanings.  These
 * threads are not in a thread pool because the work can be divided
 * exactly into one thread per cpu.  So the parent just starts each
 * thread and waits for them all to finish.
 *
 * Each worker gets a range of characters from the main input string
 * and is responsible for calling regsub() once for each nucleic acid
 * code.  Thus, if there are 11 nucleic acid codes, each thread calls
 * regsub() 11 times but the scope of the regsub() call is limited to
 * just the range of characters it has been assigned. */
static void*
process_nacodes_worker(nacodes_worker_data_t data)
{
    char* s_in = NULL;
    char* s_out = NULL;
    struct nucleic_acid_code* nacode = NULL;

    /* Get the character range as a C-style string. */
    s_in = Tcl_GetString(data->range);

    /* Iterate over the nucleic acid codes. */
    for (nacode = nacodes ; nacode->code ; ++nacode) {

        /* Perform the substitution. */
        s_out = regsub(nacode->code, s_in, nacode->meaning, NULL);

        /* Free s_in on all but the first pass because s_in
         * belongs to Tcl on the first pass. */
        if (nacode != nacodes) {
            g_free(s_in);
            s_in = NULL;
        }
        /* If this is the last pass, save the result and clean up. */
        if ((nacode + 1)->code == NULL) {
            Tcl_SetStringObj(data->range, s_out, strlen(s_out));
            g_free(s_out);
            s_out = NULL;
        } else {
            /* Otherwise, prepare for the next iteration. */
            s_in = s_out;
            s_out = NULL;
        }
    }

    return NULL;
}

/* Process the nucleic acid codes by substituting each nucleic acid
 * code in "s" with its meaning as defined in the static "nacodes"
 * structure (see top of file).  On return, "s" will hold the
 * substituted string. */
static void
process_nacodes(int cpu_count,
                Tcl_Obj* s)
{
    int i = 0;
    int first = 0;
    int last = 0;
    int s_length = 0;
    int range_length = 0;
    int thread_rv = 0;
    nacodes_worker_data_t data = NULL;
    pthread_t* threads = NULL;

    /* Sanity check to make sure we don't divide by zero. */
    if (cpu_count == 0) {
        return;
    }

    /* Get the total length of s. */
    s_length = Tcl_GetCharLength(s);
    if (s_length == 0) {
        return;
    }

    /* Allocate the "data" array which is used to pass data to and
     * from the threads. */
    data = calloc(cpu_count, sizeof(*data));

    /* Allocate the "threads" array which holds the thread IDs. */
    threads = calloc(cpu_count, sizeof(*threads));

    /* Calculate the number of characters to feed each thread.  Note
     * that we checked above to make sure cpu_count is not zero. */
    range_length = s_length / cpu_count;

    /* Start one thread for each cpu. */
    for (i = 0 ; i < cpu_count ; ++i) {

        /* First, initialize the thread's client data. */

        /* Calculate the first and last index for the range.  Both
         * "first" and "last" indexes are inclusive because that is
         * what Tcl_GetRange() requires.  We also need to make sure
         * the very last range has all the characters in case
         * range_length does not divide s_length evenly. */
        first = range_length * i;
        last = range_length * (i + 1) - 1;
        if (i + 1 == cpu_count) {
            last = s_length - 1;
        }

        /* Pack the data for the worker thread. */
        data[i].range = Tcl_GetRange(s, first, last);
        Tcl_IncrRefCount(data[i].range);

        /* Second, start the thread. */
        thread_rv = pthread_create(&threads[i],
                                   NULL,
                                   (thread_start_t)process_nacodes_worker,
                                   &data[i]);
        if (thread_rv) {
            fprintf(stderr, "*** Error: pthread_create: failed");
            exit(1);
        }
    }

    /* Wait for each thread to finish. */
    for (i = 0 ; i < cpu_count ; ++i) {
        thread_rv = pthread_join(threads[i], NULL);
        if (thread_rv) {
            fprintf(stderr, "*** Error: pthread_join: failed");
            exit(1);
        }
    }

    /* Merge results. */
    Tcl_SetObjLength(s, 0);
    for (i = 0 ; i < cpu_count ; ++i) {
        Tcl_AppendObjToObj(s, data[i].range);
    }

    /* Clean up. */
    for (i = 0 ; i < cpu_count ; ++i) {
        Tcl_DecrRefCount(data[i].range);
    }
    free(threads);
    free(data);
}

/*************************************************************************
 * get_cpu_count()
 *************************************************************************/

/* Return the number of cpus.  If an error occurs, 0 cpus will be
 * reported.  There are other ways to do this, but this is a program
 * to test regexp processing so ... */
static int
get_cpu_count(void)
{
    int rv = 0;
    FILE* f = NULL;
    Tcl_Obj* s = NULL;

    /* Allocate a string. */
    s = Tcl_NewStringObj("", 0);
    Tcl_IncrRefCount(s);

    /* Open /proc/cpuinfo. */
    f = fopen("/proc/cpuinfo", "r");
    if (!f) {
        goto out;
    }

    /* Load file into s. */
    load_file(f, s);

    /* Count the number of cpus.  "\M" matches at the end of a word. */
    rv = regcount("^processor\\M", s);

 out:

    /* Clean up. */
    if (f) {
        fclose(f);
    }
    if (s) {
        Tcl_DecrRefCount(s);
    }

    return rv;
}

/*************************************************************************
 * main()
 *************************************************************************/

int
main(int argc,
     char* argv[])
{
    int rv = 0;
    int cpu_count = 0;
    int init_length = 0;
    int code_length = 0;
    int seq_length = 0;
    char* s_cstr = NULL;
    Tcl_Interp *tcl = NULL;
    Tcl_Obj* s = NULL;

    /* Initialize Tcl. */
    Tcl_FindExecutable(argv[0]);
    tcl = Tcl_CreateInterp();
    Tcl_Preserve((ClientData)tcl);

    /* Count the number of cpus.  If the cpu count could not be
     * determined, assume 4 cpus. */
    cpu_count = get_cpu_count();
    if (!cpu_count) {
        cpu_count = 4;
    }

    /* Allocate s. */
    s = Tcl_NewStringObj("", 0);
    Tcl_IncrRefCount(s);

    /* Load stdin into s. */
    load_file(stdin, s);

    /* Get the length of s. */
    init_length = Tcl_GetCharLength(s);

    /* Strip off section headers and EOLs from s.  This is a little
     * messy because we have to go from Tcl-string to C-string and
     * back to Tcl-string. */
    s_cstr = regsub("(>.*)|\n", Tcl_GetString(s), "", NULL);
    Tcl_SetStringObj(s, s_cstr, strlen(s_cstr));
    g_free(s_cstr);
    s_cstr = NULL;

    /* Get the length of s. */
    code_length = Tcl_GetCharLength(s);

    /* Process the variants by counting them and printing the results. */
    process_variants(cpu_count, s);

    /* Substitute nucleic acid codes in s with their meanings. */
    process_nacodes(cpu_count, s);

    /* Get the length of s. */
    seq_length = Tcl_GetCharLength(s);

    /* Print the lengths. */
    printf("\n%d\n%d\n%d\n", init_length, code_length, seq_length);

    /* Clean up. */
    Tcl_DecrRefCount(s);

    /* Finalize Tcl. */
    Tcl_Release((ClientData)tcl);
    Tcl_Exit(rv);

    /* Not reached. */
    return rv;
}
