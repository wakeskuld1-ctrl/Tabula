import React, { useEffect, useRef } from 'react';
import init, { GridState } from 'wasm_grid';

interface WasmGridProps {
    width?: number;
    height?: number;
    columns?: string[];
    data?: any[][];
}

const WasmGrid: React.FC<WasmGridProps> = ({ width = 1800, height = 800, columns = [], data = [] }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const gridRef = useRef<GridState | null>(null);
    const scrollContainerRef = useRef<HTMLDivElement>(null);
    const [filterState, setFilterState] = React.useState<{ open: boolean, col: number, x: number, y: number }>({ open: false, col: -1, x: 0, y: 0 });
    const [filterValue, setFilterValue] = React.useState("");

    // Initialize Wasm module only once
    useEffect(() => {
        const initWasm = async () => {
            try {
                await init();
                if (canvasRef.current && !gridRef.current) {
                    const grid = new GridState(100.0, 30.0); // Standard size: 100x30
                    gridRef.current = grid;
                    (window as any).wasmGrid = grid;
                    
                    // Sync initial scroll position (fix for refresh misalignment)
                    if (scrollContainerRef.current) {
                        const scrollTop = scrollContainerRef.current.scrollTop;
                        const viewHeight = scrollContainerRef.current.clientHeight;
                        grid.set_scroll(scrollTop, viewHeight);
                    }

                    grid.render(canvasRef.current.id);
                }
            } catch (e) {
                console.error("Failed to initialize Wasm Grid:", e);
            }
        };
        initWasm();
    }, []);

    // Update data when props change
    useEffect(() => {
        if (!gridRef.current || !canvasRef.current) return;
        
        const grid = gridRef.current;
        
        // If no data provided, skip update (or keep default sample if desired, but better to clear)
        if (columns.length === 0 && data.length === 0) return;

        console.log(`Updating Wasm Grid: ${columns.length} cols, ${data.length} rows`);

        // Clear existing data
        grid.clear();
        
        // Resize grid
        // Rows = Header + Data
        const rowCount = Math.max(20, data.length + 1); 
        const colCount = Math.max(10, columns.length);
        grid.resize(rowCount, colCount);

        // Render Header (Row 0)
        columns.forEach((col, index) => {
            grid.set_cell(0, index, col);
        });

        // Render Data (Row 1+)
        data.forEach((row, rowIndex) => {
            row.forEach((cell, colIndex) => {
                const cellVal = cell !== null && cell !== undefined ? String(cell) : "";
                grid.set_cell(rowIndex + 1, colIndex, cellVal);
            });
        });

        // Re-render
        grid.render(canvasRef.current.id);

    }, [columns, data]);

    const handleCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
        if (!gridRef.current || !canvasRef.current) return;
        
        const rect = canvasRef.current.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        
        const changed = gridRef.current.handle_click(x, y);
        if (changed) {
            gridRef.current.render(canvasRef.current.id);
            
            // Check filter state
            if (gridRef.current.is_filter_open()) {
                const col = gridRef.current.get_active_filter_col();
                
                // Get correct column x position from Rust
                // If not available, fallback to col * 100
                // Note: Rust's get_col_x returns logic x.
                let colX = col * 100;
                if ((gridRef.current as any).get_col_x) {
                    colX = (gridRef.current as any).get_col_x(col);
                }

                // Adjust for scrollLeft
                const scrollLeft = scrollContainerRef.current?.scrollLeft || 0;
                
                setFilterState({ 
                    open: true, 
                    col, 
                    x: colX - scrollLeft, // Adjust for horizontal scroll
                    y: 30 + 5 
                });
                setFilterValue(""); 
            } else {
                setFilterState({ open: false, col: -1, x: 0, y: 0 });
            }
        }
    };

    const handleFilterChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const val = e.target.value;
        setFilterValue(val);
        if (gridRef.current && filterState.col !== -1) {
            gridRef.current.filter_by_value(filterState.col, val);
            gridRef.current.render(canvasRef.current!.id);
        }
    };

    const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
        if (!gridRef.current || !canvasRef.current) return;
        const scrollTop = e.currentTarget.scrollTop;
        const viewHeight = e.currentTarget.clientHeight;
        
        // Pass scroll info to Rust
        // We need to implement set_scroll_top(scrollTop) and render_visible(viewHeight)
        // Or just update scroll and render.
        // Assuming we will implement set_scroll in Rust.
        if ((gridRef.current as any).set_scroll) {
            (gridRef.current as any).set_scroll(scrollTop, viewHeight);
            gridRef.current.render(canvasRef.current.id);
        }
    };

    // Calculate total height for spacer
    const totalHeight = (Math.max(data.length, 1) + 1) * 30;
    
    // Calculate total width for spacer (assuming default 100px per column for now)
    // Ideally we should sum up column widths from Rust, but for now fixed 100 is fine.
    const totalWidth = Math.max(columns.length * 100, width); 

    const dpr = window.devicePixelRatio || 1;

    return (
        <div style={{ position: 'relative' }}>
            {/* Scroll Container */}
            <div 
                ref={scrollContainerRef}
                style={{ 
                    width: width, 
                    height: height, 
                    overflow: 'auto', 
                    position: 'relative',
                    border: '1px solid #ccc' 
                }}
                onScroll={handleScroll}
            >
                {/* Spacer to force scrollbar */}
                <div style={{ height: totalHeight, width: totalWidth }} />

                {/* Sticky Canvas */}
                <canvas 
                    id="wasm-grid-canvas" 
                    ref={canvasRef} 
                    width={width * dpr} 
                    height={height * dpr}
                    onClick={handleCanvasClick}
                    style={{ 
                        position: 'sticky', 
                        top: 0, 
                        // left: 0, // Removed to allow horizontal scrolling
                        width: `${width}px`,
                        height: `${height}px`,
                        background: 'white', 
                        cursor: 'pointer',
                        display: 'block' // Remove inline-block gap
                    }}
                />
            </div>

            {filterState.open && (
                <div style={{
                    position: 'absolute',
                    top: filterState.y, // This y needs to be adjusted by scrollTop if it's inside relative container?
                    // Actually, if it's outside the scroll container but relative to the wrapper div.
                    // If wrapper is relative, and scroll container is inside.
                    // The filter menu should probably be Fixed or Absolute relative to the Viewport, 
                    // or we need to subtract scrollTop from y if it's inside the scroll container.
                    // Let's keep it simple: Sticky Canvas stays at top. 
                    // Filter menu is absolute to the outer wrapper.
                    // If we put Filter Menu inside the scroll container, it will scroll with it?
                    // No, we want it to float.
                    // Let's put it outside the scroll container (as it is now).
                    // But wait, the click coordinate `y` from Canvas is relative to Canvas (which is Sticky).
                    // So `y` is screen-relative (viewport-relative) inside the container.
                    // So `filterState.y` is correct relative to the sticky canvas top.
                    left: filterState.x,
                    background: 'white',
                    border: '1px solid #999',
                    padding: '5px',
                    boxShadow: '0 2px 5px rgba(0,0,0,0.2)',
                    zIndex: 100
                }}>
                    <input 
                        id="wasm-grid-filter-input"
                        type="text" 
                        value={filterValue} 
                        onChange={handleFilterChange} 
                        placeholder="Filter..." 
                        autoFocus
                    />
                </div>
            )}
        </div>
    );
};

export default WasmGrid;
