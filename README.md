# coral

## curl test

```bash
curl -X POST --cacert /root/certs/server.crt --cert /root/certs/client.crt --key /root/certs/client.key https://server.test.com:9000/benchmark

# with header
curl -X POST -H "X-Trace-Id: DCA4DCB7-79C6-FDC4-F262-EDD742F906FA" --cacert /root/certs/server.crt --cert /root/certs/client.crt --key /root/certs/client.key https://server.test.com:9000/benchmark
```

## coral-proxy

```bash
# debug
coral-proxy --debug --ca-dir /root/certs/ca --certificate /root/certs/server.crt --private-key /root/certs/server.key --port 9000 --cpui 0 --nums 2 --addresses 127.0.0.1:9001

coral-proxy --debug --ca-dir /root/certs/ca --certificate /root/certs/server.crt --private-key /root/certs/server.key --port 9000 --cpui 0 --nums 2 --addresses 127.0.0.1:9001 --addresses 127.0.0.1:9002
```

## coral-server

```bash
# debug
coral-server --debug --port 9001 --cpui 2 --nums 2
```
