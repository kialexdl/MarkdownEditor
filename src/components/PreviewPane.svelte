<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from "svelte";
  import DOMPurify from "dompurify";
  import { api } from "../lib/api";

  export let value = "";
  export let workspaceId = "";
  export let documentPath = "";
  export let activeLine = 1;
  export let largeFile = false;
  export let outlineAvailable = true;
  export let theme: "dark" | "light" = "dark";

  interface OutlineEntry {
    id: string;
    title: string;
    level: number;
  }

  const dispatch = createEventDispatcher<{
    localLink: { href: string };
    externalLink: { href: string };
    line: { line: number };
  }>();
  let host: HTMLElement;
  let worker: Worker;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let revision = 0;
  let renderedRevision = 0;
  let mermaidCounter = 0;
  let renderError = "";
  let outline: OutlineEntry[] = [];
  let outlineVisible = true;
  const diagramCache = new Map<string, string>();

  function scheduleRender() {
    if (!worker) return;
    clearTimeout(timer);
    const currentRevision = ++revision;
    timer = setTimeout(
      () => worker.postMessage({ revision: currentRevision, source: value }),
      largeFile ? 500 : 150
    );
  }

  async function hydrateAssets(currentRevision: number) {
    const images = Array.from(host.querySelectorAll<HTMLImageElement>("img[src]"));
    await Promise.all(
      images.map(async (image) => {
        const src = image.getAttribute("src") ?? "";
        if (/^https?:/i.test(src)) {
          image.removeAttribute("src");
          image.classList.add("remote-image-blocked");
          image.alt = `${image.alt || "远程图片"}（默认未加载）`;
          return;
        }
        if (/^(data:|blob:|#)/i.test(src)) return;
        try {
          const dataUrl = await api.readAsset(workspaceId, documentPath, src);
          if (currentRevision === renderedRevision) image.src = dataUrl;
        } catch (error) {
          image.removeAttribute("src");
          image.alt = `图片加载失败：${String(error)}`;
        }
      })
    );
  }

  async function hydrateMermaid(currentRevision: number) {
    const blocks = Array.from(host.querySelectorAll<HTMLElement>("pre.mermaid-block"));
    if (blocks.length === 0) return;
    const { default: mermaid } = await import("mermaid");
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      htmlLabels: false,
      secure: ["securityLevel", "startOnLoad", "maxTextSize", "htmlLabels"],
      theme: theme === "light" ? "default" : "dark"
    });
    for (const block of blocks) {
      if (currentRevision !== renderedRevision) return;
      const source = block.textContent ?? "";
      const cacheKey = JSON.stringify([theme, source]);
      try {
        let svg = diagramCache.get(cacheKey);
        if (!svg) {
          const result = await mermaid.render(`mdme-${currentRevision}-${++mermaidCounter}`, source);
          svg = DOMPurify.sanitize(result.svg, { USE_PROFILES: { svg: true, svgFilters: true } });
          if (/<(?:text|foreignObject)\b/i.test(result.svg) && !/<text\b/i.test(svg)) {
            throw new Error("图表文字在安全清理后丢失，请检查 Mermaid 配置");
          }
          diagramCache.set(cacheKey, svg);
        }
        const container = document.createElement("div");
        container.className = "mermaid-rendered";
        container.dataset.sourceLine = block.dataset.sourceLine ?? "1";
        container.innerHTML = svg;
        block.replaceWith(container);
      } catch (error) {
        block.classList.add("render-error");
        block.textContent = `Mermaid 渲染失败\n${error instanceof Error ? error.message : String(error)}`;
      }
    }
  }

  async function applyHtml(html: string, currentRevision: number) {
    if (currentRevision !== revision) return;
    renderedRevision = currentRevision;
    renderError = "";
    host.innerHTML = DOMPurify.sanitize(html, {
      USE_PROFILES: { html: true },
      ADD_ATTR: ["data-source-line", "class", "id", "rel"]
    });
    refreshOutline();
    await Promise.all([hydrateAssets(currentRevision), hydrateMermaid(currentRevision)]);
    scrollToLine(activeLine, false);
  }

  function refreshOutline() {
    outline = Array.from(host.querySelectorAll<HTMLHeadingElement>("h1, h2, h3, h4, h5, h6"))
      .map((heading) => ({
        id: heading.id,
        title: heading.textContent?.trim() ?? "",
        level: Number(heading.tagName.slice(1))
      }))
      .filter((entry) => Boolean(entry.id && entry.title));
  }

  function toggleOutline() {
    outlineVisible = !outlineVisible;
    localStorage.setItem("mdme-preview-outline-visible", String(outlineVisible));
  }

  function jumpToHeading(id: string) {
    host
      .querySelector<HTMLElement>(`[id="${CSS.escape(id)}"]`)
      ?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  function onClick(event: MouseEvent) {
    const link = (event.target as Element).closest<HTMLAnchorElement>("a[href]");
    if (!link) return;
    event.preventDefault();
    const href = link.getAttribute("href") ?? "";
    if (href.startsWith("#")) {
      let id = href.slice(1);
      try {
        id = decodeURIComponent(id);
      } catch {
        return;
      }
      host
        .querySelector<HTMLElement>(`[id="${CSS.escape(id)}"]`)
        ?.scrollIntoView({ behavior: "smooth", block: "start" });
    } else if (/^(https?:|mailto:)/i.test(href)) {
      dispatch("externalLink", { href });
    } else if (!/^[a-z][a-z0-9+.-]*:/i.test(href)) {
      dispatch("localLink", { href });
    }
  }

  function onScroll() {
    const candidates = Array.from(host.querySelectorAll<HTMLElement>("[data-source-line]"));
    const top = host.getBoundingClientRect().top + 32;
    let nearest = 1;
    for (const element of candidates) {
      if (element.getBoundingClientRect().top <= top) {
        nearest = Number(element.dataset.sourceLine ?? 1);
      } else {
        break;
      }
    }
    dispatch("line", { line: nearest });
  }

  function scrollToLine(line: number, smooth = true) {
    if (!host || line < 1) return;
    const candidates = Array.from(host.querySelectorAll<HTMLElement>("[data-source-line]"));
    let target: HTMLElement | undefined;
    for (const candidate of candidates) {
      const sourceLine = Number(candidate.dataset.sourceLine ?? 0);
      if (sourceLine <= line) target = candidate;
      if (sourceLine > line) break;
    }
    target?.scrollIntoView({ behavior: smooth ? "smooth" : "auto", block: "start" });
  }

  onMount(() => {
    outlineVisible = localStorage.getItem("mdme-preview-outline-visible") !== "false";
    worker = new Worker(new URL("../lib/render.worker.ts", import.meta.url), { type: "module" });
    worker.onmessage = (event) => {
      const { revision: resultRevision, html, error } = event.data;
      if (error) {
        renderError = error;
      } else {
        void applyHtml(html, resultRevision);
      }
    };
    host.addEventListener("click", onClick);
    host.addEventListener("scroll", onScroll, { passive: true });
    scheduleRender();
  });

  $: value, theme, scheduleRender();
  $: if (renderedRevision > 0) scrollToLine(activeLine);

  onDestroy(() => {
    clearTimeout(timer);
    worker?.terminate();
    host?.removeEventListener("click", onClick);
    host?.removeEventListener("scroll", onScroll);
  });
</script>

<div class="preview-wrap" class:outline-hidden={!outlineVisible} class:outline-unavailable={!outlineAvailable}>
  {#if outlineAvailable}
    <header class="preview-header">
      <span>{outlineVisible ? "文档目录" : "目录已隐藏"}</span>
      <button
        type="button"
        class="outline-toggle"
        on:click={toggleOutline}
        aria-expanded={outlineVisible}
        title={outlineVisible ? "隐藏文档目录" : "显示文档目录"}
      >{outlineVisible ? "隐藏" : "显示目录"}</button>
    </header>
  {/if}
  {#if renderError}<div class="preview-error">预览失败：{renderError}</div>{/if}
  <article class="markdown-body" bind:this={host}></article>
  {#if outlineAvailable && outlineVisible}
    <aside class="preview-outline" aria-label="文档目录">
      {#if outline.length > 0}
        <nav>
          {#each outline as item (item.id)}
            <button
              type="button"
              class="outline-link"
              style={`padding-left: ${12 + (item.level - 1) * 10}px`}
              title={item.title}
              on:click={() => jumpToHeading(item.id)}
            >{item.title}</button>
          {/each}
        </nav>
      {:else}
        <p>当前文档没有标题</p>
      {/if}
    </aside>
  {/if}
</div>

<style>
  .preview-wrap { height: 100%; min-width: 0; position: relative; display: grid; grid-template-columns: minmax(0, 1fr) 210px; grid-template-rows: 34px minmax(0, 1fr); background: var(--preview-bg); }
  .preview-wrap.outline-hidden { grid-template-columns: minmax(0, 1fr); }
  .preview-wrap.outline-unavailable { grid-template-columns: minmax(0, 1fr); grid-template-rows: minmax(0, 1fr); }
  .preview-header { grid-column: 1 / -1; display: flex; align-items: center; justify-content: flex-end; gap: 8px; padding: 0 8px; border-bottom: 1px solid var(--border); background: var(--surface); color: var(--muted); font-size: 11px; }
  .preview-header span { margin-right: auto; padding-left: 4px; }
  .outline-toggle { padding: 2px 7px; font-size: 11px; }
  .markdown-body { min-width: 0; min-height: 0; overflow: auto; padding: 34px clamp(28px, 6vw, 90px) 80px; color: var(--text); line-height: 1.72; }
  .preview-outline { min-width: 0; min-height: 0; overflow: auto; border-left: 1px solid var(--border); background: var(--surface); }
  .preview-outline nav { padding: 8px 0 20px; }
  .preview-outline p { margin: 0; padding: 18px 12px; color: var(--muted); font-size: 11px; }
  .outline-link { width: 100%; display: block; overflow: hidden; padding-block: 6px; padding-right: 10px; border: 0; border-radius: 0; background: transparent; color: var(--muted); font-size: 11.5px; text-align: left; text-overflow: ellipsis; white-space: nowrap; }
  .outline-link:hover:not(:disabled) { border: 0; background: var(--surface-2); color: var(--text-strong); }
  .preview-error { position: absolute; inset: 46px 16px auto; z-index: 2; padding: 10px 12px; border: 1px solid var(--danger); border-radius: 8px; background: var(--danger-soft); }
  .outline-unavailable .preview-error { inset: 12px 16px auto; }
</style>
