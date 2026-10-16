export type Task = {
  id: string;
  title: string;
  checked: boolean;
  lineIndex: number;
};

export type Proposal = {
  operation: 'add' | 'edit' | 'delete';
  title: string;
  before?: string;
  after?: string;
  taskId?: string;
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
  return lines.join('\n');
}

export function appendTask(markdown: string, title: string): string {
  const cleanTitle = title.trim();
  if (!cleanTitle) return markdown;

  const separator = markdown.endsWith('\n') || markdown.length === 0 ? '' : '\n';
  return `${markdown}${separator}- [ ] ${cleanTitle}\n`;
}

export function applyProposal(markdown: string, proposal: Proposal): string {
  if (proposal.operation === 'add') return appendTask(markdown, proposal.after ?? proposal.title);

  if (!proposal.taskId || proposal.before === undefined || proposal.after === undefined) {
    throw new Error('La proposition est incomplète.');
  }

  const task = parseTodo(markdown).find(candidate => candidate.id === proposal.taskId);
  if (!task || task.title !== proposal.before) {
    throw new Error('Le fichier a changé depuis la proposition.');
  }

  const lines = markdown.split(/\r?\n/);
  if (proposal.operation === 'edit') {
    lines[task.lineIndex] = lines[task.lineIndex].replace(task.title, proposal.after);
  } else {
    lines.splice(task.lineIndex, 1);
  }
  return lines.join('\n');
}
