#include "at28c256.hpp"
#include "handler.hpp"
#include "protocol.hpp"
#include "types.hpp"
#include "util.hpp"
#include <Arduino.h>

namespace
{
    using handler::State;
    using protocol::DeviceMessage;
    using types::Error;

    std::expected<protocol::DeviceMessage, Error>
    process_next_message(State &state)
    {
        auto message = TRY(protocol::recv_message());
        auto resp = TRY(handler::handle(message, state));
        return resp;
    }

} // namespace

void setup()
{
    Serial.begin(115200);

    at28c256::setup();
    protocol::send_message(DeviceMessage::Ready{});

    State state{};

    while (!state.exit) {
        if (auto resp = process_next_message(state))
            protocol::send_message(*resp);
        else
            protocol::send_message(DeviceMessage::Err{resp.error()});
    }
}

void loop()
{
}
