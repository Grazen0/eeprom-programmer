#ifndef EEPROM_PROGRAMMER_PROTOCOL_HPP
#define EEPROM_PROGRAMMER_PROTOCOL_HPP

#include "types.hpp"
#include <Arduino.h>
#include <concepts>
#include <expected>
#include <span>
#include <type_traits>
#include <utility>
#include <variant>

namespace protocol
{
    class HostMessage {
    public:
        struct Exit {};
        struct Read {
            u16 start;
            u8 len;

            Read(u16 start, u8 len);
        };
        struct Write {
            u16 start;
            std::span<const u8> data;

            Write(u16 start, std::span<const u8> data);
        };

        struct Lock {};
        struct Unlock {};
        struct Erase {};

    private:
        using Value = std::variant<Exit, Read, Write, Lock, Unlock, Erase>;
        Value value;

    public:
        template<typename T>
            requires(!std::same_as<std::remove_cvref_t<T>, HostMessage>)
        HostMessage(T &&value) // NOLINT(google-explicit-constructor)
            : value{std::forward<T>(value)}
        {
        }

        template<typename F>
        decltype(auto) visit(F &&f)
        {
            return std::visit(std::forward<F>(f), value);
        }

        template<typename F>
        decltype(auto) visit(F &&f) const
        {
            return std::visit(std::forward<F>(f), value);
        }
    };

    class DeviceMessage {
    public:
        struct Err {
            types::Error err;

            explicit Err(types::Error err);
        };
        struct Ready {};
        struct Ok {};
        struct Bytes {
            std::span<const u8> data;

            explicit Bytes(std::span<const u8> data);
        };

    private:
        using Value = std::variant<Err, Ready, Ok, Bytes>;
        Value value;

    public:
        template<typename T>
            requires(!std::same_as<std::remove_cvref_t<T>, DeviceMessage>)
        DeviceMessage(T &&value) // NOLINT(google-explicit-constructor)
            : value{std::forward<T>(value)}
        {
        }

        template<typename F>
        decltype(auto) visit(F &&f)
        {
            return std::visit(std::forward<F>(f), value);
        }

        template<typename F>
        decltype(auto) visit(F &&f) const
        {
            return std::visit(std::forward<F>(f), value);
        }
    };

    std::expected<HostMessage, types::Error> recv_message();

    void send_message(const DeviceMessage &message);

} // namespace protocol

#endif
