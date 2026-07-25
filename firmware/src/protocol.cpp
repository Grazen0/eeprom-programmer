#include "protocol.hpp"
#include "cobs.hpp"
#include "types.hpp"
#include "util.hpp"
#include <Arduino.h>
#include <cstddef>
#include <span>
#include <variant>

using types::Error;

namespace
{
    using protocol::DeviceMessage;
    using protocol::DeviceMessageBytes;
    using protocol::DeviceMessageErr;
    using protocol::DeviceMessageOk;
    using protocol::DeviceMessageReady;
    using protocol::HostMessage;
    using protocol::HostMessageErase;
    using protocol::HostMessageExit;
    using protocol::HostMessageLock;
    using protocol::HostMessageRead;
    using protocol::HostMessageUnlock;
    using protocol::HostMessageWrite;

    constexpr u8 PACKET_DELIM = 0;

    u8 serial_read_u8()
    {
        while (Serial.available() < 1) {
        }

        return Serial.read();
    }

    std::array<u8, SERIAL_RX_BUFFER_SIZE> encoded_buf;

    std::expected<std::span<u8>, Error> recv_packet(std::span<u8> buf)
    {
        size_t i = 0;
        u8 b = 0;

        while ((b = serial_read_u8()) != PACKET_DELIM) {
            if (i == encoded_buf.size()) {
                // wait for rest of packet to arrive
                while (serial_read_u8() != PACKET_DELIM) {
                }

                return std::unexpected{Error::PACKET_TOO_LONG};
            }

            encoded_buf.at(i++) = b;
        }

        return cobs::decode<PACKET_DELIM>(std::span{encoded_buf.data(), i},
                                          buf);
    }

    void send_packet(std::span<const u8> packet)
    {
        std::span<u8> encoded = cobs::encode<PACKET_DELIM>(packet, encoded_buf);

        Serial.write(encoded.data(), encoded.size());
        Serial.write(PACKET_DELIM);
    }

    std::expected<HostMessage, Error>
    deserialize_host_message(std::span<const u8> packet)
    {
        enum class HostOp : u8 {
            READ = 0,
            WRITE = 1,
            LOCK = 2,
            UNLOCK = 3,
            ERASE = 4,
            EXIT = 255,
        };

        if (packet.empty())
            return std::unexpected{Error::PACKET_EMPTY};

        auto op = static_cast<HostOp>(packet[0]);
        auto rest = packet.subspan<1>();

        switch (op) {
            case HostOp::EXIT:
                if (!rest.empty())
                    return std::unexpected{Error::MALFORMED_MESSAGE};

                return HostMessageExit{};

            case HostOp::READ: {
                if (rest.size() != 3)
                    return std::unexpected{Error::MALFORMED_MESSAGE};

                u16 start = util::concat_u16(rest[0], rest[1]);
                u8 len = rest[2];

                return HostMessageRead{start, len};
            }
            case HostOp::WRITE: {
                if (rest.size() < 2)
                    return std::unexpected{Error::MALFORMED_MESSAGE};

                u16 start = util::concat_u16(rest[0], rest[1]);
                auto data = rest.subspan<2>();

                return HostMessageWrite{start, data};
            }
            case HostOp::LOCK:
                if (!rest.empty())
                    return std::unexpected{Error::MALFORMED_MESSAGE};

                return HostMessageLock{};

            case HostOp::UNLOCK:
                if (!rest.empty())
                    return std::unexpected{Error::MALFORMED_MESSAGE};

                return HostMessageUnlock{};

            case HostOp::ERASE:
                if (!rest.empty())
                    return std::unexpected{Error::MALFORMED_MESSAGE};

                return HostMessageErase{};

            default:
                return std::unexpected{Error::INVALID_OP};
        }
    }

    enum class DeviceOp : u8 {
        ERR = 0,
        READY = 1,
        OK = 2,
        BYTES = 3,
    };

    std::span<u8> serialize(const DeviceMessageErr &message, std::span<u8> buf)
    {
        auto packet = buf.subspan<0, 2>();

        buf.at(0) = static_cast<u8>(DeviceOp::ERR);
        buf.at(1) = static_cast<u8>(message.err);

        return packet;
    }

    std::span<u8> serialize([[maybe_unused]] const DeviceMessageReady &message,
                            std::span<u8> buf)
    {
        auto packet = buf.subspan<0, 1>();
        buf.at(0) = static_cast<u8>(DeviceOp::READY);
        return packet;
    }

    std::span<u8> serialize([[maybe_unused]] const DeviceMessageOk &message,
                            std::span<u8> buf)
    {
        auto packet = buf.subspan<0, 1>();
        buf.at(0) = static_cast<u8>(DeviceOp::OK);
        return packet;
    }

    std::span<u8> serialize(const DeviceMessageBytes &message,
                            std::span<u8> buf)
    {
        auto packet = buf.subspan(0, 1 + message.data.size());

        buf.at(0) = static_cast<u8>(DeviceOp::BYTES);

        for (size_t i = 0; i < message.data.size(); ++i)
            packet.at(1 + i) = message.data[i];

        return packet;
    }

    std::span<u8> serialize(const DeviceMessage &message, std::span<u8> buf)
    {
        auto visitor = [&](const auto &m) { return serialize(m, buf); };
        return std::visit(visitor, message);
    }

    std::array<u8, SERIAL_RX_BUFFER_SIZE> message_buf;
} // namespace

namespace protocol
{
    HostMessageRead::HostMessageRead(u16 start, u8 len)
        : start{start},
          len{len}
    {
    }

    HostMessageWrite::HostMessageWrite(u16 start, std::span<const u8> data)
        : start{start},
          data{data}
    {
    }

    DeviceMessageErr::DeviceMessageErr(Error err)
        : err{err}
    {
    }

    DeviceMessageBytes::DeviceMessageBytes(std::span<const u8> data)
        : data{data}
    {
    }

    std::expected<HostMessage, Error> recv_message()
    {
        auto packet = TRY(recv_packet(message_buf));
        return deserialize_host_message(packet);
    }

    void send_message(const DeviceMessage &message)
    {
        auto packet = serialize(message, message_buf);
        send_packet(packet);
    }

} // namespace protocol
