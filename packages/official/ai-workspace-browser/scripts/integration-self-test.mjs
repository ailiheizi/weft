import { access } from 'node:fs/promises';
import { constants } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';

const root = process.cwd();
const mainPath = join(root, 'dist/src/main/main.js');

await access(mainPath, constants.F_OK);

const result = spawnSync('npm', ['exec', 'electron', '--', mainPath], {
  cwd: root,
  env: { ...process.env, AI_WORKSPACE_BROWSER_SELF_TEST: '1' },
  stdio: 'inherit',
  shell: true
});

process.exit(result.status ?? 1);
