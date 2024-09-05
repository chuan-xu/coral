# coral web frame

1. 支持HTTP2、HTTP3和websocket, 
2. 支持axum和tower生态, 多个协议的请求共用handler和middleware
3. 支持代理和负载均衡
4. 跨节点的tracing
5. protobuf格式的日志, 自动记录trace_id

## coral-proxy

Usage

```bash
Usage: coral-proxy [OPTIONS] --port <PORT> --tls-cert <TLS_CERT> --tls-key <TLS_KEY> --cpui <CPUI> --nums <NUMS>

Options:
      --port <PORT>                    server port
      --tls-ca <TLS_CA>                ca directory
      --tls-cert <TLS_CERT>            server/client certificate
      --tls-key <TLS_KEY>              server/client private
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

## coral-server

```bash
Usage: coral-server [OPTIONS] --port <PORT> --tls-cert <TLS_CERT> --tls-key <TLS_KEY> --cpui <CPUI> --nums <NUMS> --domain <DOMAIN> --service-address <SERVICE_ADDRESS>

Options:
      --port <PORT>                        server port
      --tls-ca <TLS_CA>                    ca directory
      --tls-cert <TLS_CERT>                server/client certificate
      --tls-key <TLS_KEY>                  server/client private
      --dir <DIR>                          directory for storing logs
      --prefix <PREFIX>                    Log file name prefix
      --rotation <ROTATION>                Log file splitting period
      --otel-endpoint <OTEL_ENDPOINT>      telemetry collector address
      --otel-kvs <OTEL_KVS>                telemetry resource key value
      --cpui <CPUI>                        start number of cpu cores
      --nums <NUMS>                        number of runtime
      --domain <DOMAIN>                    self can be discovered by domain
      --service-address <SERVICE_ADDRESS>  service address: https://xxx.xxx.com:xx/xxx
  -h, --help                               Print help
  -V, --version                            Print version
```

## Run

需配置OpenTelemetry controller

```bash

# 代理服务
./bin/coral-proxy --port 9000 --tls-ca $PWD/cicd/self_sign_cert/ca --tls-cert $PWD/cicd/self_sign_cert/server.crt --tls-key $PWD/cicd/self_sign_cert/server.key --cpui 0 --nums 2 --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9000 --otel-kvs threads=2 --otel-kvs version=0.1

# server可启动多个节点 用于负载均衡
./bin/coral-server --port 9002 --tls-ca $PWD/cicd/self_sign_cert/ca --tls-cert $PWD/cicd/self_sign_cert/server.crt --tls-key $PWD/cicd/self_sign_cert/server.key --domain server.test.com --service-address https://server.test.com:9001/coral-proxy-endpoints --cpui 0 --nums 2 --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9002 --otel-kvs threads=2 --otel-kvs version=0.1
```

### tracing

![](/tests/jaeger.png "")
![](/tests/trace.png "")
![](/tests/graph.png "")
