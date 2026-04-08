// ── Content parsing (shared between NoteCard read-only and RichEditor) ──

export type ContentSegment =
  | { type: 'text'; value: string }
  | { type: 'ayah'; refId: string }
  | { type: 'hadith'; refId: string }
  | { type: 'narrator'; refId: string }
  | { type: 'url'; value: string };

const MENTION_PATTERN = /@(\d+:\d+)|@(\w+:\d+)|@(narrator:[^\s,]+)|(https?:\/\/\S+)/g;

export function parseContent(text: string): ContentSegment[] {
  const parts: ContentSegment[] = [];
  let lastIndex = 0;
  let match;

  // Reset regex state
  MENTION_PATTERN.lastIndex = 0;

  while ((match = MENTION_PATTERN.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push({ type: 'text', value: text.slice(lastIndex, match.index) });
    }
    if (match[1]) {
      parts.push({ type: 'ayah', refId: match[1] });
    } else if (match[2]) {
      parts.push({ type: 'hadith', refId: match[2] });
    } else if (match[3]) {
      const id = match[3].replace('narrator:', '');
      parts.push({ type: 'narrator', refId: id });
    } else if (match[4]) {
      parts.push({ type: 'url', value: match[4] });
    }
    lastIndex = MENTION_PATTERN.lastIndex;
  }
  if (lastIndex < text.length) {
    parts.push({ type: 'text', value: text.slice(lastIndex) });
  }
  return parts;
}

// ── Contenteditable utilities ──

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

/**
 * Convert plain text storage format to HTML for the contenteditable editor.
 * Embedded refs become <span class="ref-atom" contenteditable="false" data-ref-type="..." data-ref-id="...">
 * which will be hydrated with Svelte components after insertion.
 */
export function deserializeToHtml(text: string): string {
  const segments = parseContent(text);
  let html = '';

  for (const seg of segments) {
    switch (seg.type) {
      case 'text':
        html += escapeHtml(seg.value).replace(/\n/g, '<br>');
        break;
      case 'ayah':
        html += `<span class="ref-atom" contenteditable="false" data-ref-type="ayah" data-ref-id="${escapeHtml(seg.refId)}"></span>`;
        break;
      case 'hadith':
        html += `<span class="ref-atom" contenteditable="false" data-ref-type="hadith" data-ref-id="${escapeHtml(seg.refId)}"></span>`;
        break;
      case 'narrator':
        html += `<span class="ref-atom" contenteditable="false" data-ref-type="narrator" data-ref-id="${escapeHtml(seg.refId)}"></span>`;
        break;
      case 'url':
        html += `<a href="${escapeHtml(seg.value)}" target="_blank" rel="noopener" data-autolink="true">${escapeHtml(seg.value)}</a>`;
        break;
    }
  }
  return html;
}

/**
 * Serialize the contenteditable DOM back to plain text storage format.
 */
export function serializeEditor(container: HTMLElement): string {
  let result = '';

  for (const node of container.childNodes) {
    if (node.nodeType === Node.TEXT_NODE) {
      // Strip \u00A0 (non-breaking space) used for cursor positioning after atoms
      let text = node.textContent ?? '';
      text = text.replace(/\u00A0/g, ' ').replace(/^ $/, ' ');
      result += text;
    } else if (node.nodeType === Node.ELEMENT_NODE) {
      const el = node as HTMLElement;
      if (el.classList.contains('ref-atom')) {
        const refType = el.dataset.refType;
        const refId = el.dataset.refId;
        if (refType === 'narrator') {
          result += `@narrator:${refId}`;
        } else {
          result += `@${refId}`;
        }
      } else if (el.tagName === 'A' && el.dataset.autolink) {
        result += el.getAttribute('href') ?? el.textContent ?? '';
      } else if (el.tagName === 'BR') {
        result += '\n';
      } else {
        // Recurse into divs/spans the browser may create
        const inner = serializeEditor(el);
        result += inner;
        // Only add newline for block elements that have content and aren't the last child
        if ((el.tagName === 'DIV' || el.tagName === 'P') && inner.length > 0 && el.nextSibling) {
          result += '\n';
        }
      }
    }
  }
  return result.replace(/\n+$/, '');
}

/**
 * Place cursor immediately after the given element.
 */
export function placeCursorAfter(element: Node) {
  const range = document.createRange();
  range.setStartAfter(element);
  range.collapse(true);
  const sel = window.getSelection();
  sel?.removeAllRanges();
  sel?.addRange(range);
}

/**
 * Get the current @ mention context at the cursor position.
 * Returns the query typed after @, and the Range to replace.
 */
export function getAtMentionContext(container: HTMLElement): { query: string; range: Range } | null {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0 || !sel.isCollapsed) return null;

  const anchorNode = sel.anchorNode;
  const anchorOffset = sel.anchorOffset;

  if (!anchorNode || !container.contains(anchorNode)) return null;

  // Must be in a text node
  if (anchorNode.nodeType !== Node.TEXT_NODE) return null;

  const text = anchorNode.textContent ?? '';
  const before = text.slice(0, anchorOffset);
  const atIdx = before.lastIndexOf('@');

  if (atIdx < 0) return null;
  // @ must be at start or preceded by whitespace/newline
  if (atIdx > 0 && before[atIdx - 1] !== ' ' && before[atIdx - 1] !== '\n' && before[atIdx - 1] !== '\u00A0') return null;

  const query = before.slice(atIdx + 1);

  // Create a range spanning from @ to cursor
  const range = document.createRange();
  range.setStart(anchorNode, atIdx);
  range.setEnd(anchorNode, anchorOffset);

  return { query, range };
}

/**
 * Replace a Range with an element and place cursor after it.
 */
export function replaceRangeWithAtom(range: Range, element: HTMLElement) {
  range.deleteContents();
  range.insertNode(element);
  // Add a space after so cursor has somewhere to land
  const space = document.createTextNode('\u00A0');
  element.after(space);
  placeCursorAfter(space);
}
