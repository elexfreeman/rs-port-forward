**rs-port-forward: A Secure and Efficient Port Forwarding Tool in Rust**

**Introduction**

`rs-port-forward` is a Rust application designed to create secure and efficient port forwarding tunnels. It enables you to seamlessly redirect traffic from a local port on your machine to a specified remote host and port. This functionality is particularly useful for:

- **Accessing internal services:** Forward traffic from your local machine to internal servers on a remote network that may not be directly accessible from the outside.
- **Developing on a remote machine:** Tunnel local development tools (e.g., databases, code editors) to a remote server for efficient development workflows.
- **Testing and debugging:** Establish temporary connections for testing purposes or troubleshooting network connectivity.

**Features**

- **Configuration file support:** Loads configuration details for multiple port forwarding rules from a JSON file (configurable location).
- **Error handling:** Logs errors and connection failures with timestamps for better debugging.
- **Asynchronous I/O:** Leverages Tokio for non-blocking, efficient I/O operations, ensuring fast and responsive port forwarding.
- **Concise and well-structured code:** The codebase adheres to Rust best practices, making it easier to understand and maintain.

**Requirements**

- Rust compiler (version 1.56 or later recommended)
- Tokio runtime (included by default with a Rust installation)
- Serde for serialization/deserialization (can be installed with `cargo install serde serde_json`)

**Installation**

1. Clone this repository: `git clone https://github.com/your-username/rs-port-forward.git`
2. Navigate to the project directory: `cd rs-port-forward`
3. Build the project: `cargo build --release`

**Configuration**

The application uses a JSON configuration file named `rs-port-forward.config.json`. You can place this file in a directory of your choice. The default location for Linux is `/etc/`, while Windows doesn't have a pre-defined location. If necessary, edit the `file_path` logic in `load_config` to match your configuration file location.

The configuration file should have the following structure:

```json
{
  "connect_list": [
    {
      "name": "My Proxy Connection",  // Optional name for the connection
      "local_port": 8080,
      "remote_address": "remote_server.example.com",
      "remote_port": 80
    },
    {
      // ... additional connection configurations
    }
  ]
}
```

**Usage**

1. Start the application: `./target/release/rs-port-forward`
   - If the configuration file is in a non-standard location, modify the command to include the path (e.g., `./target/release/rs-port-forward --config /path/to/your/config.json`).
2. The application will load and print the configured connections.
3. Traffic on the specified local ports will be forwarded to the corresponding remote hosts and ports.

**Example**

The configuration above would create a port forwarding tunnel:

- When you access `http://localhost:8080` in your browser, the request will be forwarded to `http://remote_server.example.com:80`.
- Responses from the remote server will be sent back to your local machine, appearing as if you were directly connected to the remote server.

**Additional Notes**

- For security reasons, it's recommended to only forward necessary ports and avoid exposing sensitive services publicly.
- This is a basic port forwarding tool, and advanced security features like authentication and encryption are not included.
- Consider using established tools like SSH or dedicated secure tunneling solutions for more complex scenarios.

**Disclaimer**

This software is provided as-is with no warranty or guarantee of functionality. Use it at your own risk.

**Contributing**

We welcome contributions to improve this project! Feel free to submit pull requests for bug fixes, enhancements, or new features. For significant changes, please open an issue for discussion beforehand.
