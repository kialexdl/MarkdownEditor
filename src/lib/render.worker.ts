/// <reference lib="webworker" />

import MarkdownIt from "markdown-it";
import taskLists from "markdown-it-task-lists";

interface RenderRequest {
  revision: number;
  source: string;
}

function slugify(value: string, counts: Map<string, number>): string {
  const base = value
    .trim()
    .toLowerCase()
    .replace(/[\s]+/g, "-")
    .replace(/[^\p{L}\p{N}_-]/gu, "") || "section";
  const count = counts.get(base) ?? 0;
  counts.set(base, count + 1);
  return count === 0 ? base : `${base}-${count}`;
}

function createRenderer() {
  const md = new MarkdownIt({
    html: false,
    linkify: true,
    breaks: false,
    typographer: false
  });
  md.use(taskLists, { enabled: false, label: true });

  md.core.ruler.after("block", "source-lines", (state) => {
    for (const token of state.tokens) {
      if (token.map && token.nesting === 1) {
        token.attrSet("data-source-line", String(token.map[0] + 1));
      }
    }
  });

  const defaultHeading = md.renderer.rules.heading_open;
  md.renderer.rules.heading_open = (tokens, index, options, env, self) => {
    const inline = tokens[index + 1];
    const renderEnv = env ?? {};
    const counts = (renderEnv.slugCounts ??= new Map<string, number>()) as Map<string, number>;
    tokens[index].attrSet("id", slugify(inline?.content ?? "section", counts));
    return defaultHeading
      ? defaultHeading(tokens, index, options, renderEnv, self)
      : self.renderToken(tokens, index, options);
  };

  const defaultFence = md.renderer.rules.fence;
  md.renderer.rules.fence = (tokens, index, options, env, self) => {
    const token = tokens[index];
    const language = token.info.trim().split(/\s+/)[0]?.toLowerCase();
    if (language === "mermaid") {
      const line = token.map ? token.map[0] + 1 : 1;
      return `<pre class="mermaid-block" data-source-line="${line}"><code>${md.utils.escapeHtml(token.content)}</code></pre>`;
    }
    return defaultFence
      ? defaultFence(tokens, index, options, env, self)
      : self.renderToken(tokens, index, options);
  };

  const defaultLinkOpen = md.renderer.rules.link_open;
  md.renderer.rules.link_open = (tokens, index, options, env, self) => {
    tokens[index].attrSet("rel", "noopener noreferrer");
    return defaultLinkOpen
      ? defaultLinkOpen(tokens, index, options, env, self)
      : self.renderToken(tokens, index, options);
  };
  return md;
}

const markdown = createRenderer();

self.onmessage = (event: MessageEvent<RenderRequest>) => {
  const { revision, source } = event.data;
  try {
    const html = markdown.render(source, { slugCounts: new Map<string, number>() });
    self.postMessage({ revision, html, error: null });
  } catch (error) {
    self.postMessage({
      revision,
      html: "",
      error: error instanceof Error ? error.message : String(error)
    });
  }
};

export {};
