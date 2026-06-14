export interface AiWorkspaceBrowserPluginHost {
  openBrowserWindow?: () => Promise<void> | void;
}

export interface AiWorkspaceBrowserPackage {
  id: string;
  name: string;
  version: string;
  capabilities: string[];
  contextPolicy: {
    defaultScreenshots: false;
    browserNativeContextOnly: true;
  };
  activate(host: AiWorkspaceBrowserPluginHost): Promise<void>;
}

export const aiWorkspaceBrowserPlugin: AiWorkspaceBrowserPackage = {
  id: 'weft.ai-workspace-browser',
  name: 'AI Workspace Browser',
  version: '0.1.0',
  capabilities: [
    'browser.window',
    'browser.tabs',
    'browser.context.dom',
    'browser.context.accessibility',
    'browser.actions.grounded',
    'browser.memory.workspace'
  ],
  contextPolicy: {
    defaultScreenshots: false,
    browserNativeContextOnly: true
  },
  async activate(host) {
    if (host.openBrowserWindow) await host.openBrowserWindow();
  }
};

export function createAiWorkspaceBrowserPlugin(): AiWorkspaceBrowserPackage {
  return aiWorkspaceBrowserPlugin;
}

export default aiWorkspaceBrowserPlugin;
