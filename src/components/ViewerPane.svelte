<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { DocumentSession } from "../lib/types";
  import EditorPane from "./EditorPane.svelte";

  export let document: DocumentSession;
  const dispatch = createEventDispatcher<{ openExternal: void }>();
</script>

{#if document.kind === "text"}
  <EditorPane value={document.content} readOnly={true} lineEnding={document.lineEnding} />
{:else if document.kind === "image" && document.dataUrl}
  <div class="asset-view"><img src={document.dataUrl} alt={document.name} /></div>
{:else}
  <div class="unsupported-view">
    <div class="unsupported-icon">{document.kind === "pdf" ? "PDF" : "FILE"}</div>
    <h2>{document.name}</h2>
    <p>{(document.size / 1024).toFixed(1)} KiB · 此文件只能查看，编辑已禁用。</p>
    <button class="primary" on:click={() => dispatch("openExternal")}>使用系统默认应用打开</button>
  </div>
{/if}
