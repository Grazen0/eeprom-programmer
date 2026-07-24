#include "at28c256.hpp"
#include <Arduino.h>

namespace
{
    constexpr u8 CHIP_ENABLE = 50;
    constexpr u8 OUTPUT_ENABLE = 51;
    constexpr u8 WRITE_ENABLE = 52;

    void set_address(u16 addr)
    {
        PORTC = addr & 0xFF;
        PORTA = (addr >> 8) & 0xFF;
    }
} // namespace

namespace at28c256
{
    void setup()
    {
        DDRC = 0b11111111;
        DDRA = 0b11111111;

        digitalWrite(OUTPUT_ENABLE, HIGH);
        digitalWrite(WRITE_ENABLE, HIGH);
        digitalWrite(CHIP_ENABLE, HIGH);

        pinMode(CHIP_ENABLE, OUTPUT);
        pinMode(OUTPUT_ENABLE, OUTPUT);
        pinMode(WRITE_ENABLE, OUTPUT);

        delay(50);
    }

    void enable()
    {
        digitalWrite(CHIP_ENABLE, LOW);
        delay(100);
    }

    void disable()
    {
        digitalWrite(CHIP_ENABLE, HIGH);
    }

    u8 read_data(u16 addr)
    {
        DDRL = 0b00000000;
        set_address(addr);

        digitalWrite(OUTPUT_ENABLE, LOW);
        u8 value = PINL;
        digitalWrite(OUTPUT_ENABLE, HIGH);

        return value;
    }

    void write_data(u16 addr, u8 value)
    {
        DDRL = 0b11111111;
        set_address(addr);

        PORTL = value;

        digitalWrite(WRITE_ENABLE, LOW);
        digitalWrite(WRITE_ENABLE, HIGH);
    }

    void lock()
    {
        // can be anything according to datasheet
        constexpr u8 XX = 0x42;
        constexpr u16 XX_ADDR = 0x6767;

        write_data(0x5555, 0xAA);
        write_data(0x2AAA, 0x55);
        write_data(0x5555, 0xA0);
        write_data(XX_ADDR, XX);
        write_data(XX_ADDR, XX);
    }

    void unlock()
    {
        // can be anything according to datasheet
        constexpr u8 XX = 0x42;
        constexpr u16 XX_ADDR = 0x6767;

        write_data(0x5555, 0xAA);
        write_data(0x2AAA, 0x55);
        write_data(0x5555, 0x80);
        write_data(0x5555, 0xAA);
        write_data(0x2AAA, 0x55);
        write_data(0x5555, 0x20);
        write_data(XX_ADDR, XX);
        write_data(XX_ADDR, XX);
    }

    void erase()
    {
        write_data(0x5555, 0xAA);
        write_data(0x2AAA, 0x55);
        write_data(0x5555, 0x80);
        write_data(0x5555, 0xAA);
        write_data(0x2AAA, 0x55);
        write_data(0x5555, 0x10);
    }

} // namespace at28c256
