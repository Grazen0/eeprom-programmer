enum Command : uint8_t {
  Command_Read = 0,
  Command_Write = 1,
  Command_Verify = 2,
};

enum Opcode : uint8_t {
  Opcode_Ready = 0x00,
  Opcode_Print = 0x01,
  Opcode_Chunk = 0x02,
  Opcode_ReadEnd = 0x03,
};

const size_t DELAY_TIME = 2;

const uint8_t CHIP_ENABLE = 50;
const uint8_t OUTPUT_ENABLE = 51;
const uint8_t WRITE_ENABLE = 52;
const size_t ROM_SIZE = 0x8000;

void set_address(uint16_t addr) {
  PORTC = addr & 0xFF;
  PORTA = (addr >> 8) & 0xFF;
}

uint8_t read_data(uint16_t addr) {
  DDRL = B00000000;
  set_address(addr);

  digitalWrite(OUTPUT_ENABLE, LOW);
  uint8_t value = PINL;
  digitalWrite(OUTPUT_ENABLE, HIGH);

  return value;
}

void write_data(uint16_t addr, uint8_t x) {
  set_address(addr);

  DDRL = B11111111;
  PORTL = x;

  digitalWrite(WRITE_ENABLE, LOW);
  delayMicroseconds(1);
  digitalWrite(WRITE_ENABLE, HIGH);
  delay(10);
}

void print_u16(uint16_t x) {
  if (x < 0x10)
    Serial.print("000");
  else if (x < 0x100)
    Serial.print("00");
  else if (x < 0x1000)
    Serial.print("0");

  Serial.print(x, HEX);
}

void print_u8(uint8_t x) {
  if (x < 0x10)
    Serial.print("0");

  Serial.print(x, HEX);
}

uint16_t serial_read_u16() {
  while (Serial.available() < 2);

  uint8_t buf[2];
  Serial.readBytes(buf, 2);
  return ((uint16_t)buf[1] << 8) | (uint16_t)buf[0];
}

uint16_t serial_read_u8() {
  while (Serial.available() < 1);
  return Serial.read();
}

void write_str(char str[]) {
  Serial.write(Opcode_Print);

  const uint16_t len = strlen(str);
  Serial.write(len & 0xFF);
  Serial.write((len >> 8) & 0xFF);
  Serial.print(str);
}

const uint8_t CHUNK_ACK = 0xFF;

void read_eeprom() {
  const uint16_t start = serial_read_u16();
  const uint16_t end = serial_read_u16();

  const size_t CHUNK_SIZE = 255;

  const size_t chunks = (end - start) / CHUNK_SIZE;

  for (size_t c = 0; c < chunks; ++c) {
    Serial.write(Opcode_Chunk);
    Serial.write(CHUNK_SIZE);

    const uint16_t chunk_start = start + (c * CHUNK_SIZE);
    const uint16_t chunk_end = chunk_start + CHUNK_SIZE;

    for (uint16_t addr = chunk_start; addr < chunk_end; ++addr) {
      const uint8_t value = read_data(addr);
      Serial.write(value);
    }

    // Wait for chunk ACK
    while (Serial.available() == 0 || Serial.read() != CHUNK_ACK);
  }

  const uint16_t remaining_bytes = (end - start) % CHUNK_SIZE;

  if (remaining_bytes != 0) {
    Serial.write(Opcode_Chunk);
    Serial.write(remaining_bytes);

    const uint16_t remainder_start = end - remaining_bytes;

    for (uint16_t i = 0; i < remaining_bytes; ++i) {
      const uint8_t value = read_data(remainder_start + i);
      Serial.write(value);
    }
  }

  Serial.write(Opcode_ReadEnd);
}

void setup() {
  Serial.begin(115200);
  while (!Serial);

  DDRC = B11111111;
  DDRA = B11111111;

  pinMode(CHIP_ENABLE, OUTPUT);
  pinMode(OUTPUT_ENABLE, OUTPUT);
  pinMode(WRITE_ENABLE, OUTPUT);

  digitalWrite(CHIP_ENABLE, LOW);
  digitalWrite(OUTPUT_ENABLE, HIGH);
  digitalWrite(WRITE_ENABLE, HIGH);

  delayMicroseconds(DELAY_TIME);
  Serial.write(Opcode_Ready);

  uint8_t command = serial_read_u8();
  switch (command) {
    case Command_Read:
      read_eeprom();
      break;
  }

  digitalWrite(CHIP_ENABLE, HIGH);
}

void loop() {}
