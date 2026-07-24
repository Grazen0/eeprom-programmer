#include "at28c256.hpp"
#include "cobs.hpp"
#include <Arduino.h>
#include <array>
#include <optional>
#include <span>

namespace
{
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
    std::optional<std::span<u8>> recv_packet(std::span<u8> buf)
    {
        size_t i = 0;
        u8 b = 0;

        while ((b = serial_read_u8()) != 0) {
            if (i == cobs_buf.size()) {
                // wait for rest of packet to arrive
                while (serial_read_u8() != 0) {
                }

                return std::nullopt;
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
        EXIT = 255,
    };

    enum class DevOp : u8 {
        ERR = 0,
        READY,
        OK,
        BYTES,
    };

    enum class DevError : u8 {
        PACKET_TOO_LONG = 0,
        PACKET_EMPTY,
        INVALID_OP,
        MALFORMED_MESSAGE,
        WRITE_TIMEOUT,
    };

    std::array<u8, SERIAL_RX_BUFFER_SIZE> packet_buf{};

    template<typename... Ts>
    constexpr auto make_packet(Ts... xs)
    {
        return std::array<const u8, sizeof...(Ts)>{static_cast<u8>(xs)...};
    }

    constexpr auto PACKET_READY = make_packet(DevOp::READY);
    constexpr auto PACKET_OK = make_packet(DevOp::OK);
    constexpr auto PACKET_ERR_INVALID_OP =
        make_packet(DevOp::ERR, DevError::INVALID_OP);
    constexpr auto PACKET_ERR_PACKET_EMPTY =
        make_packet(DevOp::ERR, DevError::PACKET_EMPTY);
    constexpr auto PACKET_ERR_PACKET_TOO_LONG =
        make_packet(DevOp::ERR, DevError::PACKET_TOO_LONG);
    constexpr auto PACKET_ERR_MALFORMED_MESSAGE =
        make_packet(DevOp::ERR, DevError::MALFORMED_MESSAGE);
    constexpr auto PACKET_ERR_WRITE_TIMEOUT =
        make_packet(DevOp::ERR, DevError::WRITE_TIMEOUT);

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

    void cmd_unlock(std::span<const u8> args, [[maybe_unused]] State &state)
    {
        if (!args.empty()) {
            send_packet(PACKET_ERR_MALFORMED_MESSAGE);
            return;
        }

        at28c256::unlock();

        delay(10);
        send_packet(PACKET_OK);
    }

    void cmd_lock(std::span<const u8> args, [[maybe_unused]] State &_state)
    {
        if (!args.empty()) {
            send_packet(PACKET_ERR_MALFORMED_MESSAGE);
            return;
        }

        at28c256::lock();

        delay(10);
        send_packet(PACKET_OK);
    }

    void cmd_write(std::span<const u8> args, [[maybe_unused]] State &state)
    {
        if (args.size() < 2) {
            send_packet(PACKET_ERR_MALFORMED_MESSAGE);
            return;
        }

        u16 start = concat_u16(args[0], args[1]);
        const u8 *data = &args[2];
        size_t data_len = args.size() - 2;

        constexpr size_t POLL_TIMEOUT = 1000;

        for (size_t i = 0; i < data_len; ++i) {
            u16 addr = start + i;
            at28c256::write_data(addr, data[i]);

            if ((addr & 0x3F) == 0x3F) {
                // reached a 64-byte (page) boundary,
                // wait for write cycle to finish
                size_t count = 0;

                while ((at28c256::read_data(addr) & 0x80) != (data[i] & 0x80)) {
                    if (++count >= POLL_TIMEOUT) {
                        send_packet(PACKET_ERR_WRITE_TIMEOUT);
                        return;
                    }
                }
            }
        }

        delay(10);
        send_packet(PACKET_OK);
    }

    std::array<u8, SERIAL_RX_BUFFER_SIZE> resp_buf;

    void cmd_read(std::span<const u8> args,
                  [[maybe_unused]] struct State &state)
    {
        if (args.size() != 3) {
            send_packet(PACKET_ERR_MALFORMED_MESSAGE);
            return;
        }

        u16 start =
            static_cast<u16>(args[0]) | (static_cast<u16>(args[1]) << 8);
        u8 data_len = args[2];

        size_t resp_len = data_len + 1;
        resp_buf[0] = static_cast<u8>(DevOp::BYTES);

        u8 *out_data = &resp_buf[1];

        for (size_t i = 0; i < data_len; ++i)
            out_data[i] = at28c256::read_data(start + i);

        delay(10);
        send_packet(std::span{resp_buf.data(), resp_len});
    }
} // namespace

void setup()
{
    Serial.begin(115200);

    at28c256::setup();
    at28c256::enable();

    send_packet(PACKET_READY);

    State state{};

    while (!state.exit) {
        auto packet = recv_packet(packet_buf);

        if (!packet) {
            send_packet(PACKET_ERR_PACKET_TOO_LONG);
            continue;
        }

        if (packet->size() == 0) {
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
            default:
                send_packet(PACKET_ERR_INVALID_OP);
                break;
        }
    }

    at28c256::disable();
}

void loop()
{
}
