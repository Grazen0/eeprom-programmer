#ifndef EEPROM_PROGRAMMER_TYPES_HPP
#define EEPROM_PROGRAMMER_TYPES_HPP

#include <Arduino.h>

namespace types
{
    enum class Error : u8 {
        PACKET_TOO_LONG = 0,
        PACKET_EMPTY = 1,
        INVALID_OP = 2,
        MALFORMED_MESSAGE = 3,
        WRITE_TIMEOUT = 4,
    };
} // namespace types

#endif
