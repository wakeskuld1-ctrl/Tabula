# E2E 测试用例 (Test Cases)

本文档记录了 `e2e_test.js` 自动化脚本覆盖的测试场景。旨在确保 Wasm Grid 组件的核心功能、交互逻辑及布局适配性得到完整验证。

## 测试环境配置
- **Viewport**: 1920 x 1080 (全屏桌面模拟)
- **Browser**: Chrome/Chromium (Puppeteer Headless/Headed)
- **Base URL**: http://127.0.0.1:5174

## 测试用例列表

| ID | 模块 | 测试场景 (Scenario) | 预期结果 (Expected Result) | 覆盖状态 |
|:---|:---|:---|:---|:---|
| **TC01** | **布局 (Layout)** | **全屏自适应测试** | Canvas 宽度应填满屏幕宽度 (减去 Padding)，高度应填满屏幕剩余空间。 | ✅ 已覆盖 |
| **TC02** | **初始化 (Init)** | **应用加载与状态栏** | 应用成功加载，状态栏可见，后端连接状态显示正常。 | ✅ 已覆盖 |
| **TC03** | **数据 (Data)** | **表格数据加载** | 从下拉框选择表格后，Grid 应渲染数据，并在 DOM/Console 中确认加载完成。 | ✅ 已覆盖 |
| **TC04** | **交互 (Interaction)** | **单元格选中 (Selection)** | 点击任意非表头单元格，Rust 内部状态 (`selected_row`, `selected_col`) 应正确更新。 | ✅ 已覆盖 |
| **TC05** | **交互 (Interaction)** | **排序触发 (Sorting)** | 点击表头文本区域 (Header Text)，触发数据排序。第一次升序，第二次降序。 | ✅ 已覆盖 |
| **TC06** | **交互 (Interaction)** | **筛选菜单打开 (Filter Open)** | 点击表头右侧漏斗图标，筛选菜单应弹出，且输入框可见。 | ✅ 已覆盖 |
| **TC07** | **交互 (Interaction)** | **筛选逻辑验证 (Filter Logic)** | 在筛选框输入文本，Grid 应仅显示匹配的行 (前端过滤)。 | ✅ 已覆盖 |
| **TC08** | **滚动 (Scrolling)** | **垂直虚拟滚动 (Vertical)** | 向下滚动 Grid，可视区域的起始行号 (`start_idx`) 应随滚动增加。 | ✅ 已覆盖 |
| **TC09** | **滚动 (Scrolling)** | **横向滚动适配 (Horizontal)** | 向右滚动 Grid，内容应发生位移。 | ✅ 已覆盖 |
| **TC10** | **布局 (Layout)** | **滚动后菜单定位 (Positioning)** | **关键用例**: 在横向滚动状态下点击筛选图标，菜单弹出位置应准确跟随图标 (需扣除 `scrollLeft`)。 | ✅ 已覆盖 |
| **TC11** | **显示 (Display)** | **特定长文件名表格加载** | 验证如 `PingCode_Project_YMP...` 等复杂命名表格选中后，Canvas 能正常渲染内容（非空白）。 | ⏳ 待实现 |
| **TC12** | **视觉 (Visual)** | **全屏铺满检测 (Full Screen Fill)** | 验证表格是否填满整个屏幕区域（无大面积留白），即渲染区域 (Rendered Area) >= 视口区域 (Viewport)。 | ⏳ 待实现 |

## 待补充测试用例 (Future)

| ID | 模块 | 计划场景 | 备注 |
|:---|:---|:---|:---|
| **TC11** | **编辑 (Edit)** | 双击单元格进入编辑模式，修改值并保存 | 需实现 Input Overlay |
| **TC12** | **列宽 (Resize)** | 拖拽表头边缘调整列宽 | 需实现 Resize Handle |
| **TC13** | **键盘 (Keyboard)** | 使用方向键移动选中框 | 需实现 KeyDown 事件监听 |
| **TC14** | **大数据 (Performance)** | 加载 10万行数据，验证滚动帧率 | 需准备 Mock 大数据接口 |

## 执行方式
```bash
node e2e_test.js
```
测试报告将生成于 `e2e_report.md`。
