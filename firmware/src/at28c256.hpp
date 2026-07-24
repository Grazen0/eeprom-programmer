#ifndef EEPROM_PROGRAMMER_AT28C256_HPP
#define EEPROM_PROGRAMMER_AT28C256_HPP

#include <Arduino.h>
#include <span>

namespace at28c256
{
    void setup();

    u8 read(u16 addr);

    bool write(u16 addr, u8 value);

    bool write_many(u16 start, std::span<const u8> data);

    void lock();

    void unlock();

    void erase();

} // namespace at28c256

#endif
