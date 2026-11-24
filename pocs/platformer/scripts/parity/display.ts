// One virtual X server for every `shot` the suite runs, so the GL driver is the same for all backends.

export type Display = { name: string; stop: () => void };

export async function startDisplay(): Promise<Display> {
  if (!Bun.which("Xvfb")) throw new Error("Xvfb not found: the shot cases need it (apt install xvfb)");
  const p = Bun.spawn(["Xvfb", "-displayfd", "1", "-screen", "0", "1280x720x24", "-nolisten", "tcp"], {
    stdout: "pipe",
    stderr: "ignore",
  });
  const reader = p.stdout.getReader();
  let text = "";
  while (!text.includes("\n")) {
    const { value, done } = await reader.read();
    if (done) throw new Error("Xvfb exited before it named a display");
    text += new TextDecoder().decode(value);
  }
  reader.releaseLock();
  return { name: `:${text.trim()}`, stop: () => p.kill() };
}
