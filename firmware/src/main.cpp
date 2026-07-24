#include "cobs.hpp"
#include "eeprom.hpp"
#include <Arduino.h>

#define countof(ARR) (sizeof(ARR) / sizeof((ARR)[0]))

static u8 serial_read_u8()
{
    while (Serial.available() < 1) {
    }

    return Serial.read();
}

static constexpr u8 PACKET_DELIM = 0;

static constexpr size_t COBS_BUF_CAPACITY = SERIAL_RX_BUFFER_SIZE;
static u8 cobs_buf[COBS_BUF_CAPACITY] = {};

static bool recv_packet(u8 packet[], size_t &packet_len)
{
    size_t i = 0;
    u8 b = 0;

    while ((b = serial_read_u8()) != 0) {
        if (i == COBS_BUF_CAPACITY) {
            // wait for rest of packet to arrive
            while (serial_read_u8() != 0) {
            }

            return false;
        }

        cobs_buf[i++] = b;
    }

    packet_len = cobs::decode<PACKET_DELIM>(cobs_buf, i, packet);
    return true;
}

static void send_packet(const u8 data[], size_t n)
{
    size_t n_encoded = cobs::encode<PACKET_DELIM>(data, n, cobs_buf);

    Serial.write(cobs_buf, n_encoded);
    Serial.write(PACKET_DELIM);
}

enum HostOp : u8 {
    HOST_OP_EXIT = 0,
    HOST_OP_READ,
    HOST_OP_WRITE,
    HOST_OP_LOCK,
    HOST_OP_UNLOCK,
};

enum DevOp : u8 {
    DEV_OP_ERR = 0,
    DEV_OP_READY,
    DEV_OP_OK,
    DEV_OP_BYTES,
};

enum DevError : u8 {
    ERR_PACKET_TOO_LONG = 0,
    ERR_PACKET_EMPTY,
    ERR_INVALID_OP,
    ERR_MALFORMED_MESSAGE,
    ERR_WRITE_TIMEOUT,
};

static constexpr size_t PACKET_BUF_CAPACITY = SERIAL_RX_BUFFER_SIZE;
static u8 packet_buf[PACKET_BUF_CAPACITY] = {};

static const u8 PACKET_READY[] = {DEV_OP_READY};
static const u8 PACKET_OK[] = {DEV_OP_OK};
static const u8 PACKET_ERR_INVALID_OP[] = {DEV_OP_ERR, ERR_INVALID_OP};
static const u8 PACKET_ERR_PACKET_EMPTY[] = {DEV_OP_ERR, ERR_PACKET_EMPTY};
static const u8 PACKET_ERR_PACKET_TOO_LONG[] = {DEV_OP_ERR,
                                                ERR_PACKET_TOO_LONG};
static const u8 PACKET_ERR_MALFORMED_MESSAGE[] = {DEV_OP_ERR,
                                                  ERR_MALFORMED_MESSAGE};
static const u8 PACKET_ERR_WRITE_TIMEOUT[] = {DEV_OP_ERR, ERR_WRITE_TIMEOUT};

template<size_t N>
static inline void send_packet_const(const u8 (&packet)[N])
{
    send_packet(packet, N);
}

struct State {
    bool exit;
};

static void cmd_exit(const u8 args[], size_t args_len, State &state)
{
    if (args_len != 0) {
        send_packet_const(PACKET_ERR_MALFORMED_MESSAGE);
        return;
    }

    state.exit = true;
    send_packet_const(PACKET_OK);
}

static void cmd_unlock(const u8 args[], size_t args_len, struct State &state)
{
    if (args_len != 0) {
        send_packet_const(PACKET_ERR_MALFORMED_MESSAGE);
        return;
    }

    eeprom::unlock();

    delay(10);
    send_packet_const(PACKET_OK);
}

static void cmd_lock(const u8 args[], size_t args_len, struct State &state)
{
    if (args_len != 0) {
        send_packet_const(PACKET_ERR_MALFORMED_MESSAGE);
        return;
    }

    eeprom::lock();

    delay(10);
    send_packet_const(PACKET_OK);
}

static void cmd_write(const u8 args[], size_t args_len, struct State &state)
{
    if (args_len < 2) {
        send_packet_const(PACKET_ERR_MALFORMED_MESSAGE);
        return;
    }

    u16 start = (u16)args[0] | ((u16)args[1] << 8);
    const u8 *data = &args[2];
    size_t data_len = args_len - 2;

    static constexpr size_t POLL_TIMEOUT = 1000;

    for (size_t i = 0; i < data_len; ++i) {
        u16 addr = start + i;
        eeprom::write_data(addr, data[i]);

        if ((addr & 0x3F) == 0x3F) {
            // reached a 64-byte (page) boundary,
            // wait for write cycle to finish
            size_t count = 0;

            while ((eeprom::read_data(addr) & 0x80) != (data[i] & 0x80)) {
                if (++count >= POLL_TIMEOUT) {
                    send_packet_const(PACKET_ERR_WRITE_TIMEOUT);
                    return;
                }
            }
        }
    }

    delay(10);
    send_packet_const(PACKET_OK);
}

static constexpr size_t RESP_BUF_CAPACITY = SERIAL_RX_BUFFER_SIZE;
static u8 resp_buf[RESP_BUF_CAPACITY];

static void cmd_read(const u8 args[], size_t args_len, struct State &state)
{
    if (args_len != 3) {
        send_packet_const(PACKET_ERR_MALFORMED_MESSAGE);
        return;
    }

    u16 start = (u16)args[0] | ((u16)args[1] << 8);
    u8 data_len = args[2];

    size_t resp_len = data_len + 1;
    resp_buf[0] = DEV_OP_BYTES;

    u8 *out_data = &resp_buf[1];

    for (size_t i = 0; i < data_len; ++i)
        out_data[i] = eeprom::read_data(start + i);

    delay(10);
    send_packet(resp_buf, resp_len);
}

using CommandHandler = void (*)(const u8 args[], size_t args_len, State &state);

static const CommandHandler CMD_HANDLERS[] = {
    [HOST_OP_EXIT] = cmd_exit,     [HOST_OP_READ] = cmd_read,
    [HOST_OP_WRITE] = cmd_write,   [HOST_OP_LOCK] = cmd_lock,
    [HOST_OP_UNLOCK] = cmd_unlock,
};

void setup()
{
    Serial.begin(115200);

    eeprom::setup();
    eeprom::enable();

    send_packet_const(PACKET_READY);

    State state{};

    while (!state.exit) {
        size_t packet_len = 0;

        if (!recv_packet(packet_buf, packet_len)) {
            send_packet_const(PACKET_ERR_PACKET_TOO_LONG);
            continue;
        }

        if (packet_len == 0) {
            send_packet_const(PACKET_ERR_PACKET_EMPTY);
            continue;
        }

        u8 op = packet_buf[0];
        u8 *args = &packet_buf[1];
        size_t args_len = packet_len - 1;

        if (op >= countof(CMD_HANDLERS) || CMD_HANDLERS[op] == nullptr) {
            send_packet_const(PACKET_ERR_INVALID_OP);
            return;
        }

        CMD_HANDLERS[op](args, args_len, state);
    }

    eeprom::disable();
}

void loop()
{
}
