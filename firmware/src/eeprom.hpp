#ifndef EEPROM_PROGRAMMER_EEPROM_HPP
#define EEPROM_PROGRAMMER_EEPROM_HPP

#include <Arduino.h>

namespace eeprom
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

} // namespace eeprom

#endif
