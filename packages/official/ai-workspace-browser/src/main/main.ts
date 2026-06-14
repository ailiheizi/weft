import { app, BaseWindow, WebContentsView } from 'electron';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { BrowserRuntime } from './browser-runtime.js';
import { registerBrowserIpc } from './ipc.js';

const __dirname = fileURLToPath(new URL('.', import.meta.url));

async function createWindow(): Promise<void> {
  const window = new BaseWindow({
    width: 1440,
    height: 920,
    minWidth: 900,
    minHeight: 640,
    title: 'AI Workspace Browser'
  });

  const rendererView = new WebContentsView({
    webPreferences: {
      preload: join(__dirname, '../preload/preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false
    }
  });

  window.contentView.addChildView(rendererView);
  rendererView.setBounds({ x: 0, y: 0, width: 1440, height: 920 });
  window.on('resize', () => {
    const bounds = window.getBounds();
    rendererView.setBounds({ x: 0, y: 0, width: bounds.width, height: bounds.height });
  });

  const runtime = new BrowserRuntime(window, rendererView.webContents);
  registerBrowserIpc(runtime);
  window.on('closed', () => runtime.dispose());

  if (process.env.VITE_DEV_SERVER_URL) {
    await rendererView.webContents.loadURL(process.env.VITE_DEV_SERVER_URL);
  } else {
    await rendererView.webContents.loadFile(join(__dirname, '../../renderer/index.html'));
  }

  if (process.env.AI_WORKSPACE_BROWSER_SELF_TEST === '1') {
    await runtime.runFixtureSelfTest();
    app.quit();
  } else {
    await runtime.initialize();
  }
}

app.whenReady().then(() => {
  void createWindow().catch((error) => {
    console.error(error);
    app.exit(1);
  });
  app.on('activate', () => {
    if (BaseWindow.getAllWindows().length === 0) void createWindow().catch((error) => {
      console.error(error);
      app.exit(1);
    });
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});
