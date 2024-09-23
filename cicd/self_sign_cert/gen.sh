#!/bin/bash

openssl genrsa -out server.key 2048
openssl genrsa -out client.key 2048

# 生成证书请求
echo -e '\n\n\n\n\n\n\n\n\n' | openssl req -new -days 3650 -out server.csr -key server.key
echo -e '\n\n\n\n\n\n\n\n\n' | openssl req -new -days 3650 -out client.csr -key client.key

# 生成证书
openssl x509 -req -in server.csr -signkey server.key -out server.crt -extfile open.cnf -extensions server_ext
openssl x509 -req -in client.csr -signkey client.key -out client.crt -extfile open.cnf -extensions client_ext

cp server.crt ca
cp client.crt ca
