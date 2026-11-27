import assert from 'node:assert/strict';
import { test } from 'node:test';
import { describeChange } from './proposal.ts';
import { parseTodo } from './todo.ts';

const tasks = parseTodo('- [ ] Review the diff\n- [x] Update STATE.md\n');

test('a rename shows the current title before the proposed one', () => {
  assert.deepEqual(describeChange({ operation: 'update', taskId: 'task-0', title: 'Review the full diff' }, tasks), {
    kind: 'rename',
    before: 'Review the diff',
    after: 'Review the full diff',
    checked: null,
    known: true,
  });
});

test('checking and unchecking are two different kinds', () => {
  assert.equal(describeChange({ operation: 'update', taskId: 'task-0', checked: true }, tasks).kind, 'check');
  assert.equal(describeChange({ operation: 'update', taskId: 'task-1', checked: false }, tasks).kind, 'uncheck');
});

test('a state change keeps the title unchanged', () => {
  assert.deepEqual(describeChange({ operation: 'update', taskId: 'task-0', checked: true, title: null }, tasks), {
    kind: 'check',
    before: 'Review the diff',
    after: null,
    checked: true,
    known: true,
  });
});

test('a rename that also changes the state reports both', () => {
  const view = describeChange({ operation: 'update', taskId: 'task-1', title: 'Update STATE.md again', checked: false }, tasks);
  assert.equal(view.kind, 'rename');
  assert.equal(view.before, 'Update STATE.md');
  assert.equal(view.after, 'Update STATE.md again');
  assert.equal(view.checked, false);
});

test('a title identical to the current one is not a rename', () => {
  assert.equal(describeChange({ operation: 'update', taskId: 'task-0', title: 'Review the diff', checked: true }, tasks).kind, 'check');
});

test('add and delete show only the side that exists', () => {
  assert.deepEqual(describeChange({ operation: 'add', title: 'Document JSONL' }, tasks), {
    kind: 'add', before: null, after: 'Document JSONL', checked: null, known: true,
  });
  assert.deepEqual(describeChange({ operation: 'delete', taskId: 'task-1' }, tasks), {
    kind: 'delete', before: 'Update STATE.md', after: null, checked: null, known: true,
  });
});

test('a change on a task absent from the list is flagged as unknown', () => {
  const view = describeChange({ operation: 'delete', taskId: 'task-9' }, tasks);
  assert.equal(view.known, false);
  assert.equal(view.before, 'task-9');
});

test('an empty update is flagged as invalid', () => {
  const view = describeChange({ operation: 'update', taskId: 'task-0' }, tasks);
  assert.equal(view.kind, 'invalid');
});

test('a no-op title or state change is flagged as invalid', () => {
  assert.equal(describeChange({ operation: 'update', taskId: 'task-0', title: 'Review the diff' }, tasks).kind, 'invalid');
  assert.equal(describeChange({ operation: 'update', taskId: 'task-1', checked: true }, tasks).kind, 'invalid');
});
