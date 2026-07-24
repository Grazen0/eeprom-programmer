#include "at28c256.hpp"
#include "cobs.hpp"
#include <Arduino.h>
#include <array>
#include <expected>
#include <span>

namespace
{
    enum class DeviceError : u8 {
        PACKET_TOO_LONG = 0,
        PACKET_EMPTY = 1,
        INVALID_OP = 2,
        MALFORMED_MESSAGE = 3,
        WRITE_TIMEOUT = 4,
    };

    constexpr u16 concat_u16(u8 lo, u8 hi)
    {
        return static_cast<u16>(lo) | (static_cast<u16>(hi) << 8);
    }

    u8 serial_read_u8()
    {
        while (Serial.available() < 1) {
        }

        return Serial.read();
    }

    constexpr u8 PACKET_DELIM = 0;

    std::array<u8, SERIAL_RX_BUFFER_SIZE> cobs_buf;

    // returns std::nullopt if received packet was too long
    std::expected<std::span<u8>, DeviceError> recv_packet(std::span<u8> buf)
    {
        size_t i = 0;
        u8 b = 0;

        while ((b = serial_read_u8()) != PACKET_DELIM) {
            if (i == cobs_buf.size()) {
                // wait for rest of packet to arrive
                while (serial_read_u8() != PACKET_DELIM) {
                }

                return std::unexpected{DeviceError::PACKET_TOO_LONG};
            }

            cobs_buf.at(i++) = b;
        }

        return cobs::decode<PACKET_DELIM>(std::span{cobs_buf.data(), i}, buf);
    }

    void send_packet(std::span<const u8> packet)
    {
        std::span<u8> encoded = cobs::encode<PACKET_DELIM>(packet, cobs_buf);

        Serial.write(encoded.data(), encoded.size());
        Serial.write(PACKET_DELIM);
    }

    enum class HostOp : u8 {
        READ = 0,
        WRITE = 1,
        LOCK = 2,
        UNLOCK = 3,
        ERASE = 4,
        EXIT = 255,
    };

    enum class DeviceOp : u8 {
        ERR = 0,
        READY = 1,
        OK = 2,
        BYTES = 3,
    };

    template<typename... Ts>
    constexpr auto make_packet(Ts... xs)
    {
        return std::array<const u8, sizeof...(Ts)>{static_cast<u8>(xs)...};
    }

    constexpr auto PACKET_READY = make_packet(DeviceOp::READY);
    constexpr auto PACKET_OK = make_packet(DeviceOp::OK);
    constexpr auto PACKET_ERR_INVALID_OP =
        make_packet(DeviceOp::ERR, DeviceError::INVALID_OP);
    constexpr auto PACKET_ERR_PACKET_EMPTY =
        make_packet(DeviceOp::ERR, DeviceError::PACKET_EMPTY);
    constexpr auto PACKET_ERR_MALFORMED_MESSAGE =
        make_packet(DeviceOp::ERR, DeviceError::MALFORMED_MESSAGE);
    constexpr auto PACKET_ERR_WRITE_TIMEOUT =
        make_packet(DeviceOp::ERR, DeviceError::WRITE_TIMEOUT);

    struct State {
        bool exit = false;
    };

    void cmd_exit(std::span<const u8> args, State &state)
    {
        if (!args.empty()) {
            send_packet(PACKET_ERR_MALFORMED_MESSAGE);
            return;
        }

        state.exit = true;
        send_packet(PACKET_OK);
    }

    void cmd_read(std::span<const u8> args, [[maybe_unused]] State &state)
    {
        if (args.size() != 3) {
            send_packet(PACKET_ERR_MALFORMED_MESSAGE);
            return;
        }

        u16 start = concat_u16(args[0], args[1]);
        size_t data_len = args[2];

        static std::array<u8, SERIAL_RX_BUFFER_SIZE> resp_buf;

        std::span<u8> resp{resp_buf.data(), 1 + data_len};
        resp[0] = static_cast<u8>(DeviceOp::BYTES);

        std::span<u8> resp_data = resp.subspan<1>();

        for (size_t i = 0; i < data_len; ++i)
            resp_data[i] = at28c256::read(start + i);

        delay(10);
        send_packet(resp);
    }

    void cmd_write(std::span<const u8> args, [[maybe_unused]] State &state)
    {
        if (args.size() < 2) {
            send_packet(PACKET_ERR_MALFORMED_MESSAGE);
            return;
        }

        u16 start = concat_u16(args[0], args[1]);
        auto data = args.subspan<2>();

        if (!at28c256::write_many(start, data)) {
            send_packet(PACKET_ERR_WRITE_TIMEOUT);
            return;
        }

        send_packet(PACKET_OK);
    }

    void cmd_lock(std::span<const u8> args, [[maybe_unused]] State &_state)
    {
        if (!args.empty()) {
            send_packet(PACKET_ERR_MALFORMED_MESSAGE);
            return;
        }

        at28c256::lock();
        send_packet(PACKET_OK);
    }

    void cmd_unlock(std::span<const u8> args, [[maybe_unused]] State &state)
    {
        if (!args.empty()) {
            send_packet(PACKET_ERR_MALFORMED_MESSAGE);
            return;
        }

        at28c256::unlock();
        send_packet(PACKET_OK);
    }

    void cmd_erase(std::span<const u8> args, [[maybe_unused]] State &state)
    {
        if (!args.empty()) {
            send_packet(PACKET_ERR_MALFORMED_MESSAGE);
            return;
        }

        at28c256::erase();

        delay(20);
        send_packet(PACKET_OK);
    }
} // namespace

void setup()
{
    Serial.begin(115200);

    at28c256::setup();

    send_packet(PACKET_READY);

    State state{};

    while (!state.exit) {
        static std::array<u8, SERIAL_RX_BUFFER_SIZE> packet_buf;
        auto packet = recv_packet(packet_buf);

        if (!packet) {
            std::array resp = make_packet(DeviceOp::ERR, packet.error());
            send_packet(resp);
            continue;
        }

        if (packet->empty()) {
            send_packet(PACKET_ERR_PACKET_EMPTY);
            continue;
        }

        auto op = static_cast<HostOp>(packet->at(0));
        std::span args = packet->subspan<1>();

        switch (op) {
            case HostOp::EXIT:
                cmd_exit(args, state);
                break;
            case HostOp::READ:
                cmd_read(args, state);
                break;
            case HostOp::WRITE:
                cmd_write(args, state);
                break;
            case HostOp::LOCK:
                cmd_lock(args, state);
                break;
            case HostOp::UNLOCK:
                cmd_unlock(args, state);
                break;
            case HostOp::ERASE:
                cmd_erase(args, state);
                break;
            default:
                send_packet(PACKET_ERR_INVALID_OP);
                break;
        }
    }
}

void loop()
{
}
