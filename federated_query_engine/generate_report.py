import json
import csv
import os
import datetime

def generate_report():
    # 1. Load Application Metrics Snapshot
    app_metrics = {}
    if os.path.exists('metrics_final.json'):
        with open('metrics_final.json', 'r') as f:
            app_metrics = json.load(f)

    # 2. Load System Metrics (Time Series)
    system_metrics = {'timestamps': [], 'cpu': [], 'disk': [], 'memory': []}
    if os.path.exists('stress_test_metrics.csv'):
        with open('stress_test_metrics.csv', 'r') as f:
            reader = csv.reader(f)
            headers = next(reader, None)
            for row in reader:
                if len(row) < 4: continue
                try:
                    ts = row[0].split('.')[0] # Remove ms
                    system_metrics['timestamps'].append(ts)
                    system_metrics['cpu'].append(float(row[1]))
                    system_metrics['disk'].append(float(row[2]))
                    system_metrics['memory'].append(float(row[3]))
                except:
                    continue

    # 3. Load Application Metrics (Time Series)
    app_series = {
        'timestamps': [],
        'l1_hit_rate': [], 'l2_hit_rate': [],
        'l0_req_rate': [], 'l1_eviction_rate': [], 'l2_eviction_rate': [],
        'memory_usage': [],
        'l0_latency': [], 'l1_latency': [], 'l2_latency': []
    }
    
    if os.path.exists('app_metrics_series.csv'):
        with open('app_metrics_series.csv', 'r') as f:
            reader = csv.DictReader(f)
            prev_row = None
            start_time = None
            
            for row in reader:
                try:
                    ts = int(row['timestamp'])
                    if start_time is None: start_time = ts
                    rel_time = ts - start_time
                    
                    # Parse current values
                    curr = {k: float(v) for k, v in row.items() if k != 'timestamp'}
                    
                    if prev_row:
                        dt = 1.0 # Assume 1s interval
                        
                        # Rates (Delta)
                        l0_reqs = curr['l0_requests'] - prev_row['l0_requests']
                        l1_hits = curr['l1_hits'] - prev_row['l1_hits']
                        l1_misses = curr['l1_misses'] - prev_row['l1_misses']
                        l2_hits = curr['l2_hits'] - prev_row['l2_hits']
                        l2_misses = curr['l2_misses'] - prev_row['l2_misses']
                        l1_evicts = curr['l1_eviction_count'] - prev_row['l1_eviction_count']
                        l2_evicts = curr['l2_eviction_count'] - prev_row['l2_eviction_count']
                        
                        # Latencies (Avg for interval)
                        l0_lat_delta = curr['l0_exec_latency_us'] - prev_row['l0_exec_latency_us']
                        l1_lat_delta = curr['l1_io_latency_us'] - prev_row['l1_io_latency_us']
                        l2_lat_delta = curr['l2_read_latency_us'] - prev_row['l2_read_latency_us']
                        
                        # Calculations
                        app_series['timestamps'].append(f"{rel_time}s")
                        app_series['l0_req_rate'].append(l0_reqs)
                        app_series['l1_eviction_rate'].append(l1_evicts)
                        app_series['l2_eviction_rate'].append(l2_evicts)
                        app_series['memory_usage'].append(curr['memory_usage'] / 1024 / 1024) # MB
                        
                        # Hit Rates
                        l1_total = l1_hits + l1_misses
                        app_series['l1_hit_rate'].append((l1_hits / l1_total * 100) if l1_total > 0 else 0)
                        
                        l2_total = l2_hits + l2_misses
                        app_series['l2_hit_rate'].append((l2_hits / l2_total * 100) if l2_total > 0 else 0)
                        
                        # Latencies
                        app_series['l0_latency'].append((l0_lat_delta / l0_reqs / 1000) if l0_reqs > 0 else 0) # ms
                        app_series['l1_latency'].append((l1_lat_delta / l1_total) if l1_total > 0 else 0) # us
                        app_series['l2_latency'].append((l2_lat_delta / l2_hits) if l2_hits > 0 else 0) # us

                    prev_row = curr
                except Exception as e:
                    print(f"Skipping row: {e}")
                    continue

    # 4. Calculate Summary Stats
    l1_hits = app_metrics.get('l1_hits', 0)
    l1_misses = app_metrics.get('l1_misses', 0)
    l2_hits = app_metrics.get('l2_hits', 0)
    l2_misses = app_metrics.get('l2_misses', 0)
    
    l1_hit_rate = (l1_hits / (l1_hits + l1_misses) * 100) if (l1_hits + l1_misses) > 0 else 0
    l2_hit_rate = (l2_hits / (l2_hits + l2_misses) * 100) if (l2_hits + l2_misses) > 0 else 0

    avg_l1_latency = (app_metrics.get('l1_io_latency_us', 0) / (l1_hits + l1_misses)) if (l1_hits + l1_misses) > 0 else 0
    avg_l0_latency = (app_metrics.get('l0_exec_latency_us', 0) / app_metrics.get('l0_requests', 1)) if app_metrics.get('l0_requests', 0) > 0 else 0
    avg_l2_latency = (app_metrics.get('l2_read_latency_us', 0) / l2_hits) if l2_hits > 0 else 0

    # 5. Generate HTML
    html_content = f"""
<!DOCTYPE html>
<html>
<head>
    <title>Federated Query Engine - Detailed Performance Report</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body {{ font-family: 'Segoe UI', sans-serif; margin: 20px; background: #f4f7f6; color: #333; }}
        .container {{ max_width: 1400px; margin: 0 auto; background: white; padding: 30px; border-radius: 12px; box-shadow: 0 4px 12px rgba(0,0,0,0.05); }}
        h1 {{ color: #2c3e50; border-bottom: 3px solid #3498db; padding-bottom: 15px; margin-bottom: 30px; }}
        h2 {{ color: #34495e; margin-top: 40px; margin-bottom: 20px; font-size: 1.4em; border-left: 5px solid #3498db; padding-left: 10px; }}
        
        .metrics-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin-bottom: 30px; }}
        .metric-card {{ background: #fff; padding: 20px; border-radius: 8px; text-align: center; border: 1px solid #e1e4e8; box-shadow: 0 2px 4px rgba(0,0,0,0.02); transition: transform 0.2s; }}
        .metric-card:hover {{ transform: translateY(-3px); box-shadow: 0 4px 8px rgba(0,0,0,0.1); }}
        .metric-value {{ font-size: 28px; font-weight: 700; color: #2980b9; margin-bottom: 5px; }}
        .metric-label {{ color: #7f8c8d; font-size: 14px; text-transform: uppercase; letter-spacing: 0.5px; }}
        
        .chart-row {{ display: grid; grid-template-columns: 1fr 1fr; gap: 30px; margin-bottom: 30px; }}
        .chart-container {{ position: relative; height: 350px; background: white; padding: 15px; border-radius: 8px; border: 1px solid #eee; }}
        .full-width {{ grid-column: 1 / -1; }}
        
        table {{ width: 100%; border-collapse: collapse; margin-top: 20px; font-size: 14px; }}
        th, td {{ padding: 12px 15px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background-color: #f8f9fa; color: #2c3e50; font-weight: 600; }}
        tr:hover {{ background-color: #f1f1f1; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Performance Stress Test Report</h1>
        <p>Generated at: {datetime.datetime.now()}</p>

        <!-- Summary Cards -->
        <h2>Key Performance Indicators</h2>
        <div class="metrics-grid">
            <div class="metric-card">
                <div class="metric-value">{app_metrics.get('query_count', 0)}</div>
                <div class="metric-label">Total Queries</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{app_metrics.get('l0_requests', 0)}</div>
                <div class="metric-label">L0 Requests</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{l2_hit_rate:.1f}%</div>
                <div class="metric-label">L2 Hit Rate</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{l1_hit_rate:.1f}%</div>
                <div class="metric-label">L1 Hit Rate</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{avg_l2_latency:.0f} µs</div>
                <div class="metric-label">Avg L2 Latency</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{avg_l1_latency:.0f} µs</div>
                <div class="metric-label">Avg L1 Latency</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{avg_l0_latency/1000:.1f} ms</div>
                <div class="metric-label">Avg L0 Latency</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{app_metrics.get('l2_eviction_count', 0)}</div>
                <div class="metric-label">L2 Evictions</div>
            </div>
        </div>

        <!-- Charts Row 1 -->
        <div class="chart-row">
            <div class="chart-container">
                <canvas id="throughputChart"></canvas>
            </div>
            <div class="chart-container">
                <canvas id="latencyChart"></canvas>
            </div>
        </div>

        <!-- Charts Row 2 -->
        <div class="chart-row">
             <div class="chart-container">
                <canvas id="evictionChart"></canvas>
            </div>
            <div class="chart-container">
                <canvas id="memoryChart"></canvas>
            </div>
        </div>
        
        <!-- System Metrics -->
        <div class="chart-row">
            <div class="chart-container full-width">
                <canvas id="systemChart"></canvas>
            </div>
        </div>

        <!-- Detailed Table -->
        <h2>Detailed Metrics Snapshot</h2>
        <table>
            <thead>
                <tr>
                    <th>Metric Name</th>
                    <th>Value</th>
                    <th>Description</th>
                </tr>
            </thead>
            <tbody>
                {''.join([f"<tr><td>{k}</td><td>{v}</td><td>-</td></tr>" for k, v in app_metrics.items()])}
            </tbody>
        </table>
    </div>

    <script>
        // Common Options
        const commonOptions = {{
            responsive: true,
            maintainAspectRatio: false,
            interaction: {{ mode: 'index', intersect: false }},
            plugins: {{ legend: {{ position: 'top' }} }},
            scales: {{ x: {{ grid: {{ display: false }} }} }}
        }};

        // 1. Throughput Chart
        new Chart(document.getElementById('throughputChart'), {{
            type: 'line',
            data: {{
                labels: {json.dumps(app_series['timestamps'])},
                datasets: [
                    {{ label: 'L0 Requests/s', data: {json.dumps(app_series['l0_req_rate'])}, borderColor: '#e74c3c', fill: false }},
                    {{ label: 'L1 Hit Rate %', data: {json.dumps(app_series['l1_hit_rate'])}, borderColor: '#f1c40f', borderDash: [5, 5], yAxisID: 'y1' }}
                ]
            }},
            options: {{
                ...commonOptions,
                scales: {{
                    y: {{ title: {{ display: true, text: 'Requests/s' }} }},
                    y1: {{ position: 'right', max: 100, title: {{ display: true, text: 'Hit Rate %' }} }}
                }}
            }}
        }});

        // 2. Latency Chart
        new Chart(document.getElementById('latencyChart'), {{
            type: 'line',
            data: {{
                labels: {json.dumps(app_series['timestamps'])},
                datasets: [
                    {{ label: 'L0 Latency (ms)', data: {json.dumps(app_series['l0_latency'])}, borderColor: '#8e44ad', fill: false }},
                    {{ label: 'L1 Latency (us)', data: {json.dumps(app_series['l1_latency'])}, borderColor: '#2980b9', yAxisID: 'y1', fill: false }}
                ]
            }},
            options: {{
                ...commonOptions,
                scales: {{
                    y: {{ title: {{ display: true, text: 'L0 (ms)' }} }},
                    y1: {{ position: 'right', title: {{ display: true, text: 'L1 (us)' }} }}
                }}
            }}
        }});

        // 3. Eviction Chart
        new Chart(document.getElementById('evictionChart'), {{
            type: 'bar',
            data: {{
                labels: {json.dumps(app_series['timestamps'])},
                datasets: [
                    {{ label: 'L2 Evictions', data: {json.dumps(app_series['l2_eviction_rate'])}, backgroundColor: '#e67e22' }},
                    {{ label: 'L1 Evictions', data: {json.dumps(app_series['l1_eviction_rate'])}, backgroundColor: '#95a5a6' }}
                ]
            }},
            options: commonOptions
        }});

        // 4. Memory Chart
        new Chart(document.getElementById('memoryChart'), {{
            type: 'line',
            data: {{
                labels: {json.dumps(app_series['timestamps'])},
                datasets: [
                    {{ label: 'App Memory (MB)', data: {json.dumps(app_series['memory_usage'])}, borderColor: '#2ecc71', backgroundColor: 'rgba(46, 204, 113, 0.1)', fill: true }}
                ]
            }},
            options: commonOptions
        }});
        
        // 5. System Chart
        new Chart(document.getElementById('systemChart'), {{
            type: 'line',
            data: {{
                labels: {json.dumps(system_metrics['timestamps'])},
                datasets: [
                    {{ label: 'CPU %', data: {json.dumps(system_metrics['cpu'])}, borderColor: '#ff6384', tension: 0.1 }},
                    {{ label: 'Disk %', data: {json.dumps(system_metrics['disk'])}, borderColor: '#36a2eb', tension: 0.1 }}
                ]
            }},
            options: {{
                ...commonOptions,
                scales: {{ y: {{ beginAtZero: true, max: 100 }} }}
            }}
        }});
    </script>
</body>
</html>
    """

    with open('performance_report.html', 'w', encoding='utf-8') as f:
        f.write(html_content)
    
    print("Report generated: performance_report.html")

if __name__ == '__main__':
    generate_report()
