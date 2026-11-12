import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';

const executable = process.argv[2];
if (!executable) throw new Error('Usage: node scripts/functions-smoke.mjs <taskhud-functions-executable>');

const root = await mkdtemp(join(tmpdir(), 'taskhud-functions-smoke-'));
const projectRoot = join(root, 'project');
const configPath = join(root, 'config.json');
await mkdir(projectRoot);
await writeFile(
  join(projectRoot, 'TODO.md'),
  '# Smoke\n\n- [ ] Read the sheet\n- [ ] Make a summary sheet\n- [x] Old exercise\n',
);
await writeFile(configPath, `${JSON.stringify({
  activeProjectId: 'smoke',
  projects: [{ id: 'smoke', name: 'Smoke project', root: projectRoot }],
})}\n`);

const child = spawn(executable, ['--config', configPath], {
  stdio: ['pipe', 'pipe', 'pipe'],
  windowsHide: true,
});

let buffer = '';
const pending = [];
child.stdout.setEncoding('utf8');
child.stdout.on('data', chunk => {
  buffer += chunk;
  for (;;) {
    const newline = buffer.indexOf('\n');
    if (newline < 0) break;
    const line = buffer.slice(0, newline).trim();
    buffer = buffer.slice(newline + 1);
    if (line && pending.length) pending.shift()(JSON.parse(line));
  }
});

function call(functionName, argumentsValue) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error(`Timeout sur ${functionName}`)), 5000);
    pending.push(message => {
      clearTimeout(timeout);
      resolve(message);
    });
    child.stdin.write(`${JSON.stringify({ function: functionName, arguments: argumentsValue })}\n`);
  });
}

function result(response) {
  if (!response.ok) throw new Error(response.error);
  return response.result;
}

try {
  const tasks = result(await call('get_tasks', { projectId: 'smoke' }));
  if (tasks.tasks.length !== 3 || tasks.content !== undefined) throw new Error('get_tasks must be compact');

  const proposal = result(await call('propose_tasks', {
    project: 'Smoke project',
    reason: 'Smoke functions',
    tasks: ['Faire un exercice'],
  }));
  const proposals = result(await call('list_proposals', { projectId: 'smoke' }));
  if (proposals.length !== 1 || proposals[0].id !== proposal.id) throw new Error('proposition absente');

  const allOperations = result(await call('propose_task_changes', {
    projectId: 'smoke',
    baseVersion: tasks.version,
    reason: 'Smoke test of the four operations',
    changes: [
      { operation: 'add', title: 'Do a review exercise', afterTaskId: null },
      { operation: 'update', taskId: 'task-2', title: 'Reread the sheet', checked: null },
      { operation: 'update', taskId: 'task-3', title: null, checked: true },
      { operation: 'delete', taskId: 'task-4' },
    ],
  }));
  const allProposals = result(await call('list_proposals', { projectId: 'smoke' }));
  if (allProposals.length !== 2 || allProposals.some(item => item.id !== proposal.id && item.id !== allOperations.id)) {
    throw new Error('the multi-operation proposal is missing');
  }
  const detailed = result(await call('get_tasks_with_content', { projectId: 'smoke' }));
  if (!detailed.content.includes('Read the sheet')) throw new Error('explicit content missing');
  if ((await readFile(join(projectRoot, 'TODO.md'), 'utf8')).includes('Do a review exercise')) {
    throw new Error('the connector modified TODO.md without a human decision');
  }
  console.log(JSON.stringify({ pass: true, proposalId: proposal.id, allOperationsId: allOperations.id }));
} finally {
  child.kill();
  await rm(root, { recursive: true, force: true });
}
