import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const required = [
  "package.json",
  "README.md",
  "src/App.svelte",
  "src/lib/api.ts",
  "src/lib/render.worker.ts",
  "src-tauri/Cargo.toml",
  "src-tauri/tauri.conf.json",
  "src-tauri/icons/icon.ico",
  "src-tauri/src/lib.rs",
  "src-tauri/src/filesystem.rs",
  "src-tauri/src/git.rs",
  "scripts/package.ps1",
  "scripts/verify-package.ps1",
  ".github/workflows/ci.yml"
];

for (const path of required) {
  if (!existsSync(resolve(root, path))) throw new Error(`Missing required source file: ${path}`);
}

for (const path of ["package.json", "tsconfig.json", "src-tauri/tauri.conf.json", "src-tauri/capabilities/default.json"]) {
  JSON.parse(readFileSync(resolve(root, path), "utf8"));
}

const tauriConfig = JSON.parse(readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8"));
if (tauriConfig.bundle?.active !== false) {
  throw new Error("Tauri installer bundling must remain disabled for the portable release");
}

const packageScript = readFileSync(resolve(root, "scripts/package.ps1"), "utf8");
for (const boundary of ["--no-bundle", "Compress-Archive", "MarkdownEditor.exe", "SHA256SUMS.txt"]) {
  if (!packageScript.includes(boundary)) throw new Error(`Portable package boundary is missing: ${boundary}`);
}
const packageVerifier = readFileSync(resolve(root, "scripts/verify-package.ps1"), "utf8");
if (!packageVerifier.includes("exactly one root file named MarkdownEditor.exe")) {
  throw new Error("Portable ZIP structure validation is missing");
}

const api = readFileSync(resolve(root, "src/lib/api.ts"), "utf8");
const handlers = readFileSync(resolve(root, "src-tauri/src/lib.rs"), "utf8");
const workspaceBackend = readFileSync(resolve(root, "src-tauri/src/workspace.rs"), "utf8");
const commands = [...api.matchAll(/invoke(?:<[^>]*>)?\(\s*"([a-z_]+)"/g)].map((match) => match[1]);
const missingHandlers = commands.filter((command) => !handlers.includes(`::${command},`));
if (missingHandlers.length > 0) {
  throw new Error(`Frontend commands missing Rust handlers: ${missingHandlers.join(", ")}`);
}
for (const removedCommand of ["add_root", "open_workspace_window"]) {
  if (api.includes(removedCommand) || handlers.includes(removedCommand) || workspaceBackend.includes(removedCommand)) {
    throw new Error(`Removed workspace command is still present: ${removedCommand}`);
  }
}

const filesystem = readFileSync(resolve(root, "src-tauri/src/filesystem.rs"), "utf8");
if (!filesystem.includes('document_kind(&path) != "markdown"')) {
  throw new Error("Backend Markdown-only write guard is missing");
}
if (filesystem.includes("trim_start_matches(|character|")) {
  throw new Error("Filesystem path trimming must use a char pattern compatible with clippy -D warnings");
}

const app = readFileSync(resolve(root, "src/App.svelte"), "utf8");
if (app.includes("as CustomEvent")) {
  throw new Error("App must pass FileEntry data directly instead of fabricating CustomEvent objects");
}
if (!app.includes('class="splitter"') || !app.includes("resizeWithKeyboard")) {
  throw new Error("Split view separator must support pointer and keyboard interaction");
}
for (const feature of ["defaultMarkdownMode", "closeAllSessions", "restrictContextMenu"]) {
  if (!app.includes(feature)) throw new Error(`Application interaction is missing: ${feature}`);
}

const preview = readFileSync(resolve(root, "src/components/PreviewPane.svelte"), "utf8");
if (!preview.includes("let host: HTMLElement;")) {
  throw new Error("Preview host type must match the article element");
}
if (!preview.includes("refreshOutline") || !preview.includes("mdme-preview-outline-visible")) {
  throw new Error("Preview outline and visibility persistence are missing");
}
if (!preview.includes("htmlLabels: false") || !preview.includes('"htmlLabels"')) {
  throw new Error("Mermaid must use secured SVG text labels so sanitization cannot remove diagram text");
}
if (!preview.includes("JSON.stringify([theme, source])") || !preview.includes("$: value, theme, scheduleRender();")) {
  throw new Error("Mermaid rendering and cache keys must react to light/dark theme changes");
}

const editor = readFileSync(resolve(root, "src/components/EditorPane.svelte"), "utf8");
if (
  !editor.includes("markdownContrastHighlightStyle") ||
  !editor.includes('class: "cm-md-language-label"') ||
  !editor.includes("color: var(--warning) !important")
) {
  throw new Error("Markdown fenced-code language labels need an explicit high-contrast editor style");
}

const renderWorker = readFileSync(resolve(root, "src/lib/render.worker.ts"), "utf8");
if (renderWorker.includes("function createRenderer(): MarkdownIt") || !renderWorker.includes("const renderEnv = env ?? {};")) {
  throw new Error("Markdown renderer types are not compatible with the current markdown-it declarations");
}

const git = readFileSync(resolve(root, "src-tauri/src/git.rs"), "utf8");
for (const boundary of ["GIT_OPTIONAL_LOCKS", "GIT_TERMINAL_PROMPT", "GIT_TIMEOUT", "OUTPUT_LIMIT"]) {
  if (!git.includes(boundary)) throw new Error(`Git safety boundary is missing: ${boundary}`);
}

console.log(`Source validation passed: ${required.length} required files, ${commands.length} IPC commands.`);
