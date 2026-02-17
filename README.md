# hermes

APC (Agent Client Protocol) client for Neovim, built with Rust using `agent-client-protocol`, `nvim-oxi`, and `nvim-utils`.

## Overview

Hermes is a clean, well-tested implementation of an Agent Client Protocol client designed for integration with Neovim. It enables Neovim to communicate with AI coding agents following the standardized APC specification.

## Features

- ✅ Full implementation of the APC `Client` trait
- ✅ Configurable capabilities (filesystem, terminal)
- ✅ Comprehensive test coverage (unit, integration, and doc tests)
- ✅ Clean architecture following SOLID principles
- ✅ Extensive API documentation
- ✅ Security-focused with no known vulnerabilities

## Quick Start

### Building

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

## Usage

### Basic Client Creation

```rust
use hermes::client::{ApcClient, ClientConfig};

// Create a client with default configuration
let config = ClientConfig::default();
let client = ApcClient::new(config);
```

### Custom Configuration

```rust
use hermes::client::{ApcClient, ClientConfig};

// Create a client with custom settings
let config = ClientConfig {
    name: "my-editor".to_string(),
    version: "1.0.0".to_string(),
    enable_fs: true,      // Enable file system operations
    enable_terminal: false, // Disable terminal operations
};
let client = ApcClient::new(config);
```

### Handling Session Notifications

```rust
use hermes::client::ApcClient;
use agent_client_protocol::{Client, SessionNotification, SessionId, SessionUpdate};
use agent_client_protocol::{ContentChunk, ContentBlock, TextContent};

async fn handle_notification(client: &ApcClient) -> agent_client_protocol::Result<()> {
    let notification = SessionNotification::new(
        SessionId::new("session-1"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new("Hello from agent"))
        )),
    );
    
    client.session_notification(notification).await
}
```

### Neovim Plugin Integration

```rust
use hermes::nvim::{PluginState, setup};

// Initialize the plugin
let state = PluginState::new();

// Setup Neovim plugin
let result = setup();
```

## Architecture

The project follows clean code principles:

- **Single Responsibility**: Each component has a clear, focused purpose
- **Dependency Injection**: Configuration is injected via `ClientConfig`
- **Open/Closed Principle**: Extensible through trait implementation
- **Interface Segregation**: Clean trait boundaries with optional capabilities
- **Comprehensive Documentation**: All public APIs are thoroughly documented

## Project Structure

```
hermes/
├── src/
│   ├── lib.rs       # Library entry point
│   ├── client.rs    # APC client implementation
│   └── nvim.rs      # Neovim integration
├── tests/
│   └── integration_test.rs  # Integration tests
├── Cargo.toml       # Project dependencies
└── README.md        # This file
```

## Dependencies

- **agent-client-protocol** v0.9.4 - APC protocol implementation
- **nvim-oxi** v0.6.0 - Neovim Rust bindings
- **nvim-utils** v0.1.12 - Neovim plugin utilities
- **tokio** v1.49+ - Async runtime
- **serde** v1.0 - Serialization/deserialization
- **async-trait** v0.1 - Async trait support
- **thiserror** v2.0 - Error handling
- **anyhow** v1.0 - Error context

## Testing

The project includes comprehensive test coverage:

- **Unit Tests**: 8 tests covering core functionality
- **Integration Tests**: 7 tests verifying protocol compliance
- **Documentation Tests**: 12 tests demonstrating API usage

Run all tests:
```bash
cargo test
```

Run only unit tests:
```bash
cargo test --lib
```

Run only integration tests:
```bash
cargo test --test integration_test
```

Run only documentation tests:
```bash
cargo test --doc
```

## Security

- All dependencies are checked for known vulnerabilities
- CodeQL security scanning shows 0 alerts
- Using latest stable versions of all dependencies
- Regular security audits recommended

## License

See [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please ensure:
1. All tests pass (`cargo test`)
2. Code follows clean code principles
3. New features include tests
4. Public APIs are documented
5. No new security vulnerabilities introduced

