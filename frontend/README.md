# Frontend UI

This is the React frontend for Tabula.

## 公式样例（全量） / Full Formula Examples

<details>
<summary>点击展开 / Expand</summary>

<!-- FORMULA_DOCS_START -->
| 函数名 / Function | 语法 / Syntax | 示例 / Example | 参数说明 / Parameter Notes | 用途 / Purpose | 备注 / Notes |
| --- | --- | --- | --- | --- | --- |
| ABS | ABS(number) | =ABS(1) | number/数值 | 数学计算与取整 / Math and rounding | — |
| ACOS | ACOS(number) | =ACOS(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| ACOSH | ACOSH(number) | =ACOSH(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| ACOT | ACOT(number) | =ACOT(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| ACOTH | ACOTH(number) | =ACOTH(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| ADDRESS | ADDRESS(number1, number2, [number3], [condition4], [text5]) | =ADDRESS(1, 1) | number1/数值1, number2/数值2, number3/数值3 (optional/可选), condition4/条件4 (optional/可选), text5/文本5 (optional/可选) | 通用计算 / General calculation | — |
| AND | AND(condition1) | =AND(TRUE) | condition1/条件1 | 逻辑判断与条件处理 / Logical conditions | — |
| ARABIC | ARABIC(text) | =ARABIC("text") | text/文本 | 通用计算 / General calculation | — |
| ARRAY_CONSTRAIN | ARRAY_CONSTRAIN(range1, integer2, integer3) | =ARRAY_CONSTRAIN(A1:A5, 1, 1) | range1/范围1, integer2/整数2, integer3/整数3 | 数组与矩阵处理 / Array and matrix operations | — |
| ARRAYFORMULA | ARRAYFORMULA(value) | =ARRAYFORMULA(1) | value/值 | 逻辑判断与条件处理 / Logical conditions | — |
| ASIN | ASIN(number) | =ASIN(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| ASINH | ASINH(number) | =ASINH(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| ATAN | ATAN(number) | =ATAN(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| ATAN2 | ATAN2(number1, number2) | =ATAN2(1, 1) | number1/数值1, number2/数值2 | 三角函数与角度转换 / Trigonometry | — |
| ATANH | ATANH(number) | =ATANH(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| AVEDEV | AVEDEV(value) | =AVEDEV(1) | value/值 | 通用计算 / General calculation | — |
| AVERAGE | AVERAGE(range) | =AVERAGE(1) | range/范围 | 计算范围内数值平均值 / Average values in a range | — |
| AVERAGEA | AVERAGEA(value) | =AVERAGEA(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| AVERAGEIF | AVERAGEIF(range, criteria, [average_range]) | =AVERAGEIF(A1:A5, 1) | range/范围, criteria/条件, average_range/平均范围 (optional/可选) | 统计与汇总计算 / Statistical and aggregate calculations | — |
| BASE | BASE(number1, number2, [number3]) | =BASE(1, 1) | number1/数值1, number2/数值2, number3/数值3 (optional/可选) | 进制与位运算 / Base and bitwise operations | — |
| BESSELI | BESSELI(number1, number2) | =BESSELI(1, 1) | number1/数值1, number2/数值2 | 统计分布函数 / Statistical distributions | — |
| BESSELJ | BESSELJ(number1, number2) | =BESSELJ(1, 1) | number1/数值1, number2/数值2 | 统计分布函数 / Statistical distributions | — |
| BESSELK | BESSELK(number1, number2) | =BESSELK(1, 1) | number1/数值1, number2/数值2 | 统计分布函数 / Statistical distributions | — |
| BESSELY | BESSELY(number1, number2) | =BESSELY(1, 1) | number1/数值1, number2/数值2 | 统计分布函数 / Statistical distributions | — |
| BETA.DIST | BETA.DIST(number1, number2, number3, condition4, number5, number6) | =BETA.DIST(1, 1, 1, TRUE, 1, 1) | number1/数值1, number2/数值2, number3/数值3, condition4/条件4, number5/数值5, number6/数值6 | 统计分布函数 / Statistical distributions | — |
| BETA.INV | BETA.INV(number1, number2, number3, number4, number5) | =BETA.INV(1, 1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, number4/数值4, number5/数值5 | 统计分布函数 / Statistical distributions | — |
| BETADIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| BETAINV | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| BIN2DEC | BIN2DEC(text) | =BIN2DEC("text") | text/文本 | 进制与位运算 / Base and bitwise operations | — |
| BIN2HEX | BIN2HEX(text1, [number2]) | =BIN2HEX("text") | text1/文本1, number2/数值2 (optional/可选) | 进制与位运算 / Base and bitwise operations | — |
| BIN2OCT | BIN2OCT(text1, [number2]) | =BIN2OCT("text") | text1/文本1, number2/数值2 (optional/可选) | 进制与位运算 / Base and bitwise operations | — |
| BINOM.DIST | BINOM.DIST(number1, number2, number3, condition4) | =BINOM.DIST(1, 1, 1, TRUE) | number1/数值1, number2/数值2, number3/数值3, condition4/条件4 | 统计分布函数 / Statistical distributions | — |
| BINOM.INV | BINOM.INV(number1, number2, number3) | =BINOM.INV(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 统计分布函数 / Statistical distributions | — |
| BINOMDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| BITAND | BITAND(integer1, integer2) | =BITAND(1, 1) | integer1/整数1, integer2/整数2 | 逻辑判断与条件处理 / Logical conditions | — |
| BITLSHIFT | BITLSHIFT(integer1, integer2) | =BITLSHIFT(1, 1) | integer1/整数1, integer2/整数2 | 逻辑判断与条件处理 / Logical conditions | — |
| BITOR | BITOR(integer1, integer2) | =BITOR(1, 1) | integer1/整数1, integer2/整数2 | 逻辑判断与条件处理 / Logical conditions | — |
| BITRSHIFT | BITRSHIFT(integer1, integer2) | =BITRSHIFT(1, 1) | integer1/整数1, integer2/整数2 | 逻辑判断与条件处理 / Logical conditions | — |
| BITXOR | BITXOR(integer1, integer2) | =BITXOR(1, 1) | integer1/整数1, integer2/整数2 | 逻辑判断与条件处理 / Logical conditions | — |
| CEILING | CEILING(number1, number2) | =CEILING(1, 1) | number1/数值1, number2/数值2 | 数学计算与取整 / Math and rounding | — |
| CEILING.MATH | CEILING.MATH(number1, number2, number3) | =CEILING.MATH(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 数学计算与取整 / Math and rounding | — |
| CEILING.PRECISE | CEILING.PRECISE(number1, number2) | =CEILING.PRECISE(1, 1) | number1/数值1, number2/数值2 | 数学计算与取整 / Math and rounding | — |
| CHAR | CHAR(number) | =CHAR(1) | number/数值 | 通用计算 / General calculation | — |
| CHIDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CHIDISTRT | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CHIINV | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CHIINVRT | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CHISQ.DIST | CHISQ.DIST(number1, number2, condition3) | =CHISQ.DIST(1, 1, TRUE) | number1/数值1, number2/数值2, condition3/条件3 | 统计分布函数 / Statistical distributions | — |
| CHISQ.DIST.RT | CHISQ.DIST.RT(number1, number2) | =CHISQ.DIST.RT(1, 1) | number1/数值1, number2/数值2 | 统计分布函数 / Statistical distributions | — |
| CHISQ.INV | CHISQ.INV(number1, number2) | =CHISQ.INV(1, 1) | number1/数值1, number2/数值2 | 统计分布函数 / Statistical distributions | — |
| CHISQ.INV.RT | CHISQ.INV.RT(number1, number2) | =CHISQ.INV.RT(1, 1) | number1/数值1, number2/数值2 | 统计分布函数 / Statistical distributions | — |
| CHISQ.TEST | CHISQ.TEST(range1, range2) | =CHISQ.TEST(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 统计分布函数 / Statistical distributions | — |
| CHITEST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CHOOSE | CHOOSE(integer1, value2) | =CHOOSE(1, 1) | integer1/整数1, value2/值2 | 逻辑判断与条件处理 / Logical conditions | — |
| CLEAN | CLEAN(text) | =CLEAN("text") | text/文本 | 通用计算 / General calculation | — |
| CODE | CODE(text) | =CODE("text") | text/文本 | 通用计算 / General calculation | — |
| COLUMN | COLUMN([value]) | =COLUMN() | value/值 (optional/可选) | 通用计算 / General calculation | — |
| COLUMNS | COLUMNS(range) | =COLUMNS(A1:A5) | range/范围 | 通用计算 / General calculation | — |
| COMBIN | COMBIN(number1, number2) | =COMBIN(1, 1) | number1/数值1, number2/数值2 | 进制与位运算 / Base and bitwise operations | — |
| COMBINA | COMBINA(number1, number2) | =COMBINA(1, 1) | number1/数值1, number2/数值2 | 进制与位运算 / Base and bitwise operations | — |
| COMPLEX | COMPLEX(number1, number2, text3) | =COMPLEX(1, 1, "text") | number1/数值1, number2/数值2, text3/文本3 | 通用计算 / General calculation | — |
| CONCATENATE | CONCATENATE(text1) | =CONCATENATE("text") | text1/文本1 | 文本处理 / Text processing | — |
| CONFIDENCE | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CONFIDENCE.NORM | CONFIDENCE.NORM(number1, number2, number3) | =CONFIDENCE.NORM(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 逻辑判断与条件处理 / Logical conditions | — |
| CONFIDENCE.T | CONFIDENCE.T(number1, number2, number3) | =CONFIDENCE.T(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 通用计算 / General calculation | — |
| CORREL | CORREL(range1, range2) | =CORREL(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| COS | COS(number) | =COS(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| COSH | COSH(number) | =COSH(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| COT | COT(number) | =COT(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| COTH | COTH(number) | =COTH(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| COUNT | COUNT(range) | =COUNT(1) | range/范围 | 统计范围内数值个数 / Count numeric values in a range | — |
| COUNTA | COUNTA(value) | =COUNTA(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| COUNTBLANK | COUNTBLANK(value) | =COUNTBLANK(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| COUNTIF | COUNTIF(range, criteria) | =COUNTIF(A1:A5, 1) | range/范围, criteria/条件 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| COUNTIFS | COUNTIFS(criteria_range1, criteria1) | =COUNTIFS(A1:A5, 1) | criteria_range1/条件范围1, criteria1/条件1 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| COUNTUNIQUE | COUNTUNIQUE(value) | =COUNTUNIQUE(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| COVAR | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| COVARIANCE.P | COVARIANCE.P(range1, range2) | =COVARIANCE.P(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| COVARIANCE.S | COVARIANCE.S(range1, range2) | =COVARIANCE.S(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| COVARIANCEP | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| COVARIANCES | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CRITBINOM | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CSC | CSC(number) | =CSC(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| CSCH | CSCH(number) | =CSCH(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| CUMIPMT | CUMIPMT(number1, number2, number3, integer4, integer5, integer6) | =CUMIPMT(1, 1, 1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, integer4/整数4, integer5/整数5, integer6/整数6 | 财务计算 / Financial calculations | — |
| CUMPRINC | CUMPRINC(number1, number2, number3, integer4, integer5, integer6) | =CUMPRINC(1, 1, 1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, integer4/整数4, integer5/整数5, integer6/整数6 | 财务计算 / Financial calculations | — |
| DATE | DATE(year, month, day) | =DATE(1, 1, 1) | year/年, month/月, day/日 | 生成日期值 / Create a date value | — |
| DATEDIF | DATEDIF(number1, number2, text3) | =DATEDIF(1, 1, "text") | number1/数值1, number2/数值2, text3/文本3 | 日期与时间处理 / Date and time handling | — |
| DATEVALUE | DATEVALUE(text) | =DATEVALUE("text") | text/文本 | 日期与时间处理 / Date and time handling | — |
| DAY | DAY(number) | =DAY(1) | number/数值 | 日期与时间处理 / Date and time handling | — |
| DAYS | DAYS(number1, number2) | =DAYS(1, 1) | number1/数值1, number2/数值2 | 日期与时间处理 / Date and time handling | — |
| DAYS360 | DAYS360(number1, number2, condition3) | =DAYS360(1, 1, TRUE) | number1/数值1, number2/数值2, condition3/条件3 | 日期与时间处理 / Date and time handling | — |
| DB | DB(number1, number2, integer3, integer4, integer5) | =DB(1, 1, 1, 1, 1) | number1/数值1, number2/数值2, integer3/整数3, integer4/整数4, integer5/整数5 | 财务计算 / Financial calculations | — |
| DDB | DDB(number1, number2, integer3, number4, number5) | =DDB(1, 1, 1, 1, 1) | number1/数值1, number2/数值2, integer3/整数3, number4/数值4, number5/数值5 | 财务计算 / Financial calculations | — |
| DEC2BIN | DEC2BIN(number1, [number2]) | =DEC2BIN(1) | number1/数值1, number2/数值2 (optional/可选) | 进制与位运算 / Base and bitwise operations | — |
| DEC2HEX | DEC2HEX(number1, [number2]) | =DEC2HEX(1) | number1/数值1, number2/数值2 (optional/可选) | 进制与位运算 / Base and bitwise operations | — |
| DEC2OCT | DEC2OCT(number1, [number2]) | =DEC2OCT(1) | number1/数值1, number2/数值2 (optional/可选) | 进制与位运算 / Base and bitwise operations | — |
| DECIMAL | DECIMAL(text1, number2) | =DECIMAL("text", 1) | text1/文本1, number2/数值2 | 进制与位运算 / Base and bitwise operations | — |
| DEGREES | DEGREES(number) | =DEGREES(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| DELTA | DELTA(number1, number2) | =DELTA(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| DEVSQ | DEVSQ(value) | =DEVSQ(1) | value/值 | 通用计算 / General calculation | — |
| DOLLARDE | DOLLARDE(number1, number2) | =DOLLARDE(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| DOLLARFR | DOLLARFR(number1, number2) | =DOLLARFR(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| EDATE | EDATE(number1, number2) | =EDATE(1, 1) | number1/数值1, number2/数值2 | 日期与时间处理 / Date and time handling | — |
| EFFECT | EFFECT(number1, number2) | =EFFECT(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| EOMONTH | EOMONTH(number1, number2) | =EOMONTH(1, 1) | number1/数值1, number2/数值2 | 日期与时间处理 / Date and time handling | — |
| ERF | ERF(number1, [number2]) | =ERF(1) | number1/数值1, number2/数值2 (optional/可选) | 统计分布函数 / Statistical distributions | — |
| ERFC | ERFC(number) | =ERFC(1) | number/数值 | 统计分布函数 / Statistical distributions | — |
| EVEN | EVEN(number) | =EVEN(1) | number/数值 | 通用计算 / General calculation | — |
| EXACT | EXACT(text1, text2) | =EXACT("text", "text") | text1/文本1, text2/文本2 | 通用计算 / General calculation | — |
| EXP | EXP(number) | =EXP(1) | number/数值 | 数学计算与取整 / Math and rounding | — |
| EXPON.DIST | EXPON.DIST(number1, number2, condition3) | =EXPON.DIST(1, 1, TRUE) | number1/数值1, number2/数值2, condition3/条件3 | 数学计算与取整 / Math and rounding | — |
| EXPONDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| F.DIST | F.DIST(number1, number2, number3, condition4) | =F.DIST(1, 1, 1, TRUE) | number1/数值1, number2/数值2, number3/数值3, condition4/条件4 | 统计分布函数 / Statistical distributions | — |
| F.DIST.RT | F.DIST.RT(number1, number2, number3) | =F.DIST.RT(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 统计分布函数 / Statistical distributions | — |
| F.INV | F.INV(number1, number2, number3) | =F.INV(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 通用计算 / General calculation | — |
| F.INV.RT | F.INV.RT(number1, number2, number3) | =F.INV.RT(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 通用计算 / General calculation | — |
| F.TEST | F.TEST(range1, range2) | =F.TEST(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 通用计算 / General calculation | — |
| FACT | FACT(number) | =FACT(1) | number/数值 | 通用计算 / General calculation | — |
| FACTDOUBLE | FACTDOUBLE(number) | =FACTDOUBLE(1) | number/数值 | 通用计算 / General calculation | — |
| FALSE | FALSE() | =FALSE() | 无参数 / No parameters | 通用计算 / General calculation | — |
| FDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| FDISTRT | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| FILTER | FILTER(range1, range2) | =FILTER(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 数组与矩阵处理 / Array and matrix operations | — |
| FIND | FIND(find_text, within_text, start_num) | =FIND("text", "text", 1) | find_text/查找文本, within_text/范围文本, start_num/起始位置 | 文本处理 / Text processing | — |
| FINV | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| FINVRT | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| FISHER | FISHER(number) | =FISHER(1) | number/数值 | 通用计算 / General calculation | — |
| FISHERINV | FISHERINV(number) | =FISHERINV(1) | number/数值 | 通用计算 / General calculation | — |
| FLOOR | FLOOR(number1, number2) | =FLOOR(1, 1) | number1/数值1, number2/数值2 | 逻辑判断与条件处理 / Logical conditions | — |
| FLOOR.MATH | FLOOR.MATH(number1, number2, number3) | =FLOOR.MATH(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 逻辑判断与条件处理 / Logical conditions | — |
| FLOOR.PRECISE | FLOOR.PRECISE(number1, number2) | =FLOOR.PRECISE(1, 1) | number1/数值1, number2/数值2 | 逻辑判断与条件处理 / Logical conditions | — |
| FORMULATEXT | FORMULATEXT(value) | =FORMULATEXT(1) | value/值 | 文本处理 / Text processing | — |
| FTEST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| FV | FV(number1, number2, number3, number4, number5) | =FV(1, 1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, number4/数值4, number5/数值5 | 财务计算 / Financial calculations | — |
| FVSCHEDULE | FVSCHEDULE(number1, range2) | =FVSCHEDULE(1, A1:A5) | number1/数值1, range2/范围2 | 财务计算 / Financial calculations | — |
| GAMMA | GAMMA(number) | =GAMMA(1) | number/数值 | 统计分布函数 / Statistical distributions | — |
| GAMMA.DIST | GAMMA.DIST(number1, number2, number3, condition4) | =GAMMA.DIST(1, 1, 1, TRUE) | number1/数值1, number2/数值2, number3/数值3, condition4/条件4 | 统计分布函数 / Statistical distributions | — |
| GAMMA.INV | GAMMA.INV(number1, number2, number3) | =GAMMA.INV(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 统计分布函数 / Statistical distributions | — |
| GAMMADIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| GAMMAINV | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| GAMMALN | GAMMALN(number) | =GAMMALN(1) | number/数值 | 数学计算与取整 / Math and rounding | — |
| GAMMALN.PRECISE | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| GAUSS | GAUSS(number) | =GAUSS(1) | number/数值 | 通用计算 / General calculation | — |
| GCD | GCD(value) | =GCD(1) | value/值 | 通用计算 / General calculation | — |
| GEOMEAN | GEOMEAN(value) | =GEOMEAN(1) | value/值 | 通用计算 / General calculation | — |
| HARMEAN | HARMEAN(value) | =HARMEAN(1) | value/值 | 通用计算 / General calculation | — |
| HEX2BIN | HEX2BIN(text1, [number2]) | =HEX2BIN("text") | text1/文本1, number2/数值2 (optional/可选) | 进制与位运算 / Base and bitwise operations | — |
| HEX2DEC | HEX2DEC(text) | =HEX2DEC("text") | text/文本 | 进制与位运算 / Base and bitwise operations | — |
| HEX2OCT | HEX2OCT(text1, [number2]) | =HEX2OCT("text") | text1/文本1, number2/数值2 (optional/可选) | 进制与位运算 / Base and bitwise operations | — |
| HF.ADD | HF.ADD(number1, number2) | =HF.ADD(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| HF.CONCAT | HF.CONCAT(text1, text2) | =HF.CONCAT("text", "text") | text1/文本1, text2/文本2 | 文本处理 / Text processing | — |
| HF.DIVIDE | HF.DIVIDE(number1, number2) | =HF.DIVIDE(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| HF.EQ | HF.EQ(value1, value2) | =HF.EQ(1, 1) | value1/值1, value2/值2 | 通用计算 / General calculation | — |
| HF.GT | HF.GT(value1, value2) | =HF.GT(1, 1) | value1/值1, value2/值2 | 通用计算 / General calculation | — |
| HF.GTE | HF.GTE(value1, value2) | =HF.GTE(1, 1) | value1/值1, value2/值2 | 通用计算 / General calculation | — |
| HF.LT | HF.LT(value1, value2) | =HF.LT(1, 1) | value1/值1, value2/值2 | 通用计算 / General calculation | — |
| HF.LTE | HF.LTE(value1, value2) | =HF.LTE(1, 1) | value1/值1, value2/值2 | 通用计算 / General calculation | — |
| HF.MINUS | HF.MINUS(number1, number2) | =HF.MINUS(1, 1) | number1/数值1, number2/数值2 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| HF.MULTIPLY | HF.MULTIPLY(number1, number2) | =HF.MULTIPLY(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| HF.NE | HF.NE(value1, value2) | =HF.NE(1, 1) | value1/值1, value2/值2 | 通用计算 / General calculation | — |
| HF.POW | HF.POW(number1, number2) | =HF.POW(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| HF.UMINUS | HF.UMINUS(number) | =HF.UMINUS(1) | number/数值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| HF.UNARY_PERCENT | HF.UNARY_PERCENT(number) | =HF.UNARY_PERCENT(1) | number/数值 | 通用计算 / General calculation | — |
| HF.UPLUS | HF.UPLUS(number) | =HF.UPLUS(1) | number/数值 | 通用计算 / General calculation | — |
| HLOOKUP | HLOOKUP(lookup_value, table, return_row, join_row) | =HLOOKUP(1, A1:A5, 1, TRUE) | lookup_value/查找值, table/表, return_row/返回行, join_row/匹配行 | 查找与引用数据 / Lookup and reference data | — |
| HOUR | HOUR(number) | =HOUR(1) | number/数值 | 日期与时间处理 / Date and time handling | — |
| HYPERLINK | HYPERLINK(text1, [text2]) | =HYPERLINK("text") | text1/文本1, text2/文本2 (optional/可选) | 通用计算 / General calculation | — |
| HYPGEOM.DIST | HYPGEOM.DIST(number1, number2, number3, number4, condition5) | =HYPGEOM.DIST(1, 1, 1, 1, TRUE) | number1/数值1, number2/数值2, number3/数值3, number4/数值4, condition5/条件5 | 统计分布函数 / Statistical distributions | — |
| HYPGEOMDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| IF | IF(condition, value_if_true, value_if_false) | =IF(TRUE, 1, 1) | condition/条件, value_if_true/真值, value_if_false/假值 | 按条件返回不同结果 / Return different results based on a condition | — |
| IFERROR | IFERROR(value, value_if_error) | =IFERROR(1, 1) | value/值, value_if_error/出错替代值 | 逻辑判断与条件处理 / Logical conditions | — |
| IFNA | IFNA(value, value_if_na) | =IFNA(1, 1) | value/值, value_if_na/NA替代值 | 逻辑判断与条件处理 / Logical conditions | — |
| IFS | IFS(condition1, value1) | =IFS(TRUE, 1) | condition1/条件1, value1/值1 | 逻辑判断与条件处理 / Logical conditions | — |
| IMABS | IMABS(complex) | =IMABS("1+2i") | complex/复数 | 数学计算与取整 / Math and rounding | — |
| IMAGINARY | IMAGINARY(complex) | =IMAGINARY("1+2i") | complex/复数 | 通用计算 / General calculation | — |
| IMARGUMENT | IMARGUMENT(complex) | =IMARGUMENT("1+2i") | complex/复数 | 通用计算 / General calculation | — |
| IMCONJUGATE | IMCONJUGATE(complex) | =IMCONJUGATE("1+2i") | complex/复数 | 通用计算 / General calculation | — |
| IMCOS | IMCOS(complex) | =IMCOS("1+2i") | complex/复数 | 三角函数与角度转换 / Trigonometry | — |
| IMCOSH | IMCOSH(complex) | =IMCOSH("1+2i") | complex/复数 | 三角函数与角度转换 / Trigonometry | — |
| IMCOT | IMCOT(complex) | =IMCOT("1+2i") | complex/复数 | 三角函数与角度转换 / Trigonometry | — |
| IMCSC | IMCSC(complex) | =IMCSC("1+2i") | complex/复数 | 三角函数与角度转换 / Trigonometry | — |
| IMCSCH | IMCSCH(complex) | =IMCSCH("1+2i") | complex/复数 | 三角函数与角度转换 / Trigonometry | — |
| IMDIV | IMDIV(complex1, complex2) | =IMDIV("1+2i", "1+2i") | complex1/复数1, complex2/复数2 | 通用计算 / General calculation | — |
| IMEXP | IMEXP(complex) | =IMEXP("1+2i") | complex/复数 | 数学计算与取整 / Math and rounding | — |
| IMLN | IMLN(complex) | =IMLN("1+2i") | complex/复数 | 数学计算与取整 / Math and rounding | — |
| IMLOG10 | IMLOG10(complex) | =IMLOG10("1+2i") | complex/复数 | 数学计算与取整 / Math and rounding | — |
| IMLOG2 | IMLOG2(complex) | =IMLOG2("1+2i") | complex/复数 | 数学计算与取整 / Math and rounding | — |
| IMPOWER | IMPOWER(complex1, number2) | =IMPOWER("1+2i", 1) | complex1/复数1, number2/数值2 | 数学计算与取整 / Math and rounding | — |
| IMPRODUCT | IMPRODUCT(value) | =IMPRODUCT(1) | value/值 | 通用计算 / General calculation | — |
| IMREAL | IMREAL(complex) | =IMREAL("1+2i") | complex/复数 | 通用计算 / General calculation | — |
| IMSEC | IMSEC(complex) | =IMSEC("1+2i") | complex/复数 | 三角函数与角度转换 / Trigonometry | — |
| IMSECH | IMSECH(complex) | =IMSECH("1+2i") | complex/复数 | 三角函数与角度转换 / Trigonometry | — |
| IMSIN | IMSIN(complex) | =IMSIN("1+2i") | complex/复数 | 三角函数与角度转换 / Trigonometry | — |
| IMSINH | IMSINH(complex) | =IMSINH("1+2i") | complex/复数 | 三角函数与角度转换 / Trigonometry | — |
| IMSQRT | IMSQRT(complex) | =IMSQRT("1+2i") | complex/复数 | 数学计算与取整 / Math and rounding | — |
| IMSUB | IMSUB(complex1, complex2) | =IMSUB("1+2i", "1+2i") | complex1/复数1, complex2/复数2 | 通用计算 / General calculation | — |
| IMSUM | IMSUM(value) | =IMSUM(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| IMTAN | IMTAN(complex) | =IMTAN("1+2i") | complex/复数 | 三角函数与角度转换 / Trigonometry | — |
| INDEX | INDEX(range, row, column) | =INDEX(A1:A5, 1, 1) | range/范围, row/行号, column/列号 | 返回范围内指定行列的值 / Return the value at a specific row/column | — |
| INT | INT(number) | =INT(1) | number/数值 | 数学计算与取整 / Math and rounding | — |
| INTERVAL | INTERVAL(number) | =INTERVAL(1) | number/数值 | 数学计算与取整 / Math and rounding | — |
| IPMT | IPMT(number1, number2, number3, number4, number5, number6) | =IPMT(1, 1, 1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, number4/数值4, number5/数值5, number6/数值6 | 财务计算 / Financial calculations | — |
| IRR | IRR(range1, number2) | =IRR(A1:A5, 1) | range1/范围1, number2/数值2 | 财务计算 / Financial calculations | — |
| ISBINARY | ISBINARY(text) | =ISBINARY("text") | text/文本 | 进制与位运算 / Base and bitwise operations | — |
| ISBLANK | ISBLANK(value) | =ISBLANK(1) | value/值 | 通用计算 / General calculation | — |
| ISERR | ISERR(value) | =ISERR(1) | value/值 | 通用计算 / General calculation | — |
| ISERROR | ISERROR(value) | =ISERROR(1) | value/值 | 逻辑判断与条件处理 / Logical conditions | — |
| ISEVEN | ISEVEN(number) | =ISEVEN(1) | number/数值 | 通用计算 / General calculation | — |
| ISFORMULA | ISFORMULA(value) | =ISFORMULA(1) | value/值 | 逻辑判断与条件处理 / Logical conditions | — |
| ISLOGICAL | ISLOGICAL(value) | =ISLOGICAL(1) | value/值 | 数学计算与取整 / Math and rounding | — |
| ISNA | ISNA(value) | =ISNA(1) | value/值 | 错误检测与处理 / Error handling | — |
| ISNONTEXT | ISNONTEXT(value) | =ISNONTEXT(1) | value/值 | 文本处理 / Text processing | — |
| ISNUMBER | ISNUMBER(value) | =ISNUMBER(1) | value/值 | 通用计算 / General calculation | — |
| ISO.CEILING | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| ISODD | ISODD(number) | =ISODD(1) | number/数值 | 通用计算 / General calculation | — |
| ISOWEEKNUM | ISOWEEKNUM(number) | =ISOWEEKNUM(1) | number/数值 | 日期与时间处理 / Date and time handling | — |
| ISPMT | ISPMT(number1, number2, number3, number4) | =ISPMT(1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, number4/数值4 | 财务计算 / Financial calculations | — |
| ISREF | ISREF(value) | =ISREF(1) | value/值 | 通用计算 / General calculation | — |
| ISTEXT | ISTEXT(value) | =ISTEXT(1) | value/值 | 文本处理 / Text processing | — |
| LARGE | LARGE(range1, number2) | =LARGE(A1:A5, 1) | range1/范围1, number2/数值2 | 通用计算 / General calculation | — |
| LCM | LCM(value) | =LCM(1) | value/值 | 通用计算 / General calculation | — |
| LEFT | LEFT(text, num_chars) | =LEFT("text", 1) | text/文本, num_chars/字符数 | 从左侧截取文本 / Extract text from the left | — |
| LEN | LEN(text) | =LEN("text") | text/文本 | 文本处理 / Text processing | — |
| LN | LN(number) | =LN(1) | number/数值 | 数学计算与取整 / Math and rounding | — |
| LOG | LOG(number1, number2) | =LOG(1, 1) | number1/数值1, number2/数值2 | 数学计算与取整 / Math and rounding | — |
| LOG10 | LOG10(number) | =LOG10(1) | number/数值 | 数学计算与取整 / Math and rounding | — |
| LOGINV | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| LOGNORM.DIST | LOGNORM.DIST(number1, number2, number3, condition4) | =LOGNORM.DIST(1, 1, 1, TRUE) | number1/数值1, number2/数值2, number3/数值3, condition4/条件4 | 逻辑判断与条件处理 / Logical conditions | — |
| LOGNORM.INV | LOGNORM.INV(number1, number2, number3) | =LOGNORM.INV(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 逻辑判断与条件处理 / Logical conditions | — |
| LOGNORMDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| LOGNORMINV | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| LOWER | LOWER(text) | =LOWER("text") | text/文本 | 文本处理 / Text processing | — |
| MATCH | MATCH(lookup_value, range, match_type) | =MATCH(1, A1:A5, 1) | lookup_value/查找值, range/范围, match_type/匹配方式 | 返回查找值在范围内的位置 / Return position of a lookup value | — |
| MAX | MAX(range) | =MAX(1) | range/范围 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| MAXA | MAXA(value) | =MAXA(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| MAXIFS | MAXIFS(range1, range2, value3) | =MAXIFS(A1:A5, A1:A5, 1) | range1/范围1, range2/范围2, value3/值3 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| MAXPOOL | MAXPOOL(range1, number2, [number3]) | =MAXPOOL(A1:A5, 1) | range1/范围1, number2/数值2, number3/数值3 (optional/可选) | 统计与汇总计算 / Statistical and aggregate calculations | — |
| MEDIAN | MEDIAN(range) | =MEDIAN(1) | range/范围 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| MEDIANPOOL | MEDIANPOOL(range1, number2, [number3]) | =MEDIANPOOL(A1:A5, 1) | range1/范围1, number2/数值2, number3/数值3 (optional/可选) | 统计与汇总计算 / Statistical and aggregate calculations | — |
| MID | MID(text, start_num, num_chars) | =MID("text", 1, 1) | text/文本, start_num/起始位置, num_chars/字符数 | 从中间截取文本 / Extract text from the middle | — |
| MIN | MIN(range) | =MIN(1) | range/范围 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| MINA | MINA(value) | =MINA(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| MINIFS | MINIFS(range1, range2, value3) | =MINIFS(A1:A5, A1:A5, 1) | range1/范围1, range2/范围2, value3/值3 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| MINUTE | MINUTE(number) | =MINUTE(1) | number/数值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| MIRR | MIRR(range1, number2, number3) | =MIRR(A1:A5, 1, 1) | range1/范围1, number2/数值2, number3/数值3 | 财务计算 / Financial calculations | — |
| MMULT | MMULT(range1, range2) | =MMULT(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 数组与矩阵处理 / Array and matrix operations | — |
| MOD | MOD(number1, number2) | =MOD(1, 1) | number1/数值1, number2/数值2 | 数学计算与取整 / Math and rounding | — |
| MONTH | MONTH(number) | =MONTH(1) | number/数值 | 日期与时间处理 / Date and time handling | — |
| MROUND | MROUND(number1, number2) | =MROUND(1, 1) | number1/数值1, number2/数值2 | 数学计算与取整 / Math and rounding | — |
| MULTINOMIAL | MULTINOMIAL(number) | =MULTINOMIAL(1) | number/数值 | 通用计算 / General calculation | — |
| N | N(value) | =N(1) | value/值 | 通用计算 / General calculation | — |
| NA | NA() | =NA() | 无参数 / No parameters | 通用计算 / General calculation | — |
| NEGBINOM.DIST | NEGBINOM.DIST(number1, number2, number3, condition4) | =NEGBINOM.DIST(1, 1, 1, TRUE) | number1/数值1, number2/数值2, number3/数值3, condition4/条件4 | 统计分布函数 / Statistical distributions | — |
| NEGBINOMDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| NETWORKDAYS | NETWORKDAYS(number1, number2, [range3]) | =NETWORKDAYS(1, 1) | number1/数值1, number2/数值2, range3/范围3 (optional/可选) | 日期与时间处理 / Date and time handling | — |
| NETWORKDAYS.INTL | NETWORKDAYS.INTL(number1, number2, value3, [range4]) | =NETWORKDAYS.INTL(1, 1, 1) | number1/数值1, number2/数值2, value3/值3, range4/范围4 (optional/可选) | 日期与时间处理 / Date and time handling | — |
| NOMINAL | NOMINAL(number1, number2) | =NOMINAL(1, 1) | number1/数值1, number2/数值2 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| NORM.DIST | NORM.DIST(number1, number2, number3, condition4) | =NORM.DIST(1, 1, 1, TRUE) | number1/数值1, number2/数值2, number3/数值3, condition4/条件4 | 逻辑判断与条件处理 / Logical conditions | — |
| NORM.INV | NORM.INV(number1, number2, number3) | =NORM.INV(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 逻辑判断与条件处理 / Logical conditions | — |
| NORM.S.DIST | NORM.S.DIST(number1, condition2) | =NORM.S.DIST(1, TRUE) | number1/数值1, condition2/条件2 | 逻辑判断与条件处理 / Logical conditions | — |
| NORM.S.INV | NORM.S.INV(number) | =NORM.S.INV(1) | number/数值 | 逻辑判断与条件处理 / Logical conditions | — |
| NORMDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| NORMINV | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| NORMSDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| NORMSINV | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| NOT | NOT(condition) | =NOT(TRUE) | condition/条件 | 逻辑判断与条件处理 / Logical conditions | — |
| NOW | NOW() | =NOW() | 无参数 / No parameters | 返回当前日期时间 / Return current date and time | — |
| NPER | NPER(number1, number2, number3, number4, number5) | =NPER(1, 1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, number4/数值4, number5/数值5 | 财务计算 / Financial calculations | — |
| NPV | NPV(number1, value2) | =NPV(1, 1) | number1/数值1, value2/值2 | 财务计算 / Financial calculations | — |
| OCT2BIN | OCT2BIN(text1, [number2]) | =OCT2BIN("text") | text1/文本1, number2/数值2 (optional/可选) | 进制与位运算 / Base and bitwise operations | — |
| OCT2DEC | OCT2DEC(text) | =OCT2DEC("text") | text/文本 | 进制与位运算 / Base and bitwise operations | — |
| OCT2HEX | OCT2HEX(text1, [number2]) | =OCT2HEX("text") | text1/文本1, number2/数值2 (optional/可选) | 进制与位运算 / Base and bitwise operations | — |
| ODD | ODD(number) | =ODD(1) | number/数值 | 通用计算 / General calculation | — |
| OFFSET | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| OR | OR(condition1) | =OR(TRUE) | condition1/条件1 | 逻辑判断与条件处理 / Logical conditions | — |
| PDURATION | PDURATION(number1, number2, number3) | =PDURATION(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 通用计算 / General calculation | — |
| PEARSON | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| PHI | PHI(number) | =PHI(1) | number/数值 | 通用计算 / General calculation | — |
| PI | PI() | =PI() | 无参数 / No parameters | 三角函数与角度转换 / Trigonometry | — |
| PMT | PMT(number1, number2, number3, number4, number5) | =PMT(1, 1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, number4/数值4, number5/数值5 | 财务计算 / Financial calculations | — |
| POISSON | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| POISSON.DIST | POISSON.DIST(number1, number2, condition3) | =POISSON.DIST(1, 1, TRUE) | number1/数值1, number2/数值2, condition3/条件3 | 统计分布函数 / Statistical distributions | — |
| POISSONDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| POWER | POWER(number1, number2) | =POWER(1, 1) | number1/数值1, number2/数值2 | 数学计算与取整 / Math and rounding | — |
| PPMT | PPMT(number1, number2, number3, number4, number5, number6) | =PPMT(1, 1, 1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, number4/数值4, number5/数值5, number6/数值6 | 财务计算 / Financial calculations | — |
| PRODUCT | PRODUCT(value) | =PRODUCT(1) | value/值 | 通用计算 / General calculation | — |
| PROPER | PROPER(text) | =PROPER("text") | text/文本 | 文本处理 / Text processing | — |
| PV | PV(number1, number2, number3, number4, number5) | =PV(1, 1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, number4/数值4, number5/数值5 | 财务计算 / Financial calculations | — |
| QUOTIENT | QUOTIENT(number1, number2) | =QUOTIENT(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| RADIANS | RADIANS(number) | =RADIANS(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| RAND | RAND() | =RAND() | 无参数 / No parameters | 逻辑判断与条件处理 / Logical conditions | — |
| RANDBETWEEN | RANDBETWEEN(number1, number2) | =RANDBETWEEN(1, 1) | number1/数值1, number2/数值2 | 逻辑判断与条件处理 / Logical conditions | — |
| RATE | RATE(number1, number2, number3, number4, number5, number6) | =RATE(1, 1, 1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, number4/数值4, number5/数值5, number6/数值6 | 财务计算 / Financial calculations | — |
| REPLACE | REPLACE(old_text, start_num, num_chars, new_text) | =REPLACE("text", 1, 1, "text") | old_text/旧文本, start_num/起始位置, num_chars/字符数, new_text/新文本 | 文本处理 / Text processing | — |
| REPT | REPT(text1, number2) | =REPT("text", 1) | text1/文本1, number2/数值2 | 通用计算 / General calculation | — |
| RIGHT | RIGHT(text, num_chars) | =RIGHT("text", 1) | text/文本, num_chars/字符数 | 从右侧截取文本 / Extract text from the right | — |
| ROMAN | ROMAN(number1, [value2]) | =ROMAN(1) | number1/数值1, value2/值2 (optional/可选) | 通用计算 / General calculation | — |
| ROUND | ROUND(number, num_digits) | =ROUND(1, 1) | number/数值, num_digits/小数位 | 按指定位数四舍五入 / Round to a specified number of digits | — |
| ROUNDDOWN | ROUNDDOWN(number, num_digits) | =ROUNDDOWN(1, 1) | number/数值, num_digits/小数位 | 数学计算与取整 / Math and rounding | — |
| ROUNDUP | ROUNDUP(number, num_digits) | =ROUNDUP(1, 1) | number/数值, num_digits/小数位 | 数学计算与取整 / Math and rounding | — |
| ROW | ROW([value]) | =ROW() | value/值 (optional/可选) | 通用计算 / General calculation | — |
| ROWS | ROWS(range) | =ROWS(A1:A5) | range/范围 | 通用计算 / General calculation | — |
| RRI | RRI(number1, number2, number3) | =RRI(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 通用计算 / General calculation | — |
| RSQ | RSQ(range1, range2) | =RSQ(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 通用计算 / General calculation | — |
| SEARCH | SEARCH(find_text, within_text, start_num) | =SEARCH("text", "text", 1) | find_text/查找文本, within_text/范围文本, start_num/起始位置 | 文本处理 / Text processing | — |
| SEC | SEC(number) | =SEC(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| SECH | SECH(number) | =SECH(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| SECOND | SECOND(number) | =SECOND(1) | number/数值 | 日期与时间处理 / Date and time handling | — |
| SERIESSUM | SERIESSUM(number1, number2, number3, range4) | =SERIESSUM(1, 1, 1, A1:A5) | number1/数值1, number2/数值2, number3/数值3, range4/范围4 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| SHEET | SHEET(text) | =SHEET("text") | text/文本 | 通用计算 / General calculation | — |
| SHEETS | SHEETS(text) | =SHEETS("text") | text/文本 | 通用计算 / General calculation | — |
| SIGN | SIGN(number) | =SIGN(1) | number/数值 | 数学计算与取整 / Math and rounding | — |
| SIN | SIN(number) | =SIN(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| SINH | SINH(number) | =SINH(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| SKEW | SKEW(value) | =SKEW(1) | value/值 | 通用计算 / General calculation | — |
| SKEW.P | SKEW.P(value) | =SKEW.P(1) | value/值 | 通用计算 / General calculation | — |
| SKEWP | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| SLN | SLN(number1, number2, number3) | =SLN(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 数学计算与取整 / Math and rounding | — |
| SLOPE | SLOPE(range1, range2) | =SLOPE(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 通用计算 / General calculation | — |
| SMALL | SMALL(range1, number2) | =SMALL(A1:A5, 1) | range1/范围1, number2/数值2 | 通用计算 / General calculation | — |
| SPLIT | SPLIT(text, delimiter) | =SPLIT("text", 1) | text/文本, delimiter/分隔符 | 按分隔符拆分文本 / Split text by a delimiter | — |
| SQRT | SQRT(number) | =SQRT(1) | number/数值 | 数学计算与取整 / Math and rounding | — |
| SQRTPI | SQRTPI(number) | =SQRTPI(1) | number/数值 | 数学计算与取整 / Math and rounding | — |
| STANDARDIZE | STANDARDIZE(number1, number2, number3) | =STANDARDIZE(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 逻辑判断与条件处理 / Logical conditions | — |
| STDEV | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| STDEV.P | STDEV.P(value) | =STDEV.P(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| STDEV.S | STDEV.S(value) | =STDEV.S(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| STDEVA | STDEVA(value) | =STDEVA(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| STDEVP | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| STDEVPA | STDEVPA(value) | =STDEVPA(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| STDEVS | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| STEYX | STEYX(range1, range2) | =STEYX(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 通用计算 / General calculation | — |
| SUBSTITUTE | SUBSTITUTE(text, old_text, new_text, [instance_num]) | =SUBSTITUTE("text", "text", "text") | text/文本, old_text/旧文本, new_text/新文本, instance_num/替换次数 (optional/可选) | 文本处理 / Text processing | — |
| SUBTOTAL | SUBTOTAL(number1, value2) | =SUBTOTAL(1, 1) | number1/数值1, value2/值2 | 通用计算 / General calculation | — |
| SUM | SUM(range) | =SUM(1) | range/范围 | 对范围内数值求和 / Sum numbers in a range | — |
| SUMIF | SUMIF(range, criteria, [sum_range]) | =SUMIF(A1:A5, 1) | range/范围, criteria/条件, sum_range/求和范围 (optional/可选) | 统计与汇总计算 / Statistical and aggregate calculations | — |
| SUMIFS | SUMIFS(sum_range, criteria_range1, criteria1) | =SUMIFS(A1:A5, A1:A5, 1) | sum_range/求和范围, criteria_range1/条件范围1, criteria1/条件1 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| SUMPRODUCT | SUMPRODUCT(range) | =SUMPRODUCT(A1:A5) | range/范围 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| SUMSQ | SUMSQ(value) | =SUMSQ(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| SUMX2MY2 | SUMX2MY2(range1, range2) | =SUMX2MY2(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| SUMX2PY2 | SUMX2PY2(range1, range2) | =SUMX2PY2(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| SUMXMY2 | SUMXMY2(range1, range2) | =SUMXMY2(A1:A5, A1:A5) | range1/范围1, range2/范围2 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| SWITCH | SWITCH(value1, value2, value3) | =SWITCH(1, 1, 1) | value1/值1, value2/值2, value3/值3 | 逻辑判断与条件处理 / Logical conditions | — |
| SYD | SYD(number1, number2, number3, number4) | =SYD(1, 1, 1, 1) | number1/数值1, number2/数值2, number3/数值3, number4/数值4 | 财务计算 / Financial calculations | — |
| T | T(value) | =T(1) | value/值 | 通用计算 / General calculation | — |
| T.DIST | T.DIST(number1, number2, condition3) | =T.DIST(1, 1, TRUE) | number1/数值1, number2/数值2, condition3/条件3 | 统计分布函数 / Statistical distributions | — |
| T.DIST.2T | T.DIST.2T(number1, number2) | =T.DIST.2T(1, 1) | number1/数值1, number2/数值2 | 统计分布函数 / Statistical distributions | — |
| T.DIST.RT | T.DIST.RT(number1, number2) | =T.DIST.RT(1, 1) | number1/数值1, number2/数值2 | 统计分布函数 / Statistical distributions | — |
| T.INV | T.INV(number1, number2) | =T.INV(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| T.INV.2T | T.INV.2T(number1, number2) | =T.INV.2T(1, 1) | number1/数值1, number2/数值2 | 通用计算 / General calculation | — |
| T.TEST | T.TEST(range1, range2, integer3, integer4) | =T.TEST(A1:A5, A1:A5, 1, 1) | range1/范围1, range2/范围2, integer3/整数3, integer4/整数4 | 通用计算 / General calculation | — |
| TAN | TAN(number) | =TAN(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| TANH | TANH(number) | =TANH(1) | number/数值 | 三角函数与角度转换 / Trigonometry | — |
| TBILLEQ | TBILLEQ(number1, number2, number3) | =TBILLEQ(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 通用计算 / General calculation | — |
| TBILLPRICE | TBILLPRICE(number1, number2, number3) | =TBILLPRICE(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 通用计算 / General calculation | — |
| TBILLYIELD | TBILLYIELD(number1, number2, number3) | =TBILLYIELD(1, 1, 1) | number1/数值1, number2/数值2, number3/数值3 | 通用计算 / General calculation | — |
| TDIST | TDIST(number1, number2, integer3) | =TDIST(1, 1, 1) | number1/数值1, number2/数值2, integer3/整数3 | 通用计算 / General calculation | — |
| TDIST2T | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| TDISTRT | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| TEXT | TEXT(value, format_text) | =TEXT(1, "text") | value/值, format_text/格式 | 文本处理 / Text processing | — |
| TIME | TIME(hour, minute, second) | =TIME(1, 1, 1) | hour/时, minute/分, second/秒 | 日期与时间处理 / Date and time handling | — |
| TIMEVALUE | TIMEVALUE(text) | =TIMEVALUE("text") | text/文本 | 日期与时间处理 / Date and time handling | — |
| TINV | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| TINV2T | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| TODAY | TODAY() | =TODAY() | 无参数 / No parameters | 返回当前日期 / Return current date | — |
| TRANSPOSE | TRANSPOSE(range) | =TRANSPOSE(A1:A5) | range/范围 | 数组与矩阵处理 / Array and matrix operations | — |
| TRIM | TRIM(text) | =TRIM("text") | text/文本 | 文本处理 / Text processing | — |
| TRUE | TRUE() | =TRUE() | 无参数 / No parameters | 通用计算 / General calculation | — |
| TRUNC | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| TTEST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| UNICHAR | UNICHAR(number) | =UNICHAR(1) | number/数值 | 通用计算 / General calculation | — |
| UNICODE | UNICODE(text) | =UNICODE("text") | text/文本 | 通用计算 / General calculation | — |
| UPPER | UPPER(text) | =UPPER("text") | text/文本 | 文本处理 / Text processing | — |
| VALUE | VALUE(text) | =VALUE(1) | text/文本 | 文本处理 / Text processing | — |
| VAR | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| VAR.P | VAR.P(value) | =VAR.P(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| VAR.S | VAR.S(value) | =VAR.S(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| VARA | VARA(value) | =VARA(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| VARP | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| VARPA | VARPA(value) | =VARPA(1) | value/值 | 统计与汇总计算 / Statistical and aggregate calculations | — |
| VARS | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| VERSION | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| VLOOKUP | VLOOKUP(lookup_value, table, return_col, join_col) | =VLOOKUP(1, A1:A5, 1, TRUE) | lookup_value/查找值, table/表, return_col/返回列, join_col/匹配列 | 按匹配列查找并返回指定列值 / Lookup by key column and return a target column value | — |
| WEEKDAY | WEEKDAY(number1, number2) | =WEEKDAY(1, 1) | number1/数值1, number2/数值2 | 日期与时间处理 / Date and time handling | — |
| WEEKNUM | WEEKNUM(number1, number2) | =WEEKNUM(1, 1) | number1/数值1, number2/数值2 | 日期与时间处理 / Date and time handling | — |
| WEIBULL | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| WEIBULL.DIST | WEIBULL.DIST(number1, number2, number3, condition4) | =WEIBULL.DIST(1, 1, 1, TRUE) | number1/数值1, number2/数值2, number3/数值3, condition4/条件4 | 统计分布函数 / Statistical distributions | — |
| WEIBULLDIST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| WORKDAY | WORKDAY(number1, number2, [range3]) | =WORKDAY(1, 1) | number1/数值1, number2/数值2, range3/范围3 (optional/可选) | 日期与时间处理 / Date and time handling | — |
| WORKDAY.INTL | WORKDAY.INTL(number1, number2, value3, [range4]) | =WORKDAY.INTL(1, 1, 1) | number1/数值1, number2/数值2, value3/值3, range4/范围4 (optional/可选) | 日期与时间处理 / Date and time handling | — |
| XLOOKUP | XLOOKUP(lookup_value, table, join_col, [return_col], [if_not_found], [number6]) | =XLOOKUP(1, A1:A5, A1:A5) | lookup_value/查找值, table/表, join_col/匹配列, return_col/返回列 (optional/可选), if_not_found/未找到返回值 (optional/可选), number6/数值6 (optional/可选) | 按匹配列查找并返回指定列值 / Lookup by key column and return a target column value | — |
| XNPV | XNPV(number1, range2, range3) | =XNPV(1, A1:A5, A1:A5) | number1/数值1, range2/范围2, range3/范围3 | 财务计算 / Financial calculations | — |
| XOR | XOR(condition) | =XOR(TRUE) | condition/条件 | 逻辑判断与条件处理 / Logical conditions | — |
| YEAR | YEAR(number) | =YEAR(1) | number/数值 | 日期与时间处理 / Date and time handling | — |
| YEARFRAC | YEARFRAC(number1, number2, integer3) | =YEARFRAC(1, 1, 1) | number1/数值1, number2/数值2, integer3/整数3 | 日期与时间处理 / Date and time handling | — |
| Z.TEST | Z.TEST(range1, number2, [number3]) | =Z.TEST(A1:A5, 1) | range1/范围1, number2/数值2, number3/数值3 (optional/可选) | 统计分布函数 / Statistical distributions | — |
| ZTEST | — | — | — | 不可用 / Unavailable | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
<!-- FORMULA_DOCS_END -->




</details>

## Setup

1. Install dependencies:
   ```bash
   npm install
   ```

2. Run development server:
   ```bash
   npm run dev
   ```

## Architecture
- Framework: React + Vite
- Grid Component: Luckysheet / FortuneSheet (Proposed)
- Backend API: Connects to Rust server at http://localhost:3000

## 公式样例 / Formula Examples

### 聚合类 / Aggregates
- `=SUM(A:A)` — 对 A 列求和 / Sum values in column A
- `=COUNT(A:A)` — 统计 A 列非空数量 / Count non-empty values in column A
- `=AVG(A:A)` — 计算 A 列平均值 / Average values in column A
- `=MAX(A:A)` — A 列最大值 / Max value in column A
- `=MIN(A:A)` — A 列最小值 / Min value in column A

### 查找类 / Lookup
- `=XLOOKUP(A2,"orders","order_id","amount",0)` — 从 orders 表按 order_id 查 amount / Lookup amount by order_id
- `=VLOOKUP(A2,"orders","amount","order_id")` — 与 XLOOKUP 等价的简写 / Equivalent lookup for amount

### 算术类 / Arithmetic
- `=A1+B1` — A1 与 B1 相加 / Add A1 and B1
- `=A1-B1` — A1 与 B1 相减 / Subtract B1 from A1
- `=A1*B1` — A1 与 B1 相乘 / Multiply A1 and B1
- `=A1/B1` — A1 除以 B1 / Divide A1 by B1
- `=(A1+B1)/C1` — 组合运算示例 / Combined arithmetic example
