#ifndef MACSTAT_H
#define MACSTAT_H

#include <stdbool.h>
#include <stdint.h>

typedef struct {
    bool ok;
    uint64_t total;
    uint64_t used;
    uint64_t app;
    uint64_t wired;
    uint64_t compressed;
    uint64_t cached;
    uint64_t free;
    uint64_t swap_total;
    uint64_t swap_used;
    uint64_t swap_free;
    int32_t pressure;
} MacstatMemory;

typedef struct {
    bool ok;
    uint32_t cores;
    double system;
    double user;
    double idle;
} MacstatCpu;

MacstatMemory macstat_memory_read(void);
MacstatCpu macstat_cpu_measure(uint64_t interval_ms);

#endif
