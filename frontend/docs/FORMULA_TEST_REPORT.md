# Formula Failure Retest Report

- Date: 2026-02-26
- Total Functions: 392
- OK: 392
- ERROR: 0

## 数据样例

### 基础数据 (A1:E5)

| A | B | C | D | E |
| --- | --- | --- | --- | --- |
| TextA | TextB | 2024-01-01 | 10 | 20 |
| Hello | World | 2024-01-02 | 30 | 40 |
| a | b | 2024-01-03 | 50 | 60 |
| foo | bar | 2024-01-04 | 70 | 80 |
| x | y | 2024-01-05 | 90 | 100 |

### 查找数据 (G1:I4)

| G | H | I |
| --- | --- | --- |
| key | value | flag |
| k1 | 100 | Y |
| k2 | 200 | N |
| k3 | 300 | Y |

### 概率数据 (A7:E8)

| A | B | C | D | E |
| --- | --- | --- | --- | --- |
| 0.1 | 0.2 | 0.3 | 0.4 | 0.5 |
| 0.6 | 0.7 | 0.8 | 0.9 | 0.95 |

### 现金流数据 (J1:J5)

| J |
| --- |
| -1000 |
| 200 |
| 300 |
| 400 |
| 500 |

### 日期序列 (K1:K5)

| K |
| --- |
| 1 |
| 31 |
| 61 |
| 92 |
| 122 |

## 分类结果

- Backend Only: XLOOKUP
- Both Backend & Local: AVERAGE, COUNT, COUNTA, MAX, MIN, SUM
- Local Only Count: 387

## 失败公式样例与原因

| Function | Formula | Error |
| --- | --- | --- |

## 备注

- 本报告为自动生成，失败项以当前样例与 HyperFormula 行为为准。
- 若仍出现参数数量错误，请继续补充 OVERRIDE_FORMULAS 对应公式样例。