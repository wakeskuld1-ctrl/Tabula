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
