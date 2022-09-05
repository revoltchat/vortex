# DEPRECATED, rewrite on [new branch](https://github.com/revoltchat/vortex/tree/vortex)

Please do not use Vortex in any capacity until the rewrite is complete, you will not receive any support for the current version and the new version is heavy in development.

# Revolt Vortex

## Description

The voice server for Revolt.

## Stack

- [Rust](https://www.rust-lang.org/)
- [Mediasoup](https://mediasoup.org/)
- [Warp](https://github.com/seanmonstar/warp) (HTTP)

## Resources

### Vortex

- [Vortex Issue Board](https://github.com/revoltchat/vortex/issues)

### Revolt

- [Revolt Project Board](https://github.com/revoltchat/revolt/discussions) (Submit feature requests here)
- [Revolt Testers Server](https://app.revolt.chat/invite/Testers)
- [Contribution Guide](https://developers.revolt.chat/contributing)

## Quick Start

Get Vortex up and running locally for development.

<!-- Python gets us the desired syntax highlighting, it's shell commands. -->

```py
git clone https://github.com/revoltchat/vortex
cd vortex
cargo build
# Set the environment variables as described below
cargo run
```

## Environment Variables

| Variable       | Description                                                                                                                           | Example                          |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- |
| `HTTP_HOST`    | The hostname to bind to.                                                                                                              | `0.0.0.0:8080` (default)         |
| `WS_URL`       | The websocket URL to advertise.                                                                                                       | `wss://vortex.revolt.chat`       |
| `MANAGE_TOKEN` | The token used for communication between Vortex and Delta.                                                                            | `<token>`                        |
| `RTC_MIN_PORT` | The minimum port to use for WebRTC and RTP.                                                                                           | `10000` (default)                |
| `RTC_MAX_PORT` | The maximum port to use for WebRTC and RTP.                                                                                           | `11000` (default)                |
| `DISABLE_RTP`  | Disable RTP. The value `1` disables RTP, all other values or not set will enable RTP.                                                 | `0` (default)                    |
| `RTC_IPS`      | Semicolon separated list of IPs to use for WebRTC. Hostnames are not supported yet. Either combined or split listen and announce IPs. | `<combined>;<listen>,<announce>` |

## CLI Commands

| Command       | Description           |
| ------------- | --------------------- |
| `cargo build` | Build/compile Vortex. |
| `cargo run`   | Run Vortex.           |

## License

Vortex is licensed under the [GNU Affero General Public License v3.0](https://github.com/revoltchat/vortex/blob/master/LICENSE).
