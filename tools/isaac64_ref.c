// Standalone C program to generate ISAAC64 reference output for Rust test verification.
// Directly includes NetHack's isaac64 implementation.
//
// Build:
//   cc -I../nethack/include -DUSE_ISAAC64 -o isaac64_ref isaac64_ref.c
//
// The integer.h header from NetHack is required for uint64_t typedef.

#include <stdio.h>
#include <stdint.h>
#include <string.h>

// Provide the types that NetHack's isaac64.h expects
// (normally from config.h → integer.h)
#ifndef INTEGER_H
#define INTEGER_H
#endif

// Minimal config.h shim
#ifndef CONFIG_H
#define CONFIG_H
#define USE_ISAAC64
#define NHSTDC
#endif

#include "isaac64.h"

// Pull in the actual implementation (CC0 licensed)
// We redefine config.h above so the #include inside isaac64.c is harmless.
#undef CONFIG_H
#define CONFIG_H

// Inline the implementation since we've already set up the defines
#define ISAAC64_MASK ((uint64_t)0xFFFFFFFFFFFFFFFFULL)

static inline uint32_t lower_bits(uint64_t x) {
    return (x & ((ISAAC64_SZ - 1) << 3)) >> 3;
}

static inline uint32_t upper_bits(uint64_t y) {
    return (y >> (ISAAC64_SZ_LOG + 3)) & (ISAAC64_SZ - 1);
}

static void isaac64_update(isaac64_ctx *_ctx) {
    uint64_t *m = _ctx->m;
    uint64_t *r = _ctx->r;
    uint64_t a = _ctx->a;
    uint64_t b = _ctx->b + (++_ctx->c);
    uint64_t x, y;
    int i;

    for (i = 0; i < ISAAC64_SZ / 2; i++) {
        x = m[i];
        a = ~(a ^ a << 21) + m[i + ISAAC64_SZ / 2];
        m[i] = y = m[lower_bits(x)] + a + b;
        r[i] = b = m[upper_bits(y)] + x;
        x = m[++i];
        a = (a ^ a >> 5) + m[i + ISAAC64_SZ / 2];
        m[i] = y = m[lower_bits(x)] + a + b;
        r[i] = b = m[upper_bits(y)] + x;
        x = m[++i];
        a = (a ^ a << 12) + m[i + ISAAC64_SZ / 2];
        m[i] = y = m[lower_bits(x)] + a + b;
        r[i] = b = m[upper_bits(y)] + x;
        x = m[++i];
        a = (a ^ a >> 33) + m[i + ISAAC64_SZ / 2];
        m[i] = y = m[lower_bits(x)] + a + b;
        r[i] = b = m[upper_bits(y)] + x;
    }
    for (i = ISAAC64_SZ / 2; i < ISAAC64_SZ; i++) {
        x = m[i];
        a = ~(a ^ a << 21) + m[i - ISAAC64_SZ / 2];
        m[i] = y = m[lower_bits(x)] + a + b;
        r[i] = b = m[upper_bits(y)] + x;
        x = m[++i];
        a = (a ^ a >> 5) + m[i - ISAAC64_SZ / 2];
        m[i] = y = m[lower_bits(x)] + a + b;
        r[i] = b = m[upper_bits(y)] + x;
        x = m[++i];
        a = (a ^ a << 12) + m[i - ISAAC64_SZ / 2];
        m[i] = y = m[lower_bits(x)] + a + b;
        r[i] = b = m[upper_bits(y)] + x;
        x = m[++i];
        a = (a ^ a >> 33) + m[i - ISAAC64_SZ / 2];
        m[i] = y = m[lower_bits(x)] + a + b;
        r[i] = b = m[upper_bits(y)] + x;
    }
    _ctx->b = b;
    _ctx->a = a;
    _ctx->n = ISAAC64_SZ;
}

static void isaac64_mix(uint64_t _x[8]) {
    static const unsigned char SHIFT[8] = {9, 9, 23, 15, 14, 20, 17, 14};
    int i;
    for (i = 0; i < 8; i++) {
        _x[i] -= _x[(i + 4) & 7];
        _x[(i + 5) & 7] ^= _x[(i + 7) & 7] >> SHIFT[i];
        _x[(i + 7) & 7] += _x[i];
        i++;
        _x[i] -= _x[(i + 4) & 7];
        _x[(i + 5) & 7] ^= _x[(i + 7) & 7] << SHIFT[i];
        _x[(i + 7) & 7] += _x[i];
    }
}

void isaac64_init(isaac64_ctx *_ctx, const unsigned char *_seed, int _nseed) {
    _ctx->a = _ctx->b = _ctx->c = 0;
    memset(_ctx->r, 0, sizeof(_ctx->r));
    isaac64_reseed(_ctx, _seed, _nseed);
}

void isaac64_reseed(isaac64_ctx *_ctx, const unsigned char *_seed, int _nseed) {
    uint64_t *m = _ctx->m;
    uint64_t *r = _ctx->r;
    uint64_t x[8];
    int i, j;

    if (_nseed > ISAAC64_SEED_SZ_MAX) _nseed = ISAAC64_SEED_SZ_MAX;
    for (i = 0; i < _nseed >> 3; i++) {
        r[i] ^= (uint64_t)_seed[i << 3 | 7] << 56 |
                 (uint64_t)_seed[i << 3 | 6] << 48 |
                 (uint64_t)_seed[i << 3 | 5] << 40 |
                 (uint64_t)_seed[i << 3 | 4] << 32 |
                 (uint64_t)_seed[i << 3 | 3] << 24 |
                 (uint64_t)_seed[i << 3 | 2] << 16 |
                 (uint64_t)_seed[i << 3 | 1] << 8 | _seed[i << 3];
    }
    _nseed -= i << 3;
    if (_nseed > 0) {
        uint64_t ri = _seed[i << 3];
        for (j = 1; j < _nseed; j++)
            ri |= (uint64_t)_seed[i << 3 | j] << (j << 3);
        r[i++] ^= ri;
    }
    x[0] = x[1] = x[2] = x[3] = x[4] = x[5] = x[6] = x[7] =
        (uint64_t)0x9E3779B97F4A7C13ULL;
    for (i = 0; i < 4; i++) isaac64_mix(x);
    for (i = 0; i < ISAAC64_SZ; i += 8) {
        for (j = 0; j < 8; j++) x[j] += r[i + j];
        isaac64_mix(x);
        memcpy(m + i, x, sizeof(x));
    }
    for (i = 0; i < ISAAC64_SZ; i += 8) {
        for (j = 0; j < 8; j++) x[j] += m[i + j];
        isaac64_mix(x);
        memcpy(m + i, x, sizeof(x));
    }
    isaac64_update(_ctx);
}

uint64_t isaac64_next_uint64(isaac64_ctx *_ctx) {
    if (!_ctx->n) isaac64_update(_ctx);
    return _ctx->r[--_ctx->n];
}

// NetHack's init_isaac64: converts unsigned long seed to LE bytes
static void init_like_nethack(isaac64_ctx *ctx, unsigned long seed) {
    unsigned char buf[sizeof(unsigned long)];
    unsigned i;
    for (i = 0; i < sizeof(seed); i++) {
        buf[i] = (unsigned char)(seed & 0xFF);
        seed >>= 8;
    }
    isaac64_init(ctx, buf, (int)sizeof(seed));
}

int main(void) {
    unsigned long seeds[] = {42, 0, 12345};
    int num_seeds = sizeof(seeds) / sizeof(seeds[0]);

    for (int s = 0; s < num_seeds; s++) {
        isaac64_ctx ctx;
        init_like_nethack(&ctx, seeds[s]);

        printf("=== seed %lu ===\n", seeds[s]);
        printf("raw u64 values:\n");

        // Generate fresh context for raw values
        isaac64_ctx raw_ctx;
        init_like_nethack(&raw_ctx, seeds[s]);
        for (int i = 0; i < 20; i++) {
            uint64_t val = isaac64_next_uint64(&raw_ctx);
            printf("  %llu\n", (unsigned long long)val);
        }

        // Generate fresh context for mod 100 values
        isaac64_ctx mod_ctx;
        init_like_nethack(&mod_ctx, seeds[s]);
        printf("mod 100 values (rn2(100) style):\n");
        for (int i = 0; i < 20; i++) {
            uint64_t val = isaac64_next_uint64(&mod_ctx);
            printf("  %llu\n", (unsigned long long)(val % 100));
        }
        printf("\n");
    }

    return 0;
}
