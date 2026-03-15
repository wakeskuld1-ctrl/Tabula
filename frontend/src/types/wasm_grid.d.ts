// ### Change Log
// - 2026-03-14: Reason=Legacy WasmGrid imports optional wasm module; Purpose=allow TS build when module is absent.
// - 2026-03-14: Reason=Expose minimal typing for legacy use; Purpose=avoid TS errors without bundling wasm.
// - 2026-03-14: Reason=Unknown wasm surface; Purpose=allow any GridState method during type-check.
declare module 'wasm_grid' {
  export class GridState {
    constructor(...args: any[]);
    [key: string]: any;
  }

  const init: (...args: any[]) => Promise<any>;
  export default init;
}
