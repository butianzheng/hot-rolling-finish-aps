export const MACHINE_DATE_KEY_SEP = '__';

export function makeMachineDateKey(machine: string, date: string): string {
  return `${String(machine ?? '').trim()}${MACHINE_DATE_KEY_SEP}${String(date ?? '').trim()}`;
}

export function splitMachineDateKey(key: string): { machine: string; date: string } {
  const raw = String(key ?? '');
  const idx = raw.indexOf(MACHINE_DATE_KEY_SEP);
  if (idx < 0) return { machine: raw, date: '' };
  return {
    machine: raw.slice(0, idx),
    date: raw.slice(idx + MACHINE_DATE_KEY_SEP.length),
  };
}

