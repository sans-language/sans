import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

const SHELL_METACHARACTERS = /[|;&$`()"'<>!#*?\[\]{}~\n\r]/;

function validateLspPath(lspPath: string): string | undefined {
    if (SHELL_METACHARACTERS.test(lspPath)) {
        vscode.window.showErrorMessage(
            `sans.lspPath contains invalid characters: "${lspPath}". Path must not contain shell metacharacters.`
        );
        return undefined;
    }
    if (lspPath.includes(' ')) {
        vscode.window.showErrorMessage(
            `sans.lspPath contains spaces (possible command injection): "${lspPath}". Path must point to a single executable.`
        );
        return undefined;
    }
    const resolved = path.resolve(lspPath);
    if (!fs.existsSync(resolved) && !fs.existsSync(lspPath)) {
        vscode.window.showErrorMessage(
            `sans.lspPath does not exist: "${lspPath}". Please set a valid path to the Sans language server.`
        );
        return undefined;
    }
    return lspPath;
}

export function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('sans');
    const lspPath = config.get<string>('lspPath', 'sans-lsp');

    const validatedPath = validateLspPath(lspPath);
    if (!validatedPath) {
        return;
    }

    const serverOptions: ServerOptions = {
        run: { command: validatedPath, args: [] },
        debug: { command: validatedPath, args: [] }
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

    client.start();
    context.subscriptions.push({
        dispose: () => { if (client) { client.stop(); } }
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) { return undefined; }
    return client.stop();
}
