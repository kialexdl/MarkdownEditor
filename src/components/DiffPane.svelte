<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { markdown } from "@codemirror/lang-markdown";
  import { MergeView } from "@codemirror/merge";
  import { EditorState } from "@codemirror/state";
  import { EditorView, lineNumbers } from "@codemirror/view";

  export let original = "";
  export let modified = "";

  let host: HTMLDivElement;
  let merge: MergeView | null = null;

  onMount(() => {
    const readOnly = [EditorState.readOnly.of(true), EditorView.editable.of(false), lineNumbers(), markdown(), EditorView.lineWrapping];
    merge = new MergeView({
      a: { doc: original, extensions: readOnly },
      b: { doc: modified, extensions: readOnly },
      parent: host,
      highlightChanges: true,
      gutter: true,
      collapseUnchanged: { margin: 3, minSize: 8 }
    });
  });

  onDestroy(() => merge?.destroy());
</script>

<div class="diff-host" bind:this={host}></div>

<style>
  .diff-host { height: 100%; overflow: hidden; background: var(--editor-bg); }
  .diff-host :global(.cm-editor) { height: 100%; font-size: 13px; }
</style>
