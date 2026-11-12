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
