export function createRunId(prefix: string = 'run'): string {
  const p = String(prefix || 'run').trim() || 'run';
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return `${p}_${crypto.randomUUID()}`;
  }
  return `${p}_${Date.now()}_${Math.random().toString(16).slice(2, 10)}`;
}
