// ### 变更记录
// - 2026-03-15 00:20: 原因=新增公式帮助过滤逻辑; 目的=支持UI搜索过滤
// - 2026-03-15 21:45: 原因=修复乱码并补全注释; 目的=确保可读性与维护性

// ### 变更记录
// - 2026-03-15 00:20: 原因=保持类型清晰; 目的=便于前端与JSON对齐
// - 2026-03-15 21:45: 原因=补齐字段说明; 目的=降低后续误用风险
export type FormulaHelpItem = {
  // ### 变更记录
  // - 2026-03-15 00:20: 原因=函数名是主键; 目的=作为唯一识别字段
  name: string;
  // ### 变更记录
  // - 2026-03-15 00:20: 原因=语法需要展示; 目的=提示用法结构
  syntax: string;
  // ### 变更记录
  // - 2026-03-15 00:20: 原因=示例帮助理解; 目的=快速上手
  example: string;
  // ### 变更记录
  // - 2026-03-15 00:20: 原因=参数别名用于业务理解; 目的=双语提示
  paramNotes: string;
  // ### 变更记录
  // - 2026-03-15 00:20: 原因=用途说明是用户关心点; 目的=中文用途补全
  purpose: string;
  // ### 变更记录
  // - 2026-03-15 00:20: 原因=备注用于提示不可用项; 目的=保留占位信息
  note: string;
};

// ### 变更记录
// - 2026-03-15 00:20: 原因=过滤逻辑需要统一入口; 目的=便于UI与测试复用
// - 2026-03-15 21:45: 原因=修复乱码并补齐注释; 目的=保持团队可读性
export function filterFormulaHelpItems(items: FormulaHelpItem[], query: string) {
  // ### 变更记录
  // - 2026-03-15 00:20: 原因=空查询默认展示全部; 目的=避免误过滤
  if (!query || query.trim() === "") {
    return items;
  }

  // ### 变更记录
  // - 2026-03-15 00:20: 原因=搜索需忽略大小写; 目的=提升可用性
  const needle = query.trim().toLowerCase();

  // ### 变更记录
  // - 2026-03-15 00:20: 原因=多字段匹配; 目的=覆盖函数名/语法/用途/参数说明
  return items.filter((item) => {
    const haystack = [
      item.name,
      item.syntax,
      item.example,
      item.paramNotes,
      item.purpose,
      item.note,
    ]
      .join(" ")
      .toLowerCase();
    return haystack.includes(needle);
  });
}

// ### 变更记录
// - 2026-03-15 23:10: 原因=常驻 tips 需要统一选择入口; 目的=集中处理默认展示与过滤逻辑
// - 2026-03-15 23:10: 原因=输入可能带 '=' 前缀; 目的=对齐公式栏输入习惯
export function selectFormulaHelpItems(items: FormulaHelpItem[], query: string, limit: number) {
  // ### 变更记录
  // - 2026-03-15 23:10: 原因=避免空数组导致异常渲染; 目的=保证调用方安全
  if (!Array.isArray(items) || items.length === 0) {
    return [];
  }

  // ### 变更记录
  // - 2026-03-15 23:10: 原因=空输入应返回默认 Top N; 目的=保持常驻面板稳定展示
  const trimmed = (query || "").trim();
  if (trimmed === "") {
    return items.slice(0, Math.max(0, limit));
  }

  // ### 变更记录
  // - 2026-03-15 23:10: 原因=公式输入通常以 '=' 开头; 目的=过滤时忽略该前缀
  const normalized = trimmed.startsWith("=") ? trimmed.slice(1) : trimmed;

  // ### 变更记录
  // - 2026-03-15 23:10: 原因=沿用已有过滤逻辑; 目的=保持过滤行为一致
  const filtered = filterFormulaHelpItems(items, normalized);
  return filtered.slice(0, Math.max(0, limit));
}

// ### 变更记录
// - 2026-03-15: 原因=公式帮助需要条件触发; 目的=统一 '=' 与 fx 的显示规则
// - 2026-03-15: 原因=选中公式不应触发; 目的=引入聚焦门禁
export function shouldShowFormulaHelp(params: { text: string; isFxToggled: boolean; isFocused: boolean }) {
  // ### 变更记录
  // - 2026-03-15: 原因=输入可能带前导空格; 目的=避免误判触发条件
  const trimmed = (params.text || "").trimStart();

  // ### 变更记录
  // - 2026-03-15: 原因=fx 强制显示; 目的=支持按钮打开提示
  if (params.isFxToggled) {
    return true;
  }

  // ### 变更记录
  // - 2026-03-15: 原因=仅编辑时显示; 目的=避免选中公式就弹提示
  return Boolean(params.isFocused && trimmed.startsWith("="));
}

// ### 变更记录
// - 2026-03-15: 原因=折叠展示需要单行摘要; 目的=用途+语法合并展示
export function formatFormulaTipSummary(item: FormulaHelpItem) {
  // ### 变更记录
  // - 2026-03-15: 原因=字段可能为空; 目的=避免出现多余空格或 undefined
  const purpose = (item?.purpose || "").trim();
  const syntax = (item?.syntax || "").trim();

  // ### 变更记录
  // - 2026-03-15: 原因=按用途+语法拼接; 目的=符合“分类 = 语法”展示规范
  if (purpose && syntax) {
    return `${purpose} =${syntax}`;
  }

  // ### 变更记录
  // - 2026-03-15: 原因=用途缺失时仍需可读; 目的=退化为语法或名称
  if (!purpose && syntax) {
    return `=${syntax}`;
  }
  if (purpose) {
    return purpose;
  }
  return (item?.name || "").trim();
}
