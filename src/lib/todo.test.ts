import assert from 'node:assert/strict';
import { test } from 'node:test';
import { appendTask, parseTodo, toggleTask } from './todo.ts';

test('toggleTask keeps CRLF line endings', () => {
  const markdown = '# Today\r\n\r\n- [ ] First\r\n- [x] Second\r\n';
  assert.equal(toggleTask(markdown, 'task-2'), '# Today\r\n\r\n- [x] First\r\n- [x] Second\r\n');
});

test('toggleTask keeps LF line endings', () => {
  assert.equal(toggleTask('- [ ] First\n- [x] Second\n', 'task-1'), '- [ ] First\n- [ ] Second\n');
});

test('appendTask keeps CRLF line endings', () => {
  assert.equal(appendTask('- [ ] Start\r\n', '  New task  '), '- [ ] Start\r\n- [ ] New task\r\n');
  assert.equal(appendTask('- [ ] Start', 'New task'), '- [ ] Start\n- [ ] New task\n');
  assert.equal(appendTask('# A\r\n- [ ] Start', 'New task'), '# A\r\n- [ ] Start\r\n- [ ] New task\r\n');
});

test('parseTodo still reads CRLF files', () => {
  assert.deepEqual(parseTodo('- [ ] First\r\n').map(task => task.title), ['First']);
});

type SyntaxCase = { line: string; task: { checked: boolean; title: string } | null };

test('parseTodo follows the shared task syntax table', async () => {
  const { readFile } = await import('node:fs/promises');
  const table = JSON.parse(
    await readFile(new URL('../../crates/taskhud-core/fixtures/task-syntax.json', import.meta.url), 'utf-8'),
  ) as { cases: SyntaxCase[] };
  for (const { line, task } of table.cases) {
    const parsed = parseTodo(line).map(({ checked, title }) => ({ checked, title }));
    assert.deepEqual(parsed, task ? [task] : [], JSON.stringify(line));
  }
});

test('toggleTask unchecks an upper-case [X]', () => {
  assert.equal(toggleTask('- [X] Done\n', 'task-0'), '- [ ] Done\n');
});
