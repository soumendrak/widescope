export function formatTraceText(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch (wholeError) {
    const records = text.split(/\r?\n/).filter((line) => line.trim());
    if (records.length < 2) throw wholeError;
    return records.map((line) => JSON.stringify(JSON.parse(line))).join('\n');
  }
}
