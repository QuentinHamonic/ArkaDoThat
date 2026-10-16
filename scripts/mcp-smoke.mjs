import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';

const executable = process.argv[2];
if (!executable) {
  throw new Error('Usage: node scripts/mcp-smoke.mjs <taskhud-mcp-executable>');
}

const root = await mkdtemp(join(tmpdir(), 'taskhud-mcp-smoke-'));
const projectRoot = join(root, 'project');
const configPath = join(root, 'config.json');
await writeFile(join(root, 'placeholder'), '');
await mkdir(projectRoot);
await writeFile(join(projectRoot, 'TODO.md'), '# Smoke\n\n- [ ] Vérifier MCP\n');
await writeFile(
  configPath,
  `${JSON.stringify({
    activeProjectId: 'smoke',
    projects: [{ id: 'smoke', name: 'Smoke project', root: projectRoot }],
  })}\n`,
);

const child = spawn(executable, ['--config', configPath], {
  stdio: ['pipe', 'pipe', 'pipe'],
  windowsHide: true,
});

let buffer = '';
const pending = new Map();
let nextId = 1;
child.stdout.setEncoding('utf8');
child.stdout.on('data', chunk => {
  buffer += chunk;
  for (;;) {
    const newline = buffer.indexOf('\n');
    if (newline < 0) break;
    const line = buffer.slice(0, newline).trim();
    buffer = buffer.slice(newline + 1);
    if (!line) continue;
    const message = JSON.parse(line);
    if (message.id !== undefined && pending.has(message.id)) {
      pending.get(message.id)(message);
      pending.delete(message.id);
    }
  }
});

const stderr = [];
child.stderr.setEncoding('utf8');
child.stderr.on('data', chunk => stderr.push(chunk));

function request(method, params) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`Timeout MCP sur ${method}`));
    }, 5000);
    pending.set(id, message => {
      clearTimeout(timeout);
      resolve(message);
    });
    child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`);
  });
}

function textResult(response) {
  if (response.error) throw new Error(JSON.stringify(response.error));
  return JSON.parse(response.result.content[0].text);
}

try {
  const initialized = await request('initialize', {
    protocolVersion: '2025-11-25',
    capabilities: {},
    clientInfo: { name: 'taskhud-smoke', version: '0.1.0' },
  });
  if (!initialized.result?.serverInfo) throw new Error('Réponse initialize incomplète');
  child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' })}\n`);

  const tools = (await request('tools/list', {})).result?.tools ?? [];
  const toolNames = tools.map(tool => tool.name);
  const requiredTools = [
    'list_projects',
    'get_project_tasks',
    'get_active_project',
    'propose_task_changes',
    'list_proposals',
    'get_proposal',
  ];
  for (const name of requiredTools) {
    if (!toolNames.includes(name)) throw new Error(`Outil MCP absent: ${name}`);
  }

  const projects = textResult(await request('tools/call', {
    name: 'list_projects',
    arguments: {},
  }));
  if (projects[0]?.id !== 'smoke' || projects[0]?.todoExists !== true) {
    throw new Error('list_projects ne retourne pas le projet smoke attendu');
  }

  const snapshot = textResult(await request('tools/call', {
    name: 'get_project_tasks',
    arguments: { projectId: 'smoke' },
  }));
  const proposalArgs = {
    projectId: 'smoke',
    baseVersion: snapshot.version,
    reason: 'Smoke test',
    changes: [{ operation: 'add', title: 'Vérifier la déduplication', afterTaskId: null }],
  };
  const firstProposal = textResult(await request('tools/call', {
    name: 'propose_task_changes',
    arguments: proposalArgs,
  }));
  const duplicateProposal = textResult(await request('tools/call', {
    name: 'propose_task_changes',
    arguments: proposalArgs,
  }));
  if (firstProposal.id !== duplicateProposal.id) throw new Error('Déduplication MCP absente');

  const proposals = textResult(await request('tools/call', {
    name: 'list_proposals',
    arguments: { projectId: 'smoke' },
  }));
  if (proposals.length !== 1) throw new Error('La proposition smoke n’est pas persistée une seule fois');
  if ((await readFile(join(projectRoot, 'TODO.md'), 'utf8')).includes('Vérifier la déduplication')) {
    throw new Error('Le MCP a modifié TODO.md sans validation humaine');
  }

  console.log(JSON.stringify({ pass: true, tools: toolNames, proposalId: firstProposal.id }));
} finally {
  child.kill();
  await rm(root, { recursive: true, force: true });
}
