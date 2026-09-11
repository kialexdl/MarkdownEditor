<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { api } from "../lib/api";
  import type { DocumentSession, GitHistoryEntry, GitInfo } from "../lib/types";

  export let workspaceId: string;
  export let document: DocumentSession;

  const dispatch = createEventDispatcher<{
    close: void;
    diff: { original: string; title: string };
  }>();
  let info: GitInfo | null = null;
  let entries: GitHistoryEntry[] = [];
  let loading = true;
  let loadingMore = false;
  let hasMore = false;
  let error = "";
  let selected = "";

  function formatDate(value: string) {
    const date = new Date(value);
    return Number.isNaN(date.valueOf()) ? value : date.toLocaleString("zh-CN", { hour12: false });
  }

  async function load() {
    loading = true;
    error = "";
    try {
      info = await api.getGitInfo(workspaceId, document.path);
      if (info.available) {
        const page = await api.listGitHistory(workspaceId, document.path);
        entries = page.entries;
        hasMore = page.hasMore;
      }
    } catch (reason) {
      error = String(reason);
    } finally {
      loading = false;
    }
  }

  async function loadMore() {
    loadingMore = true;
    try {
      const page = await api.listGitHistory(workspaceId, document.path, entries.length, 50);
      entries = [...entries, ...page.entries];
      hasMore = page.hasMore;
    } catch (reason) {
      error = String(reason);
    } finally {
      loadingMore = false;
    }
  }

  async function selectCommit(entry: GitHistoryEntry) {
    selected = entry.commit;
    error = "";
    try {
      const original = await api.readGitRevision(
        workspaceId,
        document.path,
        entry.commit,
        entry.pathAtCommit
      );
      dispatch("diff", { original, title: `${entry.shortCommit} · ${entry.subject}` });
    } catch (reason) {
      error = String(reason);
    }
  }

  onMount(load);
</script>

<aside class="git-panel">
  <header class="panel-header">
    <div>
      <strong>文件历史</strong>
      {#if info?.branch}<span class="branch">{info.branch}</span>{/if}
    </div>
    <button class="icon-button" title="关闭历史" on:click={() => dispatch("close")}>×</button>
  </header>

  {#if info?.fileStatus}<div class="git-status">工作区变更：{info.fileStatus}</div>{/if}
  {#if loading}<div class="panel-message">正在读取 Git 历史…</div>{/if}
  {#if error}<div class="panel-message error">{error}</div>{/if}
  {#if !loading && info && !info.available}
    <div class="panel-message">{info.message || "当前文件不在 Git 仓库中。"}</div>
  {/if}
  {#if !loading && info?.available && entries.length === 0}
    <div class="panel-message">当前文件还没有提交记录。</div>
  {/if}

  <div class="history-list">
    {#each entries as entry (entry.commit)}
      <button class:selected={selected === entry.commit} class="history-item" on:click={() => selectCommit(entry)}>
        <span class="commit-line"><code>{entry.shortCommit}</code><strong>{entry.subject}</strong></span>
        <span class="commit-meta">{entry.author} · {formatDate(entry.authoredAt)}</span>
      </button>
    {/each}
    {#if hasMore}
      <button class="load-more" disabled={loadingMore} on:click={loadMore}>{loadingMore ? "读取中…" : "加载更多"}</button>
    {/if}
  </div>
</aside>
