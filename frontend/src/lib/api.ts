import type {
  ApiBook,
  ApiHadithFamily,
  ApiManuscript,
  ApiMatnDiff,
  ApiReciter,
  ApiSurah,
  ApiVariantReading,
  AnalysisStatsResponse,
  AyahHadithResponse,
  FamilyDetailResponse,
  GraphData,
  HadithDetailResponse,
  NarratorDetailResponse,
  PaginatedResponse,
  ApiHadith,
  ApiNarratorWithCount,
  SearchResponse,
  StatsResponse,
  QuranSearchResponse,
  QuranStatsResponse,
  SurahDetailResponse,
  UnifiedSearchResponse,
  ApiQuranWord,
  RootSearchResponse,
  AyahSimilarResponse,
  ApiPhraseWithAyahs,
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

export async function getJuynbollSummary(): Promise<import('./types').JuynbollSummaryResponse> {
  return get('/analysis/juynboll/summary');
}

export async function getNarratorClStatus(id: string): Promise<import('./types').NarratorClStatus> {
  return get(`/narrators/${encodeURIComponent(id)}/cl-status`);
}

export async function getMatnDiff(a: string, b: string): Promise<ApiMatnDiff> {
  return get(`/diff?a=${encodeURIComponent(a)}&b=${encodeURIComponent(b)}`);
}

// ── Quran API ──

export async function getQuranStats(): Promise<QuranStatsResponse> {
  return get('/quran/stats');
}

export async function getSurahs(): Promise<ApiSurah[]> {
  return get('/quran/surahs');
}

export async function getSurah(number: number): Promise<SurahDetailResponse> {
  return get(`/quran/surahs/${number}`);
}

export async function searchQuran(
  q: string,
  type: 'text' | 'semantic' | 'hybrid' | 'tafsir' = 'text',
  limit = 20
): Promise<QuranSearchResponse> {
  return get(`/quran/search?q=${encodeURIComponent(q)}&type=${type}&limit=${limit}`);
}

export async function getAyahHadiths(
  surah: number,
  ayah: number,
  includeSemantic = false,
  semanticLimit = 5
): Promise<AyahHadithResponse> {
  const sp = new URLSearchParams();
  if (includeSemantic) sp.set('include_semantic', 'true');
  if (semanticLimit !== 5) sp.set('semantic_limit', String(semanticLimit));
  const query = sp.toString() ? `?${sp}` : '';
  return get(`/quran/ayah/${surah}:${ayah}/hadiths${query}`);
}

export async function getSurahHadithCounts(
  surah: number
): Promise<Record<string, number>> {
  return get(`/quran/surahs/${surah}/hadith-counts`);
}

// ── Unified Quran & Sunnah API ──

export async function searchUnified(
  q: string,
  type: 'hybrid' | 'semantic' = 'hybrid',
  limit = 20,
  page = 1
): Promise<UnifiedSearchResponse> {
  return get(`/unified/search?q=${encodeURIComponent(q)}&type=${type}&limit=${limit}&page=${page}`);
}

// ── Quran Word Morphology API ──

export async function getAyahWords(surah: number, ayah: number): Promise<ApiQuranWord[]> {
  return get<ApiQuranWord[]>(`/quran/ayah/${surah}:${ayah}/words`);
}

export async function searchByRoot(root: string): Promise<RootSearchResponse> {
  return get<RootSearchResponse>(`/quran/search/root/${encodeURIComponent(root)}`);
}

// ── Quran Recitation API ──

export async function getReciters(): Promise<ApiReciter[]> {
  return get<ApiReciter[]>('/quran/reciters');
}

// ── Manuscript & Variant Reading API ──

export async function getManuscripts(params: {
  page?: number;
  limit?: number;
}): Promise<PaginatedResponse<ApiManuscript>> {
  const sp = new URLSearchParams();
  if (params.page) sp.set('page', String(params.page));
  if (params.limit) sp.set('limit', String(params.limit));
  return get(`/quran/manuscripts?${sp}`);
}

export async function getManuscript(id: string): Promise<ApiManuscript> {
  return get(`/quran/manuscripts/${encodeURIComponent(id)}`);
}

export async function getAyahVariants(surah: number, ayah: number): Promise<ApiVariantReading[]> {
  return get<ApiVariantReading[]>(`/quran/ayah/${surah}:${ayah}/variants`);
}

// ── Similar Ayahs / Mutashabihat API ──

export async function getAyahSimilar(surah: number, ayah: number): Promise<AyahSimilarResponse> {
  return get<AyahSimilarResponse>(`/quran/ayah/${surah}:${ayah}/similar`);
}

export async function getPhraseDetail(id: string): Promise<ApiPhraseWithAyahs> {
  return get<ApiPhraseWithAyahs>(`/quran/phrases/${encodeURIComponent(id)}`);
}
