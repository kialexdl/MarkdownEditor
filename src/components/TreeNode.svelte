<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { api } from "../lib/api";
  import type { FileEntry } from "../lib/types";

  export let workspaceId: string;
  export let entry: FileEntry;
  export let depth = 0;
  export let refreshToken = 0;

  const dispatch = createEventDispatcher<{
    open: { entry: FileEntry };
    create: { parentPath: string };
  }>();
  let expanded = false;
  let loading = false;
  let error = "";
  let children: FileEntry[] = [];
  let lastRefresh = -1;

  async function load() {
    if (!entry.isDirectory) return;
    loading = true;
    lastRefresh = refreshToken;
    error = "";
    try {
      children = await api.listDirectory(workspaceId, entry.path);
    } catch (reason) {
      error = String(reason);
    } finally {
      loading = false;
    }
  }

  async function activate() {
    if (entry.isDirectory) {
      expanded = !expanded;
      if (expanded && children.length === 0) await load();
    } else {
      dispatch("open", { entry });
    }
  }

  function createHere(event: MouseEvent) {
    event.stopPropagation();
    dispatch("create", { parentPath: entry.path });
  }

  $: if (expanded && refreshToken !== lastRefresh) void load();
</script>

<div class="tree-row" class:directory={entry.isDirectory} style={`--depth:${depth}`} on:click={activate} role="button" tabindex="0" on:keydown={(event) => event.key === "Enter" && activate()}>
  <span class="chevron">{entry.isDirectory ? (expanded ? "⌄" : "›") : ""}</span>
  <span class="file-icon">{entry.isDirectory ? "▰" : entry.kind === "markdown" ? "M" : entry.kind === "image" ? "◩" : "·"}</span>
  <span class="tree-name" title={entry.path}>{entry.name}</span>
  {#if entry.isDirectory}<button class="tree-add" title="在此新建 Markdown" on:click={createHere}>＋</button>{/if}
</div>
{#if expanded}
  {#if loading && children.length === 0}<div class="tree-note" style={`--depth:${depth + 1}`}>读取中…</div>{/if}
  {#if error}<div class="tree-note error" style={`--depth:${depth + 1}`}>{error}</div>{/if}
  {#each children as child (child.path)}
    <svelte:self
      workspaceId={workspaceId}
      entry={child}
      depth={depth + 1}
      {refreshToken}
      on:open={(event) => dispatch("open", event.detail)}
      on:create={(event) => dispatch("create", event.detail)}
    />
  {/each}
{/if}
