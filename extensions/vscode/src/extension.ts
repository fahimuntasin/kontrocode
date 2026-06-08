import * as vscode from 'vscode';
import { spawn, ChildProcess } from 'child_process';

let agentProcess: ChildProcess | null = null;
let outputChannel: vscode.OutputChannel;

export function activate(context: vscode.ExtensionContext) {
    outputChannel = vscode.OutputChannel.from("KontroCode", { log: true });
    outputChannel.appendLine("KontroCode activated");

    const askCmd = vscode.commands.registerCommand('kontrocode.ask', async () => {
        await handleAsk();
    });

    const reviewCmd = vscode.commands.registerCommand('kontrocode.review', async () => {
        await handleReview();
    });

    const explainCmd = vscode.commands.registerCommand('kontrocode.explain', async () => {
        await handleExplain();
    });

    const testCmd = vscode.commands.registerCommand('kontrocode.generateTests', async () => {
        await handleGenerateTests();
    });

    const fixCmd = vscode.commands.registerCommand('kontrocode.fixIssues', async () => {
        await handleFix();
    });

    context.subscriptions.push(askCmd, reviewCmd, explainCmd, testCmd, fixCmd);

    startAgent();
}

async function handleAsk() {
    const input = await vscode.window.showInputBox({
        prompt: 'What do you want KontroCode to do?',
        placeHolder: 'Build me a Flutter auth screen...'
    });
    if (!input) return;
    await sendToAgent({ method: 'ask', params: { prompt: input } });
}

async function handleReview() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;
    const doc = editor.document;
    const code = doc.getText();
    await sendToAgent({ method: 'review', params: { code, language: doc.languageId } });
}

async function handleExplain() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;
    const selection = editor.selection;
    const code = editor.document.getText(selection.isEmpty ? undefined : selection);
    await sendToAgent({ method: 'explain', params: { code } });
}

async function handleGenerateTests() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;
    const code = editor.document.getText();
    await sendToAgent({ method: 'generate_tests', params: { code } });
}

async function handleFix() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;
    const code = editor.document.getText();
    await sendToAgent({ method: 'fix', params: { code } });
}

function startAgent() {
    const config = vscode.workspace.getConfiguration('kontrocode');
    const agentPath = config.get<string>('agentPath') || 'kontrocode-agent';

    agentProcess = spawn(agentPath, ['acp'], {
        stdio: ['pipe', 'pipe', 'pipe']
    });

    agentProcess.stdout?.on('data', (data: Buffer) => {
        try {
            const response = JSON.parse(data.toString());
            handleResponse(response);
        } catch {
            outputChannel.appendLine(`[agent]: ${data.toString().trim()}`);
        }
    });

    agentProcess.stderr?.on('data', (data: Buffer) => {
        outputChannel.appendLine(`[stderr]: ${data.toString().trim()}`);
    });

    agentProcess.on('exit', (code) => {
        outputChannel.appendLine(`Agent exited with code ${code}`);
        agentProcess = null;
    });
}

function sendToAgent(message: any) {
    if (!agentProcess || agentProcess.killed) {
        vscode.window.showErrorMessage('KontroCode agent is not running');
        return;
    }
    const json = JSON.stringify(message) + '\n';
    agentProcess.stdin?.write(json);
    outputChannel.appendLine(`[send]: ${json.trim()}`);
}

function handleResponse(response: any) {
    outputChannel.appendLine(`[response]: ${JSON.stringify(response)}`);
    if (response.type === 'completion') {
        showInlineCompletion(response.text);
    }
}

function showInlineCompletion(text: string) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;
    vscode.window.showInformationMessage(`KontroCode: ${text.substring(0, 100)}...`);
}

export function deactivate() {
    agentProcess?.kill();
    outputChannel?.dispose();
}
