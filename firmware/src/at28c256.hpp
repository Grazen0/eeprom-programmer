#ifndef EEPROM_PROGRAMMER_AT28C256_HPP
#define EEPROM_PROGRAMMER_AT28C256_HPP

#include <Arduino.h>

namespace at28c256
{
    void setup();

    void enable();

    void disable();

    u8 read_data(u16 addr);

    void write_data(u16 addr, u8 value);

    void write_data_careful(u16 addr, u8 value);

    void lock();

    void unlock();

    void erase();

} // namespace at28c256

#endif
