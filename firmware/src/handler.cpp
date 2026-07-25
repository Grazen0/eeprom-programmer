#include "handler.hpp"
#include "at28c256.hpp"
#include "protocol.hpp"
#include "types.hpp"
#include <expected>
#include <variant>

namespace
{
} // namespace

namespace handler
{

    namespace
    {
        using protocol::DeviceMessage;
        using protocol::DeviceMessageBytes;
        using protocol::DeviceMessageErr;
        using protocol::DeviceMessageOk;
        using protocol::HostMessageErase;
        using protocol::HostMessageExit;
        using protocol::HostMessageLock;
        using protocol::HostMessageRead;
        using protocol::HostMessageUnlock;
        using protocol::HostMessageWrite;
        using types::Error;

        std::expected<DeviceMessage, Error>
        handle([[maybe_unused]] const HostMessageExit &message, State &state)
        {
            state.exit = true;
            return DeviceMessageOk{};
        }

        std::expected<DeviceMessage, Error>
        handle(const HostMessageRead &message, [[maybe_unused]] State &state)
        {
            static std::array<u8, SERIAL_RX_BUFFER_SIZE> data_buf;

            for (size_t i = 0; i < message.len; ++i)
                data_buf.at(i) = at28c256::read(message.start + i);

            return DeviceMessageBytes{
                std::span{data_buf.data(), message.len}
            };
        }

        std::expected<DeviceMessage, Error>
        handle(const HostMessageWrite &message, [[maybe_unused]] State &state)
        {
            if (!at28c256::write_many(message.start, message.data)) {
                return DeviceMessageErr{types::Error::WRITE_TIMEOUT};
            }

            return DeviceMessageOk{};
        }

        std::expected<DeviceMessage, Error>
        handle([[maybe_unused]] const HostMessageLock &message,
               [[maybe_unused]] State &state)
        {
            at28c256::lock();
            return DeviceMessageOk{};
        }

        std::expected<DeviceMessage, Error>
        handle([[maybe_unused]] const HostMessageUnlock &message,
               [[maybe_unused]] State &state)
        {
            at28c256::unlock();
            return DeviceMessageOk{};
        }

        std::expected<DeviceMessage, Error>
        handle([[maybe_unused]] const HostMessageErase &message,
               [[maybe_unused]] State &state)
        {
            at28c256::erase();
            return DeviceMessageOk{};
        }

    } // namespace

    using protocol::HostMessage;

    std::expected<DeviceMessage, Error> handle(const HostMessage &message,
                                               State &state)
    {
        auto visitor = [&](const auto &msg) { return handle(msg, state); };
        return std::visit(visitor, message);
    }

} // namespace handler
