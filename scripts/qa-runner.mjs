import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const args = process.argv.slice(2);
const phase = args.includes('--phase=design') ? 'design' : 'verify';
const taskIndex = args.indexOf('--task');
const taskPath = taskIndex >= 0 ? resolve(args[taskIndex + 1]) : null;
const outputIndex = args.indexOf('--output');
const outputPath = outputIndex >= 0 ? resolve(args[outputIndex + 1]) : join(root, 'qa', `report-${phase}.json`);
const commitIndex = args.indexOf('--commit');
const commit = commitIndex >= 0 ? args[commitIndex + 1] : 'working-tree';

const prompt = await readFile(join(root, 'qa', 'QA-PROMPT.md'), 'utf8');
const spec = await readFile(join(root, 'docs', 'SPEC.md'), 'utf8');
const task = taskPath ? await readFile(taskPath, 'utf8') : 'Préparer les scénarios QA indépendants du MVP décrit dans docs/SPEC.md.';
const request = `${prompt}\n\nPhase: ${phase}\nRéférence Git: ${commit}\n\nTâche:\n${task}\n\nSpécification:\n${spec}`;

const child = spawn('claude.exe', [
  '--print',
  '--no-session-persistence',
  '--output-format', 'json',
  '--allowedTools=Read,Glob,Grep,Bash(git:*)',
  request,
], { cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });

let stdout = '';
let stderr = '';
child.stdout.setEncoding('utf8');
child.stderr.setEncoding('utf8');
child.stdout.on('data', chunk => { stdout += chunk; });
child.stderr.on('data', chunk => { stderr += chunk; });

const exitCode = await new Promise(resolveExit => child.on('close', resolveExit));
await mkdir(join(root, 'qa'), { recursive: true });
const report = {
  runner: 'taskhud-qa-runner-v1',
  phase,
  commit,
  exitCode,
  executedAt: new Date().toISOString(),
  claude: stdout.trim(),
  stderr: stderr.trim(),
  verdict: exitCode === 0 && stdout.trim() ? 'REVIEW_REQUIRED' : 'BLOCKED',
};
await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(JSON.stringify({ output: outputPath, exitCode, verdict: report.verdict }));
process.exitCode = exitCode === 0 && stdout.trim() ? 0 : 1;
