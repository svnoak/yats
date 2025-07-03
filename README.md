# Yet Another Tunneling Service (YATS)

This project provides a simple yet powerful tunneling service that allows you to expose a local web service to the internet. It consists of three main components:

1.  **Server**: A Rust-based WebSocket server that listens for incoming connections from clients and forwards HTTP requests to them.
2.  **Client**: A Rust-based WebSocket client that connects to the server, receives forwarded HTTP requests, and sends them to a local web service.
3.  **Local App**: A sample Python-based web service that listens on port 8080 and prints the details of incoming requests.

## Prerequisites

Before you can run this project, you need to have the following installed:

*   [Rust](https://www.rust-lang.org/tools/install)
*   [Python 3](https://www.python.org/downloads/)

## How to Run

To get the tunneling service up and running, you need to start all three components in the following order:

### 1. Run the Local App

The local app is a simple Python web service that listens on port 8080. To start it, run the following command in the `localapp` directory:

```bash
python3 app.py
```

### 2. Run the Server

The server is a Rust-based WebSocket server that listens on port 3000. Before you can run it, you need to create a `.env` file in the `server` directory with the following content:

```
SECRET_TOKEN=your-secret-token
```

Replace `your-secret-token` with a secret token of your choice. This token is used to authenticate the client.

Once you have created the `.env` file, you can build and run the server with the following commands in the `server` directory:

```bash
cargo build
cargo run
```

### 3. Run the Client

The client is a Rust-based WebSocket client that connects to the server and forwards HTTP requests to the local app. Before you can run it, you need to create a `.env` file in the `client` directory with the following content:

```
SECRET_TOKEN=your-secret-token
SERVER_WS_URL=ws://localhost:3000/ws
CLIENT_ID=your-client-id
TARGET_HTTP_SERVICE_URL=http://localhost:8080
```

Replace `your-secret-token` with the same secret token you used for the server, and `your-client-id` with a unique ID for your client.

Once you have created the `.env` file, you can build and run the client with the following commands in the `client` directory:

```bash
cargo build
cargo run
```

## Testing the Setup

Once all three components are running, you can test the setup by sending an HTTP request to the server. For example, you can use `curl` to send a GET request to the following URL:

```bash
curl http://localhost:3000/your-client-id/test
```

Replace `your-client-id` with the client ID you specified in the client's `.env` file. You should see the details of the request printed in the console where you are running the local app.

## Developer Notes

*   The server is responsible for authenticating clients, managing WebSocket connections, and forwarding HTTP requests.
*   The client is responsible for connecting to the server, receiving forwarded HTTP requests, and sending them to the local app.
*   The local app is a simple web service that can be replaced with any web service you want to expose to the internet.

## Docker

A `Dockerfile` is provided to build and run the server in a Docker container. This is a multi-stage build that first builds the Rust application and then copies the binary to a smaller `debian:buster-slim` image.

To build the Docker image, run the following command in the root directory:

```bash
docker build -t yats-server -f server/Dockerfile .
```

To run the server in a Docker container, use the following command:

```bash
docker run -p 3000:3000 -e SECRET_TOKEN=your-secret-token yats-server
```

## Nginx Configuration

An example `nginx.conf` is provided to demonstrate how to run the server behind an Nginx reverse proxy. This configuration includes rate limiting to prevent abuse.