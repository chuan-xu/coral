#!/bin/bash

openssl genrsa -out tx.key 2048
openssl genrsa -out hw.key 2048
openssl genrsa -out client.key 2048

# 生成证书请求
echo -e '\n\n\n\n\n\n\n\n\n' | openssl req -new -days 3650 -out tx.csr -key tx.key
echo -e '\n\n\n\n\n\n\n\n\n' | openssl req -new -days 3650 -out hw.csr -key hw.key
echo -e '\n\n\n\n\n\n\n\n\n' | openssl req -new -days 3650 -out client.csr -key client.key

# 生成证书
openssl x509 -req -in tx.csr -signkey tx.key -out tx.crt -extfile tls.cnf -extensions tx_ext
openssl x509 -req -in tx.csr -signkey hw.key -out hw.crt -extfile tls.cnf -extensions hw_ext
openssl x509 -req -in tx.csr -signkey client.key -out client.crt -extfile tls.cnf -extensions client_ext
