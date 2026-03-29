import type {
  ApiBook,
  ApiHadithFamily,
  ApiMatnDiff,
  AnalysisStatsResponse,
  FamilyDetailResponse,
  GraphData,
  HadithDetailResponse,
  NarratorDetailResponse,
  PaginatedResponse,
  ApiHadith,
  ApiNarratorWithCount,
  SearchResponse,
  StatsResponse,
} from './types';

const BASE = '/api';

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  if (!res.ok) throw new Error(`API error: ${res.status}`);
  return res.json();
}

export async function getStats(): Promise<StatsResponse> {
  return get('/stats');
}

export async function getBooks(): Promise<ApiBook[]> {
  return get('/books');
}

export async function searchAll(
  q: string,
  type: 'text' | 'semantic' = 'text',
  limit = 20
): Promise<SearchResponse> {
  return get(`/search?q=${encodeURIComponent(q)}&type=${type}&limit=${limit}`);
}

export async function getHadiths(params: {
  book?: number;
  page?: number;
  limit?: number;
}): Promise<PaginatedResponse<ApiHadith>> {
  const sp = new URLSearchParams();
  if (params.book) sp.set('book', String(params.book));
  if (params.page) sp.set('page', String(params.page));
  if (params.limit) sp.set('limit', String(params.limit));
  return get(`/hadiths?${sp}`);
}

export async function getHadith(id: string): Promise<HadithDetailResponse> {
  return get(`/hadiths/${encodeURIComponent(id)}`);
}

export async function getNarrators(params: {
  q?: string;
  page?: number;
  limit?: number;
}): Promise<PaginatedResponse<ApiNarratorWithCount>> {
  const sp = new URLSearchParams();
  if (params.q) sp.set('q', params.q);
  if (params.page) sp.set('page', String(params.page));
  if (params.limit) sp.set('limit', String(params.limit));
  return get(`/narrators?${sp}`);
}

export async function getNarrator(id: string): Promise<NarratorDetailResponse> {
  return get(`/narrators/${encodeURIComponent(id)}`);
}

export async function getChainGraph(hadithId: string): Promise<GraphData> {
  return get(`/chain/${encodeURIComponent(hadithId)}`);
}

export async function getNarratorGraph(id: string): Promise<GraphData> {
  return get(`/narrators/${encodeURIComponent(id)}/graph`);
}

export async function updateNarrator(
  id: string,
  data: Record<string, unknown>
): Promise<void> {
  const res = await fetch(`${BASE}/narrators/${encodeURIComponent(id)}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error(`API error: ${res.status}`);
}

// ── Analysis API ──

export async function getFamilies(params: {
  page?: number;
  limit?: number;
}): Promise<PaginatedResponse<ApiHadithFamily>> {
  const sp = new URLSearchParams();
  if (params.page) sp.set('page', String(params.page));
  if (params.limit) sp.set('limit', String(params.limit));
  return get(`/families?${sp}`);
}

export async function getFamily(id: string): Promise<FamilyDetailResponse> {
  return get(`/families/${encodeURIComponent(id)}`);
}

export async function getAnalysisStats(): Promise<AnalysisStatsResponse> {
  return get('/analysis/stats');
}

export async function getMatnDiff(a: string, b: string): Promise<ApiMatnDiff> {
  return get(`/diff?a=${encodeURIComponent(a)}&b=${encodeURIComponent(b)}`);
}
