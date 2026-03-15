import http from 'http';

const postData = JSON.stringify({
  sql: 'SELECT * FROM PingCode_Project_YMP_需求_export_20251210153349_sheet1 LIMIT 5'
});

const options = {
  hostname: '127.0.0.1',
  port: 3000,
  path: '/api/execute',
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Content-Length': Buffer.byteLength(postData)
  }
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

req.write(postData);
req.end();
