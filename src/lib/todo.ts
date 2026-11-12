export type Task = {
  id: string;
  title: string;
  checked: boolean;
  lineIndex: number;
};

// Task syntax shared with the Rust core (cases in crates/taskhud-core/fixtures/task-syntax.json):
// optional spaces or tabs, then `- [ ]`, `- [x]` or `- [X]`, then at least one space or tab and a non-empty title.
const taskPattern = /^[ \t]*- \[([ xX])\][ \t]+(.*)$/;

export function parseTodo(markdown: string): Task[] {
  return markdown.split(/\r?\n/).flatMap((line, lineIndex) => {
    const match = line.match(taskPattern);
    const title = match?.[2].trim();
    if (!match || !title) return [];

    return [{
      id: `task-${lineIndex}`,
      title,
      checked: match[1].toLowerCase() === 'x',
      lineIndex,
    }];
  });
}

export function toggleTask(markdown: string, taskId: string): string {
  const task = parseTodo(markdown).find(candidate => candidate.id === taskId);
  if (!task) return markdown;

  const lines = markdown.split(/\r?\n/);
  const line = lines[task.lineIndex];
  lines[task.lineIndex] = line.replace(/\[([ xX])\]/, task.checked ? '[ ]' : '[x]');
  return lines.join(lineEnding(markdown));
}

export function appendTask(markdown: string, title: string): string {
  const cleanTitle = title.trim();
  if (!cleanTitle) return markdown;

  const newline = lineEnding(markdown);
  const separator = markdown.endsWith('\n') || markdown.length === 0 ? '' : newline;
  return `${markdown}${separator}- [ ] ${cleanTitle}${newline}`;
}

// Tasks of `markdown` that `other` does not have with the same title and state.
export function tasksMissingFrom(markdown: string, other: string): Task[] {
  const key = (task: Task) => `${task.checked ? 'x' : ' '}${task.title}`;
  const otherKeys = new Set(parseTodo(other).map(key));
  return parseTodo(markdown).filter(task => !otherKeys.has(key(task)));
}

// Same rule as the Rust core: a file that already contains CRLF keeps CRLF on every line written back.
function lineEnding(markdown: string): string {
  return markdown.includes('\r\n') ? '\r\n' : '\n';
}
