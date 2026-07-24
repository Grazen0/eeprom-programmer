#ifndef EEPROM_PROGRAMMER_COBS_HPP
#define EEPROM_PROGRAMMER_COBS_HPP

#include <Arduino.h>

namespace cobs
{
    // Both implementations taken from
    // https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing#Implementation

    template<u8 DELIM>
    size_t encode(const u8 data[], size_t length, u8 buffer[])
    {
        u8 *encode = buffer;
        u8 *codep = encode++;
        u8 code = 1;

        for (const u8 *byte = data; length--; ++byte) {
            if (*byte != DELIM)
                *encode++ = *byte, ++code;

            if (*byte == DELIM || code == 0xFF) {
                *codep = code, code = 1, codep = encode;
                if (*byte == DELIM || length)
                    ++encode;
            }
        }

        *codep = code;
        return (size_t)(encode - buffer);
    }

    template<u8 DELIM>
    size_t decode(const u8 buffer[], size_t length, u8 data[])
    {
        const u8 *byte = buffer;
        u8 *decode = data;

        for (u8 code = 0xFF, block = 0; byte < buffer + length; --block) {
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

        return (size_t)(decode - data);
    }

#endif
}
