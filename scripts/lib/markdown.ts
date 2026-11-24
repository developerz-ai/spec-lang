// Small, pure Markdown helpers. Fenced code is ignored, so examples inside fences never count as links or headings.

export function stripFences(text: string): string {
  return text.replace(/^(```|~~~)[^\n]*\n[\s\S]*?^\1[^\n]*$/gm, (block) => block.replace(/[^\n]/g, " "));
}

export type Link = { target: string; line: number };

export function links(text: string): Link[] {
  const body = stripFences(text).replace(/`[^`\n]*`/g, (s) => " ".repeat(s.length));
  const out: Link[] = [];
  for (const m of body.matchAll(/\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g)) {
    out.push({ target: m[1] as string, line: body.slice(0, m.index).split("\n").length });
  }
  return out;
}

export function headings(text: string): { level: number; title: string }[] {
  return [...stripFences(text).matchAll(/^(#{1,6})\s+(.+?)\s*$/gm)].map((m) => ({
    level: (m[1] as string).length,
    title: m[2] as string,
  }));
}

/** GitHub's anchor for a heading: lower case, punctuation dropped, spaces to hyphens. */
export function anchor(title: string): string {
  return title
    .toLowerCase()
    .replace(/[`*_]/g, "")
    .replace(/[^\p{L}\p{N}\s-]/gu, "")
    .trim()
    .replace(/\s/g, "-");
}
