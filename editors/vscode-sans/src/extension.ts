import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('sans');
    const lspPath = config.get<string>('lspPath', 'sans-lsp');

    const serverOptions: ServerOptions = {
        run: { command: lspPath, args: [] },
        debug: { command: lspPath, args: [] }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'sans' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.sans')
        }
    };

    client = new LanguageClient(
        'sans-lsp',
        'Sans Language Server',
        serverOptions,
        clientOptions
    );

    client.start().catch((err: Error) => {
        const msg = err?.message || String(err);
        if (msg.includes('ENOENT') || msg.includes('not found')) {
            vscode.window.showErrorMessage(
                `Sans Language Server not found at "${lspPath}". ` +
                'Install it or set "sans.lspPath" in settings.'
            );
        } else {
            vscode.window.showErrorMessage(
                `Sans Language Server failed to start: ${msg}`
            );
        }
        client = undefined;
    });
    context.subscriptions.push({
        dispose: () => { if (client) { client.stop(); } }
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) { return undefined; }
    return client.stop();
}
