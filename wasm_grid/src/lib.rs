use wasm_bindgen::prelude::*;
use web_sys::{console, HtmlCanvasElement, CanvasRenderingContext2d};
use wasm_bindgen::JsCast;

#[wasm_bindgen]
pub struct GridState {
    data: Vec<Vec<String>>,
    visible_rows: Vec<usize>, // Indices of data rows that are visible
    rows: u32,
    cols: u32,
    cell_width: f64,
    cell_height: f64,
    selected_row: Option<u32>,
    selected_col: Option<u32>,
    active_filter_col: Option<u32>,
    sort_col: Option<u32>,
    sort_asc: bool,
    scroll_top: f64,
    view_height: f64,
    col_widths: Vec<f64>,
}

#[wasm_bindgen]
impl GridState {
    #[wasm_bindgen(constructor)]
    pub fn new(cell_width: f64, cell_height: f64) -> GridState {
        GridState {
            rows: 0,
            cols: 0,
            cell_width,
            cell_height,
            data: Vec::new(),
            visible_rows: Vec::new(),
            selected_row: None,
            selected_col: None,
            active_filter_col: None,
            sort_col: None,
            sort_asc: true,
            scroll_top: 0.0,
            view_height: 800.0,
            col_widths: Vec::new(),
        }
    }

    pub fn set_col_width(&mut self, col: usize, width: f64) {
        if col < self.col_widths.len() {
            self.col_widths[col] = width;
        } else {
            // Expand if needed (though resize should handle this)
            if col < self.cols as usize {
                // Resize col_widths to match cols
                self.col_widths.resize(self.cols as usize, self.cell_width);
                self.col_widths[col] = width;
            }
        }
    }

    pub fn get_col_x(&self, col: usize) -> f64 {
        let mut x = 0.0;
        for i in 0..col {
            x += self.col_widths.get(i).unwrap_or(&self.cell_width);
        }
        x
    }

    pub fn get_col_width(&self, col: usize) -> f64 {
        *self.col_widths.get(col).unwrap_or(&self.cell_width)
    }

    pub fn set_scroll(&mut self, scroll_top: f64, view_height: f64) {
        self.scroll_top = scroll_top;
        self.view_height = view_height;
    }

    pub fn get_rendered_row_start(&self) -> i32 {
        // Return the first visible data row index (0-based index in visible_rows)
        // Header is fixed.
        let start_idx = (self.scroll_top / self.cell_height).floor() as usize;
        if start_idx < self.visible_rows.len() {
            start_idx as i32
        } else {
            -1
        }
    }


    pub fn set_cell(&mut self, row: u32, col: u32, value: String) {
        let r = row as usize;
        let c = col as usize;
        
        // Ensure rows exist
        if r >= self.data.len() {
            let old_len = self.data.len();
            self.data.resize(r + 1, Vec::new());
            // Add new rows to visible_rows if they are data rows (index > 0)
            for new_r in old_len..=r {
                if new_r > 0 {
                    self.visible_rows.push(new_r);
                }
            }
            // Ensure columns exist for new rows
            for i in old_len..=r {
                self.data[i].resize(self.cols as usize, String::new());
            }
        }
        
        // Ensure columns exist for this row
        if c >= self.data[r].len() {
             self.data[r].resize(c + 1, String::new());
        }

        self.data[r][c] = value;
    }

    pub fn get_cell(&self, row: u32, col: u32) -> String {
        // Map visual row to data row
        let r_idx = if row == 0 {
            0 // Header
        } else {
            // Data row: map using visible_rows
            // visual row 1 -> visible_rows[0]
            if (row as usize) - 1 < self.visible_rows.len() {
                self.visible_rows[(row as usize) - 1]
            } else {
                return String::new(); // Out of bounds
            }
        };

        let c = col as usize;
        if r_idx < self.data.len() && c < self.data[r_idx].len() {
            self.data[r_idx][c].clone()
        } else {
            String::new()
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.visible_rows.clear();
        self.selected_row = None;
        self.selected_col = None;
        self.active_filter_col = None;
        self.sort_col = None;
        self.sort_asc = true;
    }

    pub fn append_rows(&mut self, count: u32) {
        let old_rows = self.rows;
        self.rows += count;
        
        // Extend data vector
        let new_rows_vec = vec![vec![String::new(); self.cols as usize]; count as usize];
        self.data.extend(new_rows_vec);

        // Update visible_rows
        // Append new row indices
        for r in old_rows..self.rows {
             if r > 0 { // Should always be true if we are appending
                 self.visible_rows.push(r as usize);
             }
        }
    }

    pub fn resize(&mut self, rows: u32, cols: u32) {
        self.rows = rows;
        self.cols = cols;
        // Initialize data with empty strings
        self.data = vec![vec![String::new(); cols as usize]; rows as usize];
        // Initialize visible_rows with indices [1..rows] (Exclude header row 0)
        if rows > 0 {
            self.visible_rows = (1..rows as usize).collect();
        } else {
            self.visible_rows = Vec::new();
        }
        // Initialize col_widths
        self.col_widths = vec![self.cell_width; cols as usize];
    }

    pub fn sort_by_column(&mut self, col: u32, ascending: bool) {
        let c = col as usize;
        let data_ref = &self.data;
        
        self.visible_rows.sort_by(|&a, &b| {
            let val_a = if a < data_ref.len() && c < data_ref[a].len() { &data_ref[a][c] } else { "" };
            let val_b = if b < data_ref.len() && c < data_ref[b].len() { &data_ref[b][c] } else { "" };
            
            // Try numeric sort first
            if let (Ok(num_a), Ok(num_b)) = (val_a.parse::<f64>(), val_b.parse::<f64>()) {
                 if ascending {
                     num_a.partial_cmp(&num_b).unwrap_or(std::cmp::Ordering::Equal)
                 } else {
                     num_b.partial_cmp(&num_a).unwrap_or(std::cmp::Ordering::Equal)
                 }
            } else {
                // String sort
                if ascending {
                    val_a.cmp(val_b)
                } else {
                    val_b.cmp(val_a)
                }
            }
        });
        console::log_1(&format!("Sorted col {} {}", col, if ascending { "asc" } else { "desc" }).into());
    }

    pub fn filter_by_value(&mut self, col: u32, value: String) {
        let c = col as usize;
        self.visible_rows.clear();
        
        for r in 1..self.data.len() {
             let cell_val = if r < self.data.len() && c < self.data[r].len() { &self.data[r][c] } else { "" };
             if value.is_empty() || cell_val.contains(&value) {
                 self.visible_rows.push(r);
             }
        }
        console::log_1(&format!("Filtered col {} by '{}'. Found {} rows.", col, value, self.visible_rows.len()).into());
    }

    pub fn handle_click(&mut self, x: f64, y: f64) -> bool {
        // Find Column
        let mut col = 0;
        let mut curr_x = 0.0;
        let mut found_col = false;
        for c in 0..self.cols {
            let w = self.get_col_width(c as usize);
            if x >= curr_x && x < curr_x + w {
                col = c;
                found_col = true;
                break;
            }
            curr_x += w;
        }

        if !found_col {
            return false;
        }

        if y < self.cell_height {
            // Header click
            let col_width = self.get_col_width(col as usize);
            let rel_x = x - curr_x;

            if rel_x > col_width - 20.0 {
                // Clicked icon -> Toggle Filter
                if self.active_filter_col == Some(col) {
                    self.active_filter_col = None;
                } else {
                    self.active_filter_col = Some(col);
                }
                console::log_1(&format!("Toggled filter for col: {}", col).into());
            } else {
                // Clicked body -> Sort
                let new_asc = if self.sort_col == Some(col) {
                    !self.sort_asc
                } else {
                    true
                };
                self.sort_col = Some(col);
                self.sort_asc = new_asc;
                self.sort_by_column(col, new_asc);
                console::log_1(&format!("Sorted col: {} asc: {}", col, new_asc).into());
            }
            return true;
        }

        // Data Click
        // Calculate effective y relative to data start
        let data_y = y - self.cell_height + self.scroll_top;
        if data_y < 0.0 { return false; }

        let row_idx_in_visible = (data_y / self.cell_height).floor() as usize;
        
        if row_idx_in_visible < self.visible_rows.len() {
            let row = self.visible_rows[row_idx_in_visible] as u32;
            self.selected_row = Some(row);
            self.selected_col = Some(col);
            console::log_1(&format!("Clicked cell: {}, {} (Visible Index: {})", row, col, row_idx_in_visible).into());
            true
        } else {
            false
        }
    }

    pub fn get_active_filter_col(&self) -> i32 {
        match self.active_filter_col {
            Some(c) => c as i32,
            None => -1,
        }
    }

    pub fn is_filter_open(&self) -> bool {
        self.active_filter_col.is_some()
    }

    pub fn get_selected_cell(&self) -> String {
        match (self.selected_row, self.selected_col) {
            (Some(r), Some(c)) => format!("{},{}", r, c),
            _ => "None".to_string(),
        }
    }

    pub fn render(&self, canvas_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let canvas = document.get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("Canvas not found"))?
            .dyn_into::<HtmlCanvasElement>()?;

        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("Context not found"))?
            .dyn_into::<CanvasRenderingContext2d>()?;

        let width = canvas.width() as f64;
        let height = canvas.height() as f64;

        // High DPI Support
        let dpr = window.device_pixel_ratio();
        let logical_width = width / dpr;
        let logical_height = height / dpr;

        context.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0).unwrap_or(());

        // Clear canvas
        context.set_fill_style_str("white");
        context.fill_rect(0.0, 0.0, logical_width, logical_height);

        // Styling
        context.set_line_width(1.0);
        context.set_font("13px Arial"); // Slightly larger font

        // Virtual Scrolling Logic
        // Calculate start and end indices for data rows
        let start_idx = (self.scroll_top / self.cell_height).floor() as usize;
        let visible_count = (self.view_height / self.cell_height).ceil() as usize + 1; // +1 buffer
        let end_idx = (start_idx + visible_count).min(self.visible_rows.len());

        // Helper to draw a cell
        let draw_cell = |r: u32, c: u32, x: f64, y: f64, val: &str, is_header: bool, width: f64, is_sort: bool, is_filter: bool| -> Result<(), JsValue> {
            // Background
            if is_header {
                context.set_fill_style_str("#f8f9fa"); // Light gray header
                context.fill_rect(x, y, width, self.cell_height);
            } else if r % 2 == 0 {
                context.set_fill_style_str("#ffffff"); // White
                context.fill_rect(x, y, width, self.cell_height);
            } else {
                 context.set_fill_style_str("#fafafa"); // Very light gray zebra
                 context.fill_rect(x, y, width, self.cell_height);
            }

            // Border
            context.set_stroke_style_str("#e0e0e0");
            context.stroke_rect(x, y, width, self.cell_height);

            // Selection
             if let (Some(sr), Some(sc)) = (self.selected_row, self.selected_col) {
                if r == sr && c == sc {
                    context.set_stroke_style_str("#4a90e2"); // Blue border
                    context.set_line_width(2.0);
                    context.stroke_rect(x + 1.0, y + 1.0, width - 2.0, self.cell_height - 2.0);
                    context.set_line_width(1.0); // Reset
                    
                    context.set_fill_style_str("rgba(74, 144, 226, 0.1)");
                    context.fill_rect(x + 1.0, y + 1.0, width - 2.0, self.cell_height - 2.0);
                }
            }

            // Text with Clipping
            context.save(); // Save state for clipping
            context.begin_path();
            context.rect(x, y, width, self.cell_height);
            context.clip(); // Clip to cell area

            if !val.is_empty() {
                context.set_fill_style_str("#333333");
                context.fill_text(val, x + 8.0, y + 20.0)?;
            }

            // Header Icon
            if is_header {
                 // Sort Icon
                 if is_sort {
                     context.set_fill_style_str("#666");
                     let icon = if self.sort_asc { "▲" } else { "▼" };
                     context.fill_text(icon, x + width - 35.0, y + 20.0)?;
                 }

                 // Filter Icon
                 context.set_fill_style_str(if is_filter { "#4a90e2" } else { "#999" });
                 context.fill_text("▼", x + width - 15.0, y + 20.0)?;
            }
            
            context.restore(); // Restore state (remove clip)
            Ok(())
        };

        // 1. Draw Data Rows (offset by scrollTop)
        let header_height = self.cell_height;

        for idx in start_idx..end_idx {
            let r_data_idx = self.visible_rows[idx];
            let mut x = 0.0;
            
            for c in 0..self.cols {
                let col_width = self.get_col_width(c as usize);
                let y = (idx as f64 * self.cell_height) - self.scroll_top + header_height;
                
                let val = if r_data_idx < self.data.len() && (c as usize) < self.data[r_data_idx].len() {
                    &self.data[r_data_idx][c as usize]
                } else {
                    ""
                };

                // Only draw if visible (y + height > 0 && y < canvas_height)
                if y < height && y + self.cell_height > 0.0 {
                    draw_cell(r_data_idx as u32, c, x, y, val, false, col_width, false, false)?;
                }
                x += col_width;
            }
        }

        // 2. Draw Header (Fixed on top)
        // Clear header area first to cover scrolled data
        context.set_fill_style_str("white");
        context.fill_rect(0.0, 0.0, logical_width, header_height);
        
        let mut x = 0.0;
        for c in 0..self.cols {
            let col_width = self.get_col_width(c as usize);
            let y = 0.0;
            let val = if 0 < self.data.len() && (c as usize) < self.data[0].len() {
                &self.data[0][c as usize]
            } else {
                ""
            };
            
            let is_sort_col = self.sort_col == Some(c);
            let is_filter_col = self.active_filter_col == Some(c);
            
            draw_cell(0, c, x, y, val, true, col_width, is_sort_col, is_filter_col)?;
            x += col_width;
        }
        
        Ok(())
    }
}

#[wasm_bindgen]
pub fn greet() {
    console::log_1(&"Hello from Rust Wasm Grid!".into());
}

// Keep legacy function for compatibility/testing if needed, or remove.
#[wasm_bindgen]
pub fn draw_grid(canvas_id: &str) -> Result<(), JsValue> {
    let grid = GridState::new(100.0, 30.0);
    grid.render(canvas_id)
}
