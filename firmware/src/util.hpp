#ifndef EEPROM_PROGRAMMER_UTIL_HPP
#define EEPROM_PROGRAMMER_UTIL_HPP

#include <Arduino.h>

#define TRY(expr)                                \
    ({                                           \
        auto &&res = (expr);                     \
        if (!res)                                \
            return std::unexpected(res.error()); \
                                                 \
        std::move(*res);                         \
    })

namespace util
{
    constexpr u16 concat_u16(u8 lo, u8 hi)
    {
        return static_cast<u16>(lo) | (static_cast<u16>(hi) << 8);
    }
} // namespace util

#endif
