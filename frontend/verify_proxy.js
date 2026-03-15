import http from 'http';

const PORT = process.env.PORT || process.env.VITE_DEV_SERVER_PORT || '5174';
const options = {
  hostname: '127.0.0.1',
  port: Number(PORT),
  path: '/api/health',
  method: 'GET',
};

const req = http.request(options, (res) => {
  console.log(`STATUS: ${res.statusCode}`);
  let data = '';
  res.on('data', (chunk) => { data += chunk; });
  res.on('end', () => {
    console.log('BODY:', data);
  });
});

req.on('error', (e) => {
  console.error(`problem with request: ${e.message}`);
});

req.end();
