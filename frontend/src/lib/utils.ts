export function truncate(text: string | null | undefined, maxLen: number): string {
  if (!text) return '';
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen) + '...';
}

export function stripHtml(html: string | null | undefined): string {
  if (!html) return '';
  return html.replace(/<[^>]*>/g, '');
}

export function formatScore(score: number | null): string {
  if (score === null) return '';
  return score.toFixed(2);
}

/**
 * Convert turath page text to HTML for rendering.
 * Ported from usul.ai's src/lib/reader.ts
 */
export function convertPageToHtml(page: string): string {
  const footnotesChar = '_________';
  return page
    .replaceAll('</span>.', '</span>')
    .replaceAll('\n', '<br>')
    .split('<br>')
    .map(block => {
      let final_ = block;
      const idx = block.indexOf(footnotesChar);
      if (idx > -1) {
        const txt = block.slice(0, idx);
        const footnotes = block.slice(idx + footnotesChar.length);
        final_ = txt + `<p class="footnotes">${footnotes}</p>`;
      }
      return `<div class="block">${final_}</div>`;
    })
    .join('');
}
