#ifndef EEPROM_PROGRAMMER_COBS_HPP
#define EEPROM_PROGRAMMER_COBS_HPP

#include <Arduino.h>
#include <span>

namespace cobs
{
    // Both implementations taken from
    // https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing#Implementation

    template<u8 DELIM>
    size_t encode(std::span<const u8> data, std::span<u8> buffer)
    {
        size_t length = data.size();

        u8 *encode = buffer.data();
        u8 *codep = encode++;
        u8 code = 1;

        for (const u8 *byte = data.data(); length--; ++byte) {
            if (*byte != DELIM)
                *encode++ = *byte, ++code;

            if (*byte == DELIM || code == 0xFF) {
                *codep = code, code = 1, codep = encode;
                if (*byte == DELIM || length)
                    ++encode;
            }
        }

        *codep = code;
        return static_cast<size_t>(encode - buffer.data());
    }

    template<u8 DELIM>
    size_t decode(std::span<const u8> buffer, std::span<u8> data)
    {
        auto byte = buffer.begin();
        auto decode = data.begin();

        for (u8 code = 0xFF, block = 0; byte < buffer.end(); --block) {
            if (block) {
                *decode++ = *byte++;
            } else {
                block = *byte++;
                if (block && (code != 0xFF))
                    *decode++ = 0;

                code = block;
                if (code == DELIM)
                    break;
            }
        }

        return decode - data.begin();
    }

} // namespace cobs

#endif
