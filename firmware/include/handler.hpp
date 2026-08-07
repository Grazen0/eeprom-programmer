#ifndef EEPROM_PROGRAMMER_HANDLER_HPP
#define EEPROM_PROGRAMMER_HANDLER_HPP

#include "protocol.hpp"
#include <Arduino.h>

namespace handler
{
    struct State {
        bool exit = false;
    };

    std::expected<protocol::DeviceMessage, types::Error>
    handle(const protocol::HostMessage &message, State &state);

} // namespace handler

#endif
