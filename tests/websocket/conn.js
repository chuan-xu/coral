import WebSocket  from 'ws';
import fs from 'fs';


const ws = new WebSocket("wss://server.test.com:9000", {
  cert: fs.readFileSync("../../cicd/self_sign_cert/client.crt"),
  key: fs.readFileSync("../../cicd/self_sign_cert/client.key"),
  ca: fs.readFileSync("../../cicd/self_sign_cert/server.crt")
});

ws.on('error', console.error);

ws.on('open', function open() {
  ws.send('something');
});

ws.on('message', function message(data) {
  console.log('received: %s', data);
});

ws.on('close', function close() {
  console.log('disconnected');
});