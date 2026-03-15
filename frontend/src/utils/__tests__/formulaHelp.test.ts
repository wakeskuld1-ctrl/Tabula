// ### 变更记录
// - 2026-03-15 21:30: 原因=修复乱码并补齐公式帮助测试; 目的=保证过滤逻辑与双语文案可用
// - 2026-03-15 21:30: 原因=补充界面文案常量测试; 目的=避免再次出现乱码回归
// - 2026-03-15 23:10: 原因=新增常驻 tips 需要选择逻辑测试; 目的=确保默认展示与过滤一致

import { describe, it, expect } from "vitest";
// ### 变更记录
// - 2026-03-15 23:10: 原因=新增选择函数测试; 目的=覆盖常驻 tips 逻辑入口
// ### 变更记录
// - 2026-03-15: 原因=新增触发与折叠行为; 目的=覆盖显示条件与摘要格式
import {
  filterFormulaHelpItems,
  selectFormulaHelpItems,
  shouldShowFormulaHelp,
  formatFormulaTipSummary,
} from "../formulaHelp";
import { APP_LABELS } from "../appLabels";

// ### 变更记录
// - 2026-03-15 21:30: 原因=测试需要稳定样本; 目的=覆盖名称/语法/用途/参数说明字段
// - 2026-03-15 21:30: 原因=覆盖中英文搜索场景; 目的=保证国际化过滤一致
const sampleItems = [
  {
    name: "SUM",
    syntax: "SUM(range)",
    example: "=SUM(A1:A5)",
    paramNotes: "range/范围",
    purpose: "统计与汇总计算 / Statistical",
    note: "—",
  },
  {
    name: "VLOOKUP",
    syntax: "VLOOKUP(lookup_value, table)",
    example: "=VLOOKUP(A1, table)",
    paramNotes: "lookup_value/查找值",
    purpose: "查找与引用数据 / Lookup",
    note: "—",
  },
];

// ### 变更记录
// - 2026-03-15 21:30: 原因=验证空查询行为; 目的=确保默认返回全量数据
// - 2026-03-15 21:30: 原因=验证字段匹配; 目的=覆盖名称/用途检索路径
// - 2026-03-15 23:10: 原因=新增常驻 tips 策略; 目的=确保前缀过滤与默认截断可用
describe("filterFormulaHelpItems", () => {
  it("returns all items when query is empty", () => {
    expect(filterFormulaHelpItems(sampleItems, "").length).toBe(2);
  });

  it("filters by name", () => {
    expect(filterFormulaHelpItems(sampleItems, "sum")[0].name).toBe("SUM");
  });

  it("filters by purpose", () => {
    expect(filterFormulaHelpItems(sampleItems, "查找")[0].name).toBe("VLOOKUP");
  });
});

// ### 变更记录
// - 2026-03-15 23:10: 原因=常驻 tips 需要默认截断; 目的=验证 Top N 返回逻辑
// - 2026-03-15 23:10: 原因=公式输入支持前缀过滤; 目的=验证 '=前缀' 能命中目标函数
describe("selectFormulaHelpItems", () => {
  it("returns top items when query is empty", () => {
    const result = selectFormulaHelpItems(sampleItems, "", 1);
    expect(result.length).toBe(1);
    expect(result[0].name).toBe("SUM");
  });

  it("filters items when query is provided", () => {
    const result = selectFormulaHelpItems(sampleItems, "=VLOOK", 5);
    expect(result[0].name).toBe("VLOOKUP");
  });
});

// ### 变更记录
// - 2026-03-15: 原因=按条件显示公式帮助; 目的=验证 '=' 和 fx 触发逻辑
// - 2026-03-15: 原因=修复选中公式误触发; 目的=增加聚焦门禁覆盖
describe("shouldShowFormulaHelp", () => {
  it("shows when input starts with '=' and focused", () => {
    expect(shouldShowFormulaHelp({ text: "=SUM", isFxToggled: false, isFocused: true })).toBe(true);
  });

  it("shows when fx is toggled even without focus", () => {
    expect(shouldShowFormulaHelp({ text: "A1", isFxToggled: true, isFocused: false })).toBe(true);
  });

  it("hides when no trigger", () => {
    expect(shouldShowFormulaHelp({ text: "A1", isFxToggled: false, isFocused: false })).toBe(false);
  });

  it("hides when formula present but input not focused", () => {
    expect(shouldShowFormulaHelp({ text: "=SUM", isFxToggled: false, isFocused: false })).toBe(false);
  });
});

// ### 变更记录
// - 2026-03-15: 原因=折叠信息需展示用途+语法; 目的=验证摘要格式
describe("formatFormulaTipSummary", () => {
  it("formats purpose and syntax in one line", () => {
    const item = sampleItems[0];
    expect(formatFormulaTipSummary(item)).toBe(
      "统计与汇总计算 / Statistical =SUM(range)"
    );
  });
});

// ### 变更记录
// - 2026-03-15 21:30: 原因=锁定界面文案; 目的=防止乱码或回归
// - 2026-03-15 21:30: 原因=覆盖关键入口; 目的=保障公式帮助与状态栏文案一致
describe("APP_LABELS", () => {
  it("exposes bilingual formula help labels", () => {
    expect(APP_LABELS.formulaHelp.button).toBe("公式帮助 / Formula Help");
    expect(APP_LABELS.formulaHelp.title).toBe("公式帮助 / Formula Help");
    expect(APP_LABELS.formulaHelp.close).toBe("关闭 / Close");
    expect(APP_LABELS.formulaHelp.searchPlaceholder).toBe("搜索函数 / Search formulas");
    expect(APP_LABELS.formulaHelp.empty).toBe("公式提示为空或未匹配 / No matching formulas");
    expect(APP_LABELS.formulaHelp.headers.functionName).toBe("函数名 / Function");
    expect(APP_LABELS.formulaHelp.headers.syntax).toBe("语法 / Syntax");
    expect(APP_LABELS.formulaHelp.headers.example).toBe("示例 / Example");
    expect(APP_LABELS.formulaHelp.headers.paramNotes).toBe("参数说明 / Parameter Notes");
    expect(APP_LABELS.formulaHelp.headers.purpose).toBe("用途 / Purpose");
  });

  it("exposes status bar labels", () => {
    expect(APP_LABELS.table.placeholder).toBe("选择数据表... / Select table...");
    expect(APP_LABELS.loading.text).toBe("加载中... / Loading...");
    expect(APP_LABELS.backend.label).toBe("后端状态: / Backend:");
  });
});
