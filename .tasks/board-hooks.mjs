// Generates native host hook entries. It never edits host settings or enables hooks.
// The inline bridge locates the board from the host payload, independent of shell cwd.
import process from 'node:process';

const host = process.argv[2];
if (!['codex', 'claude'].includes(host)) {
  console.error('Usage: node .tasks/board-hooks.mjs codex|claude');
  process.exit(1);
}
const bridge = String.raw`
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
let raw = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => { raw += chunk; });
process.stdin.on('end', () => {
  let input;
  try { input = JSON.parse(raw); } catch { process.exit(0); }
  if (typeof input.cwd !== 'string' || !path.isAbsolute(input.cwd)) process.exit(0);
  let dir = input.cwd;
  for (;;) {
    const board = path.join(dir, '.tasks', 'board-server.mjs');
    if (fs.existsSync(board) && fs.existsSync(path.join(dir, '.tasks', 'TASKS.md'))) {
      const result = spawnSync(process.execPath, [board, 'hook', process.argv[3]], {
        input: raw, encoding: 'utf8', windowsHide: true, timeout: 15000,
        cwd: dir, env: { ...process.env, SHAUGHV_TASKS_HOOK_HOST: process.argv[2] },
      });
      if (result.stdout) process.stdout.write(result.stdout);
      if (result.stderr) process.stderr.write(result.stderr);
      process.exit(result.error ? 1 : result.status ?? 1);
    }
    const parent = path.dirname(dir);
    if (parent === dir) process.exit(0);
    dir = parent;
  }
});
`;
const inline = bridge.replace(/\r?\n/g, ' ');
const hooks = {};
for (const event of ['SessionStart', 'PostToolUse', 'SubagentStart', 'SubagentStop']) {
  const command = `node -e "${inline}" -- shaughv-tasks-board-v1 ${host} ${event}`;
  // The trailing argument is a stable ownership marker for removal.
  const entry = { hooks: [{ type: 'command', command, timeout: 20 }] };
  if (event === 'PostToolUse') entry.matcher = host === 'claude' ? 'Bash|ExitPlanMode' : 'Bash';
  hooks[event] = [entry];
}
process.stdout.write(JSON.stringify({ hooks }, null, 2) + '\n');
