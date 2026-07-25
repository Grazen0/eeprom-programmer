#ifndef EEPROM_PROGRAMMER_PROTOCOL_HPP
#define EEPROM_PROGRAMMER_PROTOCOL_HPP

#include "types.hpp"
#include <Arduino.h>
#include <expected>
#include <span>
#include <variant>

namespace protocol
{
    struct HostMessageExit {};
    struct HostMessageRead {
        u16 start;
        u8 len;

        HostMessageRead(u16 start, u8 len);
    };
    struct HostMessageWrite {
        u16 start;
        std::span<const u8> data;

        HostMessageWrite(u16 start, std::span<const u8> data);
    };

    struct HostMessageLock {};
    struct HostMessageUnlock {};
    struct HostMessageErase {};

    using HostMessage =
        std::variant<HostMessageExit, HostMessageRead, HostMessageWrite,
                     HostMessageLock, HostMessageUnlock, HostMessageErase>;

    struct DeviceMessageErr {
        types::Error err;

        explicit DeviceMessageErr(types::Error err);
    };
    struct DeviceMessageReady {};
    struct DeviceMessageOk {};
    struct DeviceMessageBytes {
        std::span<const u8> data;

        explicit DeviceMessageBytes(std::span<const u8> data);
    };

    using DeviceMessage = std::variant<DeviceMessageErr, DeviceMessageReady,
                                       DeviceMessageOk, DeviceMessageBytes>;

    std::expected<HostMessage, types::Error> recv_message();

    void send_message(const DeviceMessage &message);

} // namespace protocol

#endif
