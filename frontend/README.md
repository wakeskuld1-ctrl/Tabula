# Frontend UI

This is the React frontend for the Federated Query Engine.

## 公式样例（全量） / Full Formula Examples

<details>
<summary>点击展开 / Expand</summary>

<!-- FORMULA_DOCS_START -->
| 函数名 / Function | 语法 / Syntax | 示例 / Example | 备注 / Notes |
| --- | --- | --- | --- |
| ABS | ABS(number1) | =ABS(1) | — |
| ACOS | ACOS(number1) | =ACOS(1) | — |
| ACOSH | ACOSH(number1) | =ACOSH(1) | — |
| ACOT | ACOT(number1) | =ACOT(1) | — |
| ACOTH | ACOTH(number1) | =ACOTH(1) | — |
| ADDRESS | ADDRESS(number1, number2, [number3], [boolean4], [text5]) | =ADDRESS(1, 1) | — |
| AND | AND(boolean1) | =AND(TRUE) | — |
| ARABIC | ARABIC(text1) | =ARABIC("text") | — |
| ARRAY_CONSTRAIN | ARRAY_CONSTRAIN(range1, integer2, integer3) | =ARRAY_CONSTRAIN(A1:A5, 1, 1) | — |
| ARRAYFORMULA | ARRAYFORMULA(value1) | =ARRAYFORMULA(1) | — |
| ASIN | ASIN(number1) | =ASIN(1) | — |
| ASINH | ASINH(number1) | =ASINH(1) | — |
| ATAN | ATAN(number1) | =ATAN(1) | — |
| ATAN2 | ATAN2(number1, number2) | =ATAN2(1, 1) | — |
| ATANH | ATANH(number1) | =ATANH(1) | — |
| AVEDEV | AVEDEV(value1) | =AVEDEV(1) | — |
| AVERAGE | AVERAGE(value1) | =AVERAGE(1) | — |
| AVERAGEA | AVERAGEA(value1) | =AVERAGEA(1) | — |
| AVERAGEIF | AVERAGEIF(range1, noerror2, [range3]) | =AVERAGEIF(A1:A5, 1) | — |
| BASE | BASE(number1, number2, [number3]) | =BASE(1, 1) | — |
| BESSELI | BESSELI(number1, number2) | =BESSELI(1, 1) | — |
| BESSELJ | BESSELJ(number1, number2) | =BESSELJ(1, 1) | — |
| BESSELK | BESSELK(number1, number2) | =BESSELK(1, 1) | — |
| BESSELY | BESSELY(number1, number2) | =BESSELY(1, 1) | — |
| BETA.DIST | BETA.DIST(number1, number2, number3, boolean4, number5, number6) | =BETA.DIST(1, 1, 1, TRUE, 1, 1) | — |
| BETA.INV | BETA.INV(number1, number2, number3, number4, number5) | =BETA.INV(1, 1, 1, 1, 1) | — |
| BETADIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| BETAINV | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| BIN2DEC | BIN2DEC(text1) | =BIN2DEC("text") | — |
| BIN2HEX | BIN2HEX(text1, [number2]) | =BIN2HEX("text") | — |
| BIN2OCT | BIN2OCT(text1, [number2]) | =BIN2OCT("text") | — |
| BINOM.DIST | BINOM.DIST(number1, number2, number3, boolean4) | =BINOM.DIST(1, 1, 1, TRUE) | — |
| BINOM.INV | BINOM.INV(number1, number2, number3) | =BINOM.INV(1, 1, 1) | — |
| BINOMDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| BITAND | BITAND(integer1, integer2) | =BITAND(1, 1) | — |
| BITLSHIFT | BITLSHIFT(integer1, integer2) | =BITLSHIFT(1, 1) | — |
| BITOR | BITOR(integer1, integer2) | =BITOR(1, 1) | — |
| BITRSHIFT | BITRSHIFT(integer1, integer2) | =BITRSHIFT(1, 1) | — |
| BITXOR | BITXOR(integer1, integer2) | =BITXOR(1, 1) | — |
| CEILING | CEILING(number1, number2) | =CEILING(1, 1) | — |
| CEILING.MATH | CEILING.MATH(number1, number2, number3) | =CEILING.MATH(1, 1, 1) | — |
| CEILING.PRECISE | CEILING.PRECISE(number1, number2) | =CEILING.PRECISE(1, 1) | — |
| CHAR | CHAR(number1) | =CHAR(1) | — |
| CHIDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CHIDISTRT | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CHIINV | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CHIINVRT | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CHISQ.DIST | CHISQ.DIST(number1, number2, boolean3) | =CHISQ.DIST(1, 1, TRUE) | — |
| CHISQ.DIST.RT | CHISQ.DIST.RT(number1, number2) | =CHISQ.DIST.RT(1, 1) | — |
| CHISQ.INV | CHISQ.INV(number1, number2) | =CHISQ.INV(1, 1) | — |
| CHISQ.INV.RT | CHISQ.INV.RT(number1, number2) | =CHISQ.INV.RT(1, 1) | — |
| CHISQ.TEST | CHISQ.TEST(range1, range2) | =CHISQ.TEST(A1:A5, A1:A5) | — |
| CHITEST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CHOOSE | CHOOSE(integer1, value2) | =CHOOSE(1, 1) | — |
| CLEAN | CLEAN(text1) | =CLEAN("text") | — |
| CODE | CODE(text1) | =CODE("text") | — |
| COLUMN | COLUMN([noerror1]) | =COLUMN() | — |
| COLUMNS | COLUMNS(range1) | =COLUMNS(A1:A5) | — |
| COMBIN | COMBIN(number1, number2) | =COMBIN(1, 1) | — |
| COMBINA | COMBINA(number1, number2) | =COMBINA(1, 1) | — |
| COMPLEX | COMPLEX(number1, number2, text3) | =COMPLEX(1, 1, "text") | — |
| CONCATENATE | CONCATENATE(text1) | =CONCATENATE("text") | — |
| CONFIDENCE | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CONFIDENCE.NORM | CONFIDENCE.NORM(number1, number2, number3) | =CONFIDENCE.NORM(1, 1, 1) | — |
| CONFIDENCE.T | CONFIDENCE.T(number1, number2, number3) | =CONFIDENCE.T(1, 1, 1) | — |
| CORREL | CORREL(range1, range2) | =CORREL(A1:A5, A1:A5) | — |
| COS | COS(number1) | =COS(1) | — |
| COSH | COSH(number1) | =COSH(1) | — |
| COT | COT(number1) | =COT(1) | — |
| COTH | COTH(number1) | =COTH(1) | — |
| COUNT | COUNT(value1) | =COUNT(1) | — |
| COUNTA | COUNTA(value1) | =COUNTA(1) | — |
| COUNTBLANK | COUNTBLANK(value1) | =COUNTBLANK(1) | — |
| COUNTIF | COUNTIF(range1, noerror2) | =COUNTIF(A1:A5, 1) | — |
| COUNTIFS | COUNTIFS(range1, noerror2) | =COUNTIFS(A1:A5, 1) | — |
| COUNTUNIQUE | COUNTUNIQUE(value1) | =COUNTUNIQUE(1) | — |
| COVAR | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| COVARIANCE.P | COVARIANCE.P(range1, range2) | =COVARIANCE.P(A1:A5, A1:A5) | — |
| COVARIANCE.S | COVARIANCE.S(range1, range2) | =COVARIANCE.S(A1:A5, A1:A5) | — |
| COVARIANCEP | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| COVARIANCES | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CRITBINOM | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| CSC | CSC(number1) | =CSC(1) | — |
| CSCH | CSCH(number1) | =CSCH(1) | — |
| CUMIPMT | CUMIPMT(number1, number2, number3, integer4, integer5, integer6) | =CUMIPMT(1, 1, 1, 1, 1, 1) | — |
| CUMPRINC | CUMPRINC(number1, number2, number3, integer4, integer5, integer6) | =CUMPRINC(1, 1, 1, 1, 1, 1) | — |
| DATE | DATE(number1, number2, number3) | =DATE(1, 1, 1) | — |
| DATEDIF | DATEDIF(number1, number2, text3) | =DATEDIF(1, 1, "text") | — |
| DATEVALUE | DATEVALUE(text1) | =DATEVALUE("text") | — |
| DAY | DAY(number1) | =DAY(1) | — |
| DAYS | DAYS(number1, number2) | =DAYS(1, 1) | — |
| DAYS360 | DAYS360(number1, number2, boolean3) | =DAYS360(1, 1, TRUE) | — |
| DB | DB(number1, number2, integer3, integer4, integer5) | =DB(1, 1, 1, 1, 1) | — |
| DDB | DDB(number1, number2, integer3, number4, number5) | =DDB(1, 1, 1, 1, 1) | — |
| DEC2BIN | DEC2BIN(number1, [number2]) | =DEC2BIN(1) | — |
| DEC2HEX | DEC2HEX(number1, [number2]) | =DEC2HEX(1) | — |
| DEC2OCT | DEC2OCT(number1, [number2]) | =DEC2OCT(1) | — |
| DECIMAL | DECIMAL(text1, number2) | =DECIMAL("text", 1) | — |
| DEGREES | DEGREES(number1) | =DEGREES(1) | — |
| DELTA | DELTA(number1, number2) | =DELTA(1, 1) | — |
| DEVSQ | DEVSQ(value1) | =DEVSQ(1) | — |
| DOLLARDE | DOLLARDE(number1, number2) | =DOLLARDE(1, 1) | — |
| DOLLARFR | DOLLARFR(number1, number2) | =DOLLARFR(1, 1) | — |
| EDATE | EDATE(number1, number2) | =EDATE(1, 1) | — |
| EFFECT | EFFECT(number1, number2) | =EFFECT(1, 1) | — |
| EOMONTH | EOMONTH(number1, number2) | =EOMONTH(1, 1) | — |
| ERF | ERF(number1, [number2]) | =ERF(1) | — |
| ERFC | ERFC(number1) | =ERFC(1) | — |
| EVEN | EVEN(number1) | =EVEN(1) | — |
| EXACT | EXACT(text1, text2) | =EXACT("text", "text") | — |
| EXP | EXP(number1) | =EXP(1) | — |
| EXPON.DIST | EXPON.DIST(number1, number2, boolean3) | =EXPON.DIST(1, 1, TRUE) | — |
| EXPONDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| F.DIST | F.DIST(number1, number2, number3, boolean4) | =F.DIST(1, 1, 1, TRUE) | — |
| F.DIST.RT | F.DIST.RT(number1, number2, number3) | =F.DIST.RT(1, 1, 1) | — |
| F.INV | F.INV(number1, number2, number3) | =F.INV(1, 1, 1) | — |
| F.INV.RT | F.INV.RT(number1, number2, number3) | =F.INV.RT(1, 1, 1) | — |
| F.TEST | F.TEST(range1, range2) | =F.TEST(A1:A5, A1:A5) | — |
| FACT | FACT(number1) | =FACT(1) | — |
| FACTDOUBLE | FACTDOUBLE(number1) | =FACTDOUBLE(1) | — |
| FALSE | FALSE() | =FALSE() | — |
| FDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| FDISTRT | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| FILTER | FILTER(range1, range2) | =FILTER(A1:A5, A1:A5) | — |
| FIND | FIND(text1, text2, number3) | =FIND("text", "text", 1) | — |
| FINV | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| FINVRT | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| FISHER | FISHER(number1) | =FISHER(1) | — |
| FISHERINV | FISHERINV(number1) | =FISHERINV(1) | — |
| FLOOR | FLOOR(number1, number2) | =FLOOR(1, 1) | — |
| FLOOR.MATH | FLOOR.MATH(number1, number2, number3) | =FLOOR.MATH(1, 1, 1) | — |
| FLOOR.PRECISE | FLOOR.PRECISE(number1, number2) | =FLOOR.PRECISE(1, 1) | — |
| FORMULATEXT | FORMULATEXT(noerror1) | =FORMULATEXT(1) | — |
| FTEST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| FV | FV(number1, number2, number3, number4, number5) | =FV(1, 1, 1, 1, 1) | — |
| FVSCHEDULE | FVSCHEDULE(number1, range2) | =FVSCHEDULE(1, A1:A5) | — |
| GAMMA | GAMMA(number1) | =GAMMA(1) | — |
| GAMMA.DIST | GAMMA.DIST(number1, number2, number3, boolean4) | =GAMMA.DIST(1, 1, 1, TRUE) | — |
| GAMMA.INV | GAMMA.INV(number1, number2, number3) | =GAMMA.INV(1, 1, 1) | — |
| GAMMADIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| GAMMAINV | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| GAMMALN | GAMMALN(number1) | =GAMMALN(1) | — |
| GAMMALN.PRECISE | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| GAUSS | GAUSS(number1) | =GAUSS(1) | — |
| GCD | GCD(value1) | =GCD(1) | — |
| GEOMEAN | GEOMEAN(value1) | =GEOMEAN(1) | — |
| HARMEAN | HARMEAN(value1) | =HARMEAN(1) | — |
| HEX2BIN | HEX2BIN(text1, [number2]) | =HEX2BIN("text") | — |
| HEX2DEC | HEX2DEC(text1) | =HEX2DEC("text") | — |
| HEX2OCT | HEX2OCT(text1, [number2]) | =HEX2OCT("text") | — |
| HF.ADD | HF.ADD(number1, number2) | =HF.ADD(1, 1) | — |
| HF.CONCAT | HF.CONCAT(text1, text2) | =HF.CONCAT("text", "text") | — |
| HF.DIVIDE | HF.DIVIDE(number1, number2) | =HF.DIVIDE(1, 1) | — |
| HF.EQ | HF.EQ(noerror1, noerror2) | =HF.EQ(1, 1) | — |
| HF.GT | HF.GT(noerror1, noerror2) | =HF.GT(1, 1) | — |
| HF.GTE | HF.GTE(noerror1, noerror2) | =HF.GTE(1, 1) | — |
| HF.LT | HF.LT(noerror1, noerror2) | =HF.LT(1, 1) | — |
| HF.LTE | HF.LTE(noerror1, noerror2) | =HF.LTE(1, 1) | — |
| HF.MINUS | HF.MINUS(number1, number2) | =HF.MINUS(1, 1) | — |
| HF.MULTIPLY | HF.MULTIPLY(number1, number2) | =HF.MULTIPLY(1, 1) | — |
| HF.NE | HF.NE(noerror1, noerror2) | =HF.NE(1, 1) | — |
| HF.POW | HF.POW(number1, number2) | =HF.POW(1, 1) | — |
| HF.UMINUS | HF.UMINUS(number1) | =HF.UMINUS(1) | — |
| HF.UNARY_PERCENT | HF.UNARY_PERCENT(number1) | =HF.UNARY_PERCENT(1) | — |
| HF.UPLUS | HF.UPLUS(number1) | =HF.UPLUS(1) | — |
| HLOOKUP | HLOOKUP(noerror1, range2, number3, boolean4) | =HLOOKUP(1, A1:A5, 1, TRUE) | — |
| HOUR | HOUR(number1) | =HOUR(1) | — |
| HYPERLINK | HYPERLINK(text1, [text2]) | =HYPERLINK("text") | — |
| HYPGEOM.DIST | HYPGEOM.DIST(number1, number2, number3, number4, boolean5) | =HYPGEOM.DIST(1, 1, 1, 1, TRUE) | — |
| HYPGEOMDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| IF | IF(boolean1, value2, value3) | =IF(TRUE, 1, 1) | — |
| IFERROR | IFERROR(value1, value2) | =IFERROR(1, 1) | — |
| IFNA | IFNA(value1, value2) | =IFNA(1, 1) | — |
| IFS | IFS(boolean1, value2) | =IFS(TRUE, 1) | — |
| IMABS | IMABS(complex1) | =IMABS("1+2i") | — |
| IMAGINARY | IMAGINARY(complex1) | =IMAGINARY("1+2i") | — |
| IMARGUMENT | IMARGUMENT(complex1) | =IMARGUMENT("1+2i") | — |
| IMCONJUGATE | IMCONJUGATE(complex1) | =IMCONJUGATE("1+2i") | — |
| IMCOS | IMCOS(complex1) | =IMCOS("1+2i") | — |
| IMCOSH | IMCOSH(complex1) | =IMCOSH("1+2i") | — |
| IMCOT | IMCOT(complex1) | =IMCOT("1+2i") | — |
| IMCSC | IMCSC(complex1) | =IMCSC("1+2i") | — |
| IMCSCH | IMCSCH(complex1) | =IMCSCH("1+2i") | — |
| IMDIV | IMDIV(complex1, complex2) | =IMDIV("1+2i", "1+2i") | — |
| IMEXP | IMEXP(complex1) | =IMEXP("1+2i") | — |
| IMLN | IMLN(complex1) | =IMLN("1+2i") | — |
| IMLOG10 | IMLOG10(complex1) | =IMLOG10("1+2i") | — |
| IMLOG2 | IMLOG2(complex1) | =IMLOG2("1+2i") | — |
| IMPOWER | IMPOWER(complex1, number2) | =IMPOWER("1+2i", 1) | — |
| IMPRODUCT | IMPRODUCT(value1) | =IMPRODUCT(1) | — |
| IMREAL | IMREAL(complex1) | =IMREAL("1+2i") | — |
| IMSEC | IMSEC(complex1) | =IMSEC("1+2i") | — |
| IMSECH | IMSECH(complex1) | =IMSECH("1+2i") | — |
| IMSIN | IMSIN(complex1) | =IMSIN("1+2i") | — |
| IMSINH | IMSINH(complex1) | =IMSINH("1+2i") | — |
| IMSQRT | IMSQRT(complex1) | =IMSQRT("1+2i") | — |
| IMSUB | IMSUB(complex1, complex2) | =IMSUB("1+2i", "1+2i") | — |
| IMSUM | IMSUM(value1) | =IMSUM(1) | — |
| IMTAN | IMTAN(complex1) | =IMTAN("1+2i") | — |
| INDEX | INDEX(range1, number2, number3) | =INDEX(A1:A5, 1, 1) | — |
| INT | INT(number1) | =INT(1) | — |
| INTERVAL | INTERVAL(number1) | =INTERVAL(1) | — |
| IPMT | IPMT(number1, number2, number3, number4, number5, number6) | =IPMT(1, 1, 1, 1, 1, 1) | — |
| IRR | IRR(range1, number2) | =IRR(A1:A5, 1) | — |
| ISBINARY | ISBINARY(text1) | =ISBINARY("text") | — |
| ISBLANK | ISBLANK(value1) | =ISBLANK(1) | — |
| ISERR | ISERR(value1) | =ISERR(1) | — |
| ISERROR | ISERROR(value1) | =ISERROR(1) | — |
| ISEVEN | ISEVEN(number1) | =ISEVEN(1) | — |
| ISFORMULA | ISFORMULA(noerror1) | =ISFORMULA(1) | — |
| ISLOGICAL | ISLOGICAL(value1) | =ISLOGICAL(1) | — |
| ISNA | ISNA(value1) | =ISNA(1) | — |
| ISNONTEXT | ISNONTEXT(value1) | =ISNONTEXT(1) | — |
| ISNUMBER | ISNUMBER(value1) | =ISNUMBER(1) | — |
| ISO.CEILING | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| ISODD | ISODD(number1) | =ISODD(1) | — |
| ISOWEEKNUM | ISOWEEKNUM(number1) | =ISOWEEKNUM(1) | — |
| ISPMT | ISPMT(number1, number2, number3, number4) | =ISPMT(1, 1, 1, 1) | — |
| ISREF | ISREF(value1) | =ISREF(1) | — |
| ISTEXT | ISTEXT(value1) | =ISTEXT(1) | — |
| LARGE | LARGE(range1, number2) | =LARGE(A1:A5, 1) | — |
| LCM | LCM(value1) | =LCM(1) | — |
| LEFT | LEFT(text1, number2) | =LEFT("text", 1) | — |
| LEN | LEN(text1) | =LEN("text") | — |
| LN | LN(number1) | =LN(1) | — |
| LOG | LOG(number1, number2) | =LOG(1, 1) | — |
| LOG10 | LOG10(number1) | =LOG10(1) | — |
| LOGINV | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| LOGNORM.DIST | LOGNORM.DIST(number1, number2, number3, boolean4) | =LOGNORM.DIST(1, 1, 1, TRUE) | — |
| LOGNORM.INV | LOGNORM.INV(number1, number2, number3) | =LOGNORM.INV(1, 1, 1) | — |
| LOGNORMDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| LOGNORMINV | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| LOWER | LOWER(text1) | =LOWER("text") | — |
| MATCH | MATCH(noerror1, range2, number3) | =MATCH(1, A1:A5, 1) | — |
| MAX | MAX(value1) | =MAX(1) | — |
| MAXA | MAXA(value1) | =MAXA(1) | — |
| MAXIFS | MAXIFS(range1, range2, noerror3) | =MAXIFS(A1:A5, A1:A5, 1) | — |
| MAXPOOL | MAXPOOL(range1, number2, [number3]) | =MAXPOOL(A1:A5, 1) | — |
| MEDIAN | MEDIAN(value1) | =MEDIAN(1) | — |
| MEDIANPOOL | MEDIANPOOL(range1, number2, [number3]) | =MEDIANPOOL(A1:A5, 1) | — |
| MID | MID(text1, number2, number3) | =MID("text", 1, 1) | — |
| MIN | MIN(value1) | =MIN(1) | — |
| MINA | MINA(value1) | =MINA(1) | — |
| MINIFS | MINIFS(range1, range2, noerror3) | =MINIFS(A1:A5, A1:A5, 1) | — |
| MINUTE | MINUTE(number1) | =MINUTE(1) | — |
| MIRR | MIRR(range1, number2, number3) | =MIRR(A1:A5, 1, 1) | — |
| MMULT | MMULT(range1, range2) | =MMULT(A1:A5, A1:A5) | — |
| MOD | MOD(number1, number2) | =MOD(1, 1) | — |
| MONTH | MONTH(number1) | =MONTH(1) | — |
| MROUND | MROUND(number1, number2) | =MROUND(1, 1) | — |
| MULTINOMIAL | MULTINOMIAL(number1) | =MULTINOMIAL(1) | — |
| N | N(value1) | =N(1) | — |
| NA | NA() | =NA() | — |
| NEGBINOM.DIST | NEGBINOM.DIST(number1, number2, number3, boolean4) | =NEGBINOM.DIST(1, 1, 1, TRUE) | — |
| NEGBINOMDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| NETWORKDAYS | NETWORKDAYS(number1, number2, [range3]) | =NETWORKDAYS(1, 1) | — |
| NETWORKDAYS.INTL | NETWORKDAYS.INTL(number1, number2, noerror3, [range4]) | =NETWORKDAYS.INTL(1, 1, 1) | — |
| NOMINAL | NOMINAL(number1, number2) | =NOMINAL(1, 1) | — |
| NORM.DIST | NORM.DIST(number1, number2, number3, boolean4) | =NORM.DIST(1, 1, 1, TRUE) | — |
| NORM.INV | NORM.INV(number1, number2, number3) | =NORM.INV(1, 1, 1) | — |
| NORM.S.DIST | NORM.S.DIST(number1, boolean2) | =NORM.S.DIST(1, TRUE) | — |
| NORM.S.INV | NORM.S.INV(number1) | =NORM.S.INV(1) | — |
| NORMDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| NORMINV | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| NORMSDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| NORMSINV | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| NOT | NOT(boolean1) | =NOT(TRUE) | — |
| NOW | NOW() | =NOW() | — |
| NPER | NPER(number1, number2, number3, number4, number5) | =NPER(1, 1, 1, 1, 1) | — |
| NPV | NPV(number1, value2) | =NPV(1, 1) | — |
| OCT2BIN | OCT2BIN(text1, [number2]) | =OCT2BIN("text") | — |
| OCT2DEC | OCT2DEC(text1) | =OCT2DEC("text") | — |
| OCT2HEX | OCT2HEX(text1, [number2]) | =OCT2HEX("text") | — |
| ODD | ODD(number1) | =ODD(1) | — |
| OFFSET | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| OR | OR(boolean1) | =OR(TRUE) | — |
| PDURATION | PDURATION(number1, number2, number3) | =PDURATION(1, 1, 1) | — |
| PEARSON | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| PHI | PHI(number1) | =PHI(1) | — |
| PI | PI() | =PI() | — |
| PMT | PMT(number1, number2, number3, number4, number5) | =PMT(1, 1, 1, 1, 1) | — |
| POISSON | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| POISSON.DIST | POISSON.DIST(number1, number2, boolean3) | =POISSON.DIST(1, 1, TRUE) | — |
| POISSONDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| POWER | POWER(number1, number2) | =POWER(1, 1) | — |
| PPMT | PPMT(number1, number2, number3, number4, number5, number6) | =PPMT(1, 1, 1, 1, 1, 1) | — |
| PRODUCT | PRODUCT(value1) | =PRODUCT(1) | — |
| PROPER | PROPER(text1) | =PROPER("text") | — |
| PV | PV(number1, number2, number3, number4, number5) | =PV(1, 1, 1, 1, 1) | — |
| QUOTIENT | QUOTIENT(number1, number2) | =QUOTIENT(1, 1) | — |
| RADIANS | RADIANS(number1) | =RADIANS(1) | — |
| RAND | RAND() | =RAND() | — |
| RANDBETWEEN | RANDBETWEEN(number1, number2) | =RANDBETWEEN(1, 1) | — |
| RATE | RATE(number1, number2, number3, number4, number5, number6) | =RATE(1, 1, 1, 1, 1, 1) | — |
| REPLACE | REPLACE(text1, number2, number3, text4) | =REPLACE("text", 1, 1, "text") | — |
| REPT | REPT(text1, number2) | =REPT("text", 1) | — |
| RIGHT | RIGHT(text1, number2) | =RIGHT("text", 1) | — |
| ROMAN | ROMAN(number1, [noerror2]) | =ROMAN(1) | — |
| ROUND | ROUND(number1, number2) | =ROUND(1, 1) | — |
| ROUNDDOWN | ROUNDDOWN(number1, number2) | =ROUNDDOWN(1, 1) | — |
| ROUNDUP | ROUNDUP(number1, number2) | =ROUNDUP(1, 1) | — |
| ROW | ROW([noerror1]) | =ROW() | — |
| ROWS | ROWS(range1) | =ROWS(A1:A5) | — |
| RRI | RRI(number1, number2, number3) | =RRI(1, 1, 1) | — |
| RSQ | RSQ(range1, range2) | =RSQ(A1:A5, A1:A5) | — |
| SEARCH | SEARCH(text1, text2, number3) | =SEARCH("text", "text", 1) | — |
| SEC | SEC(number1) | =SEC(1) | — |
| SECH | SECH(number1) | =SECH(1) | — |
| SECOND | SECOND(number1) | =SECOND(1) | — |
| SERIESSUM | SERIESSUM(number1, number2, number3, range4) | =SERIESSUM(1, 1, 1, A1:A5) | — |
| SHEET | SHEET(text1) | =SHEET("text") | — |
| SHEETS | SHEETS(text1) | =SHEETS("text") | — |
| SIGN | SIGN(number1) | =SIGN(1) | — |
| SIN | SIN(number1) | =SIN(1) | — |
| SINH | SINH(number1) | =SINH(1) | — |
| SKEW | SKEW(value1) | =SKEW(1) | — |
| SKEW.P | SKEW.P(value1) | =SKEW.P(1) | — |
| SKEWP | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| SLN | SLN(number1, number2, number3) | =SLN(1, 1, 1) | — |
| SLOPE | SLOPE(range1, range2) | =SLOPE(A1:A5, A1:A5) | — |
| SMALL | SMALL(range1, number2) | =SMALL(A1:A5, 1) | — |
| SPLIT | SPLIT(text1, number2) | =SPLIT("text", 1) | — |
| SQRT | SQRT(number1) | =SQRT(1) | — |
| SQRTPI | SQRTPI(number1) | =SQRTPI(1) | — |
| STANDARDIZE | STANDARDIZE(number1, number2, number3) | =STANDARDIZE(1, 1, 1) | — |
| STDEV | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| STDEV.P | STDEV.P(value1) | =STDEV.P(1) | — |
| STDEV.S | STDEV.S(value1) | =STDEV.S(1) | — |
| STDEVA | STDEVA(value1) | =STDEVA(1) | — |
| STDEVP | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| STDEVPA | STDEVPA(value1) | =STDEVPA(1) | — |
| STDEVS | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| STEYX | STEYX(range1, range2) | =STEYX(A1:A5, A1:A5) | — |
| SUBSTITUTE | SUBSTITUTE(text1, text2, text3, [number4]) | =SUBSTITUTE("text", "text", "text") | — |
| SUBTOTAL | SUBTOTAL(number1, value2) | =SUBTOTAL(1, 1) | — |
| SUM | SUM(value1) | =SUM(1) | — |
| SUMIF | SUMIF(range1, noerror2, [range3]) | =SUMIF(A1:A5, 1) | — |
| SUMIFS | SUMIFS(range1, range2, noerror3) | =SUMIFS(A1:A5, A1:A5, 1) | — |
| SUMPRODUCT | SUMPRODUCT(range1) | =SUMPRODUCT(A1:A5) | — |
| SUMSQ | SUMSQ(value1) | =SUMSQ(1) | — |
| SUMX2MY2 | SUMX2MY2(range1, range2) | =SUMX2MY2(A1:A5, A1:A5) | — |
| SUMX2PY2 | SUMX2PY2(range1, range2) | =SUMX2PY2(A1:A5, A1:A5) | — |
| SUMXMY2 | SUMXMY2(range1, range2) | =SUMXMY2(A1:A5, A1:A5) | — |
| SWITCH | SWITCH(noerror1, value2, value3) | =SWITCH(1, 1, 1) | — |
| SYD | SYD(number1, number2, number3, number4) | =SYD(1, 1, 1, 1) | — |
| T | T(value1) | =T(1) | — |
| T.DIST | T.DIST(number1, number2, boolean3) | =T.DIST(1, 1, TRUE) | — |
| T.DIST.2T | T.DIST.2T(number1, number2) | =T.DIST.2T(1, 1) | — |
| T.DIST.RT | T.DIST.RT(number1, number2) | =T.DIST.RT(1, 1) | — |
| T.INV | T.INV(number1, number2) | =T.INV(1, 1) | — |
| T.INV.2T | T.INV.2T(number1, number2) | =T.INV.2T(1, 1) | — |
| T.TEST | T.TEST(range1, range2, integer3, integer4) | =T.TEST(A1:A5, A1:A5, 1, 1) | — |
| TAN | TAN(number1) | =TAN(1) | — |
| TANH | TANH(number1) | =TANH(1) | — |
| TBILLEQ | TBILLEQ(number1, number2, number3) | =TBILLEQ(1, 1, 1) | — |
| TBILLPRICE | TBILLPRICE(number1, number2, number3) | =TBILLPRICE(1, 1, 1) | — |
| TBILLYIELD | TBILLYIELD(number1, number2, number3) | =TBILLYIELD(1, 1, 1) | — |
| TDIST | TDIST(number1, number2, integer3) | =TDIST(1, 1, 1) | — |
| TDIST2T | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| TDISTRT | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| TEXT | TEXT(number1, text2) | =TEXT(1, "text") | — |
| TIME | TIME(number1, number2, number3) | =TIME(1, 1, 1) | — |
| TIMEVALUE | TIMEVALUE(text1) | =TIMEVALUE("text") | — |
| TINV | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| TINV2T | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| TODAY | TODAY() | =TODAY() | — |
| TRANSPOSE | TRANSPOSE(range1) | =TRANSPOSE(A1:A5) | — |
| TRIM | TRIM(text1) | =TRIM("text") | — |
| TRUE | TRUE() | =TRUE() | — |
| TRUNC | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| TTEST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| UNICHAR | UNICHAR(number1) | =UNICHAR(1) | — |
| UNICODE | UNICODE(text1) | =UNICODE("text") | — |
| UPPER | UPPER(text1) | =UPPER("text") | — |
| VALUE | VALUE(value1) | =VALUE(1) | — |
| VAR | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| VAR.P | VAR.P(value1) | =VAR.P(1) | — |
| VAR.S | VAR.S(value1) | =VAR.S(1) | — |
| VARA | VARA(value1) | =VARA(1) | — |
| VARP | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| VARPA | VARPA(value1) | =VARPA(1) | — |
| VARS | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| VERSION | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| VLOOKUP | VLOOKUP(noerror1, range2, number3, boolean4) | =VLOOKUP(1, A1:A5, 1, TRUE) | — |
| WEEKDAY | WEEKDAY(number1, number2) | =WEEKDAY(1, 1) | — |
| WEEKNUM | WEEKNUM(number1, number2) | =WEEKNUM(1, 1) | — |
| WEIBULL | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| WEIBULL.DIST | WEIBULL.DIST(number1, number2, number3, boolean4) | =WEIBULL.DIST(1, 1, 1, TRUE) | — |
| WEIBULLDIST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
| WORKDAY | WORKDAY(number1, number2, [range3]) | =WORKDAY(1, 1) | — |
| WORKDAY.INTL | WORKDAY.INTL(number1, number2, noerror3, [range4]) | =WORKDAY.INTL(1, 1, 1) | — |
| XLOOKUP | XLOOKUP(noerror1, range2, range3, [value4], [number5], [number6]) | =XLOOKUP(1, A1:A5, A1:A5) | — |
| XNPV | XNPV(number1, range2, range3) | =XNPV(1, A1:A5, A1:A5) | — |
| XOR | XOR(boolean1) | =XOR(TRUE) | — |
| YEAR | YEAR(number1) | =YEAR(1) | — |
| YEARFRAC | YEARFRAC(number1, number2, integer3) | =YEARFRAC(1, 1, 1) | — |
| Z.TEST | Z.TEST(range1, number2, [number3]) | =Z.TEST(A1:A5, 1) | — |
| ZTEST | — | — | 参数元数据不可用，待清理 / Parameter metadata unavailable, pending cleanup |
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
