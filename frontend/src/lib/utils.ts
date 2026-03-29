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
