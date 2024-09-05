# coral test

## coral-server

1. Run

```bash

# 日志输出至控制台
# 节点 1
./bin/coral-server --port 9002 --tls-ca $PWD/cicd/self_sign_cert/ca --tls-cert $PWD/cicd/self_sign_cert/server.crt --tls-key $PWD/cicd/self_sign_cert/server.key --domain server.test.com --service-address https://server.test.com:9001/coral-proxy-endpoints --cpui 0 --nums 2

# 日志输出至文件
# 节点 1
./bin/coral-server --port 9002 --tls-ca $PWD/cicd/self_sign_cert/ca --tls-cert $PWD/cicd/self_sign_cert/server.crt --tls-key $PWD/cicd/self_sign_cert/server.key --domain server.test.com --service-address https://server.test.com:9001/coral-proxy-endpoints --cpui 0 --nums 2 --dir $PWD/log --prefix server0.log

# 添加open-telemetry collector
# 节点 1
./bin/coral-server --port 9002 --tls-ca $PWD/cicd/self_sign_cert/ca --tls-cert $PWD/cicd/self_sign_cert/server.crt --tls-key $PWD/cicd/self_sign_cert/server.key --domain server.test.com --service-address https://server.test.com:9001/coral-proxy-endpoints --cpui 0 --nums 2 --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9002 --otel-kvs threads=2 --otel-kvs version=0.1

# 添加日志文件和opemetry collector
./bin/coral-server --port 9002 --tls-ca $PWD/cicd/self_sign_cert/ca --tls-cert $PWD/cicd/self_sign_cert/server.crt --tls-key $PWD/cicd/self_sign_cert/server.key --domain server.test.com --service-address https://server.test.com:9001/coral-proxy-endpoints --cpui 0 --nums 2 --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9002 --otel-kvs threads=2 --otel-kvs version=0.1 --dir $PWD/log --prefix server0.log
```

## coral-proxy

1. Run

修改hosts，添加127.0.0.1 server.test.com

```bash
# 日志输出至控制台
./bin/coral-proxy --port 9000 --tls-ca $PWD/cicd/self_sign_cert/ca --tls-cert $PWD/cicd/self_sign_cert/server.crt --tls-key $PWD/cicd/self_sign_cert/server.key --cpui 0 --nums 2

# 日志输出至文件
./bin/coral-proxy --port 9000 --tls-ca $PWD/cicd/self_sign_cert/ca --tls-cert $PWD/cicd/self_sign_cert/server.crt --tls-key $PWD/cicd/self_sign_cert/server.key --cpui 0 --nums 2 --dir $PWD/log --prefix proxy.log

# 添加open-telemetry collector
./bin/coral-proxy --port 9000 --tls-ca $PWD/cicd/self_sign_cert/ca --tls-cert $PWD/cicd/self_sign_cert/server.crt --tls-key $PWD/cicd/self_sign_cert/server.key --cpui 0 --nums 2 --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9000 --otel-kvs threads=2 --otel-kvs version=0.1

# 添加日志文件和open-telemetry collector
./bin/coral-proxy --port 9000 --tls-ca $PWD/cicd/self_sign_cert/ca --tls-cert $PWD/cicd/self_sign_cert/server.crt --tls-key $PWD/cicd/self_sign_cert/server.key --cpui 0 --nums 2 --otel-endpoint http://172.17.0.1:4317 --otel-kvs service.name=coral --otel-kvs port=9000 --otel-kvs threads=2 --otel-kvs version=0.1 --dir $PWD/log --prefix proxy.log
```

## curl test

```bash
# http2
curl -X POST --http2 --cacert $PWD/cicd/self_sign_cert/server.crt --cert $PWD/cicd/self_sign_cert/client.crt --key $PWD/cicd/self_sign_cert/client.key https://server.test.com:9000/trace

curl -X POST --http2 --cacert $PWD/cicd/self_sign_cert/server.crt --cert $PWD/cicd/self_sign_cert/client.crt --key $PWD/cicd/self_sign_cert/client.key https://server.test.com:9000/benchmark

# http3
curl -X POST --http3 --cacert $PWD/cicd/self_sign_cert/server.crt --cert $PWD/cicd/self_sign_cert/client.crt --key $PWD/cicd/self_sign_cert/client.key https://server.test.com:9001/trace

curl -X POST --http3 --cacert $PWD/cicd/self_sign_cert/server.crt --cert $PWD/cicd/self_sign_cert/client.crt --key $PWD/cicd/self_sign_cert/client.key https://server.test.com:9001/benchmark

# with header
curl -X POST -H "X-Trace-Id: DCA4DCB7-79C6-FDC4-F262-EDD742F906FA" --cacert /root/certs/server.crt --cert /root/certs/client.crt --key /root/certs/client.key https://server.test.com:9000/benchmark
```