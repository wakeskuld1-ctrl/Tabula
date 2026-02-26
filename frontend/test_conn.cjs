const http = require('http');

const options = {
  hostname: '127.0.0.1',
  port: 5174,
  path: '/',
  method: 'GET',
  timeout: 5000
};

const req = http.request(options, (res) => {
  console.log(`STATUS: ${res.statusCode}`);
  res.setEncoding('utf8');
  res.on('data', (chunk) => {
    console.log(`BODY: ${chunk.substring(0, 100)}...`);
  });
});

req.on('error', (e) => {
  console.error(`problem with request: ${e.message}`);
});

req.on('timeout', () => {
    console.error('request timed out');
    req.destroy();
});

req.end();
