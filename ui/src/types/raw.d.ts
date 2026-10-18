/**
 * Vite's `?raw` suffix imports (source-text assertions in the UI
 * grep-gate tests). Declared once; every `*.tsx?raw`/`*.ts?raw`
 * import resolves to the file's text.
 */
declare module "*?raw" {
  const content: string;
  export default content;
}
