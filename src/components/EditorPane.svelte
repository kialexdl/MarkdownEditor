<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from "svelte";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { defaultHighlightStyle, HighlightStyle, syntaxHighlighting } from "@codemirror/language";
  import { searchKeymap } from "@codemirror/search";
  import { EditorState } from "@codemirror/state";
  import { tags } from "@lezer/highlight";
  import {
    drawSelection,
    dropCursor,
    EditorView,
    highlightActiveLine,
    highlightActiveLineGutter,
    highlightSpecialChars,
    keymap,
    lineNumbers,
    rectangularSelection
  } from "@codemirror/view";

  export let value = "";
  export let readOnly = false;
  export let lineEnding: "\n" | "\r\n" = "\n";

  const dispatch = createEventDispatcher<{
    change: { value: string };
    line: { line: number };
    save: void;
  }>();
  let host: HTMLDivElement;
  let view: EditorView | null = null;

  const theme = EditorView.theme({
    "&": { height: "100%", backgroundColor: "var(--editor-bg)", color: "var(--text)" },
    ".cm-scroller": { overflow: "auto", fontFamily: "var(--mono)", fontSize: "14px", lineHeight: "1.65" },
    ".cm-content": { padding: "18px 0", caretColor: "var(--accent)" },
    ".cm-gutters": { backgroundColor: "var(--editor-bg)", color: "var(--muted)", border: "none" },
    ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "var(--active-line)" },
    ".cm-selectionBackground, ::selection": { backgroundColor: "var(--selection) !important" },
    ".cm-cursor": { borderLeftColor: "var(--accent)" }
  });

  const markdownContrastHighlightStyle = HighlightStyle.define([
    { tag: tags.labelName, class: "cm-md-language-label" }
  ]);

  function emitLine(current: EditorView) {
    const head = current.state.selection.main.head;
    dispatch("line", { line: current.state.doc.lineAt(head).number });
  }

  onMount(() => {
    const saveKey = {
      key: "Mod-s",
      preventDefault: true,
      run: () => {
        dispatch("save");
        return true;
      }
    };
    const state = EditorState.create({
      doc: value,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightSpecialChars(),
        history(),
        drawSelection(),
        dropCursor(),
        rectangularSelection(),
        highlightActiveLine(),
        EditorView.lineWrapping,
        markdown(),
        syntaxHighlighting(defaultHighlightStyle),
        syntaxHighlighting(markdownContrastHighlightStyle),
        keymap.of([saveKey, indentWithTab, ...defaultKeymap, ...historyKeymap, ...searchKeymap]),
        EditorState.lineSeparator.of(lineEnding),
        EditorState.readOnly.of(readOnly),
        EditorView.editable.of(!readOnly),
        theme,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            dispatch("change", { value: update.state.doc.toString() });
          }
          if (update.selectionSet || update.docChanged) emitLine(update.view);
        })
      ]
    });
    view = new EditorView({ state, parent: host });
    emitLine(view);
  });

  $: if (view && value !== view.state.doc.toString()) {
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } });
  }

  export function focusLine(line: number) {
    if (!view || line < 1 || line > view.state.doc.lines) return;
    const target = view.state.doc.line(line);
    view.dispatch({ selection: { anchor: target.from }, scrollIntoView: true });
    view.focus();
  }

  export function scrollToLine(line: number) {
    if (!view || line < 1 || line > view.state.doc.lines) return;
    const target = view.state.doc.line(line);
    view.dispatch({ effects: EditorView.scrollIntoView(target.from, { y: "start" }) });
  }

  onDestroy(() => view?.destroy());
</script>

<div class="editor-host" bind:this={host}></div>

<style>
  .editor-host { height: 100%; min-width: 0; overflow: hidden; }
  .editor-host :global(.cm-md-language-label) { color: var(--warning) !important; font-weight: 700; }
</style>
