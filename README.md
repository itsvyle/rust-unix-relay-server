# rust-unix-relay-server

A simple server to relay messages sent to a unix socket

All connected clients will receive all messages, and can send messages.

Note that the max message size is 1024 bytes

Test it by running:

```bash
socat -d -d - UNIX-CONNECT:/tmp/relay_socket
```

# Docker image

After having cloned the image, run it by mounting a directory to the container at path `/tmp/sockets/`

This folder has to be specific to each instance of the container, as the socket file will be created in it with the same name.

```bash
docker run -v /tmp/relay_sockets/:/tmp/sockets/ -d rust-unix-relay-server
```

You can then connect to the socket at the path `<your directory>/relay`
