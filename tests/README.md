# coral test

## coral-server

1. Usage

```bash
Usage: coral-server [OPTIONS] --port <PORT> --cpui <CPUI> --nums <NUMS>

Options:
      --port <PORT>                    server port
      --dir <DIR>                      directory for storing logs
      --prefix <PREFIX>                Log file name prefix
      --rotation <ROTATION>            Log file splitting period
      --otel-endpoint <OTEL_ENDPOINT>  telemetry collector address
      --otel-kvs <OTEL_KVS>            telemetry resource key value
      --cpui <CPUI>                    start number of cpu cores
      --nums <NUMS>                    number of runtime
  -h, --help                           Print help
  -V, --version                        Print version
```

2. Run

```bash

# 日志输出至控制台
# 节点 1
./bin/coral-server --port 9001 --cpui 2 --nums 2 --cache-addr 127.0.0.1:6379
#节点 2
./bin/coral-server --port 9002 --cpui 4 --nums 2 --cache-addr 127.0.0.1:6379

# 日志输出至文件
# 节点 1
./bin/coral-server --port 9001 --cpui 2 --nums 2 --dir $PWD/log --prefix server0.log --cache-addr 127.0.0.1:6379
# 节点 2
./bin/coral-server --port 9002 --cpui 4 --nums 2 --dir $PWD/log --prefix server1.log --cache-addr 127.0.0.1:6379

# 添加open-telemetry collector
# 节点 1
./bin/coral-server --port 9001 --cpui 2 --nums 2 --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9001 --otel-kvs threads=2 --otel-kvs version=0.1 --cache-addr 127.0.0.1:6379
# 节点 2
./bin-server --port 9002 --cpui 4 --nums 2 --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9002 --otel-kvs threads=2 --otel-kvs version=0.1 --cache-addr 127.0.0.1:6379

# 添加日志文件和opemetry collector
./bin/coral-server --port 9001 --cpui 2 --nums 2 --dir $PWD/log --prefix server0.log --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9001 --otel-kvs threads=2 --otel-kvs version=0.1 --cache-addr 127.0.0.1:6379
# 节点 2
./bin-server --port 9002 --cpui 4 --nums 2 --dir $PWD/log --prefix server1.log --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9002 --otel-kvs threads=2 --otel-kvs version=0.1 --cache-addr 127.0.0.1:6379
```

## coral-proxy

1. Usage

```bash
Usage: coral-proxy [OPTIONS] --certificate <CERTIFICATE> --private-key <PRIVATE_KEY> --port <PORT> --cpui <CPUI> --nums <NUMS>

Options:
      --ca-dir <CA_DIR>                ca directory
      --certificate <CERTIFICATE>      server certificate
      --private-key <PRIVATE_KEY>      server private
      --port <PORT>                    server port
      --addresses <ADDRESSES>          multiple backend address, exp 192.168.1.3:9001
      --dir <DIR>                      directory for storing logs
      --prefix <PREFIX>                Log file name prefix
      --rotation <ROTATION>            Log file splitting period
      --otel-endpoint <OTEL_ENDPOINT>  telemetry collector address
      --otel-kvs <OTEL_KVS>            telemetry resource key value
      --cpui <CPUI>                    start number of cpu cores
      --nums <NUMS>                    number of runtime
  -h, --help                           Print help
  -V, --version                        Print version
```

2. Run

修改hosts，添加127.0.0.1 server.test.com

```bash
# 日志输出至控制台
./bin/coral-proxy --ca-dir $PWD/cicd/self_sign_cert/ca --certificate $PWD/cicd/self_sign_cert/server.crt --private-key $PWD/cicd/self_sign_cert/server.key --port 9000 --cache-addr 127.0.0.1:6379  --cpui 0 --nums 2

# 日志输出至文件
./bin/coral-proxy --ca-dir $PWD/cicd/self_sign_cert/ca --certificate $PWD/cicd/self_sign_cert/server.crt --private-key $PWD/cicd/self_sign_cert/server.key --port 9000 --cache-addr 127.0.0.1:6379 --cpui 0 --nums 2 --dir $PWD/log --prefix proxy.log

# 添加open-telemetry collector
./bin/coral-proxy --ca-dir $PWD/cicd/self_sign_cert/ca --certificate $PWD/cicd/self_sign_cert/server.crt --private-key $PWD/cicd/self_sign_cert/server.key --port 9000 --cache-addr 127.0.0.1:6379 --cpui 0 --nums 2 --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9000 --otel-kvs threads=2 --otel-kvs version=0.1

# 添加日志文件和open-telemetry collector
./bin/coral-proxy --ca-dir $PWD/cicd/self_sign_cert/ca --certificate $PWD/cicd/self_sign_cert/server.crt --private-key $PWD/cicd/self_sign_cert/server.key --port 9000 --cache-addr 127.0.0.1:6379 --cpui 0 --nums 2 --dir $PWD/log --prefix proxy.log --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9000 --otel-kvs threads=2 --otel-kvs version=0.1
```

## curl test

```bash
curl -X POST --cacert ./cicd/self_sign_cert/server.crt --cert ./cicd/self_sign_cert/client.crt --key ./cicd/self_sign_cert/client.key https://server.test.com:9000/benchmark

# with header
curl -X POST -H "X-Trace-Id: DCA4DCB7-79C6-FDC4-F262-EDD742F906FA" --cacert /root/certs/server.crt --cert /root/certs/client.crt --key /root/certs/client.key https://server.test.com:9000/benchmark
```