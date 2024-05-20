# rust-unix-relay-server

A simple server to relay messages sent to a unix socket

All connected clients will receive all messages, and can send messages.

Note that the max message size is 1024 bytes

Test it by running:

```bash
socat -d -d - UNIX-CONNECT:/tmp/relay_socket
```
