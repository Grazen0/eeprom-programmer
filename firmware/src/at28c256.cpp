#include "at28c256.hpp"
#include <Arduino.h>
#include <cstddef>
#include <span>

namespace
{
    constexpr u8 CHIP_ENABLE = 50;
    constexpr u8 OUTPUT_ENABLE = 51;
    constexpr u8 WRITE_ENABLE = 52;

    constexpr u8 DIR_INPUT = 0b00000000;
    constexpr u8 DIR_OUTPUT = 0b11111111;

    constexpr u16 PAGE_SIZE = 64;

    void set_address(u16 addr)
    {
        PORTC = addr & 0xFF;
        PORTA = (addr >> 8) & 0xFF;
    }

    void write_fast(u16 addr, u8 value)
    {
        set_address(addr);

        DDRL = DIR_OUTPUT;
        PORTL = value;

        digitalWrite(CHIP_ENABLE, LOW);
        digitalWrite(WRITE_ENABLE, LOW);
        digitalWrite(WRITE_ENABLE, HIGH);
        digitalWrite(CHIP_ENABLE, HIGH);
    }

    [[nodiscard]] bool wait_write_cycle(u16 last_addr, u8 last_value)
    {
        constexpr u32 POLL_TIMEOUT = 10'000;
        u32 count = 0;

        while ((at28c256::read(last_addr) & 0x80) != (last_value & 0x80)) {
            if (++count >= POLL_TIMEOUT)
                return false;
        }

        return true;
    }

} // namespace

namespace at28c256
{
    void setup()
    {
        DDRC = DIR_OUTPUT;
        DDRA = DIR_OUTPUT;

        digitalWrite(OUTPUT_ENABLE, HIGH);
        digitalWrite(WRITE_ENABLE, HIGH);
        digitalWrite(CHIP_ENABLE, HIGH);

        pinMode(CHIP_ENABLE, OUTPUT);
        pinMode(OUTPUT_ENABLE, OUTPUT);
        pinMode(WRITE_ENABLE, OUTPUT);

        delay(20);
    }

    u8 read(u16 addr)
    {
        DDRL = DIR_INPUT;
        set_address(addr);

        digitalWrite(CHIP_ENABLE, LOW);
        digitalWrite(OUTPUT_ENABLE, LOW);
        u8 value = PINL;
        digitalWrite(OUTPUT_ENABLE, HIGH);
        digitalWrite(CHIP_ENABLE, HIGH);

        return value;
    }

    bool write(u16 addr, u8 value)
    {
        write_fast(addr, value);
        return wait_write_cycle(addr, value);
    }

    bool write_many(u16 start, std::span<const u8> data)
    {
        if (data.empty())
            return true;

        for (size_t i = 0; i < data.size(); ++i) {
            u16 addr = start + i;
            write_fast(addr, data[i]);

            if (((addr + 1) % PAGE_SIZE) == 0 || i == data.size() - 1) {
                // reached a 64-byte (page) boundary,
                // wait for write cycle to finish
                if (!wait_write_cycle(addr, data[i]))
                    return false;
            }
        }

        return true;
    }

    void lock()
    {
        // can be anything according to datasheet
        constexpr u8 XX = 0xFF;
        constexpr u16 XX_ADDR = 0xFFFF;

        write_fast(0x5555, 0xAA);
        write_fast(0x2AAA, 0x55);
        write_fast(0x5555, 0xA0);
        write(XX_ADDR, XX);
    }

    void unlock()
    {
        // can be anything according to datasheet
        constexpr u8 XX = 0xFF;
        constexpr u16 XX_ADDR = 0xFFFF;

        write_fast(0x5555, 0xAA);
        write_fast(0x2AAA, 0x55);
        write_fast(0x5555, 0x80);
        write_fast(0x5555, 0xAA);
        write_fast(0x2AAA, 0x55);
        write_fast(0x5555, 0x20);
        write(XX_ADDR, XX);
    }

    void erase()
    {
        write_fast(0x5555, 0xAA);
        write_fast(0x2AAA, 0x55);
        write_fast(0x5555, 0x80);
        write_fast(0x5555, 0xAA);
        write_fast(0x2AAA, 0x55);
        write_fast(0x5555, 0x10);
        delay(25);
    }

} // namespace at28c256
