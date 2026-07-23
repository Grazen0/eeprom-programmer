static inline void serial_wait_available(size_t count)
{
    while (Serial.available() < count) {
    }
}

static inline u16 serial_read_u16()
{
    serial_wait_available(2);

    u8 buf[2];
    Serial.readBytes(buf, 2);
    return ((u16)buf[1] << 8) | (u16)buf[0];
}

static inline u8 serial_read_u8()
{
    serial_wait_available(1);
    return Serial.read();
}

static inline void serial_write_u16(u16 value)
{
    Serial.write(value & 0xFF);
    Serial.write((value >> 8) & 0xFF);
}

static constexpr u8 PACKET_DELIM = 0;

// Source:
// https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing#Implementation
static size_t cobs_encode(const u8 data[], size_t length, u8 buffer[])
{
    u8 *encode = buffer;
    u8 *codep = encode++;
    u8 code = 1;

    for (const u8 *byte = data; length--; ++byte) {
        if (*byte != PACKET_DELIM)
            *encode++ = *byte, ++code;

        if (*byte == PACKET_DELIM || code == 0xFF) {
            *codep = code, code = 1, codep = encode;
            if (*byte == PACKET_DELIM || length)
                ++encode;
        }
    }
    *codep = code;

    return (size_t)(encode - buffer);
}

// Source:
// https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing#Implementation
static size_t cobs_decode(const u8 buffer[], size_t length, u8 data[])
{
    const u8 *byte = buffer;
    u8 *decode = data;

    for (u8 code = 0xFF, block = 0; byte < buffer + length; --block) {
        if (block) {
            *decode++ = *byte++;
        } else {
            block = *byte++;
            if (block && (code != 0xFF))
                *decode++ = 0;

            code = block;
            if (code == PACKET_DELIM)
                break;
        }
    }

    return (size_t)(decode - data);
}

static constexpr u8 CHIP_ENABLE = 50;
static constexpr u8 OUTPUT_ENABLE = 51;
static constexpr u8 WRITE_ENABLE = 52;

static void set_address(u16 addr)
{
    PORTC = addr & 0xFF;
    PORTA = (addr >> 8) & 0xFF;
}

static u8 read_data(u16 addr)
{
    DDRL = B00000000;
    set_address(addr);

    digitalWrite(OUTPUT_ENABLE, LOW);
    u8 value = PINL;
    digitalWrite(OUTPUT_ENABLE, HIGH);

    return value;
}

static void write_data(u16 addr, u8 value)
{
    DDRL = B11111111;
    set_address(addr);

    PORTL = value;

    digitalWrite(WRITE_ENABLE, LOW);
    digitalWrite(WRITE_ENABLE, HIGH);
}

static void write_data_careful(u16 addr, u8 value)
{
    DDRL = B11111111;
    set_address(addr);

    PORTL = value;

    digitalWrite(WRITE_ENABLE, LOW);
    delay(10);
    digitalWrite(WRITE_ENABLE, HIGH);
    delayMicroseconds(50);
}

static void lock_eeprom()
{
    write_data(0x5555, 0xAA);
    write_data(0x2AAA, 0x55);
    write_data(0x5555, 0xA0);
}

static void unlock_eeprom()
{
    write_data(0x5555, 0xAA);
    write_data(0x2AAA, 0x55);
    write_data(0x5555, 0x80);
    write_data(0x5555, 0xAA);
    write_data(0x2AAA, 0x55);
    write_data(0x5555, 0x20);
}

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

    packet_len = cobs_decode(cobs_buf, i, packet);
    return true;
}

static void send_packet(const u8 data[], size_t n)
{
    size_t n_encoded = cobs_encode(data, n, cobs_buf);

    Serial.write(cobs_buf, n_encoded);
    Serial.write(PACKET_DELIM);
}

enum HostOp : u8 {
    HOST_OP_READ = 0,
    HOST_OP_WRITE,
    HOST_OP_LOCK,
    HOST_OP_UNLOCK,
    HOST_OP_EXIT = 255,
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
};

static constexpr size_t PACKET_CAPACITY = SERIAL_RX_BUFFER_SIZE;
static u8 packet[PACKET_CAPACITY] = {};

static const u8 PACKET_READY[] = {DEV_OP_READY};
static const u8 PACKET_OK[] = {DEV_OP_OK};
static const u8 PACKET_ERR_INVALID_OP[] = {DEV_OP_ERR, ERR_INVALID_OP};
static const u8 PACKET_ERR_PACKET_EMPTY[] = {DEV_OP_ERR, ERR_PACKET_EMPTY};
static const u8 PACKET_ERR_PACKET_TOO_LONG[] = {DEV_OP_ERR,
                                                ERR_PACKET_TOO_LONG};
static const u8 PACKET_ERR_MALFORMED_MESSAGE[] = {DEV_OP_ERR,
                                                  ERR_MALFORMED_MESSAGE};

template <size_t N>
static inline void send_packet_const(const u8 (&packet)[N])
{
    send_packet(packet, N);
}

void setup()
{
    Serial.begin(115200);
    DDRC = B11111111;
    DDRA = B11111111;

    digitalWrite(OUTPUT_ENABLE, HIGH);
    digitalWrite(WRITE_ENABLE, HIGH);
    digitalWrite(CHIP_ENABLE, HIGH);

    pinMode(CHIP_ENABLE, OUTPUT);
    pinMode(OUTPUT_ENABLE, OUTPUT);
    pinMode(WRITE_ENABLE, OUTPUT);

    delay(50);
    digitalWrite(CHIP_ENABLE, LOW);
    delay(200);

    send_packet_const(PACKET_READY);

    bool exit = false;

    while (!exit) {
        size_t packet_len = 0;

        if (!recv_packet(packet, packet_len)) {
            send_packet_const(PACKET_ERR_PACKET_TOO_LONG);
            continue;
        }

        if (packet_len == 0) {
            send_packet_const(PACKET_ERR_PACKET_EMPTY);
            continue;
        }

        u8 op = packet[0];
        u8 *args = &packet[1];
        size_t args_len = packet_len - 1;

        switch (op) {
            case HOST_OP_EXIT:
                if (args_len != 0) {
                    send_packet_const(PACKET_ERR_MALFORMED_MESSAGE);
                    continue;
                }

                exit = true;
                send_packet_const(PACKET_OK);
                break;

            case HOST_OP_UNLOCK:
                if (args_len != 0) {
                    send_packet_const(PACKET_ERR_MALFORMED_MESSAGE);
                    continue;
                }

                unlock_eeprom();

                delay(20);
                send_packet_const(PACKET_OK);
                break;

            case HOST_OP_LOCK:
                if (args_len != 0) {
                    send_packet_const(PACKET_ERR_MALFORMED_MESSAGE);
                    continue;
                }

                lock_eeprom();

                delay(20);
                send_packet_const(PACKET_OK);
                break;

            case HOST_OP_WRITE: {
                if (args_len < 2) {
                    send_packet_const(PACKET_ERR_MALFORMED_MESSAGE);
                    continue;
                }

                u16 start = (u16)args[0] | ((u16)args[1] << 8);
                u8 *data = &args[2];
                size_t data_len = args_len - 2;

                for (size_t i = 0; i < data_len; ++i)
                    write_data(start + i, data[i]);

                delay(20);
                send_packet_const(PACKET_OK);
                break;
            }

            case HOST_OP_READ: {
                if (args_len != 3) {
                    send_packet_const(PACKET_ERR_MALFORMED_MESSAGE);
                    continue;
                }

                u16 start = (u16)args[0] | ((u16)args[1] << 8);
                u8 len = args[2];

                size_t resp_len = len + 1;
                u8 resp[resp_len];
                resp[0] = DEV_OP_BYTES;

                u8 *out_data = &resp[1];

                for (size_t i = 0; i < len; ++i)
                    out_data[i] = read_data(start + i);

                delay(20);
                send_packet(resp, resp_len);
                break;
            }

            default:
                send_packet_const(PACKET_ERR_INVALID_OP);
        }
    }

    digitalWrite(CHIP_ENABLE, HIGH);
}

void loop()
{
}
