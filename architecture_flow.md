# 数据流与连接方式对比 (Old vs New)

## 1. 旧版流程 (Wasm/FortuneSheet)
**特点**: 单一会话，隐式状态，依赖全局 `SessionContext`。

```mermaid
sequenceDiagram
    participant User
    participant App as Frontend (App.tsx)
    participant API as Backend API
    participant Context as SessionContext (DataFusion)

    User->>App: 选择表格 (Select Table)
    App->>API: /api/hydrate (或 /api/execute)
    API->>Context: 注册 Table (Register Table)
    Context-->>API: OK
    App->>API: /api/execute (SQL: SELECT * FROM table)
    API->>Context: 执行 SQL
    Context-->>API: Result Rows
    API-->>App: JSON Rows
    App->>User: 渲染 WasmGrid/FortuneSheet
```

## 2. 新版流程 (Glide Data Grid + Multi-Session)
**特点**: 多会话支持，显式 Session ID，分页加载，依赖 `SessionManager`。

```mermaid
sequenceDiagram
    participant User
    participant App as Frontend (App.tsx)
    participant Glide as GlideGrid Component
    participant API as Backend API
    participant SM as SessionManager
    participant Context as SessionContext

    User->>App: 选择表格 (Select Table)
    App->>App: 清空 currentSession
    App->>API: /api/sessions?table_name=...
    API->>SM: list_sessions()
    SM-->>API: Session List
    API-->>App: JSON Sessions
    
    rect rgb(240, 240, 240)
        Note right of App: 关键缺失步骤: 自动选择默认 Session
        App->>App: Set currentSession = Sessions[0].id
    end

    App->>Glide: Render <GlideGrid sessionId={currentSession} />
    Glide->>API: /api/grid-data?session_id=...&page=1
    
    alt session_id 存在且有效
        API->>SM: switch_session(table, session_id)
        SM->>Context: Update Active Table Pointer
    else session_id 为空
        API->>API: Skip Switch (使用当前 Active)
    end

    API->>Context: SQL: SELECT count(*) / SELECT * LIMIT...
    Context-->>API: Data
    API-->>Glide: { data, columns, total_rows }
    Glide->>User: 渲染表格 (虚拟滚动)
```

## 当前问题分析
1. **Frontend**: `App.tsx` 在切换表格时清空了 `currentSession`，但获取到 Session 列表后没有自动选中一个默认的。
2. **Backend**: `GlideGrid` 发送了空的 `session_id` (或无效的)，后端尝试 `switch_session` 失败，返回 "Session not found"。

## 解决方案
1. **后端优化**: 允许 `/api/grid-data` 的 `session_id` 为空（若为空则不切换，使用默认）。
2. **前端优化**: 加载 Session 列表后，默认选中第一个（最新的）Session。
