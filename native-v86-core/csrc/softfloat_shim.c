#include <stdint.h>

struct uint128 { uint64_t v0, v64; };
struct uint128_extra { uint64_t extra; struct uint128 v; };

struct uint128_extra
softfloat_shiftRightJam128Extra(uint64_t a64, uint64_t a0, uint64_t extra, uint_fast32_t dist)
{
    struct uint128_extra z;
    if (dist < 64) {
        z.v.v64 = a64 >> dist;
        z.v.v0 = (a64 << ((-dist) & 63)) | (a0 >> dist);
        z.extra = a0 << ((-dist) & 63);
    } else {
        z.v.v64 = 0;
        if (dist == 64) {
            z.v.v0 = a64;
            z.extra = a0;
        } else {
            extra |= a0;
            if (dist < 128) {
                z.v.v0 = a64 >> (dist & 63);
                z.extra = a64 << ((-dist) & 63);
            } else {
                z.v.v0 = 0;
                z.extra = (dist == 128) ? a64 : (a64 != 0);
            }
        }
    }
    z.extra |= (extra != 0);
    return z;
}

void
softfloat_shiftRightJam256M(const uint64_t *aPtr, uint_fast32_t dist, uint64_t *zPtr)
{
    uint64_t value[4] = { aPtr[0], aPtr[1], aPtr[2], aPtr[3] };
    uint_fast32_t word_dist = dist >> 6;
    uint_fast32_t bit_dist = dist & 63;
    uint64_t jam = 0;

    for (unsigned i = 0; i < 4; ++i) zPtr[i] = 0;
    if (word_dist >= 4) {
        for (unsigned i = 0; i < 4; ++i) jam |= value[i];
        zPtr[0] = jam != 0;
        return;
    }
    for (unsigned out = 0; out < 4 - word_dist; ++out) {
        unsigned src = out + word_dist;
        uint64_t word = value[src] >> bit_dist;
        if (bit_dist && src + 1 < 4) word |= value[src + 1] << (64 - bit_dist);
        zPtr[out] = word;
    }
    for (unsigned i = 4 - word_dist; i < 4; ++i) jam |= value[i];
    if (bit_dist && word_dist < 4) jam |= value[3 - word_dist] & ((((uint64_t)1) << bit_dist) - 1);
    zPtr[0] |= (jam != 0);
}
