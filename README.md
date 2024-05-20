# rust-unix-relay-server

A simple server to relay messages sent to a unix socket

Test it by running:

```bash
socat -d -d - UNIX-CONNECT:/tmp/relay_socket
```
