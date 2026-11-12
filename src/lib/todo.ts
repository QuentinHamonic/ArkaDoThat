export type Task = {
  id: string;
  title: string;
  checked: boolean;
  lineIndex: number;
};

const taskPattern = /^(\s*)-\s*\[([ xX])\]\s+(.*?)\s*$/;

export function parseTodo(markdown: string): Task[] {
  return markdown.split(/\r?\n/).flatMap((line, lineIndex) => {
    const match = line.match(taskPattern);
    if (!match) return [];

    return [{
      id: `task-${lineIndex}`,
      title: match[3],
      checked: match[2].toLowerCase() === 'x',
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

// Same rule as the Rust core: a file that already contains CRLF keeps CRLF on every line written back.
function lineEnding(markdown: string): string {
  return markdown.includes('\r\n') ? '\r\n' : '\n';
}
