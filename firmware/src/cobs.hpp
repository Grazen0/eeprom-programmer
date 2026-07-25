#ifndef EEPROM_PROGRAMMER_COBS_HPP
#define EEPROM_PROGRAMMER_COBS_HPP

#include <Arduino.h>
#include <cstddef>
#include <span>

namespace cobs
{
    // Both implementations taken from
    // https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing#Implementation

    template<u8 DELIM>
    std::span<u8> encode(std::span<const u8> data, std::span<u8> out_buf)
    {
        std::size_t length = data.size();

        std::size_t encode = 0;
        std::size_t codep = encode++;
        u8 code = 1;

        for (const u8 *byte = data.data(); length--; ++byte) {
            if (*byte != DELIM) {
                out_buf.at(encode++) = *byte;
                ++code;
            }

            if (*byte == DELIM || code == 0xFF) {
                out_buf.at(codep) = code;
                code = 1;
                codep = encode;

                if (*byte == DELIM || length)
                    ++encode;
            }
        }

        out_buf.at(codep) = code;
        return std::span{out_buf.data(), encode};
    }

    template<u8 DELIM>
    std::span<u8> decode(std::span<const u8> data, std::span<u8> out_buf)
    {
        std::size_t byte = 0;
        std::size_t decode = 0;

        for (u8 code = 0xFF, block = 0; byte < data.size(); --block) {
            if (block) {
                out_buf.at(decode++) = data.at(byte++);
            } else {
                block = data.at(byte++);

                if (block && (code != 0xFF))
                    out_buf.at(decode++) = 0;

                code = block;
                if (code == DELIM)
                    break;
            }
        }

        return std::span{out_buf.data(), decode};
    }

} // namespace cobs

#endif
