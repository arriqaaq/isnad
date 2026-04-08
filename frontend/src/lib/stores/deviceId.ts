const KEY = 'hadith-device-id';

export function getDeviceId(): string {
  if (typeof localStorage === 'undefined') return 'ssr';
  let id = localStorage.getItem(KEY);
  if (!id) {
    id = crypto.randomUUID();
    localStorage.setItem(KEY, id);
  }
  return id;
}
