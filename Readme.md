# Memory Deduplication exploit example

# how to use

1. enable ksm
   ```
   sudo bash -c "echo 1 > /sys/kernel/mm/ksm/run"
   ```
2. start server
   ```
   ./server <some path>
   ```
   this creates an unix socket at `<some path`.
3. start client
   ```
   ./client <some path>
   ```


After some measurements the client connects to the server.
The server owns a page with known content.
For every byte received on the socket it shifts a random byte into this page.
this byte is also printed to stdout.
The client extracts this random data byte by byte through memory deduplication.
