const { spawn } = require('node:child_process');

const vite = spawn('npm', ['run', 'dev'], { stdio: 'inherit', shell: true });

setTimeout(() => {
  const electron = spawn('npx', ['electron', 'dist/src/main/main.js'], { stdio: 'inherit', shell: true });
  electron.on('exit', (code) => process.exit(code ?? 0));
}, 3000);

vite.on('exit', (code) => process.exit(code ?? 0));
