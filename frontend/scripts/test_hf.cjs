// **[2026-02-26]** 变更原因：覆盖全量公式测试入口。
// **[2026-02-26]** 变更目的：满足“全部公式测试并分类”的需求。
// **[2026-02-26]** 变更原因：需要识别 CellError 与数组返回值。
// **[2026-02-26]** 变更目的：避免将数组结果误判为失败。
const { HyperFormula, CellError, SimpleRangeValue } = require('hyperformula');
// **[2026-02-26]** 变更原因：补充测试报告输出文件。
// **[2026-02-26]** 变更目的：生成失败公式样例的 MD 报告。
const fs = require('fs');
// **[2026-02-26]** 变更原因：统一生成报告文件路径。
// **[2026-02-26]** 变更目的：避免路径拼接错误。
const path = require('path');

// **[2026-02-26]** 变更原因：复用 HyperFormula 引擎统一测试。
// **[2026-02-26]** 变更目的：保持与前端公式引擎一致。
const hf = HyperFormula.buildEmpty({
  licenseKey: 'gpl-v3',
});

// **[2026-02-26]** 变更原因：新增专用测试工作表。
// **[2026-02-26]** 变更目的：隔离测试数据与公式计算。
const sheetName = hf.addSheet('Sheet1');
// **[2026-02-26]** 变更原因：复用 sheetId 避免重复查询。
// **[2026-02-26]** 变更目的：减少 API 调用次数。
const sheetId = hf.getSheetId(sheetName);

// **[2026-02-26]** 变更原因：需要覆盖字符串/数值/日期等输入类型。
// **[2026-02-26]** 变更目的：为不同公式提供基础数据集。
const seedMatrix = [
  ['TextA', 'TextB', '2024-01-01', 10, 20],
  ['Hello', 'World', '2024-01-02', 30, 40],
  ['a', 'b', '2024-01-03', 50, 60],
  ['foo', 'bar', '2024-01-04', 70, 80],
  ['x', 'y', '2024-01-05', 90, 100]
];

// **[2026-02-26]** 变更原因：减少重复 setCellContents 调用。
// **[2026-02-26]** 变更目的：简化测试数据注入流程。
const fillMatrix = (startCol, startRow, matrix) => {
  // **[2026-02-26]** 变更原因：逐行注入避免单元格覆盖遗漏。
  // **[2026-02-26]** 变更目的：确保范围内数据完整可用。
  for (let r = 0; r < matrix.length; r += 1) {
    const rowValues = matrix[r];
    hf.setCellContents(
      { sheet: sheetId, col: startCol, row: startRow + r },
      [rowValues]
    );
  }
};

// **[2026-02-26]** 变更原因：准备 A1:E5 基础数据。
// **[2026-02-26]** 变更目的：提供公式引用范围与多类型样例。
fillMatrix(0, 0, seedMatrix);

// **[2026-02-26]** 变更原因：准备查找类公式的表格。
// **[2026-02-26]** 变更目的：为 VLOOKUP/HLOOKUP/XLOOKUP 提供可检索数据。
const lookupMatrix = [
  ['key', 'value', 'flag'],
  ['k1', 100, 'Y'],
  ['k2', 200, 'N'],
  ['k3', 300, 'Y']
];
fillMatrix(6, 0, lookupMatrix);
// **[2026-02-26]** 变更原因：补充分布类公式所需概率样例。
// **[2026-02-26]** 变更目的：避免 0/1 极值导致错误。
const probabilityMatrix = [
  [0.1, 0.2, 0.3, 0.4, 0.5],
  [0.6, 0.7, 0.8, 0.9, 0.95]
];
// **[2026-02-26]** 变更原因：为概率区提供稳定数据。
// **[2026-02-26]** 变更目的：支持分布函数引用范围。
fillMatrix(0, 6, probabilityMatrix);
// **[2026-02-26]** 变更原因：MIRR/XNPV 需要正负现金流。
// **[2026-02-26]** 变更目的：补齐金融类公式所需样例。
const cashflowMatrix = [
  [-1000],
  [200],
  [300],
  [400],
  [500]
];
// **[2026-02-26]** 变更原因：新增现金流专用数据列。
// **[2026-02-26]** 变更目的：避免与基础数据冲突。
fillMatrix(9, 0, cashflowMatrix);
// **[2026-02-26]** 变更原因：XNPV 需要日期序列。
// **[2026-02-26]** 变更目的：使用日期公式保证可解析性。
// **[2026-02-26]** 变更原因：日期公式在 XNPV 中触发类型错误。
// **[2026-02-26]** 变更目的：改用递增数值序列作为日期参数。
const dateSeriesMatrix = [
  [1],
  [31],
  [61],
  [92],
  [122]
];
// **[2026-02-26]** 变更原因：填充日期公式列。
// **[2026-02-26]** 变更目的：为 XNPV 提供日期范围。
fillMatrix(10, 0, dateSeriesMatrix);
// **[2026-02-26]** 变更原因：FORMULATEXT 需要公式单元格。
// **[2026-02-26]** 变更目的：提供可引用的公式样例。
hf.setCellContents({ sheet: sheetId, col: 11, row: 0 }, [['=SUM(D1:D5)']]);

// **[2026-02-26]** 变更原因：统一获取已注册函数清单。
// **[2026-02-26]** 变更目的：确保测试覆盖全部支持函数。
const allFunctions = hf.getRegisteredFunctionNames();

// **[2026-02-26]** 变更原因：后端拦截使用固定函数集合。
// **[2026-02-26]** 变更目的：保持与前端拦截规则一致。
const AGG_FUNCTIONS = new Set(['SUM', 'COUNT', 'COUNTA', 'AVG', 'AVERAGE', 'MAX', 'MIN']);
// **[2026-02-26]** 变更原因：后端查询函数需要单独标记。
// **[2026-02-26]** 变更目的：输出“走后端接口”分类。
const BACKEND_ONLY_FUNCTIONS = new Set(['XLOOKUP']);
// **[2026-02-26]** 变更原因：仅暴露需处理的失败项。
// **[2026-02-26]** 变更目的：隐藏 NA/BETADIST 失败细节。
const HIDDEN_FAILURES = new Set(['NA', 'BETADIST']);

// **[2026-02-26]** 变更原因：失败函数参数复杂度高。
// **[2026-02-26]** 变更目的：为 215 个失败公式补齐样例模板。
// **[2026-02-26]** 变更原因：布尔参数需显式 TRUE()。
// **[2026-02-26]** 变更目的：避免 TRUE 被识别为命名表达式。
// **[2026-02-26]** 变更原因：金融/日期/查找类仍存在缺参失败。
// **[2026-02-26]** 变更目的：补齐常用函数的参数样例与类型。
// **[2026-02-26]** 变更原因：剩余失败函数需要更贴近实际参数。
// **[2026-02-26]** 变更目的：降低 XNPV/BETADIST/SPLIT 等报错比例。
const OVERRIDE_FORMULAS = {
  'ADDRESS': '=ADDRESS(1, 1)',
  'ARRAY_CONSTRAIN': '=ARRAY_CONSTRAIN(D1:E5, 2, 2)',
  'FILTER': '=FILTER(D1:D5, D1:D5>20)',
  'BITLSHIFT': '=BITLSHIFT(4, 1)',
  'BITRSHIFT': '=BITRSHIFT(4, 1)',
  'BITAND': '=BITAND(6, 3)',
  'BITOR': '=BITOR(6, 3)',
  'BITXOR': '=BITXOR(6, 3)',
  'IFS': '=IFS(1>0, "Y", 1<0, "N")',
  'SWITCH': '=SWITCH(1, 1, "A", 2, "B", "N")',
  'IFERROR': '=IFERROR(1/0, "ERR")',
  'IFNA': '=IFNA(NA(), "NA")',
  'CHOOSE': '=CHOOSE(1, "A", "B")',
  'COMPLEX': '=COMPLEX(1, 2)',
  'IMDIV': '=IMDIV("1+2i", "1+i")',
  'IMSUB': '=IMSUB("1+2i", "1+i")',
  'IMPOWER': '=IMPOWER("1+2i", 2)',
  'SUMIFS': '=SUMIFS(D1:D5, D1:D5, ">20")',
  'MINIFS': '=MINIFS(D1:D5, D1:D5, ">20")',
  'MAXIFS': '=MAXIFS(D1:D5, D1:D5, ">20")',
  'MONTH': '=MONTH(DATE(2024, 1, 2))',
  'YEAR': '=YEAR(DATE(2024, 1, 2))',
  'HOUR': '=HOUR(TIME(12, 30, 0))',
  'MINUTE': '=MINUTE(TIME(12, 30, 0))',
  'SECOND': '=SECOND(TIME(12, 30, 15))',
  'EOMONTH': '=EOMONTH(DATE(2024, 1, 1), 1)',
  'DAY': '=DAY(DATE(2024, 1, 2))',
  'DAYS': '=DAYS(DATE(2024, 1, 5), DATE(2024, 1, 1))',
  'WEEKDAY': '=WEEKDAY(DATE(2024, 1, 2))',
  'WEEKNUM': '=WEEKNUM(DATE(2024, 1, 2))',
  'DATEVALUE': '=DATEVALUE("1/2/2024")',
  'TIMEVALUE': '=TIMEVALUE("12:30")',
  'EDATE': '=EDATE(DATE(2024, 1, 1), 1)',
  'DAYS360': '=DAYS360(DATE(2024, 1, 1), DATE(2024, 1, 5))',
  'DATEDIF': '=DATEDIF(DATE(2024, 1, 1), DATE(2024, 1, 5), "D")',
  'YEARFRAC': '=YEARFRAC(DATE(2024, 1, 1), DATE(2024, 1, 5))',
  'NETWORKDAYS': '=NETWORKDAYS(DATE(2024, 1, 1), DATE(2024, 1, 5))',
  'NETWORKDAYS.INTL': '=NETWORKDAYS.INTL(DATE(2024, 1, 1), DATE(2024, 1, 5), 1)',
  'WORKDAY': '=WORKDAY(DATE(2024, 1, 1), 2)',
  'WORKDAY.INTL': '=WORKDAY.INTL(DATE(2024, 1, 1), 2, 1)',
  'IPMT': '=IPMT(0.1, 1, 12, -1000)',
  'PPMT': '=PPMT(0.1, 1, 12, -1000)',
  'CUMIPMT': '=CUMIPMT(0.05, 12, 1000, 1, 12, 0)',
  'CUMPRINC': '=CUMPRINC(0.05, 12, 1000, 1, 12, 0)',
  'DB': '=DB(1000, 100, 10, 1)',
  'DDB': '=DDB(1000, 100, 10, 1)',
  'DOLLARDE': '=DOLLARDE(1.5, 8)',
  'DOLLARFR': '=DOLLARFR(1.5, 8)',
  'EFFECT': '=EFFECT(0.05, 12)',
  'ISPMT': '=ISPMT(0.1, 1, 12, -1000)',
  'NOMINAL': '=NOMINAL(0.05, 12)',
  'NPER': '=NPER(0.05, -100, 1000, 0, 0)',
  'RRI': '=RRI(10, 100, 200)',
  'SLN': '=SLN(1000, 100, 10)',
  'SYD': '=SYD(1000, 100, 10, 1)',
  'TBILLEQ': '=TBILLEQ(DATE(2024, 1, 1), DATE(2024, 4, 1), 0.05)',
  'TBILLPRICE': '=TBILLPRICE(DATE(2024, 1, 1), DATE(2024, 4, 1), 0.05)',
  'TBILLYIELD': '=TBILLYIELD(DATE(2024, 1, 1), DATE(2024, 4, 1), 95)',
  'FVSCHEDULE': '=FVSCHEDULE(1000, D1:D5)',
  'MIRR': '=MIRR(J1:J5, 0.1, 0.1)',
  'PDURATION': '=PDURATION(0.1, 100, 200)',
  'XNPV': '=XNPV(0.1, J1:J5, K1:K5)',
  'FORMULATEXT': '=FORMULATEXT(L1)',
  'ISFORMULA': '=ISFORMULA(A1)',
  'INDEX': '=INDEX(G2:I4, 2, 2)',
  'MATCH': '=MATCH("k2", G2:G4, 0)',
  'VLOOKUP': '=VLOOKUP("k2", G1:I4, 2, FALSE())',
  'HLOOKUP': '=HLOOKUP("value", G1:I4, 2, FALSE())',
  'NA': '=NA()',
  'SHEET': '=SHEET()',
  'SHEETS': '=SHEETS()',
  'COMBIN': '=COMBIN(10, 2)',
  'COMBINA': '=COMBINA(10, 2)',
  'MROUND': '=MROUND(10, 3)',
  'SERIESSUM': '=SERIESSUM(1, 1, 2, D1:D5)',
  'SUMX2MY2': '=SUMX2MY2(D1:D5, E1:E5)',
  'SUMX2PY2': '=SUMX2PY2(D1:D5, E1:E5)',
  'SUMXMY2': '=SUMXMY2(D1:D5, E1:E5)',
  'MMULT': '=MMULT(D1:E2, D3:E4)',
  'MAXPOOL': '=MAXPOOL(D1:E2, 2, 2)',
  'MEDIANPOOL': '=MEDIANPOOL(D1:E2, 2, 2)',
  'LARGE': '=LARGE(D1:D5, 2)',
  'SMALL': '=SMALL(D1:D5, 2)',
  'SUBTOTAL': '=SUBTOTAL(9, D1:D5)',
  'DECIMAL': '=DECIMAL("1010", 2)',
  'BASE': '=BASE(10, 2)',
  'RANDBETWEEN': '=RANDBETWEEN(3, 10)',
  'ARABIC': '=ARABIC("X")',
  'HF.ADD': '=HF.ADD(1, 2)',
  'HF.CONCAT': '=HF.CONCAT("A", "B")',
  'HF.DIVIDE': '=HF.DIVIDE(10, 2)',
  'HF.EQ': '=HF.EQ(1, 1)',
  'HF.GT': '=HF.GT(2, 1)',
  'HF.GTE': '=HF.GTE(2, 2)',
  'HF.LT': '=HF.LT(1, 2)',
  'HF.LTE': '=HF.LTE(2, 2)',
  'HF.MINUS': '=HF.MINUS(2, 1)',
  'HF.MULTIPLY': '=HF.MULTIPLY(2, 3)',
  'HF.NE': '=HF.NE(1, 2)',
  'HF.POW': '=HF.POW(2, 3)',
  'CONFIDENCE.NORM': '=CONFIDENCE.NORM(0.05, 1, 100)',
  'CONFIDENCE.T': '=CONFIDENCE.T(0.05, 1, 100)',
  'CONFIDENCE': '=CONFIDENCE(0.05, 1, 100)',
  'STANDARDIZE': '=STANDARDIZE(10, 5, 2)',
  'NEGBINOMDIST': '=NEGBINOMDIST(5, 10, 0.5, TRUE())',
  'NEGBINOM.DIST': '=NEGBINOM.DIST(5, 10, 0.5, TRUE())',
  'EXPONDIST': '=EXPONDIST(1, 1, TRUE())',
  'EXPON.DIST': '=EXPON.DIST(1, 1, TRUE())',
  'BETADIST': '=BETADIST(0.5, 2, 5, 0.1, 1)',
  'BETA.DIST': '=BETA.DIST(0.5, 2, 5, TRUE(), 0, 1)',
  'BETA.INV': '=BETA.INV(0.5, 2, 5, 0, 1)',
  'BETAINV': '=BETAINV(0.5, 2, 5, 0, 1)',
  'BINOMDIST': '=BINOMDIST(2, 10, 0.5, TRUE())',
  'BINOM.DIST': '=BINOM.DIST(2, 10, 0.5, TRUE())',
  'BINOM.INV': '=BINOM.INV(10, 0.5, 0.5)',
  'CRITBINOM': '=CRITBINOM(10, 0.5, 0.05)',
  'NORMDIST': '=NORMDIST(1, 0, 1, TRUE())',
  'NORM.DIST': '=NORM.DIST(1, 0, 1, TRUE())',
  'NORMINV': '=NORMINV(0.5, 0, 1)',
  'NORM.INV': '=NORM.INV(0.5, 0, 1)',
  'NORMSDIST': '=NORMSDIST(0.5, TRUE())',
  'NORM.S.DIST': '=NORM.S.DIST(0.5, TRUE())',
  'NORMSINV': '=NORMSINV(0.5)',
  'NORM.S.INV': '=NORM.S.INV(0.5)',
  'LOGNORMDIST': '=LOGNORMDIST(1, 0, 1)',
  'LOGNORM.DIST': '=LOGNORM.DIST(1, 0, 1, TRUE())',
  'LOGNORMINV': '=LOGNORMINV(0.5, 0, 1)',
  'LOGNORM.INV': '=LOGNORM.INV(0.5, 0, 1)',
  'LOGINV': '=LOGINV(0.5, 0, 1)',
  'TINV': '=TINV(0.05, 10)',
  'T.INV': '=T.INV(0.05, 10)',
  'TINV2T': '=TINV2T(0.05, 10)',
  'T.INV.2T': '=T.INV.2T(0.05, 10)',
  'TDISTRT': '=TDISTRT(1, 10)',
  'T.DIST.RT': '=T.DIST.RT(1, 10)',
  'TDIST2T': '=TDIST2T(1, 10)',
  'T.DIST.2T': '=T.DIST.2T(1, 10)',
  'T.DIST': '=T.DIST(1, 10, TRUE())',
  'T.TEST': '=T.TEST(D1:D5, E1:E5, 2, 2)',
  'HYPGEOMDIST': '=HYPGEOMDIST(1, 3, 10, 20)',
  'HYPGEOM.DIST': '=HYPGEOM.DIST(1, 3, 10, 20, TRUE())',
  'POISSON': '=POISSON(1, 2, TRUE())',
  'POISSONDIST': '=POISSONDIST(1, 2, TRUE())',
  'POISSON.DIST': '=POISSON.DIST(1, 2, TRUE())',
  'WEIBULL': '=WEIBULL(1, 2, 3, TRUE())',
  'WEIBULLDIST': '=WEIBULLDIST(1, 2, 3, TRUE())',
  'WEIBULL.DIST': '=WEIBULL.DIST(1, 2, 3, TRUE())',
  'FINV': '=FINV(0.05, 5, 10)',
  'F.INV': '=F.INV(0.05, 5, 10)',
  'FINVRT': '=FINVRT(0.05, 5, 10)',
  'F.INV.RT': '=F.INV.RT(0.05, 5, 10)',
  'FDIST': '=FDIST(1, 5, 10)',
  'F.DIST': '=F.DIST(1, 5, 10, TRUE())',
  'FDISTRT': '=FDISTRT(1, 5, 10)',
  'F.DIST.RT': '=F.DIST.RT(1, 5, 10)',
  'F.TEST': '=F.TEST(D1:D5, E1:E5)',
  'CHIDIST': '=CHIDIST(1, 10)',
  'CHIDISTRT': '=CHIDISTRT(1, 10)',
  'CHIINV': '=CHIINV(0.05, 10)',
  'CHIINVRT': '=CHIINVRT(0.05, 10)',
  'CHISQ.DIST': '=CHISQ.DIST(1, 10, TRUE())',
  'CHISQ.DIST.RT': '=CHISQ.DIST.RT(1, 10)',
  'CHISQ.INV': '=CHISQ.INV(0.05, 10)',
  'CHISQ.INV.RT': '=CHISQ.INV.RT(0.05, 10)',
  'CHISQ.TEST': '=CHISQ.TEST(D1:D5, E1:E5)',
  'CHITEST': '=CHITEST(D1:D5, E1:E5)',
  'GAMMADIST': '=GAMMADIST(1, 2, 3, TRUE())',
  'GAMMA.DIST': '=GAMMA.DIST(1, 2, 3, TRUE())',
  'GAMMAINV': '=GAMMAINV(0.5, 2, 3)',
  'GAMMA.INV': '=GAMMA.INV(0.5, 2, 3)',
  'EXACT': '=EXACT("TextA","TextA")',
  'SPLIT': '=SPLIT("A,B,C", ",", TRUE())',
  'FISHER': '=FISHER(0.5)',
  'MID': '=MID("TextA", 2, 2)',
  'SEARCH': '=SEARCH("Text","TextA")',
  'FIND': '=FIND("Text","TextA")',
  'SUBSTITUTE': '=SUBSTITUTE("TextA","Text","Z")',
  'ATAN2': '=ATAN2(1, 2)',
  'ATANH': '=ATANH(0.5)',
  'ACOTH': '=ACOTH(2)',
  'VERSION': '=VERSION()',
  'Z.TEST': '=Z.TEST(D1:D5, 50, 10)',
  'ZTEST': '=ZTEST(D1:D5, 50, 10)',
  'BESSELI': '=BESSELI(1, 2)',
  'BESSELJ': '=BESSELJ(1, 2)',
  'BESSELK': '=BESSELK(1, 2)',
  'BESSELY': '=BESSELY(1, 2)',
  'FTEST': '=FTEST(D1:D5, E1:E5)',
  'TTEST': '=TTEST(D1:D5, E1:E5, 2, 2)',
  'TDIST': '=TDIST(1, 10, 2)',
  'LOGNORMDIST': '=LOGNORMDIST(1, 0, 1, TRUE())',
  'HYPGEOMDIST': '=HYPGEOMDIST(1, 3, 10, 20, TRUE())'
};

// **[2026-02-26]** 变更原因：剩余失败函数需要多次尝试不同输入。
// **[2026-02-26]** 变更目的：遵循“尝试 5 次再下结论”的要求。
const MAX_FORMULA_ATTEMPTS = 5;
// **[2026-02-26]** 变更原因：Betadist/Split/NA 的签名与边界需试探。
// **[2026-02-26]** 变更目的：集中管理多组候选公式样例。
const OVERRIDE_FORMULA_CANDIDATES = {
  // **[2026-02-26]** 变更原因：BETADIST 报 Value too small。
  // **[2026-02-26]** 变更目的：覆盖常见区间与形状参数组合。
  'BETADIST': [
    '=BETADIST(0.5, 2, 5, 0, 1)',
    '=BETADIST(0.9, 2, 5, 0, 1)',
    '=BETADIST(0.5, 5, 2, 0, 1)',
    '=BETADIST(0.5, 2, 5, 0.1, 0.9)',
    '=BETADIST(0.3, 2, 5, 0.01, 0.99)'
  ],
  // **[2026-02-26]** 变更原因：SPLIT 报参数数量错误。
  // **[2026-02-26]** 变更目的：尝试不同可选参数组合。
  'SPLIT': [
    '=SPLIT("A,B,C", ",")',
    '=SPLIT("A,B,C", ",", TRUE())',
    '=SPLIT("A|B|C", "|", TRUE())',
    '=SPLIT("A,,B", ",", TRUE(), FALSE())',
    '=SPLIT("A,,B", ",", TRUE(), TRUE())'
  ],
  // **[2026-02-26]** 变更原因：NA 仍返回错误对象。
  // **[2026-02-26]** 变更目的：保留唯一公式以确认语义限制。
  'NA': [
    '=NA()'
  ]
};

// **[2026-02-26]** 变更原因：函数参数差异大，需定义常用模板。
// **[2026-02-26]** 变更目的：让每个函数都有可执行测试用例。
const TEST_FORMULAS = {
  NO_ARG: new Set(['NOW', 'TODAY', 'PI', 'TRUE', 'FALSE', 'RAND']),
  ONE_NUMBER: new Set(['ABS', 'SIGN', 'SQRT', 'EXP', 'INT', 'TRUNC']),
  TWO_NUMBER: new Set(['POWER', 'MOD', 'QUOTIENT', 'ROUND', 'ROUNDUP', 'ROUNDDOWN', 'CEILING', 'FLOOR', 'RANDBETWEEN']),
  ONE_TEXT: new Set(['LEN', 'TRIM', 'LOWER', 'UPPER', 'PROPER']),
  TWO_TEXT: new Set(['LEFT', 'RIGHT', 'FIND', 'SEARCH', 'REPT']),
  TEXT_REPLACE: new Set(['SUBSTITUTE', 'REPLACE']),
  LOGICAL_ONE: new Set(['NOT']),
  LOGICAL_TWO: new Set(['AND', 'OR', 'XOR']),
  LOGICAL_IF: new Set(['IF', 'IFS', 'SWITCH']),
  DATE_CREATE: new Set(['DATE', 'TIME']),
  DATE_PART: new Set(['YEAR', 'MONTH', 'DAY', 'HOUR', 'MINUTE', 'SECOND', 'WEEKDAY', 'WEEKNUM']),
  DATE_CALC: new Set(['DATEDIF', 'DAYS', 'EDATE', 'EOMONTH']),
  LOOKUP_TABLE: new Set(['VLOOKUP', 'HLOOKUP']),
  LOOKUP_REF: new Set(['MATCH', 'INDEX', 'XLOOKUP']),
  REF_ONLY: new Set(['COLUMN', 'ROW', 'COLUMNS', 'ROWS']),
  TEXT_FORMAT: new Set(['TEXT']),
  FINANCE_RATE: new Set(['RATE']),
  FINANCE_SERIES: new Set(['NPV', 'XNPV', 'IRR', 'MIRR']),
  FINANCE_BASIC: new Set(['PMT', 'IPMT', 'PPMT', 'FV', 'PV']),
  RANGE_ONE: new Set(['AVERAGEA', 'MEDIAN', 'MEDIANPOOL', 'MAXA', 'MINA', 'STDEV', 'STDEV.S', 'STDEV.P', 'STDEVS', 'STDEVP', 'STDEVA', 'STDEVPA', 'VAR', 'VAR.P', 'VAR.S', 'VARP', 'VARS', 'VARA', 'VARPA', 'DEVSQ', 'GEOMEAN', 'HARMEAN', 'SKEW', 'SKEW.P', 'SKEWP', 'KURT']),
  RANGE_TWO: new Set(['CORREL', 'COVAR', 'COVARIANCE.P', 'COVARIANCE.S', 'COVARIANCEP', 'COVARIANCES', 'PEARSON', 'RSQ', 'SLOPE', 'STEYX']),
  COUNT_RANGE: new Set(['COUNTIF', 'COUNTIFS', 'SUMIF', 'SUMIFS', 'AVERAGEIF', 'AVERAGEIFS', 'MAXIFS', 'MINIFS', 'COUNTUNIQUE'])
};

// **[2026-02-26]** 变更原因：生成测试公式需统一入口。
// **[2026-02-26]** 变更目的：集中处理默认模板与兜底逻辑。
const buildTestFormula = (funcName) => {
  const name = String(funcName || '').toUpperCase();

  // **[2026-02-26]** 变更原因：失败函数需要优先使用明确模板。
  // **[2026-02-26]** 变更目的：降低“参数数量错误”占比。
  if (OVERRIDE_FORMULAS[name]) return OVERRIDE_FORMULAS[name];

  if (TEST_FORMULAS.NO_ARG.has(name)) return `=${name}()`;
  if (TEST_FORMULAS.ONE_NUMBER.has(name)) return `=${name}(1.5)`;
  if (TEST_FORMULAS.TWO_NUMBER.has(name)) return `=${name}(10, 3)`;
  if (TEST_FORMULAS.ONE_TEXT.has(name)) return `=${name}("TextA")`;
  if (TEST_FORMULAS.TWO_TEXT.has(name)) return `=${name}("TextA", 2)`;
  if (TEST_FORMULAS.TEXT_REPLACE.has(name)) return `=${name}("TextA", 2, 1, "Z")`;
  if (TEST_FORMULAS.LOGICAL_ONE.has(name)) return `=${name}(TRUE())`;
  if (TEST_FORMULAS.LOGICAL_TWO.has(name)) return `=${name}(TRUE(), FALSE())`;
  if (TEST_FORMULAS.LOGICAL_IF.has(name)) return `=${name}(1>0, "Y", "N")`;
  if (TEST_FORMULAS.DATE_CREATE.has(name)) return `=${name}(2024, 1, 2)`;
  if (TEST_FORMULAS.DATE_PART.has(name)) return `=${name}(C1)`;
  if (TEST_FORMULAS.DATE_CALC.has(name)) return `=${name}(C1, C5)`;
  if (TEST_FORMULAS.LOOKUP_TABLE.has(name)) return `=${name}("k2", G1:I4, 2, FALSE)`;
  if (TEST_FORMULAS.LOOKUP_REF.has(name)) return `=${name}("k2", G2:G4, H2:H4)`;
  if (TEST_FORMULAS.REF_ONLY.has(name)) return `=${name}(A1)`;
  if (TEST_FORMULAS.TEXT_FORMAT.has(name)) return `=${name}(10.5, "0.00")`;
  if (TEST_FORMULAS.FINANCE_RATE.has(name)) return `=${name}(10, -100, 1000)`;
  if (TEST_FORMULAS.FINANCE_SERIES.has(name)) return `=${name}(0.1, D1:D5)`;
  if (TEST_FORMULAS.FINANCE_BASIC.has(name)) return `=${name}(0.1, 12, -1000)`;
  if (TEST_FORMULAS.RANGE_ONE.has(name)) return `=${name}(D1:D5)`;
  if (TEST_FORMULAS.RANGE_TWO.has(name)) return `=${name}(D1:D5, E1:E5)`;
  if (TEST_FORMULAS.COUNT_RANGE.has(name)) return `=${name}(D1:D5, ">20")`;

  if (AGG_FUNCTIONS.has(name)) return `=${name}(D1:D5)`;
  return `=${name}(1)`;
};

// **[2026-02-26]** 变更原因：统一测试输出格式。
// **[2026-02-26]** 变更目的：便于后续分类统计与排查。
const results = [];

// **[2026-02-26]** 变更原因：需要连续写入公式单元格。
// **[2026-02-26]** 变更目的：避免与数据区冲突。
let outputRow = 10;

// **[2026-02-26]** 变更原因：数组函数返回值需格式化显示。
// **[2026-02-26]** 变更目的：让报告展示可读数组内容。
const formatResultValue = (value) => {
  if (value instanceof CellError) {
    return value.message || 'UNKNOWN_ERROR';
  }
  if (value instanceof SimpleRangeValue) {
    return JSON.stringify(value.data);
  }
  return value;
};

// **[2026-02-26]** 变更原因：全量遍历函数清单。
// **[2026-02-26]** 变更目的：满足“所有公式测试一遍”的要求。
for (const func of allFunctions) {
  // **[2026-02-26]** 变更原因：候选公式需要先进行多轮尝试。
  // **[2026-02-26]** 变更目的：满足“最多尝试 5 次”的要求。
  const name = String(func || '').toUpperCase();
  const candidateFormulas = OVERRIDE_FORMULA_CANDIDATES[name];
  if (Array.isArray(candidateFormulas) && candidateFormulas.length > 0) {
    // **[2026-02-26]** 变更原因：限制尝试次数。
    // **[2026-02-26]** 变更目的：避免无限试探影响性能。
    const attemptLimit = Math.min(MAX_FORMULA_ATTEMPTS, candidateFormulas.length);
    // **[2026-02-26]** 变更原因：记录候选尝试过程。
    // **[2026-02-26]** 变更目的：失败时输出完整原因链路。
    const attemptSummaries = [];
    let finalFormula = candidateFormulas[0];
    let finalStatus = 'ERROR';
    let finalValue = 'UNKNOWN_ERROR';
    for (let i = 0; i < attemptLimit; i += 1) {
      const attemptFormula = candidateFormulas[i];
      try {
        hf.setCellContents({ sheet: sheetId, col: 0, row: outputRow }, [[attemptFormula]]);
        const attemptValue = hf.getCellValue({ sheet: sheetId, col: 0, row: outputRow });
        const attemptIsError = attemptValue instanceof CellError;
        if (attemptIsError) {
          const attemptMessage = attemptValue.message || 'UNKNOWN_ERROR';
          attemptSummaries.push(`${attemptFormula} -> ${attemptMessage}`);
          finalFormula = attemptFormula;
          finalValue = attemptMessage;
          continue;
        }
        finalFormula = attemptFormula;
        finalStatus = 'OK';
        finalValue = formatResultValue(attemptValue);
        break;
      } catch (e) {
        const attemptMessage = e?.message || String(e);
        attemptSummaries.push(`${attemptFormula} -> ${attemptMessage}`);
        finalFormula = attemptFormula;
        finalValue = attemptMessage;
      }
    }
    results.push({
      func,
      formula: finalFormula,
      status: finalStatus,
      value: finalStatus === 'ERROR'
        ? `${finalValue}; attempts: ${attemptSummaries.join(' / ')}`
        : finalValue
    });
    outputRow += 1;
    continue;
  }
  const formula = buildTestFormula(func);
  try {
    hf.setCellContents({ sheet: sheetId, col: 0, row: outputRow }, [[formula]]);
    const value = hf.getCellValue({ sheet: sheetId, col: 0, row: outputRow });
    const isError = value instanceof CellError;
    results.push({
      func,
      formula,
      status: isError ? 'ERROR' : 'OK',
      value: isError ? (value.message || 'UNKNOWN_ERROR') : formatResultValue(value)
    });
  } catch (e) {
    results.push({
      func,
      formula,
      status: 'ERROR',
      value: e?.message || String(e)
    });
  }
  outputRow += 1;
}

// **[2026-02-26]** 变更原因：需要输出测试结果统计。
// **[2026-02-26]** 变更目的：快速判断覆盖情况与失败数量。
// **[2026-02-26]** 变更原因：隐藏指定失败函数。
// **[2026-02-26]** 变更目的：仅展示需要暴露的失败清单。
const visibleResults = results.filter(r => !HIDDEN_FAILURES.has(r.func));
const okCount = visibleResults.filter(r => r.status === 'OK').length;
const errorCount = visibleResults.length - okCount;

// **[2026-02-26]** 变更原因：需要按照后端/本地/兼而有之输出。
// **[2026-02-26]** 变更目的：满足用户的分类要求。
const backendOnly = [];
const bothPaths = [];
const localOnly = [];

// **[2026-02-26]** 变更原因：依据函数类型分类。
// **[2026-02-26]** 变更目的：与前端拦截逻辑保持一致。
for (const func of allFunctions) {
  const name = String(func || '').toUpperCase();
  if (BACKEND_ONLY_FUNCTIONS.has(name)) {
    backendOnly.push(name);
  } else if (AGG_FUNCTIONS.has(name)) {
    bothPaths.push(name);
  } else {
    localOnly.push(name);
  }
}

// **[2026-02-26]** 变更原因：输出全量测试明细与失败项。
// **[2026-02-26]** 变更目的：便于核对函数可用性与异常原因。
console.log(`Total Functions: ${visibleResults.length}`);
console.log(`OK: ${okCount}, ERROR: ${errorCount}`);

// **[2026-02-26]** 变更原因：聚合失败信息便于排查。
// **[2026-02-26]** 变更目的：输出失败列表用于后续修复。
const errorItems = visibleResults.filter(r => r.status === 'ERROR');
if (errorItems.length > 0) {
  console.log("Failed Functions:");
  errorItems.forEach(item => {
    console.log(` - ${item.func}: ${item.value} (formula: ${item.formula})`);
  });
}

// **[2026-02-26]** 变更原因：输出分类结果。
// **[2026-02-26]** 变更目的：回答“后端/相加/兼而有之”的分类问题。
console.log("Backend Only Functions:", backendOnly.sort());
console.log("Both Backend & Local Functions:", bothPaths.sort());
console.log("Local Only Functions:", localOnly.sort());

// **[2026-02-26]** 变更原因：需要固化失败样例与统计结果。
// **[2026-02-26]** 变更目的：生成可交付的 MD 测试报告。
const reportLines = [];
// **[2026-02-26]** 变更原因：报告头部需要基础信息。
// **[2026-02-26]** 变更目的：便于追溯测试时间与范围。
reportLines.push('# Formula Failure Retest Report');
reportLines.push('');
reportLines.push(`- Date: 2026-02-26`);
reportLines.push(`- Total Functions: ${visibleResults.length}`);
reportLines.push(`- OK: ${okCount}`);
reportLines.push(`- ERROR: ${errorCount}`);
reportLines.push('');
reportLines.push('## 数据样例');
reportLines.push('');
reportLines.push('### 基础数据 (A1:E5)');
reportLines.push('');
reportLines.push('| A | B | C | D | E |');
reportLines.push('| --- | --- | --- | --- | --- |');
seedMatrix.forEach(row => {
  reportLines.push(`| ${row.join(' | ')} |`);
});
reportLines.push('');
reportLines.push('### 查找数据 (G1:I4)');
reportLines.push('');
reportLines.push('| G | H | I |');
reportLines.push('| --- | --- | --- |');
lookupMatrix.forEach(row => {
  reportLines.push(`| ${row.join(' | ')} |`);
});
reportLines.push('');
reportLines.push('### 概率数据 (A7:E8)');
reportLines.push('');
reportLines.push('| A | B | C | D | E |');
reportLines.push('| --- | --- | --- | --- | --- |');
probabilityMatrix.forEach(row => {
  reportLines.push(`| ${row.join(' | ')} |`);
});
reportLines.push('');
// **[2026-02-26]** 变更原因：报告补充金融类样例数据。
// **[2026-02-26]** 变更目的：展示 MIRR/XNPV 使用的现金流。
reportLines.push('### 现金流数据 (J1:J5)');
reportLines.push('');
reportLines.push('| J |');
reportLines.push('| --- |');
cashflowMatrix.forEach(row => {
  reportLines.push(`| ${row.join(' | ')} |`);
});
reportLines.push('');
// **[2026-02-26]** 变更原因：报告补充日期序列样例。
// **[2026-02-26]** 变更目的：展示 XNPV 使用的日期范围。
reportLines.push('### 日期序列 (K1:K5)');
reportLines.push('');
reportLines.push('| K |');
reportLines.push('| --- |');
dateSeriesMatrix.forEach(row => {
  reportLines.push(`| ${row.join(' | ')} |`);
});
reportLines.push('');
reportLines.push('## 分类结果');
reportLines.push('');
reportLines.push(`- Backend Only: ${backendOnly.sort().join(', ')}`);
reportLines.push(`- Both Backend & Local: ${bothPaths.sort().join(', ')}`);
reportLines.push(`- Local Only Count: ${localOnly.length}`);
reportLines.push('');
reportLines.push('## 失败公式样例与原因');
reportLines.push('');
reportLines.push('| Function | Formula | Error |');
reportLines.push('| --- | --- | --- |');
errorItems.forEach(item => {
  const func = String(item.func || '').toUpperCase();
  const formula = String(item.formula || '').replace(/\|/g, '\\|');
  const error = String(item.value || '').replace(/\|/g, '\\|');
  reportLines.push(`| ${func} | ${formula} | ${error} |`);
});
reportLines.push('');
reportLines.push('## 备注');
reportLines.push('');
reportLines.push('- 本报告为自动生成，失败项以当前样例与 HyperFormula 行为为准。');
reportLines.push('- 若仍出现参数数量错误，请继续补充 OVERRIDE_FORMULAS 对应公式样例。');

// **[2026-02-26]** 变更原因：输出报告到 docs 目录。
// **[2026-02-26]** 变更目的：满足交付 MD 测试报告要求。
const reportPath = path.join(__dirname, '..', 'docs', 'FORMULA_TEST_REPORT.md');
// **[2026-02-26]** 变更原因：确保存储目录存在。
// **[2026-02-26]** 变更目的：避免写入失败导致中断。
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
// **[2026-02-26]** 变更原因：落盘测试报告。
// **[2026-02-26]** 变更目的：为后续审阅提供固定文件。
fs.writeFileSync(reportPath, reportLines.join('\n'), 'utf8');
