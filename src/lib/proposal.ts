import type { Task } from './todo.ts';

export type AgentChange =
  | { operation: 'add'; title: string; afterTaskId?: string | null }
  | { operation: 'update'; taskId: string; title?: string | null; checked?: boolean | null }
  | { operation: 'delete'; taskId: string };

export type ChangeKind = 'add' | 'delete' | 'rename' | 'check' | 'uncheck' | 'invalid';

// What the user must see to decide: the text in the list now (`before`, null for an add)
// and the text after acceptance (`after`, null when the title does not change or the task goes away).
export type ChangeView = {
  kind: ChangeKind;
  before: string | null;
  after: string | null;
  // Proposed checked state, or null when the change does not set it.
  checked: boolean | null;
  // False when the targeted task is not in the displayed list: `before` is then its raw id.
  known: boolean;
};

export function describeChange(change: AgentChange, tasks: Task[]): ChangeView {
  if (change.operation === 'add') {
    return { kind: 'add', before: null, after: change.title, checked: null, known: true };
  }

  const task = tasks.find(candidate => candidate.id === change.taskId);
  const before = task?.title ?? change.taskId;
  const known = Boolean(task);
  if (change.operation === 'delete') {
    return { kind: 'delete', before, after: null, checked: null, known };
  }

  const checked = change.checked ?? null;
  if (change.title == null && checked === null) {
    return { kind: 'invalid', before, after: null, checked: null, known };
  }
  const renamed = change.title != null && change.title !== task?.title;
  const checkedChanged = checked !== null && checked !== task?.checked;
  if (!renamed && !checkedChanged) {
    return { kind: 'invalid', before, after: null, checked: null, known };
  }
  const kind: ChangeKind = renamed ? 'rename' : checked ? 'check' : 'uncheck';
  return { kind, before, after: renamed ? change.title ?? null : null, checked, known };
}
