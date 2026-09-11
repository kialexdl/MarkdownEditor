/// <reference types="svelte" />
/// <reference types="vite/client" />

declare module "markdown-it-task-lists" {
  import type MarkdownIt from "markdown-it";
  export default function taskLists(md: MarkdownIt, options?: { enabled?: boolean; label?: boolean }): void;
}
