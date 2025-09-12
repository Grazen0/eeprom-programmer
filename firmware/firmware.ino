enum Command : uint8_t {
    Command_Read = 0x00,
    Command_Write = 0x01,
    Command_Verify = 0x02,
};

enum Opcode : uint8_t {
    Opcode_Ready = 0x00,
    Opcode_Print = 0x01,
    Opcode_Chunk = 0x02,
    Opcode_ReadEnd = 0x03,
    Opcode_ChunkRequest = 0x04,
    Opcode_InvalidChecksum = 0x05,
    Opcode_ByteMismatch = 0x06,
    Opcode_ByteRequest = 0x07,
};

constexpr size_t DELAY_TIME = 2;

constexpr uint8_t CHIP_ENABLE = 50;
constexpr uint8_t OUTPUT_ENABLE = 51;
constexpr uint8_t WRITE_ENABLE = 52;

void set_address(const uint16_t addr)
{
    PORTC = addr & 0xFF;
    PORTA = (addr >> 8) & 0xFF;
}

uint8_t read_data(const uint16_t addr)
{
    DDRL = B00000000;
    set_address(addr);

    digitalWrite(OUTPUT_ENABLE, LOW);
    delayMicroseconds(20);
    const uint8_t value = PINL;
    digitalWrite(OUTPUT_ENABLE, HIGH);
    delayMicroseconds(5);

    return value;
}

void write_data(const uint16_t addr, const uint8_t value)
{
    DDRL = B11111111;
    set_address(addr);

    PORTL = value;

    digitalWrite(WRITE_ENABLE, LOW);
    delay(2);
    digitalWrite(WRITE_ENABLE, HIGH);
    delayMicroseconds(5);
}

void write_data_careful(const uint16_t addr, const uint8_t value)
{
    DDRL = B11111111;
    set_address(addr);

    PORTL = value;

    digitalWrite(WRITE_ENABLE, LOW);
    delay(10);
    digitalWrite(WRITE_ENABLE, HIGH);
    delayMicroseconds(50);
}

void print_u16(const uint16_t x)
{
    if (x < 0x10)
        Serial.print("000");
    else if (x < 0x100)
        Serial.print("00");
    else if (x < 0x1000)
        Serial.print("0");

    Serial.print(x, HEX);
}

void print_u8(const uint8_t x)
{
    if (x < 0x10)
        Serial.print("0");

    Serial.print(x, HEX);
}

uint16_t serial_read_u16()
{
    while (Serial.available() < 2)
        ;

    uint8_t buf[2];
    Serial.readBytes(buf, 2);
    return ((uint16_t)buf[0] << 8) | (uint16_t)buf[1];
}

uint8_t serial_read_u8()
{
    while (Serial.available() < 1)
        ;
    return Serial.read();
}

void serial_write_u16(const uint16_t value)
{
    Serial.write((uint8_t)((value >> 8) & 0xFF));
    Serial.write((uint8_t)(value & 0xFF));
}

void serial_print(const char str[])
{
    Serial.write(Opcode_Print);

    const uint16_t len = strlen(str);
    serial_write_u16(len);
    Serial.print(str);
}

const uint8_t CHUNK_ACK = 0xFF;
constexpr size_t CHUNK_SIZE = 32;

uint16_t calculate_checksum(const uint8_t data[], const size_t len)
{
    uint8_t sum_1 = 0;
    uint8_t sum_2 = 0;

    for (size_t i = 0; i < len; ++i) {
        sum_1 += data[i];
        sum_2 += sum_1;
    }

    return ((uint16_t)sum_2 << 8) | (uint16_t)sum_1;
}

void read_eeprom(const uint16_t start, const uint16_t end)
{
    const size_t chunk_count = (end - start) / CHUNK_SIZE;

    for (size_t c = 0; c < chunk_count; ++c) {

        const uint16_t chunk_start = start + (c * CHUNK_SIZE);
        const uint16_t chunk_end = chunk_start + CHUNK_SIZE;

        uint8_t chunk[CHUNK_SIZE] = {};

        for (uint16_t i = 0; i < CHUNK_SIZE; ++i)
            chunk[i] = read_data(chunk_start + i);

        Serial.write(Opcode_Chunk);
        Serial.write(CHUNK_SIZE);
        serial_write_u16(calculate_checksum(chunk, CHUNK_SIZE));

        Serial.write(chunk, CHUNK_SIZE);

        // Wait for chunk ACK
        while (serial_read_u8() != CHUNK_ACK)
            ;
    }

    const uint8_t remaining_bytes = (end - start) % CHUNK_SIZE;

    if (remaining_bytes != 0) {
        const uint16_t remainder_start = end - remaining_bytes;

        uint8_t chunk[CHUNK_SIZE] = {};

        for (uint16_t i = 0; i < remaining_bytes; ++i)
            chunk[i] = read_data(remainder_start + i);

        Serial.write(Opcode_Chunk);
        Serial.write(remaining_bytes);
        serial_write_u16(calculate_checksum(chunk, remaining_bytes));

        Serial.write(chunk, remaining_bytes);
    }

    Serial.write(Opcode_ReadEnd);
}

void write_eeprom(const bool verify)
{
    uint16_t addr = 0;
    uint8_t chunk[0x100];

    while (true) {
        Serial.write(Opcode_ChunkRequest);

        const size_t chunk_size = serial_read_u8();

        if (chunk_size == 0)
            break;

        const uint16_t checksum = serial_read_u16();

        while (Serial.available() < chunk_size)
            ;
        Serial.readBytes(chunk, chunk_size);

        const uint16_t computed_checksum =
            calculate_checksum(chunk, chunk_size);

        if (checksum != computed_checksum) {
            Serial.write(Opcode_InvalidChecksum);
            serial_write_u16(checksum);
            serial_write_u16(computed_checksum);
            break;
        }

        for (uint16_t i = 0; i < chunk_size; ++i) {
            write_data(addr, chunk[i]);
            ++addr;
        }
    }

    if (verify)
        verify_eeprom(true);
}

void verify_eeprom(const bool fix)
{
    uint16_t addr = 0;
    uint8_t chunk[0x100];
    bool should_fix = false;

    while (true) {
        Serial.write(Opcode_ChunkRequest);

        const size_t chunk_size = serial_read_u8();

        if (chunk_size == 0)
            break;

        const uint16_t checksum = serial_read_u16();

        while (Serial.available() < chunk_size)
            ;
        Serial.readBytes(chunk, chunk_size);

        const uint16_t computed_checksum =
            calculate_checksum(chunk, chunk_size);

        if (checksum != computed_checksum) {
            Serial.write(Opcode_InvalidChecksum);
            serial_write_u16(checksum);
            serial_write_u16(computed_checksum);
            break;
        }

        for (uint16_t i = 0; i < chunk_size; ++i) {
            const uint8_t expected = chunk[i];
            const uint8_t actual = read_data(addr);

            if (actual != expected) {
                Serial.write(Opcode_ByteMismatch);
                serial_write_u16(addr);
                Serial.write(expected);
                Serial.write(actual);

                should_fix = true;
            }

            ++addr;
        }
    }

    if (!fix)
        return;

    while (true) {
        Serial.write(Opcode_ByteRequest);

        const uint16_t addr = serial_read_u16();

        if (addr == 0xFFFF)
            break;

        const uint16_t value = serial_read_u8();
        write_data_careful(addr, value);
    }
}

void setup()
{
    Serial.begin(115200);
    DDRC = B11111111;
    DDRA = B11111111;

    pinMode(CHIP_ENABLE, OUTPUT);
    pinMode(OUTPUT_ENABLE, OUTPUT);
    pinMode(WRITE_ENABLE, OUTPUT);

    digitalWrite(OUTPUT_ENABLE, HIGH);
    digitalWrite(WRITE_ENABLE, HIGH);
    digitalWrite(CHIP_ENABLE, LOW);

    while (!Serial)
        ;

    delay(50);
    Serial.write(Opcode_Ready);

    const uint8_t command = serial_read_u8();

    switch (command) {
    case Command_Read: {
        const uint16_t start = serial_read_u16();
        const uint16_t end = serial_read_u16();
        read_eeprom(start, end);
        break;
    }
    case Command_Write: {
        const bool verify = serial_read_u8();
        write_eeprom(true);
        break;
    }
    case Command_Verify: {
        const bool fix = serial_read_u8();
        verify_eeprom(fix);
        break;
    }
    }

    digitalWrite(CHIP_ENABLE, HIGH);
}

void loop()
{
}
