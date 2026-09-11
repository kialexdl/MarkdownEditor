<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { api } from "../lib/api";
  import type { FileEntry, WorkspaceRoot } from "../lib/types";
  import TreeNode from "./TreeNode.svelte";

  export let workspaceId: string;
  export let root: WorkspaceRoot;
  export let refreshToken = 0;

  const dispatch = createEventDispatcher<{
    open: { entry: FileEntry };
    create: { parentPath: string };
  }>();
  let entries: FileEntry[] = [];
  let loading = true;
  let error = "";
  let lastRefresh = -1;

  async function load() {
    loading = true;
    lastRefresh = refreshToken;
    try {
      entries = await api.listDirectory(workspaceId, root.path);
      error = "";
    } catch (reason) {
      error = String(reason);
    } finally {
      loading = false;
    }
  }

  $: if (refreshToken !== lastRefresh) void load();
</script>

<section class="root-section">
  <header class="root-header">
    <span title={root.path}>{root.displayName}</span>
    <button title="在根目录新建 Markdown" on:click={() => dispatch("create", { parentPath: root.path })}>＋</button>
  </header>
  {#if loading && entries.length === 0}<div class="tree-note">读取中…</div>{/if}
  {#if error}<div class="tree-note error">{error}</div>{/if}
  {#each entries as entry (entry.path)}
    <TreeNode
      {workspaceId}
      {entry}
      {refreshToken}
      on:open={(event) => dispatch("open", event.detail)}
      on:create={(event) => dispatch("create", event.detail)}
    />
  {/each}
</section>
