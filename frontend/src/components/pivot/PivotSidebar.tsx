/* **[2026-02-26]** 变更原因：清理未使用 Hook 避免严格编译失败；变更目的：保持构建通过。 */
import React, { useState } from 'react';
import './PivotSidebar.css';

export interface Field {
    id: string;
    label: string;
    type: 'string' | 'number' | 'date';
}

export interface PivotConfigState {
    rows: Field[];
    columns: Field[];
    values: Field[];
    filters: Field[];
}

interface PivotSidebarProps {
    fields: Field[];
    config: PivotConfigState;
    onConfigChange: (newConfig: PivotConfigState) => void;
    onApply: (outputMode: 'new-sheet' | 'current-sheet') => void;
    onClose: () => void;
}

export const PivotSidebar: React.FC<PivotSidebarProps> = ({ fields, config, onConfigChange, onApply, onClose }) => {
    const [draggingField, setDraggingField] = useState<Field | null>(null);
    const [outputMode, setOutputMode] = useState<'new-sheet' | 'current-sheet'>('new-sheet');

    const handleDragStart = (e: React.DragEvent, field: Field) => {
        setDraggingField(field);
        e.dataTransfer.setData('fieldId', field.id);
        e.dataTransfer.effectAllowed = 'copyMove';
    };

    const handleDragOver = (e: React.DragEvent) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = 'move';
    };

    const handleDrop = (e: React.DragEvent, targetZone: keyof PivotConfigState) => {
        e.preventDefault();
        if (!draggingField) return;

        // Clone current config
        const newConfig = { ...config };

        // Check if field already exists in target zone (avoid duplicates for simplicity)
        if (!newConfig[targetZone].find(f => f.id === draggingField.id)) {
            newConfig[targetZone] = [...newConfig[targetZone], draggingField];
            onConfigChange(newConfig);
        }
        
        setDraggingField(null);
    };

    const handleRemoveField = (zone: keyof PivotConfigState, fieldId: string) => {
        const newConfig = { ...config };
        newConfig[zone] = newConfig[zone].filter(f => f.id !== fieldId);
        onConfigChange(newConfig);
    };

    return (
        <div className="pivot-sidebar">
            <div className="pivot-header">
                <h3>PivotTable Fields</h3>
                <button onClick={onClose} className="close-btn" aria-label="关闭透视表侧栏">×</button>
            </div>
            
            <div className="pivot-search">
                <input type="text" placeholder="Search fields..." />
            </div>

            {/* **[2026-02-26]** 变更原因：输出位置选择在底部不易发现；变更目的：让用户在配置字段前先完成输出位置决策。 */}
            {/* **[2026-02-26]** 变更原因：需要默认新 Sheet；变更目的：减少覆盖现有区域的误操作。 */}
            {/* **[2026-02-26]** 变更原因：TDD 需要稳定选择器；变更目的：提供可定位的测试标识。 */}
            <div className="pivot-output-mode" data-testid="pivot-output-mode">
                <div className="pivot-output-title">输出位置</div>
                <div className="output-mode-selector">
                    <label>
                        <input 
                            type="radio" 
                            name="outputMode" 
                            checked={outputMode === 'new-sheet'} 
                            onChange={() => setOutputMode('new-sheet')} 
                            data-testid="pivot-output-new-sheet"
                        /> 
                        新建 Sheet
                    </label>
                    <label>
                        <input 
                            type="radio" 
                            name="outputMode" 
                            checked={outputMode === 'current-sheet'} 
                            onChange={() => setOutputMode('current-sheet')} 
                            data-testid="pivot-output-current-sheet"
                        /> 
                        当前选区
                    </label>
                </div>
            </div>

            {/* **[2026-02-26]** 变更原因：字段不可见问题需要可测试；变更目的：提供字段列表测试标识。 */}
            <div className="pivot-field-list" data-testid="pivot-field-list">
                {fields.map(field => (
                    <div 
                        key={field.id} 
                        className="pivot-field-item"
                        data-testid="pivot-field-item"
                        draggable
                        onDragStart={(e) => handleDragStart(e, field)}
                    >
                        <span className={`icon ${field.type}`}></span>
                        {field.label}
                    </div>
                ))}
            </div>

            <div className="pivot-zones-container">
                <div className="pivot-zone-row">
                    <DropZone 
                        title="Filters" 
                        zone="filters" 
                        items={config.filters} 
                        onDrop={handleDrop} 
                        onDragOver={handleDragOver}
                        onRemove={handleRemoveField}
                    />
                    <DropZone 
                        title="Columns" 
                        zone="columns" 
                        items={config.columns} 
                        onDrop={handleDrop} 
                        onDragOver={handleDragOver}
                        onRemove={handleRemoveField}
                    />
                </div>
                <div className="pivot-zone-row">
                    <DropZone 
                        title="Rows" 
                        zone="rows" 
                        items={config.rows} 
                        onDrop={handleDrop} 
                        onDragOver={handleDragOver}
                        onRemove={handleRemoveField}
                    />
                    <DropZone 
                        title="Values" 
                        zone="values" 
                        items={config.values} 
                        onDrop={handleDrop} 
                        onDragOver={handleDragOver}
                        onRemove={handleRemoveField}
                    />
                </div>
            </div>
            
            {/* **[2026-02-26]** 变更原因：输出选择已上移；变更目的：底部仅保留提交动作。 */}
            <div className="pivot-footer">
                <div className="action-buttons">
                    <label>
                        <input type="checkbox" /> Defer Layout Update
                    </label>
                    <button className="update-btn" onClick={() => onApply(outputMode)}>Generate Pivot</button>
                </div>
            </div>
        </div>
    );
};

interface DropZoneProps {
    title: string;
    zone: keyof PivotConfigState;
    items: Field[];
    onDrop: (e: React.DragEvent, zone: keyof PivotConfigState) => void;
    onDragOver: (e: React.DragEvent) => void;
    onRemove: (zone: keyof PivotConfigState, fieldId: string) => void;
}

const DropZone: React.FC<DropZoneProps> = ({ title, zone, items, onDrop, onDragOver, onRemove }) => {
    return (
        <div 
            className="pivot-drop-zone"
            onDrop={(e) => onDrop(e, zone)}
            onDragOver={onDragOver}
        >
            <h4>{title}</h4>
            <div className="drop-list">
                {items.length === 0 && <div className="placeholder">Drag fields here</div>}
                {items.map(item => (
                    <div key={item.id} className="dropped-item">
                        {item.label}
                        <button type="button" className="remove" aria-label={`移除字段 ${item.label}`} onClick={() => onRemove(zone, item.id)}>
                            ×
                        </button>
                    </div>
                ))}
            </div>
        </div>
    );
};
