#include "handler.hpp"
#include "at28c256.hpp"
#include "protocol.hpp"
#include "types.hpp"
#include <expected>

namespace
{
} // namespace

namespace handler
{

    namespace
    {
        using protocol::DeviceMessage;
        using protocol::HostMessage;
        using types::Error;

        std::expected<DeviceMessage, Error>
        handle([[maybe_unused]] const HostMessage::Exit &message, State &state)
        {
            state.exit = true;
            return DeviceMessage::Ok{};
        }

        std::expected<DeviceMessage, Error>
        handle(const HostMessage::Read &message, [[maybe_unused]] State &state)
        {
            static std::array<u8, SERIAL_RX_BUFFER_SIZE> data_buf;

            for (size_t i = 0; i < message.len; ++i)
                data_buf.at(i) = at28c256::read(message.start + i);

            return DeviceMessage::Bytes{
                std::span{data_buf.data(), message.len}
            };
        }

        std::expected<DeviceMessage, Error>
        handle(const HostMessage::Write &message, [[maybe_unused]] State &state)
        {
            if (!at28c256::write_many(message.start, message.data)) {
                return DeviceMessage::Err{types::Error::WRITE_TIMEOUT};
            }

            return DeviceMessage::Ok{};
        }

        std::expected<DeviceMessage, Error>
        handle([[maybe_unused]] const HostMessage::Lock &message,
               [[maybe_unused]] State &state)
        {
            at28c256::lock();
            return DeviceMessage::Ok{};
        }

        std::expected<DeviceMessage, Error>
        handle([[maybe_unused]] const HostMessage::Unlock &message,
               [[maybe_unused]] State &state)
        {
            at28c256::unlock();
            return DeviceMessage::Ok{};
        }

        std::expected<DeviceMessage, Error>
        handle([[maybe_unused]] const HostMessage::Erase &message,
               [[maybe_unused]] State &state)
        {
            at28c256::erase();
            return DeviceMessage::Ok{};
        }

    } // namespace

    using protocol::HostMessage;

    std::expected<DeviceMessage, Error> handle(const HostMessage &message,
                                               State &state)
    {
        auto visitor = [&](const auto &msg) { return handle(msg, state); };
        return message.visit(visitor);
    }

} // namespace handler
