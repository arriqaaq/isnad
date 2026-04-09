// ── Content parsing (shared between NoteCard read-only and RichEditor) ──

export type ContentSegment =
  | { type: 'text'; value: string }
  | { type: 'ayah'; refId: string }
  | { type: 'hadith'; refId: string }
  | { type: 'narrator'; refId: string }
  | { type: 'url'; value: string }
  | { type: 'html'; value: string };

const MENTION_PATTERN = /@(\d+:\d+)|@(\w+:\d+)|@(narrator:[^\s,]+)|(https?:\/\/\S+)/g;

/** Allowed HTML tags for rich text (used for sanitization). */
const ALLOWED_TAGS = new Set(['b', 'strong', 'i', 'em', 'u', 's', 'h2', 'h3', 'blockquote', 'ul', 'ol', 'li', 'hr', 'br', 'p', 'div']);

/** Detect whether stored content contains HTML formatting tags. */
export function isHtmlContent(text: string): boolean {
  return /<(b|strong|i|em|u|s|h[23]|blockquote|ul|ol|li|hr|br|p|div)\b/i.test(text);
}

/** Sanitize HTML: only allow whitelisted tags, strip all attributes except on ref-atoms. */
function sanitizeHtml(html: string): string {
  // Strip all tags except allowed ones (keep ref-atom spans for @mentions)
  return html.replace(/<\/?([a-z][a-z0-9]*)\b[^>]*>/gi, (match, tag) => {
    const lower = tag.toLowerCase();
    if (ALLOWED_TAGS.has(lower)) {
      // For closing tags, keep them. For opening tags, strip attributes.
      if (match.startsWith('</')) return `</${lower}>`;
      if (lower === 'hr' || lower === 'br') return `<${lower}>`;
      return `<${lower}>`;
    }
    return '';
  });
}

export function parseContent(text: string): ContentSegment[] {
  // If content has HTML formatting, split into html chunks and @mention refs
  if (isHtmlContent(text)) {
    return parseHtmlContent(text);
  }

  // Legacy plain text parsing
  const parts: ContentSegment[] = [];
  let lastIndex = 0;
  let match;

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

/** Parse HTML-formatted content, splitting out @mention refs from HTML. */
function parseHtmlContent(text: string): ContentSegment[] {
  const parts: ContentSegment[] = [];
  let lastIndex = 0;
  let match;

  MENTION_PATTERN.lastIndex = 0;

  while ((match = MENTION_PATTERN.exec(text)) !== null) {
    if (match.index > lastIndex) {
      const chunk = text.slice(lastIndex, match.index);
      parts.push({ type: 'html', value: sanitizeHtml(chunk) });
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
    const chunk = text.slice(lastIndex);
    parts.push({ type: 'html', value: sanitizeHtml(chunk) });
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
 * Convert storage format to HTML for the contenteditable editor.
 * Supports both legacy plain text and new HTML-formatted content.
 * Embedded refs become <span class="ref-atom" contenteditable="false" data-ref-type="..." data-ref-id="...">
 * which will be hydrated with Svelte components after insertion.
 */
export function deserializeToHtml(text: string): string {
  if (isHtmlContent(text)) {
    return deserializeHtmlContent(text);
  }
  return deserializePlainContent(text);
}

/** Deserialize legacy plain text with @mentions. */
function deserializePlainContent(text: string): string {
  // Use the plain-text-specific parsing (skip isHtmlContent check)
  const parts: ContentSegment[] = [];
  let lastIndex = 0;
  let match;
  MENTION_PATTERN.lastIndex = 0;
  while ((match = MENTION_PATTERN.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push({ type: 'text', value: text.slice(lastIndex, match.index) });
    }
    if (match[1]) parts.push({ type: 'ayah', refId: match[1] });
    else if (match[2]) parts.push({ type: 'hadith', refId: match[2] });
    else if (match[3]) parts.push({ type: 'narrator', refId: match[3].replace('narrator:', '') });
    else if (match[4]) parts.push({ type: 'url', value: match[4] });
    lastIndex = MENTION_PATTERN.lastIndex;
  }
  if (lastIndex < text.length) {
    parts.push({ type: 'text', value: text.slice(lastIndex) });
  }

  let html = '';
  for (const seg of parts) {
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

/** Deserialize HTML-formatted content, replacing @mentions with ref-atom spans. */
function deserializeHtmlContent(text: string): string {
  // Replace @mentions within the HTML with ref-atom spans
  let html = text;
  MENTION_PATTERN.lastIndex = 0;
  html = html.replace(MENTION_PATTERN, (match, ayah, hadith, narrator, url) => {
    if (ayah) {
      return `<span class="ref-atom" contenteditable="false" data-ref-type="ayah" data-ref-id="${escapeHtml(ayah)}"></span>`;
    } else if (hadith) {
      return `<span class="ref-atom" contenteditable="false" data-ref-type="hadith" data-ref-id="${escapeHtml(hadith)}"></span>`;
    } else if (narrator) {
      const id = narrator.replace('narrator:', '');
      return `<span class="ref-atom" contenteditable="false" data-ref-type="narrator" data-ref-id="${escapeHtml(id)}"></span>`;
    } else if (url) {
      return `<a href="${escapeHtml(url)}" target="_blank" rel="noopener" data-autolink="true">${escapeHtml(url)}</a>`;
    }
    return match;
  });
  return html;
}

/** Tags that should be preserved in serialized HTML output. */
const PRESERVED_TAGS = new Set(['B', 'STRONG', 'I', 'EM', 'U', 'S', 'H2', 'H3', 'BLOCKQUOTE', 'UL', 'OL', 'LI', 'HR']);

/** Block-level tags that get newlines around them. */
const BLOCK_TAGS = new Set(['DIV', 'P', 'H2', 'H3', 'BLOCKQUOTE', 'UL', 'OL', 'LI', 'HR']);

/**
 * Serialize the contenteditable DOM back to storage format.
 * Preserves formatting HTML tags (bold, italic, headings, lists, etc).
 * Ref atoms are serialized as @mentions. Plain text is kept as-is.
 */
export function serializeEditor(container: HTMLElement): string {
  let result = '';

  for (const node of container.childNodes) {
    if (node.nodeType === Node.TEXT_NODE) {
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
        result += '<br>';
      } else if (el.tagName === 'HR') {
        result += '<hr>';
      } else if (PRESERVED_TAGS.has(el.tagName)) {
        // Preserve formatting tags
        const tag = el.tagName.toLowerCase();
        const inner = serializeEditor(el);
        result += `<${tag}>${inner}</${tag}>`;
      } else {
        // Recurse into divs/spans/p the browser may create
        const inner = serializeEditor(el);
        result += inner;
        if ((el.tagName === 'DIV' || el.tagName === 'P') && inner.length > 0 && el.nextSibling) {
          result += '<br>';
        }
      }
    }
  }
  return result.replace(/(<br>)+$/, '');
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
 * Get the current / slash command context at the cursor position.
 * Returns the query typed after /, and the Range to replace.
 */
export function getSlashCommandContext(container: HTMLElement): { query: string; range: Range } | null {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0 || !sel.isCollapsed) return null;

  const anchorNode = sel.anchorNode;
  const anchorOffset = sel.anchorOffset;

  if (!anchorNode || !container.contains(anchorNode)) return null;
  if (anchorNode.nodeType !== Node.TEXT_NODE) return null;

  const text = anchorNode.textContent ?? '';
  const before = text.slice(0, anchorOffset);
  const slashIdx = before.lastIndexOf('/');

  if (slashIdx < 0) return null;
  // / must be at start of text or preceded by whitespace/newline
  if (slashIdx > 0 && before[slashIdx - 1] !== ' ' && before[slashIdx - 1] !== '\n' && before[slashIdx - 1] !== '\u00A0') return null;

  const query = before.slice(slashIdx + 1);
  // Only trigger if there's no space in the query (it's a single command token)
  if (query.includes(' ')) return null;

  const range = document.createRange();
  range.setStart(anchorNode, slashIdx);
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
