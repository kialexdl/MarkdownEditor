<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, onMount, tick } from "svelte";
  import DiffPane from "./components/DiffPane.svelte";
  import EditorPane from "./components/EditorPane.svelte";
  import FileTree from "./components/FileTree.svelte";
  import GitPanel from "./components/GitPanel.svelte";
  import PreviewPane from "./components/PreviewPane.svelte";
  import ViewerPane from "./components/ViewerPane.svelte";
  import { api } from "./lib/api";
  import type {
    DocumentSession,
    FileEntry,
    FsEventPayload,
    RecoveryEntry,
    ViewMode,
    WorkspaceRecord
  } from "./lib/types";

  let workspaces: WorkspaceRecord[] = [];
  let activeWorkspaceId = "";
  let sessions: DocumentSession[] = [];
  let activeSessionKey = "";
  let activeWorkspace: WorkspaceRecord | undefined;
  let activeDocument: DocumentSession | undefined;
  let starting = true;
  let busy = false;
  let appError = "";
  let gitVersion = "";
  let gitError = "";
  let refreshToken = 0;
  let showGit = false;
  let diffOriginal = "";
  let diffTitle = "";
  let splitRatio = 50;
  let splitHost: HTMLDivElement;
  let splitEditor: { scrollToLine: (line: number) => void } | undefined;
  let recoveries: RecoveryEntry[] = [];
  let theme: "dark" | "light" = "dark";
  let defaultMarkdownMode: ViewMode = "split";
  let unlistenFs: (() => void) | undefined;
  let fsRefreshTimer: ReturnType<typeof setTimeout> | undefined;
  const pendingFsPaths = new Set<string>();
  const autosaveTimers = new Map<string, ReturnType<typeof setTimeout>>();
  const recoveryTimers = new Map<string, ReturnType<typeof setTimeout>>();

  $: activeWorkspace = workspaces.find((workspace) => workspace.id === activeWorkspaceId);
  $: activeDocument = sessions.find((session) => keyOf(session) === activeSessionKey);

  function keyOf(session: Pick<DocumentSession, "workspaceId" | "path">) {
    return `${session.workspaceId}::${session.path}`;
  }

  function updateSession(key: string, update: (session: DocumentSession) => DocumentSession) {
    sessions = sessions.map((session) => (keyOf(session) === key ? update(session) : session));
  }

  async function initialize() {
    starting = true;
    theme = localStorage.getItem("mdme-theme") === "light" ? "light" : "dark";
    const storedMode = localStorage.getItem("mdme-default-markdown-mode");
    if (storedMode === "edit" || storedMode === "preview" || storedMode === "split") {
      defaultMarkdownMode = storedMode;
    }
    applyTheme();
    try {
      try {
        gitVersion = await api.checkGit();
      } catch (error) {
        gitError = `未检测到 Git。请先安装 Git for Windows 并将 git.exe 加入 PATH。${String(error)}`;
      }
      workspaces = await api.listWorkspaces();
      const target = workspaces[0];
      if (target) await selectWorkspace(target.id);
      recoveries = await api.listRecovery();
      unlistenFs = await listen<FsEventPayload>("workspace-fs-event", (event) => {
        queueFsEvent(event.payload);
      });
    } catch (error) {
      appError = String(error);
    } finally {
      starting = false;
    }
  }

  async function chooseWorkspace() {
    const path = await api.pickFolder();
    if (!path) return;
    busy = true;
    try {
      const workspace = await api.addWorkspace(path);
      if (!workspaces.some((item) => item.id === workspace.id)) {
        workspaces = [...workspaces, workspace];
      }
      await selectWorkspace(workspace.id);
    } catch (error) {
      appError = String(error);
    } finally {
      busy = false;
    }
  }

  async function selectWorkspace(id: string) {
    if (id === activeWorkspaceId) return;
    clearTimeout(fsRefreshTimer);
    pendingFsPaths.clear();
    const previous = activeWorkspaceId;
    activeWorkspaceId = id;
    const existing = sessions.find((session) => session.workspaceId === id);
    activeSessionKey = existing ? keyOf(existing) : "";
    showGit = false;
    closeDiff();
    if (previous) await api.unwatchWorkspace(previous).catch(() => undefined);
    await api.watchWorkspace(id).catch((error) => (appError = String(error)));
    refreshToken += 1;
  }

  async function removeActiveWorkspace() {
    if (!activeWorkspace || !confirm(`从列表移除工作区“${activeWorkspace.name}”？磁盘文件不会被删除。`)) return;
    const removedId = activeWorkspace.id;
    await api.removeWorkspace(removedId);
    workspaces = workspaces.filter((workspace) => workspace.id !== removedId);
    sessions = sessions.filter((session) => session.workspaceId !== removedId);
    activeWorkspaceId = "";
    activeSessionKey = "";
    if (workspaces[0]) await selectWorkspace(workspaces[0].id);
  }

  async function openEntry(entry: FileEntry) {
    if (entry.isDirectory) return;
    const existing = sessions.find(
      (session) => session.workspaceId === activeWorkspaceId && session.path === entry.path
    );
    if (existing) {
      activeSessionKey = keyOf(existing);
      return;
    }
    busy = true;
    try {
      const result = await api.readDocument(activeWorkspaceId, entry.path);
      const session: DocumentSession = {
        ...result,
        workspaceId: activeWorkspaceId,
        content: result.content ?? "",
        savedContent: result.content ?? "",
        viewMode: result.kind === "markdown" ? (result.largeFile ? "edit" : defaultMarkdownMode) : "edit",
        dirty: false,
        saving: false,
        conflict: false,
        activeLine: 1
      };
      sessions = [...sessions, session];
      activeSessionKey = keyOf(session);
      showGit = false;
      closeDiff();
    } catch (error) {
      appError = String(error);
    } finally {
      busy = false;
    }
  }

  async function createMarkdown(event: CustomEvent<{ parentPath: string }>) {
    if (!activeWorkspaceId) return;
    const name = prompt("请输入 Markdown 文件名", "untitled.md")?.trim();
    if (!name) return;
    try {
      const path = await api.createMarkdownFile(activeWorkspaceId, event.detail.parentPath, name);
      refreshToken += 1;
      await openEntry({ name, path, isDirectory: false, kind: "markdown", size: 0, modifiedMs: Date.now() });
    } catch (error) {
      appError = String(error);
    }
  }

  function editActive(value: string) {
    if (!activeDocument?.editable) return;
    const key = activeSessionKey;
    updateSession(key, (session) => ({
      ...session,
      content: value,
      dirty: value !== session.savedContent,
      error: undefined
    }));
    scheduleAutosave(key);
    scheduleRecovery(key);
  }

  function setActiveLine(line: number) {
    if (!activeDocument) return;
    updateSession(activeSessionKey, (session) => ({ ...session, activeLine: line }));
  }

  function syncEditorFromPreview(line: number) {
    splitEditor?.scrollToLine(line);
  }

  function scheduleAutosave(key: string) {
    clearTimeout(autosaveTimers.get(key));
    autosaveTimers.set(
      key,
      setTimeout(() => {
        const session = sessions.find((item) => keyOf(item) === key);
        if (session?.dirty && !session.conflict && session.editable) void saveSession(key);
      }, 800)
    );
  }

  function scheduleRecovery(key: string) {
    clearTimeout(recoveryTimers.get(key));
    recoveryTimers.set(
      key,
      setTimeout(() => {
        const session = sessions.find((item) => keyOf(item) === key);
        if (session?.dirty) void api.saveRecovery(session.workspaceId, session.path, session.content);
      }, 2500)
    );
  }

  async function saveSession(key = activeSessionKey, force = false) {
    const session = sessions.find((item) => keyOf(item) === key);
    if (!session?.editable || session.saving || (!session.dirty && !force)) return;
    const snapshot = session.content;
    updateSession(key, (item) => ({ ...item, saving: true, error: undefined }));
    try {
      const result = await api.saveDocument(
        session.workspaceId,
        session.path,
        snapshot,
        session.fingerprint,
        session.hasBom,
        force
      );
      if (result.status === "conflict") {
        updateSession(key, (item) => ({
          ...item,
          saving: false,
          conflict: true,
          error: result.message ?? "磁盘文件已变化"
        }));
        return;
      }
      updateSession(key, (item) => ({
        ...item,
        saving: false,
        fingerprint: result.fingerprint ?? item.fingerprint,
        savedContent: snapshot,
        dirty: item.content !== snapshot,
        conflict: false
      }));
      const latest = sessions.find((item) => keyOf(item) === key);
      if (!latest || latest.content === snapshot) {
        await api.discardRecovery(session.workspaceId, session.path).catch(() => undefined);
      } else if (latest.dirty) {
        scheduleAutosave(key);
      }
    } catch (error) {
      updateSession(key, (item) => ({ ...item, saving: false, error: String(error) }));
    }
  }

  async function saveAll() {
    for (const session of sessions.filter((item) => item.dirty && item.editable && !item.conflict)) {
      await saveSession(keyOf(session));
    }
  }

  async function reloadActive() {
    if (!activeDocument) return;
    try {
      const result = await api.readDocument(activeDocument.workspaceId, activeDocument.path);
      updateSession(activeSessionKey, (session) => ({
        ...session,
        ...result,
        content: result.content ?? "",
        savedContent: result.content ?? "",
        dirty: false,
        conflict: false,
        error: undefined
      }));
      await api.discardRecovery(activeDocument.workspaceId, activeDocument.path).catch(() => undefined);
      closeDiff();
    } catch (error) {
      appError = String(error);
    }
  }

  async function compareWithDisk() {
    if (!activeDocument) return;
    try {
      const disk = await api.readDocument(activeDocument.workspaceId, activeDocument.path);
      diffOriginal = disk.content ?? "";
      diffTitle = "磁盘版本 ↔ 编辑器版本";
    } catch (error) {
      appError = String(error);
    }
  }

  function queueFsEvent(payload: FsEventPayload) {
    if (payload.workspaceId !== activeWorkspaceId) return;
    for (const path of payload.paths) pendingFsPaths.add(path);
    clearTimeout(fsRefreshTimer);
    fsRefreshTimer = setTimeout(() => {
      const paths = [...pendingFsPaths];
      pendingFsPaths.clear();
      void handleFsEvent(payload.workspaceId, paths);
    }, 180);
  }

  async function handleFsEvent(workspaceId: string, paths: string[]) {
    if (workspaceId !== activeWorkspaceId) return;
    refreshToken += 1;
    const affected = sessions.filter(
      (session) => session.workspaceId === workspaceId && paths.includes(session.path)
    );
    for (const session of affected) {
      const key = keyOf(session);
      if (session.saving) {
        continue;
      }
      try {
        const reloaded = await api.readDocument(session.workspaceId, session.path);
        if (session.dirty) {
          if (reloaded.fingerprint.sha256 !== session.fingerprint.sha256) {
            updateSession(key, (item) => ({
              ...item,
              conflict: true,
              error: "磁盘文件已被外部修改"
            }));
          }
        } else {
          updateSession(key, (item) => ({
            ...item,
            ...reloaded,
            content: reloaded.content ?? "",
            savedContent: reloaded.content ?? ""
          }));
        }
      } catch {
        // 文件可能正处于原子替换的短暂窗口，下一次事件会再次刷新。
      }
    }
  }

  function setMode(mode: ViewMode) {
    if (!activeDocument) return;
    updateSession(activeSessionKey, (session) => ({ ...session, viewMode: mode }));
    closeDiff();
  }

  function saveDefaultMarkdownMode() {
    localStorage.setItem("mdme-default-markdown-mode", defaultMarkdownMode);
  }

  function clearSessionTimers(key: string) {
    clearTimeout(autosaveTimers.get(key));
    autosaveTimers.delete(key);
    clearTimeout(recoveryTimers.get(key));
    recoveryTimers.delete(key);
  }

  function closeSession(key: string) {
    const session = sessions.find((item) => keyOf(item) === key);
    if (!session) return;
    if (session.dirty && !confirm(`“${session.name}”尚未保存，仍要关闭吗？`)) return;
    clearSessionTimers(key);
    if (session.dirty) void api.discardRecovery(session.workspaceId, session.path).catch(() => undefined);
    sessions = sessions.filter((item) => keyOf(item) !== key);
    if (activeSessionKey === key) {
      const next = sessions.find((item) => item.workspaceId === activeWorkspaceId);
      activeSessionKey = next ? keyOf(next) : "";
    }
  }

  function closeAllSessions() {
    const targets = sessions.filter((session) => session.workspaceId === activeWorkspaceId);
    if (targets.length === 0) return;
    const dirtyCount = targets.filter((session) => session.dirty).length;
    if (
      dirtyCount > 0 &&
      !confirm(`当前工作区有 ${dirtyCount} 个文件尚未保存，仍要关闭全部 ${targets.length} 个文件吗？`)
    ) {
      return;
    }
    for (const session of targets) {
      const key = keyOf(session);
      clearSessionTimers(key);
      if (session.dirty) void api.discardRecovery(session.workspaceId, session.path).catch(() => undefined);
    }
    sessions = sessions.filter((session) => session.workspaceId !== activeWorkspaceId);
    activeSessionKey = "";
    showGit = false;
    closeDiff();
  }

  function closeDiff() {
    diffOriginal = "";
    diffTitle = "";
  }

  async function openLocalLink(href: string) {
    if (!activeDocument) return;
    try {
      const path = await api.resolveDocumentLink(activeDocument.workspaceId, activeDocument.path, href);
      const name = path.split(/[\\/]/).pop() ?? path;
      await openEntry({ name, path, isDirectory: false, kind: "markdown", size: 0, modifiedMs: 0 });
    } catch (error) {
      appError = String(error);
    }
  }

  async function openActiveExternal() {
    if (!activeDocument) return;
    await api.openExternalPath(activeDocument.workspaceId, activeDocument.path);
  }

  async function restoreRecovery(entry: RecoveryEntry) {
    const workspace = workspaces.find((item) => item.id === entry.workspaceId);
    if (!workspace) {
      appError = "恢复内容对应的工作区已不存在。";
      return;
    }
    await selectWorkspace(workspace.id);
    const name = entry.path.split(/[\\/]/).pop() ?? entry.path;
    await openEntry({ name, path: entry.path, isDirectory: false, kind: "markdown", size: 0, modifiedMs: 0 });
    await tick();
    const key = `${entry.workspaceId}::${entry.path}`;
    updateSession(key, (session) => ({ ...session, content: entry.content, dirty: true }));
    activeSessionKey = key;
    recoveries = recoveries.filter((item) => item !== entry);
  }

  async function discardRecovery(entry: RecoveryEntry) {
    await api.discardRecovery(entry.workspaceId, entry.path);
    recoveries = recoveries.filter((item) => item !== entry);
  }

  function toggleTheme() {
    theme = theme === "dark" ? "light" : "dark";
    localStorage.setItem("mdme-theme", theme);
    applyTheme();
  }

  function applyTheme() {
    document.documentElement.dataset.theme = theme;
  }

  function beginResize(event: PointerEvent) {
    if (!splitHost) return;
    const move = (next: PointerEvent) => {
      const rect = splitHost.getBoundingClientRect();
      splitRatio = Math.min(75, Math.max(25, ((next.clientX - rect.left) / rect.width) * 100));
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    event.preventDefault();
  }

  function resizeWithKeyboard(event: KeyboardEvent) {
    const delta = event.key === "ArrowLeft" ? -5 : event.key === "ArrowRight" ? 5 : 0;
    if (delta === 0) return;
    splitRatio = Math.min(75, Math.max(25, splitRatio + delta));
    event.preventDefault();
  }

  function keyboard(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      if (event.shiftKey) void saveAll();
      else void saveSession();
    }
  }

  function restrictContextMenu(event: MouseEvent) {
    const target = event.target;
    if (
      target instanceof Element &&
      target.closest("input, textarea, select, [contenteditable='true'], .cm-editor")
    ) {
      return;
    }
    event.preventDefault();
  }

  onMount(() => {
    window.addEventListener("keydown", keyboard);
    window.addEventListener("contextmenu", restrictContextMenu);
    void initialize();
  });

  onDestroy(() => {
    window.removeEventListener("keydown", keyboard);
    window.removeEventListener("contextmenu", restrictContextMenu);
    unlistenFs?.();
    clearTimeout(fsRefreshTimer);
    for (const timer of autosaveTimers.values()) clearTimeout(timer);
    for (const timer of recoveryTimers.values()) clearTimeout(timer);
  });
</script>

<div class="app-shell">
  <header class="topbar">
    <div class="brand"><span class="brand-mark">M</span><strong>MarkdownEditor</strong></div>
    <div class="workspace-controls">
      <select value={activeWorkspaceId} on:change={(event) => selectWorkspace(event.currentTarget.value)} disabled={busy}>
        <option value="">选择工作区</option>
        {#each workspaces as workspace}<option value={workspace.id}>{workspace.name}</option>{/each}
      </select>
      <button on:click={chooseWorkspace} title="打开新工作区">打开文件夹</button>
    </div>
    <div class="top-actions">
      <label class="default-mode" title="仅影响之后打开的 Markdown 文件">
        <span>默认</span>
        <select bind:value={defaultMarkdownMode} on:change={saveDefaultMarkdownMode} aria-label="Markdown 默认打开模式">
          <option value="edit">编辑</option>
          <option value="preview">预览</option>
          <option value="split">分屏</option>
        </select>
      </label>
      {#if gitVersion}<span class="git-ok" title={gitVersion}>Git ✓</span>{/if}
      <button class="icon-button" on:click={toggleTheme} title="切换主题">{theme === "dark" ? "☀" : "☾"}</button>
    </div>
  </header>

  {#if gitError}<div class="global-banner danger">{gitError}</div>{/if}
  {#if appError}<div class="global-banner"><span>{appError}</span><button on:click={() => (appError = "")}>×</button></div>{/if}

  <div class="workspace-layout" class:with-git={showGit && Boolean(activeDocument)}>
    <aside class="sidebar">
      <div class="sidebar-title">
        <span>资源管理器</span>
        <div>
          <button class="icon-button" on:click={() => (refreshToken += 1)} title="刷新">↻</button>
          <button class="icon-button" on:click={removeActiveWorkspace} disabled={!activeWorkspace} title="移除工作区">−</button>
        </div>
      </div>
      {#if activeWorkspace}
        <div class="roots-scroll">
          {#each activeWorkspace.roots as root (root.path)}
            <FileTree
              workspaceId={activeWorkspace.id}
              {root}
              {refreshToken}
              on:open={(event) => openEntry(event.detail.entry)}
              on:create={createMarkdown}
            />
          {/each}
        </div>
      {:else}
        <div class="empty-sidebar">
          <p>尚未打开工作区</p>
          <button class="primary" on:click={chooseWorkspace}>打开文件夹</button>
        </div>
      {/if}
    </aside>

    <main class="editor-area">
      <div class="tabs-bar">
        <nav class="tabs" aria-label="已打开文件">
          {#each sessions.filter((session) => session.workspaceId === activeWorkspaceId) as session (keyOf(session))}
            <div class:active={keyOf(session) === activeSessionKey} class="tab">
              <button class="tab-select" on:click={() => (activeSessionKey = keyOf(session))} title={session.path}>
                <span class:dirty={session.dirty}>{session.name}</span>
              </button>
              <button class="tab-close" on:click={() => closeSession(keyOf(session))} aria-label={`关闭 ${session.name}`}>×</button>
            </div>
          {/each}
        </nav>
        <button
          class="close-all-tabs"
          on:click={closeAllSessions}
          disabled={!sessions.some((session) => session.workspaceId === activeWorkspaceId)}
          title="关闭当前工作区的全部文件"
        >全部关闭</button>
      </div>

      {#if activeDocument}
        <div class="document-toolbar">
          <div class="document-name" title={activeDocument.path}>
            <strong>{activeDocument.name}</strong>
            {#if activeDocument.saving}<span>保存中…</span>{:else if activeDocument.dirty}<span class="dirty-label">未保存</span>{:else}<span>已保存</span>{/if}
            {#if activeDocument.largeFile}<span class="large-label">大文件模式</span>{/if}
          </div>
          <div class="document-actions">
            {#if activeDocument.kind === "markdown"}
              <div class="mode-switch">
                <button class:active={activeDocument.viewMode === "edit"} on:click={() => setMode("edit")}>编辑</button>
                <button class:active={activeDocument.viewMode === "preview"} on:click={() => setMode("preview")}>预览</button>
                <button class:active={activeDocument.viewMode === "split"} on:click={() => setMode("split")}>分屏</button>
              </div>
            {/if}
            <button on:click={() => saveSession()} disabled={!activeDocument.editable || !activeDocument.dirty || activeDocument.saving}>保存</button>
            <button class:active={showGit} on:click={() => (showGit = !showGit)}>Git 历史</button>
          </div>
        </div>

        {#if activeDocument.conflict}
          <div class="conflict-banner">
            <span>磁盘文件已在外部发生变化，自动保存已暂停。</span>
            <button on:click={compareWithDisk}>比较</button>
            <button on:click={reloadActive}>使用磁盘版本</button>
            <button class="danger-button" on:click={() => saveSession(activeSessionKey, true)}>覆盖磁盘</button>
          </div>
        {:else if activeDocument.error}
          <div class="conflict-banner error"><span>{activeDocument.error}</span></div>
        {/if}

        <section class="document-content">
          {#if diffOriginal}
            <div class="diff-layout">
              <header><strong>{diffTitle}</strong><button on:click={closeDiff}>返回文档</button></header>
              {#key `${activeDocument.path}:${diffTitle}`}<DiffPane original={diffOriginal} modified={activeDocument.content} />{/key}
            </div>
          {:else if activeDocument.kind !== "markdown"}
            <ViewerPane document={activeDocument} on:openExternal={openActiveExternal} />
          {:else if activeDocument.viewMode === "edit"}
            {#key activeSessionKey}<EditorPane value={activeDocument.content} readOnly={!activeDocument.editable} lineEnding={activeDocument.lineEnding} on:change={(event) => editActive(event.detail.value)} on:line={(event) => setActiveLine(event.detail.line)} on:save={() => saveSession()} />{/key}
          {:else if activeDocument.viewMode === "preview"}
            <PreviewPane value={activeDocument.content} workspaceId={activeDocument.workspaceId} documentPath={activeDocument.path} activeLine={activeDocument.activeLine} largeFile={activeDocument.largeFile} {theme} on:localLink={(event) => openLocalLink(event.detail.href)} on:externalLink={(event) => api.openExternalUrl(event.detail.href)} />
          {:else}
            <div class="split-view" bind:this={splitHost} style={`--split:${splitRatio}%`}>
              <div class="split-pane">{#key activeSessionKey}<EditorPane bind:this={splitEditor} value={activeDocument.content} readOnly={!activeDocument.editable} lineEnding={activeDocument.lineEnding} on:change={(event) => editActive(event.detail.value)} on:line={(event) => setActiveLine(event.detail.line)} on:save={() => saveSession()} />{/key}</div>
              <button
                type="button"
                class="splitter"
                aria-label="调整编辑与预览宽度"
                title={`编辑区 ${Math.round(splitRatio)}%，使用左右方向键调整`}
                on:pointerdown={beginResize}
                on:keydown={resizeWithKeyboard}
              ></button>
              <div class="split-pane"><PreviewPane value={activeDocument.content} workspaceId={activeDocument.workspaceId} documentPath={activeDocument.path} activeLine={activeDocument.activeLine} largeFile={activeDocument.largeFile} outlineAvailable={false} {theme} on:line={(event) => syncEditorFromPreview(event.detail.line)} on:localLink={(event) => openLocalLink(event.detail.href)} on:externalLink={(event) => api.openExternalUrl(event.detail.href)} /></div>
            </div>
          {/if}
        </section>
      {:else}
        <section class="welcome">
          <div class="welcome-mark">M</div>
          <h1>MarkdownEditor</h1>
          <p>从左侧选择 Markdown 文件，或打开一个新的工作区。</p>
          <div class="welcome-shortcuts"><span><kbd>Ctrl</kbd> + <kbd>S</kbd> 保存</span><span><kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>S</kbd> 全部保存</span></div>
        </section>
      {/if}
    </main>

    {#if showGit && activeDocument}
      {#key activeSessionKey}
        <GitPanel workspaceId={activeDocument.workspaceId} document={activeDocument} on:close={() => (showGit = false)} on:diff={(event) => { diffOriginal = event.detail.original; diffTitle = event.detail.title; }} />
      {/key}
    {/if}
  </div>

  {#if starting}<div class="loading-overlay">正在启动 MarkdownEditor…</div>{/if}
  {#if recoveries.length > 0}
    <div class="modal-backdrop">
      <section class="recovery-modal">
        <header><div><strong>发现未保存内容</strong><p>选择恢复或丢弃。恢复不会立即覆盖磁盘文件。</p></div></header>
        <div class="recovery-list">
          {#each recoveries as entry}
            <div class="recovery-item">
              <div><strong>{entry.path.split(/[\\/]/).pop()}</strong><span>{new Date(entry.savedAt).toLocaleString("zh-CN")}</span></div>
              <div><button class="primary" on:click={() => restoreRecovery(entry)}>恢复</button><button on:click={() => discardRecovery(entry)}>丢弃</button></div>
            </div>
          {/each}
        </div>
      </section>
    </div>
  {/if}
</div>
